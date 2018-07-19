use bitcoin::{
    network::constants::Network,
    util::{
        address::{Address, Payload},
        hash::Hash160,
    },
};
use secp256k1_support::PublicKey;
use std::fmt;

pub trait ToP2wpkhAddress {
    fn to_p2wpkh_address(&self, Network) -> Address;
}

impl ToP2wpkhAddress for PublicKey {
    fn to_p2wpkh_address(&self, network: Network) -> Address {
        Address::p2wpkh(&self, network)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PubkeyHash(Hash160);

impl From<Hash160> for PubkeyHash {
    fn from(hash: Hash160) -> PubkeyHash {
        PubkeyHash(hash)
    }
}

impl From<Address> for PubkeyHash {
    fn from(address: Address) -> PubkeyHash {
        match address.payload {
            Payload::WitnessProgram(witness) => PubkeyHash(Hash160::from(witness.program())),
            // TODO: from/into should never fail. Remove this panic by
            // creating a PubkeyAddress type which is guaranteed to
            // have a PubkeyHash inside it.
            _ => panic!("Address {} isn't a pubkey address", address.to_string()),
        }
    }
}

impl From<PublicKey> for PubkeyHash {
    fn from(public_key: PublicKey) -> PubkeyHash {
        PubkeyHash(Hash160::from_data(&public_key.serialize()))
    }
}

impl<'a> From<&'a [u8]> for PubkeyHash {
    fn from(data: &'a [u8]) -> PubkeyHash {
        PubkeyHash(Hash160::from(data))
    }
}

impl AsRef<[u8]> for PubkeyHash {
    fn as_ref(&self) -> &[u8] {
        &self.0[..]
    }
}

impl Into<Hash160> for PubkeyHash {
    fn into(self) -> Hash160 {
        self.0
    }
}

impl fmt::LowerHex for PubkeyHash {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        f.write_str(format!("{:?}", self.0).as_str())
    }
}

#[cfg(test)]
mod test {
    extern crate hex;
    use super::*;
    use bitcoin::util::privkey::Privkey as PrivateKey;
    use secp256k1_support::KeyPair;
    use std::str::FromStr;

    #[test]
    fn given_an_bitcoin_address_return_pubkey_hash() {
        let address = Address::from_str("bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap").unwrap();
        let pubkey_hash: PubkeyHash = address.into();

        assert_eq!(
            format!("{:x}", pubkey_hash),
            "c021f17be99c6adfbcba5d38ee0d292c0399d2f5"
        );
    }

    #[test]
    fn correct_pubkeyhash_from_private_key() {
        let private_key =
            PrivateKey::from_str("L253jooDhCtNXJ7nVKy7ijtns7vU4nY49bYWqUH8R9qUAUZt87of").unwrap();
        let keypair: KeyPair = private_key.secret_key().clone().into();
        let pubkey_hash: PubkeyHash = keypair.public_key().clone().into();

        assert_eq!(
            pubkey_hash,
            PubkeyHash::from(&hex::decode("8bc513e458372a3b3bb05818d09550295ce15949").unwrap()[..])
        )
    }

    // ToP2wpkhAddress NYI for PubkeyHash
    // #[test]
    // fn correct_address_from_pubkey_hash() {
    //     let pubkey_hash = PubkeyHash::from(&hex::decode("c021f17be99c6adfbcba5d38ee0d292c0399d2f5").unwrap()[..]);
    //     let address = pubkey_hash.to_p2wpkh_address(Network::BitcoinCoreRegtest);

    //     assert_eq!(
    //         address,
    //         Address::from_str("bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap").unwrap(),
    //     )
    // }

    #[test]
    fn generates_same_address_from_private_key_as_btc_address_generator() {
        // https://kimbatt.github.io/btc-address-generator/
        let private_key =
            PrivateKey::from_str("L4nZrdzNnawCtaEcYGWuPqagQA3dJxVPgN8ARTXaMLCxiYCy89wm").unwrap();
        let keypair: KeyPair = private_key.secret_key().clone().into();
        let address = keypair.public_key().to_p2wpkh_address(Network::Bitcoin);

        assert_eq!(
            address,
            Address::from_str("bc1qmxq0cu0jktxyy2tz3je7675eca0ydcevgqlpgh").unwrap()
        );
    }

}
