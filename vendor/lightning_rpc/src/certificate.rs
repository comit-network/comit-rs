use pem;
use std::{fs, io, path::Path};
use tls_api::{self, Certificate as TlsCertificate};
use FromFile;

// tls_api::Certificate doesn't implement Debug.
#[allow(missing_debug_implementations)]
pub struct Certificate(tls_api::Certificate);

impl FromFile for Certificate {
    type Err = ReadCertificateError;

    fn from_file<P: AsRef<Path>>(file: P) -> Result<Self, Self::Err> {
        let tls_cert =
            fs::read_to_string(file).or_else(|e| Err(ReadCertificateError::ReadFileFail(e)))?;

        let tls_cert =
            pem::parse(&tls_cert).or_else(|e| Err(ReadCertificateError::ParseFileFail(e)))?;
        let tls_cert = TlsCertificate::from_der(tls_cert.contents);
        Ok(Certificate(tls_cert))
    }
}

impl From<Certificate> for TlsCertificate {
    fn from(cert: Certificate) -> Self {
        cert.0
    }
}

#[derive(Debug)]
pub enum ReadCertificateError {
    OpenFileFail(io::Error),
    ReadFileFail(io::Error),
    ParseFileFail(pem::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_certificate() {
        // If it does not panic! then it passed
        assert!(Certificate::from_file("./sample/tls.cert".to_string()).is_ok());
    }
}
