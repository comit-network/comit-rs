use crate::{
    asset,
    halight::{
        Accepted, Cancelled, Opened, Params, Settled, WaitForAccepted, WaitForCancelled,
        WaitForOpened, WaitForSettled,
    },
    RelativeTime, Secret, SecretHash,
};
use anyhow::{bail, Context, Error};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    StatusCode, Url,
};
use serde::{de, export::fmt, Deserialize, Deserializer};
use std::{
    convert::{TryFrom, TryInto},
    fmt::Debug,
    io::Read,
    path::PathBuf,
    time::Duration,
};

/// Invoice states.  These mirror the invoice states used by lnd.
// ref: https://api.lightning.community/#invoicestate
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, strum_macros::Display)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InvoiceState {
    Open,
    Settled,
    Cancelled,
    Accepted,
}

/// Payment status.  These mirror the payment status' used by lnd.
// ref: https://api.lightning.community/#paymentstatus
#[derive(Copy, Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentStatus {
    Unknown,
    InFlight,
    Succeeded,
    Failed,
}

#[derive(Copy, Clone, Debug, thiserror::Error)]
#[error("amounts don't match: expected {expected} but received {received}")]
pub struct AmountMismatch {
    expected: asset::Bitcoin,
    received: asset::Bitcoin,
}

