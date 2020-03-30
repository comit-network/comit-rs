use crate::swap_protocols::{
    halight::{InvoiceAccepted, InvoiceCancelled, InvoiceOpened, InvoiceSettled, Params, Settled},
    rfc003::{Secret, SecretHash},
};
use anyhow::Error;
use reqwest::{Certificate, StatusCode, Url};
use serde::Deserialize;
use std::{io::Read, path::PathBuf, time::Duration};

#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
enum InvoiceState {
    #[serde(rename = "0")]
    Opened,
    #[serde(rename = "1")]
    Settled,
    #[serde(rename = "2")]
    Cancelled,
    #[serde(rename = "3")]
    Accepted,
}

#[derive(Copy, Clone, Debug, Deserialize, PartialEq)]
enum PaymentStatus {
    #[serde(rename = "0")]
    Unknown,
    #[serde(rename = "1")]
    InFlight,
    #[serde(rename = "2")]
    Succeed,
    #[serde(rename = "3")]
    Failed,
}

#[derive(Debug, Deserialize)]
struct Invoice {
    pub value: Option<String>,
    pub value_msat: Option<String>,
    pub r_hash: SecretHash,
    pub amt_paid_sat: Option<String>,
    pub amt_paid_msat: Option<String>,
    pub settled: bool,
    pub cltv_expiry: String,
    pub state: InvoiceState,
    pub r_preimage: Secret,
}

#[derive(Clone, Debug, Deserialize)]
struct Payment {
    pub value_msat: Option<String>,
    pub payment_preimage: Option<Secret>,
    pub status: PaymentStatus,
    pub payment_hash: SecretHash,
}

#[derive(Clone, Debug)]
pub struct LndConnectorParams {
    pub lnd_url: Url,
    pub retry_interval_ms: u64,
    pub certificate_path: PathBuf,
}

#[derive(Clone, Debug)]
enum LazyCertificate {
    Path(PathBuf),
    Certificate(Certificate),
}

impl LazyCertificate {
    pub fn new(path: PathBuf) -> Self {
        Self::Path(path)
    }

    pub fn read(self) -> Result<Self, Error> {
        use LazyCertificate::*;

        match self {
            Certificate(_) => Ok(self),
            Path(path) => {
                let mut buf = Vec::new();
                std::fs::File::open(path)?.read_to_end(&mut buf)?;
                let certificate = reqwest::Certificate::from_pem(&buf)?;
                Ok(LazyCertificate::Certificate(certificate))
            }
        }
    }

    pub fn certificate(&self) -> Result<&Certificate, Error> {
        use LazyCertificate::*;
        match self {
            Path(_) => Err(anyhow::anyhow!("Certificate was not read.")),
            Certificate(certificate) => Ok(certificate),
        }
    }
}

/// LND connector for connecting to an LND node when sending a lightning
/// payment.
///
/// When connecting to LND as the sender all state decisions must be made based
/// on the payment status.  This is because only the receiver has the invoice,
/// the sender makes payments using the swap parameters.
#[derive(Clone, Debug)]
pub struct LndConnectorAsSender {
    lnd_url: Url,
    retry_interval_ms: u64,
    certificate: LazyCertificate,
}

impl From<LndConnectorParams> for LndConnectorAsSender {
    fn from(params: LndConnectorParams) -> Self {
        Self {
            lnd_url: params.lnd_url,
            retry_interval_ms: params.retry_interval_ms,
            certificate: LazyCertificate::new(params.certificate_path),
        }
    }
}

impl LndConnectorAsSender {
    pub fn read_certificate(self) -> Result<Self, Error> {
        Ok(Self {
            certificate: self.certificate.read()?,
            ..self
        })
    }

    fn payment_url(&self) -> Url {
        self.lnd_url
            .join("/v1/payments?include_incomplete=true")
            .expect("append valid string to url")
    }

    async fn find_payment(
        &self,
        secret_hash: SecretHash,
        status: PaymentStatus,
    ) -> Result<Option<Payment>, Error> {
        let payments = client(self.certificate.certificate()?)?
            .get(self.payment_url())
            .send()
            .await?
            .json::<Vec<Payment>>()
            .await?;
        let payment = payments
            .iter()
            .find(|&payment| payment.payment_hash == secret_hash && payment.status == status);

        Ok(payment.cloned())
    }
}

#[async_trait::async_trait]
impl<L, A, I> InvoiceOpened<L, A, I> for LndConnectorAsSender
where
    L: Send + 'static,
    A: Send + 'static,
    I: Send + 'static,
{
    async fn invoice_opened(&self, _params: Params<L, A, I>) -> Result<(), Error> {
        // At this stage there is no way for the sender to know when the invoice is
        // added on recipient's side.
        Ok(())
    }
}

