use crate::swap_protocols::{
    halight::{data, Accepted, Cancelled, Opened, Params, Settled},
    rfc003::{Secret, SecretHash},
};
use anyhow::{Context, Error};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    StatusCode, Url,
};
use serde::Deserialize;
use std::{
    convert::{TryFrom, TryInto},
    io::Read,
    path::PathBuf,
    time::Duration,
};

/// Invoice states.  These mirror the invoice states used by lnd.
// ref: https://api.lightning.community/#invoicestate
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, strum_macros::Display)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum InvoiceState {
    Open,
    Settled,
    Cancelled,
    Accepted,
}

/// Payment status.  These mirror the payment status' used by lnd.
// ref: https://api.lightning.community/#paymentstatus
#[derive(Copy, Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum PaymentStatus {
    Unknown,
    InFlight,
    Succeeded,
    Failed,
}

// TODO: don't deserialize fields we are not using
#[derive(Debug, Deserialize)]
struct Invoice {
    pub value: String,
    pub value_msat: String,
    pub amt_paid_sat: String,
    pub amt_paid_msat: String,
    pub expiry: String,
    pub cltv_expiry: String,
    pub state: InvoiceState,
    pub r_preimage: Option<String>, // TODO: this is base64 and not hex
}

#[derive(Clone, Debug, Deserialize)]
struct PaymentsResponse {
    payments: Option<Vec<Payment>>,
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
    pub macaroon_path: PathBuf,
}

#[derive(Clone, Debug)]
enum LazyFile<T> {
    Path(PathBuf),
    Inner(T),
}

impl<T> LazyFile<T>
where
    T: TryFrom<Vec<u8>, Error = Error>,
{
    pub fn new(path: PathBuf) -> Self {
        Self::Path(path)
    }

    pub fn read(self) -> Result<Self, Error> {
        match self {
            LazyFile::Inner(_) => Ok(self),
            LazyFile::Path(path) => {
                let mut buf = Vec::new();
                std::fs::File::open(path)?.read_to_end(&mut buf)?;
                let inner = buf.try_into()?;
                Ok(LazyFile::Inner(inner))
            }
        }
    }

    pub fn inner(&self) -> Result<&T, Error> {
        match self {
            LazyFile::Path(_) => Err(anyhow::anyhow!("File was not read.")),
            LazyFile::Inner(inner) => Ok(inner),
        }
    }
}

#[derive(Clone, Debug)]
struct Certificate(reqwest::Certificate);

impl TryFrom<Vec<u8>> for Certificate {
    type Error = Error;
    fn try_from(buf: Vec<u8>) -> Result<Self, Error> {
        Ok(Certificate(reqwest::Certificate::from_pem(&buf)?))
    }
}

#[derive(Clone, Debug)]
/// The string is hex encoded
struct Macaroon(String);

