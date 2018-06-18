use bitcoin::network::constants::Network;
use bitcoin::util::address::Address;
pub use bitcoin::util::privkey::Privkey as PrivateKey;
pub use secp256k1::PublicKey;

pub trait ToAddress {
    // note: to_address() is taken by rust-bitcoin so you have call it with
    // ToAddress::to_address()
    fn to_address(&self, Network) -> Address;
}

impl ToAddress for PrivateKey {
    fn to_address(&self, network: Network) -> Address {
        let secret_pubkey = PublicKey::from_secret_key(&*super::SECP, self.secret_key()).unwrap();
        secret_pubkey.to_address(network)
    }
}

impl ToAddress for PublicKey {
    fn to_address(&self, network: Network) -> Address {
        Address::p2wpkh(&self, network)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    #[test]
    fn to_address_for_regtest() {
        let privkey =
            PrivateKey::from_str("cR6U4gNiCQsPo5gLNP2w6QsLTZkvCGEijhYVPZVhnePQKjMwmas8").unwrap();

        let address = ToAddress::to_address(&privkey, Network::BitcoinCoreRegtest);
        assert_eq!(
            address,
            Address::from_str("bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap").unwrap()
        );
    }

}