#[derive(Copy, Clone, Debug, thiserror::Error)]
#[error("cltv expiry times don't match: expected {expected} but received {received}")]
pub struct ExpiryMismatch {
    expected: RelativeTime,
    received: RelativeTime,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Invoice {
    #[serde(deserialize_with = "deserialize_amount")]
    pub value: asset::Bitcoin,
    #[serde(deserialize_with = "deserialize_relative_time")]
    pub expiry: RelativeTime,
    #[serde(deserialize_with = "deserialize_relative_time")]
    pub cltv_expiry: RelativeTime,
    pub state: InvoiceState,
    #[serde(deserialize_with = "deserialize_r_preimage")]
    pub r_preimage: Option<[u8; 32]>,
}

impl Invoice {
    fn validate(self, params: &Params) -> anyhow::Result<()> {
        if params.cltv_expiry != self.cltv_expiry {
            anyhow::bail!(ExpiryMismatch {
                expected: params.cltv_expiry,
                received: self.cltv_expiry
            })
        }

        if params.asset != self.value {
            anyhow::bail!(AmountMismatch {
                expected: params.asset,
                received: self.value
            })
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize)]
struct PaymentsResponse {
    payments: Option<Vec<Payment>>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Payment {
    #[serde(deserialize_with = "deserialize_amount")]
    pub value_sat: asset::Bitcoin,
    pub payment_preimage: Option<Secret>,
    pub status: PaymentStatus,
    pub payment_hash: SecretHash,
}

impl Payment {
    fn validate(self, params: &Params) -> anyhow::Result<()> {
        if params.asset != self.value_sat {
            anyhow::bail!(AmountMismatch {
                expected: params.asset,
                received: self.value_sat
            })
        }

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct LndConnectorParams {
    lnd_url: Url,
    retry_interval_ms: u64,
    certificate: Certificate,
    macaroon: Macaroon,
}

impl LndConnectorParams {
    pub fn new(
        lnd_url: Url,
        retry_interval_ms: u64,
        certificate_path: PathBuf,
        macaroon_path: PathBuf,
    ) -> anyhow::Result<LndConnectorParams> {
        let certificate = read_file(certificate_path)?;
        let macaroon = read_file(macaroon_path)?;
        Ok(LndConnectorParams {
            lnd_url,
            retry_interval_ms,
            certificate,
            macaroon,
        })
    }
}

fn read_file<T>(path: PathBuf) -> anyhow::Result<T>
where
    T: TryFrom<Vec<u8>, Error = Error>,
{
    let mut buf = Vec::new();
    std::fs::File::open(path)?.read_to_end(&mut buf)?;
    Ok(buf.try_into()?)
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
    certificate: Certificate,
    macaroon: Macaroon,
}

impl From<LndConnectorParams> for LndConnectorAsSender {
    fn from(params: LndConnectorParams) -> Self {
        Self {
            lnd_url: params.lnd_url,
            retry_interval_ms: params.retry_interval_ms,
            certificate: params.certificate,
            macaroon: params.macaroon,
        }
    }
}

impl LndConnectorAsSender {
    fn payment_url(&self) -> Url {
        self.lnd_url
            .join("/v1/payments?include_incomplete=true")
            .expect("append valid string to url")
    }

    async fn find_payment(
        &self,
        params: &Params,
        status: PaymentStatus,
    ) -> anyhow::Result<Option<Payment>> {
        let url = self.payment_url();
        let response = client(&self.certificate, &self.macaroon)?
            .get(url.clone())
            .send()
            .await
            .with_context(|| GetRequestFailed(url))?
            .json::<PaymentsResponse>()
            .await
            .context("failed to deserialize response as list of payments")?;
        let payment = response
            .payments
            .unwrap_or_default()
            .into_iter()
            .find(|payment| payment.payment_hash == params.secret_hash && payment.status == status);

        if let Some(payment) = payment {
            payment
                .validate(params)
                .with_context(|| format!("validation for payment {} failed", params.secret_hash))?;
        }

        Ok(payment)
    }
}

#[async_trait::async_trait]
impl WaitForOpened for LndConnectorAsSender {
    async fn wait_for_opened(&self, _: &Params) -> anyhow::Result<Opened> {
        // At this stage there is no way for the sender to know when the invoice is
        // added on receiver's side.
        Ok(Opened)
    }
}

#[async_trait::async_trait]
impl WaitForAccepted for LndConnectorAsSender {
    async fn wait_for_accepted(&self, params: &Params) -> anyhow::Result<Accepted> {
        // No validation of the parameters because once the payment has been
        // sent the sender cannot cancel it.
        while self
            .find_payment(params, PaymentStatus::InFlight)
            .await?
            .is_none()
        {
            tokio::time::delay_for(Duration::from_millis(self.retry_interval_ms)).await;
        }

        Ok(Accepted)
    }
}

#[async_trait::async_trait]
impl WaitForSettled for LndConnectorAsSender {
    async fn wait_for_settled(&self, params: &Params) -> anyhow::Result<Settled> {
        let payment = loop {
            match self.find_payment(params, PaymentStatus::Succeeded).await? {
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
impl WaitForCancelled for LndConnectorAsSender {
    async fn wait_for_cancelled(&self, params: &Params) -> anyhow::Result<Cancelled> {
        while self
            .find_payment(params, PaymentStatus::Failed)
            .await?
            .is_none()
        {
            tokio::time::delay_for(Duration::from_millis(self.retry_interval_ms)).await;
        }

        Ok(Cancelled)
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
    certificate: Certificate,
    macaroon: Macaroon,
}

impl From<LndConnectorParams> for LndConnectorAsReceiver {
    fn from(params: LndConnectorParams) -> Self {
        Self {
            lnd_url: params.lnd_url,
            retry_interval_ms: params.retry_interval_ms,
            certificate: params.certificate,
            macaroon: params.macaroon,
        }
    }
}

impl LndConnectorAsReceiver {
    fn invoice_url(&self, secret_hash: SecretHash) -> anyhow::Result<Url> {
        Ok(self
            .lnd_url
            .join("/v1/invoice/")
            .expect("append valid string to url")
            .join(format!("{:x}", secret_hash).as_str())?)
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn find_invoice(
        &self,
        params: &Params,
        expected_state: InvoiceState,
    ) -> anyhow::Result<Option<Invoice>> {
        let url = self.invoice_url(params.secret_hash)?;
        let response = client(&self.certificate, &self.macaroon)?
            .get(url.clone())
            .send()
            .await
            .with_context(|| GetRequestFailed(url))?;

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

            bail!(lnd_error)
        }

        let invoice = response
            .json::<Invoice>()
            .await
            .context("failed to deserialize response as invoice")?;

        invoice
            .validate(params)
            .with_context(|| format!("validation for invoice {} failed", params.secret_hash))?;

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
impl WaitForOpened for LndConnectorAsReceiver {
    async fn wait_for_opened(&self, params: &Params) -> anyhow::Result<Opened> {
        // Do we want to validate that the user used the correct swap parameters
        // when adding the invoice?
        while self
            .find_invoice(params, InvoiceState::Open)
            .await?
            .is_none()
        {
            tokio::time::delay_for(Duration::from_millis(self.retry_interval_ms)).await;
        }

        Ok(Opened)
    }
}

#[async_trait::async_trait]
impl WaitForAccepted for LndConnectorAsReceiver {
    async fn wait_for_accepted(&self, params: &Params) -> anyhow::Result<Accepted> {
        // Validation that sender payed the correct invoice is provided by LND.
        // Since the sender uses the params to make the payment (as apposed to
        // the invoice) LND guarantees that the params match the invoice when
        // updating the invoice status.
        while self
            .find_invoice(params, InvoiceState::Accepted)
            .await?
            .is_none()
        {
            tokio::time::delay_for(Duration::from_millis(self.retry_interval_ms)).await;
        }
        Ok(Accepted)
    }
}

#[async_trait::async_trait]
impl WaitForSettled for LndConnectorAsReceiver {
    async fn wait_for_settled(&self, params: &Params) -> anyhow::Result<Settled> {
        let invoice = loop {
            match self.find_invoice(params, InvoiceState::Settled).await? {
                Some(invoice) => break invoice,
                None => tokio::time::delay_for(Duration::from_millis(self.retry_interval_ms)).await,
            }
        };

        let preimage = invoice
            .r_preimage
            .ok_or_else(|| anyhow::anyhow!("settled invoice does not contain preimage?!"))?;

        Ok(Settled {
            secret: Secret::from_vec(&preimage)?,
        })
    }
}

#[async_trait::async_trait]
impl WaitForCancelled for LndConnectorAsReceiver {
    async fn wait_for_cancelled(&self, params: &Params) -> anyhow::Result<Cancelled> {
        while self
            .find_invoice(params, InvoiceState::Cancelled)
            .await?
            .is_none()
        {
            tokio::time::delay_for(Duration::from_millis(self.retry_interval_ms)).await;
        }
        Ok(Cancelled)
    }
}

fn client(certificate: &Certificate, macaroon: &Macaroon) -> anyhow::Result<reqwest::Client> {
    let cert = certificate.0.clone();
    let mut default_headers = HeaderMap::with_capacity(1);
    default_headers.insert(
        "Grpc-Metadata-macaroon",
        HeaderValue::from_str(&macaroon.0)?,
    );

    // The generated, self-signed lnd certificate is deemed invalid on macOS
    // Catalina because of new certificate requirements in macOS Catalina: https://support.apple.com/en-us/HT210176
    // By using this conditional compilation step for macOS we accept invalid
    // certificates. This is only a minimal security risk because by default the
    // certificate that lnd generates is configured to only allow connections
    // from localhost. Ticket that will resolve that issue: https://github.com/lightningnetwork/lnd/issues/4201
    #[cfg(target_os = "macos")]
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .add_root_certificate(cert)
        .default_headers(default_headers)
        .build()?;

    #[cfg(not(target_os = "macos"))]
    let client = reqwest::Client::builder()
        .add_root_certificate(cert)
        .default_headers(default_headers)
        .build()?;

    Ok(client)
}

#[derive(Debug, thiserror::Error)]
#[error("GET request to {0} failed")]
pub struct GetRequestFailed(Url);

pub fn deserialize_amount<'de, D>(deserializer: D) -> Result<asset::Bitcoin, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = asset::Bitcoin;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a bitcoin asset quantity as a string")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            let sat: u64 = v.parse().map_err(E::custom)?;
            Ok(asset::Bitcoin::from_sat(sat))
        }
    }

    deserializer.deserialize_any(Visitor)
}

pub fn deserialize_relative_time<'de, D>(deserializer: D) -> Result<RelativeTime, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = RelativeTime;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a blocknumber as a string")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            let value: u32 = v.parse().map_err(E::custom)?;
            Ok(RelativeTime::from(value))
        }
    }

    deserializer.deserialize_any(Visitor)
}

pub fn deserialize_r_preimage<'de, D>(deserializer: D) -> Result<Option<[u8; 32]>, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = Option<[u8; 32]>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a lightning r_preimage which is a base64 of an 32 byte array")
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            match v {
                "" => Ok(None),
                base64_preimage => {
                    let vec = base64::decode(base64_preimage.as_bytes()).map_err(E::custom)?;
                    if vec.len() != 32 {
                        return Err(de::Error::invalid_length(vec.len(), &"32"));
                    }
                    let mut r_preimage = [0; 32];
                    let vec = &vec[..32];
                    r_preimage.copy_from_slice(vec);
                    Ok(Some(r_preimage))
                }
            }
        }
    }

