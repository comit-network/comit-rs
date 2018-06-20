use bitcoin::network::constants::Network;
use bitcoin::util::address::Address;
pub use bitcoin::util::privkey::Privkey as PrivateKey;
pub use secp256k1::PublicKey;

pub trait ToP2wpkhAddress {
    // note: to_address() is taken by rust-bitcoin so you have call it with
    // ToAddress::to_address()
    fn to_p2wpkh_address(&self, Network) -> Address;
}

impl ToP2wpkhAddress for PrivateKey {
    fn to_p2wpkh_address(&self, network: Network) -> Address {
        let pubkey = PublicKey::from_secret_key(&*super::SECP, self.secret_key()).unwrap();
        pubkey.to_p2wpkh_address(network)
    }
}

impl ToP2wpkhAddress for PublicKey {
    fn to_p2wpkh_address(&self, network: Network) -> Address {
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

        let address = privkey.to_p2wpkh_address(Network::BitcoinCoreRegtest);
        assert_eq!(
            address,
            Address::from_str("bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap").unwrap()
        );
    }

    #[test]
    fn address_from_btc_address_generator_gives_same_asnwer() {
        // https://kimbatt.github.io/btc-address-generator/
        let privkey =
            PrivateKey::from_str("L4nZrdzNnawCtaEcYGWuPqagQA3dJxVPgN8ARTXaMLCxiYCy89wm").unwrap();
        let address = privkey.to_p2wpkh_address(Network::Bitcoin);

        assert_eq!(
            address,
            Address::from_str("bc1qmxq0cu0jktxyy2tz3je7675eca0ydcevgqlpgh").unwrap()
        );
    }
}