#[async_trait::async_trait]
impl<L, A, I> InvoiceAccepted<L, A, I> for LndConnectorAsSender
where
    L: Send + 'static,
    A: Send + 'static,
    I: Send + 'static,
{
    async fn invoice_accepted(&self, params: Params<L, A, I>) -> Result<(), Error> {
        // No validation of the parameters because once the payment has been
        // sent the sender cannot cancel it.
        while !self
            .find_payment(params.secret_hash, PaymentStatus::InFlight)
            .await?
            .is_some()
        {
            tokio::time::delay_for(Duration::from_millis(self.retry_interval_ms)).await;
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl<L, A, I> InvoiceSettled<L, A, I> for LndConnectorAsSender
where
    A: Send + 'static,
    L: Send + 'static,
    I: Send + 'static,
{
    async fn invoice_settled(&self, params: Params<L, A, I>) -> Result<Settled, Error> {
        let payment = loop {
            match self
                .find_payment(params.secret_hash, PaymentStatus::Succeed)
                .await?
            {
                Some(payment) => break payment,
                None => {
                    tokio::time::delay_for(Duration::from_millis(self.retry_interval_ms)).await;
                }
            }
        };

        let secret = match payment.payment_preimage {
            Some(secret) => Ok(secret),
            None => Err(anyhow::anyhow!(
                "Pre-image is not present on lnd response for a successful payment: {}",
                params.secret_hash
            )),
        }?;
        Ok(Settled { secret })
    }
}

#[async_trait::async_trait]
impl<L, A, I> InvoiceCancelled<L, A, I> for LndConnectorAsSender
where
    L: Send + 'static,
    A: Send + 'static,
    I: Send + 'static,
{
    async fn invoice_cancelled(&self, params: Params<L, A, I>) -> Result<(), Error> {
        while !self
            .find_payment(params.secret_hash, PaymentStatus::Failed)
            .await?
            .is_some()
        {
            tokio::time::delay_for(Duration::from_millis(self.retry_interval_ms)).await;
        }

        Ok(())
    }
}

/// LND connector for connecting to an LND node when receiving a lightning
/// payment.
///
/// When connecting to LND as the receiver all state decisions can be made based
/// on the invoice state.  Since as the receiver, we add the invoice we have
/// access to its state.
#[derive(Clone, Debug)]
pub struct LndConnectorAsRecipient {
    lnd_url: Url,
    retry_interval_ms: u64,
    certificate: LazyCertificate,
}

impl From<LndConnectorParams> for LndConnectorAsRecipient {
    fn from(params: LndConnectorParams) -> Self {
        Self {
            lnd_url: params.lnd_url,
            retry_interval_ms: params.retry_interval_ms,
            certificate: LazyCertificate::new(params.certificate_path),
        }
    }
}

impl LndConnectorAsRecipient {
    pub fn read_certificate(self) -> Result<Self, Error> {
        Ok(Self {
            certificate: self.certificate.read()?,
            ..self
        })
    }

    fn invoice_url(&self, secret_hash: SecretHash) -> Result<Url, Error> {
        Ok(self
            .lnd_url
            .join("/v1/invoice/")
            .expect("append valid string to url")
            .join(format!("{:x}", secret_hash).as_str())?)
    }

    async fn find_invoice(
        &self,
        secret_hash: SecretHash,
        state: InvoiceState,
    ) -> Result<Option<Invoice>, Error> {
        let invoice = client(self.certificate.certificate()?)?
            .get(self.invoice_url(secret_hash)?)
            .send()
            .await?
            .json::<Invoice>()
            .await?;

        if invoice.state == state {
            Ok(Some(invoice))
        } else {
            Ok(None)
        }
    }
}

#[async_trait::async_trait]
impl<L, A, I> InvoiceOpened<L, A, I> for LndConnectorAsRecipient
where
    L: Send + 'static,
    A: Send + 'static,
    I: Send + 'static,
{
    async fn invoice_opened(&self, params: Params<L, A, I>) -> Result<(), Error> {
        // TODO: Validate Params here (cltv_expiry and amount)
        let mut resp = client(self.certificate.certificate()?)?
            .get(self.invoice_url(params.secret_hash)?)
            .send()
            .await?;

        while resp.status() == StatusCode::NOT_FOUND {
            tokio::time::delay_for(Duration::from_millis(self.retry_interval_ms)).await;
            resp = reqwest::get(self.invoice_url(params.secret_hash)?).await?;
        }
        let _invoice_response = resp.json::<Invoice>().await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl<L, A, I> InvoiceAccepted<L, A, I> for LndConnectorAsRecipient
where
    L: Send + 'static,
    A: Send + 'static,
    I: Send + 'static,
{
    async fn invoice_accepted(&self, params: Params<L, A, I>) -> Result<(), Error> {
        while !self
            .find_invoice(params.secret_hash, InvoiceState::Accepted)
            .await?
            .is_some()
        {
            tokio::time::delay_for(Duration::from_millis(self.retry_interval_ms)).await;
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl<L, A, I> InvoiceSettled<L, A, I> for LndConnectorAsRecipient
where
    L: Send + 'static,
    A: Send + 'static,
    I: Send + 'static,
{
    async fn invoice_settled(&self, params: Params<L, A, I>) -> Result<Settled, Error> {
        let invoice = loop {
            match self
                .find_invoice(params.secret_hash, InvoiceState::Settled)
                .await?
            {
                Some(invoice) => break invoice,
                None => tokio::time::delay_for(Duration::from_millis(self.retry_interval_ms)).await,
            }
        };

        Ok(Settled {
            secret: invoice.r_preimage,
        })
    }
}

#[async_trait::async_trait]
impl<L, A, I> InvoiceCancelled<L, A, I> for LndConnectorAsRecipient
where
    L: Send + 'static,
    A: Send + 'static,
    I: Send + 'static,
{
    async fn invoice_cancelled(&self, params: Params<L, A, I>) -> Result<(), Error> {
        while !self
            .find_invoice(params.secret_hash, InvoiceState::Cancelled)
            .await?
            .is_some()
        {
            tokio::time::delay_for(Duration::from_millis(self.retry_interval_ms)).await;
        }
        Ok(())
    }
}

fn client(certificate: &Certificate) -> Result<reqwest::Client, Error> {
    let cert = certificate.clone();
    Ok(reqwest::Client::builder()
        .add_root_certificate(cert)
        .build()?)
}