    deserializer.deserialize_any(Visitor)
}
#[cfg(test)]
mod tests {
    use super::*;
    use spectral::prelude::*;
    #[test]
    fn deserialize_ln_invoice_preimage_present() {
        let r_preimage = [
            0x17u8, 0x23u8, 0x4cu8, 0x08u8, 0xf1u8, 0x39u8, 0x9eu8, 0x6fu8, 0x7au8, 0xfdu8, 0x06u8,
            0x54u8, 0x35u8, 0x79u8, 0x85u8, 0x37u8, 0x3cu8, 0xc3u8, 0x61u8, 0x81u8, 0x1au8, 0x06u8,
            0xdau8, 0x57u8, 0x35u8, 0xc3u8, 0x5eu8, 0xd3u8, 0xb6u8, 0xc2u8, 0xf9u8, 0xffu8,
        ];

        let invoice_json = r#"{
      "r_preimage": "FyNMCPE5nm96/QZUNXmFNzzDYYEaBtpXNcNe07bC+f8=",
      "value": "10000",
      "value_msat": "10000000",
      "expiry": "3600",
      "cltv_expiry": "350",
      "amt_paid_sat": "0",
      "amt_paid_msat": "0",
      "state": "SETTLED"
    }"#;

        let invoice = serde_json::from_str::<Invoice>(invoice_json).unwrap();
        assert_that(&invoice.r_preimage)
            .is_some()
            .is_equal_to(&r_preimage);
    }

    #[test]
    fn deserialize_ln_invoice_preimage_empty() {
        let invoice_json = r#"{
      "r_preimage": "",
      "value": "10000",
      "value_msat": "10000000",
      "expiry": "3600",
      "cltv_expiry": "350",
      "amt_paid_sat": "0",
      "amt_paid_msat": "0",
      "state": "SETTLED"
    }"#;

        let invoice = serde_json::from_str::<Invoice>(invoice_json).unwrap();
        assert_that(&invoice.r_preimage).is_none()
    }
    #[test]
    fn deserialize_ln_invoice_preimage_not_present() {
        let invoice_json = r#"{
      "r_preimage": null,
      "value": "10000",
      "value_msat": "10000000",
      "expiry": "3600",
      "cltv_expiry": "350",
      "amt_paid_sat": "0",
      "amt_paid_msat": "0",
      "state": "SETTLED"
    }"#;

        let invoice = serde_json::from_str::<Invoice>(invoice_json).unwrap();
        assert_that(&invoice.r_preimage).is_none()
    }
}
