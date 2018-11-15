use swap_protocols::rfc003::secret::{ExtractSecret, Secret, SecretHash};

impl ExtractSecret for ethereum_support::Transaction {
    fn extract_secret(&self, secret_hash: &SecretHash) -> Option<Secret> {
        unimplemented!()
    }
}