impl TryFrom<Vec<u8>> for Macaroon {
    type Error = Error;
    fn try_from(buf: Vec<u8>) -> Result<Self, Error> {
        Ok(Macaroon(hex::encode(buf)))
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
    certificate: LazyFile<Certificate>,
    macaroon: LazyFile<Macaroon>,
}

impl From<LndConnectorParams> for LndConnectorAsSender {
    fn from(params: LndConnectorParams) -> Self {
        Self {
            lnd_url: params.lnd_url,
            retry_interval_ms: params.retry_interval_ms,
            certificate: LazyFile::<Certificate>::new(params.certificate_path),
            macaroon: LazyFile::<Macaroon>::new(params.macaroon_path),
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

    pub fn read_macaroon(self) -> Result<Self, Error> {
        Ok(Self {
            macaroon: self.macaroon.read()?,
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
        let response = client(self.certificate.inner()?, self.macaroon.inner()?)?
            .get(self.payment_url())
            .send()
            .await?
            .json::<PaymentsResponse>()
            .await?;
        let payment = response
            .payments
            .unwrap_or_default()
            .into_iter()
            .find(|payment| payment.payment_hash == secret_hash && payment.status == status);

        Ok(payment)
    }
}

#[async_trait::async_trait]
impl Opened for LndConnectorAsSender {
    async fn opened(&self, _params: Params) -> Result<data::Opened, Error> {
        // At this stage there is no way for the sender to know when the invoice is
        // added on receiver's side.
        Ok(data::Opened)
    }
}

#[async_trait::async_trait]
impl Accepted for LndConnectorAsSender {
    async fn accepted(&self, params: Params) -> Result<data::Accepted, Error> {
        // No validation of the parameters because once the payment has been
        // sent the sender cannot cancel it.
        while self
            .find_payment(params.secret_hash, PaymentStatus::InFlight)
            .await?
            .is_none()
        {
            tokio::time::delay_for(Duration::from_millis(self.retry_interval_ms)).await;
        }

        Ok(data::Accepted)
    }
}

#[async_trait::async_trait]
impl Settled for LndConnectorAsSender {
    async fn settled(&self, params: Params) -> Result<data::Settled, Error> {
        let payment = loop {
            match self
                .find_payment(params.secret_hash, PaymentStatus::Succeeded)
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
        Ok(data::Settled { secret })
    }
}

#[async_trait::async_trait]
impl Cancelled for LndConnectorAsSender {
    async fn cancelled(&self, params: Params) -> Result<data::Cancelled, Error> {
        while self
            .find_payment(params.secret_hash, PaymentStatus::Failed)
            .await?
            .is_none()
        {
            tokio::time::delay_for(Duration::from_millis(self.retry_interval_ms)).await;
        }

        Ok(data::Cancelled)
    }
}

/// LND connector for connecting to an LND node when receiving a lightning
/// payment.
///
/// When connecting to LND as the receiver all state decisions can be made based
/// on the invoice state.  Since as the receiver, we add the invoice we have
/// access to its state.
#[derive(Clone, Debug)]
pub struct LndConnectorAsReceiver {
    lnd_url: Url,
    retry_interval_ms: u64,
    certificate: LazyFile<Certificate>,
    macaroon: LazyFile<Macaroon>,
}

impl From<LndConnectorParams> for LndConnectorAsReceiver {
    fn from(params: LndConnectorParams) -> Self {
        Self {
            lnd_url: params.lnd_url,
            retry_interval_ms: params.retry_interval_ms,
            certificate: LazyFile::<Certificate>::new(params.certificate_path),
            macaroon: LazyFile::<Macaroon>::new(params.macaroon_path),
        }
    }
}

impl LndConnectorAsReceiver {
    pub fn read_certificate(self) -> Result<Self, Error> {
        Ok(Self {
            certificate: self.certificate.read()?,
            ..self
        })
    }

    pub fn read_macaroon(self) -> Result<Self, Error> {
        Ok(Self {
            macaroon: self.macaroon.read()?,
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

    #[tracing::instrument(level = "debug", skip(self))]
    async fn find_invoice(
        &self,
        secret_hash: SecretHash,
        expected_state: InvoiceState,
    ) -> Result<Option<Invoice>, Error> {
        let response = client(self.certificate.inner()?, self.macaroon.inner()?)?
            .get(self.invoice_url(secret_hash)?)
            .send()
            .await?;

        if response.status() == StatusCode::NOT_FOUND {
            tracing::debug!("invoice not found");
            return Ok(None);
        }

        // Need to shortcut here until https://github.com/hyperium/hyper/issues/2171 or https://github.com/lightningnetwork/lnd/issues/4135 is resolved
        if response.status() == StatusCode::INTERNAL_SERVER_ERROR {
            return Ok(None);
        }

        if !response.status().is_success() {
            let status_code = response.status();
            let lnd_error = response
                .json::<LndError>()
                .await
                // yes we can fail while we already encoundered an error ...
                .with_context(|| format!("encountered {} while fetching invoice but couldn't deserialize error response 🙄", status_code))?;

            return Err(lnd_error.into());
        }

        let invoice = response
            .json::<Invoice>()
            .await
            .context("failed to deserialize response as invoice")?;

        if invoice.state == expected_state {
            Ok(Some(invoice))
        } else {
            tracing::debug!("invoice exists but is in state {}", invoice.state);
            Ok(None)
        }
    }
}

#[derive(Deserialize, Debug, thiserror::Error)]
#[error("{message}")]
struct LndError {
    error: String,
    message: String,
    code: u32,
}

#[async_trait::async_trait]
impl Opened for LndConnectorAsReceiver {
    async fn opened(&self, params: Params) -> Result<data::Opened, Error> {
        // Do we want to validate that the user used the correct swap parameters
        // when adding the invoice?
        while self
            .find_invoice(params.secret_hash, InvoiceState::Open)
            .await?
            .is_none()
        {
            tokio::time::delay_for(Duration::from_millis(self.retry_interval_ms)).await;
        }

        Ok(data::Opened)
    }
}

#[async_trait::async_trait]
impl Accepted for LndConnectorAsReceiver {
    async fn accepted(&self, params: Params) -> Result<data::Accepted, Error> {
        // Validation that sender payed the correct invoice is provided by LND.
        // Since the sender uses the params to make the payment (as apposed to
        // the invoice) LND guarantees that the params match the invoice when
        // updating the invoice status.
        while self
            .find_invoice(params.secret_hash, InvoiceState::Accepted)
            .await?
            .is_none()
        {
            tokio::time::delay_for(Duration::from_millis(self.retry_interval_ms)).await;
        }
        Ok(data::Accepted)
    }
}

#[async_trait::async_trait]
impl Settled for LndConnectorAsReceiver {
    async fn settled(&self, params: Params) -> Result<data::Settled, Error> {
        let invoice = loop {
            match self
                .find_invoice(params.secret_hash, InvoiceState::Settled)
                .await?
            {
                Some(invoice) => break invoice,
                None => tokio::time::delay_for(Duration::from_millis(self.retry_interval_ms)).await,
            }
        };

        let preimage = invoice
            .r_preimage
            .ok_or_else(|| anyhow::anyhow!("settled invoice does not contain preimage?!"))?;

        Ok(data::Settled {
            secret: Secret::from_vec(base64::decode(preimage.as_bytes())?.as_slice())?,
        })
    }
}

#[async_trait::async_trait]
impl Cancelled for LndConnectorAsReceiver {
    async fn cancelled(&self, params: Params) -> Result<data::Cancelled, Error> {
        while self
            .find_invoice(params.secret_hash, InvoiceState::Cancelled)
            .await?
            .is_none()
        {
            tokio::time::delay_for(Duration::from_millis(self.retry_interval_ms)).await;
        }
        Ok(data::Cancelled)
    }
}

fn client(certificate: &Certificate, macaroon: &Macaroon) -> Result<reqwest::Client, Error> {
    let cert = certificate.0.clone();
    let mut default_headers = HeaderMap::with_capacity(1);
    default_headers.insert(
        "Grpc-Metadata-macaroon",
        HeaderValue::from_str(&macaroon.0)?,
    );

    Ok(reqwest::Client::builder()
        .add_root_certificate(cert)
        .default_headers(default_headers)
        .build()?)
}
