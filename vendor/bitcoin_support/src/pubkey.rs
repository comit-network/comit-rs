use bitcoin::{
    network::constants::Network,
    util::{address::Payload, hash::Hash160},
    Address,
};
use bitcoin_bech32;
use hex::{self, FromHex};
use secp256k1_support::{KeyPair, PublicKey};
use serde::{
    de::{self, Deserialize, Deserializer},
    ser::{Serialize, Serializer},
};
use std::fmt;

pub trait IntoP2wpkhAddress {
    fn into_p2wpkh_address(self, network: Network) -> Address;
}

impl IntoP2wpkhAddress for PublicKey {
    fn into_p2wpkh_address(self, network: Network) -> Address {
        Address::p2wpkh(&self.into(), network)
    }
}

impl IntoP2wpkhAddress for PubkeyHash {
    fn into_p2wpkh_address(self, network: Network) -> Address {
        Address {
            payload: Payload::WitnessProgram(
                bitcoin_bech32::WitnessProgram::new(
                    bitcoin_bech32::u5::try_from_u8(0).expect("0 is a valid u5"),
                    self.as_ref().to_vec(),
                    match network {
                        Network::Regtest => bitcoin_bech32::constants::Network::Regtest,
                        Network::Testnet => bitcoin_bech32::constants::Network::Testnet,
                        Network::Bitcoin => bitcoin_bech32::constants::Network::Bitcoin,
                    },
                )
                .expect("Any pubkeyhash will succeed in conversion to WitnessProgram"),
            ),
            network,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct PubkeyHash(Hash160);

impl From<Hash160> for PubkeyHash {
    fn from(hash: Hash160) -> PubkeyHash {
        PubkeyHash(hash)
    }
}

#[allow(clippy::fallible_impl_from)]
impl From<Address> for PubkeyHash {
    fn from(address: Address) -> PubkeyHash {
        match address.payload {
            Payload::WitnessProgram(witness) => PubkeyHash(Hash160::from(witness.program())),
            // TODO: from/into should never fail. Remove this panic by
            // creating a PubkeyAddress type which is guaranteed to
            // have a PubkeyHash inside it.
            // This is tracked in ticket https://github.com/comit-network/comit-rs/issues/629
            _ => panic!("Address {} isn't a pubkey address", address.to_string()),
        }
    }
}

impl From<PublicKey> for PubkeyHash {
    fn from(public_key: PublicKey) -> PubkeyHash {
        PubkeyHash(Hash160::from_data(&public_key.inner().serialize()))
    }
}

impl From<KeyPair> for PubkeyHash {
    fn from(key_pair: KeyPair) -> Self {
        key_pair.public_key().into()
    }
}

impl<'a> From<&'a [u8]> for PubkeyHash {
    fn from(data: &'a [u8]) -> PubkeyHash {
        PubkeyHash(Hash160::from(data))
    }
}

impl FromHex for PubkeyHash {
    type Error = hex::FromHexError;

    fn from_hex<T: AsRef<[u8]>>(hex: T) -> Result<Self, Self::Error> {
        Ok(PubkeyHash::from(hex::decode(hex)?.as_ref()))
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.write_str(format!("{:?}", self.0).as_str())
    }
}

impl<'de> Deserialize<'de> for PubkeyHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'vde> de::Visitor<'vde> for Visitor {
            type Value = PubkeyHash;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
                formatter.write_str("A hex-encoded compressed SECP256k1 public key")
            }

            fn visit_str<E>(self, hex_pubkey: &str) -> Result<PubkeyHash, E>
            where
                E: de::Error,
            {
                PubkeyHash::from_hex(hex_pubkey).map_err(E::custom)
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

impl Serialize for PubkeyHash {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(hex::encode(self.0.to_bytes()).as_str())
    }
}

#[cfg(test)]
mod test {
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
        let pubkey_hash: PubkeyHash = keypair.public_key().into();

        assert_eq!(
            pubkey_hash,
            PubkeyHash::from(&hex::decode("8bc513e458372a3b3bb05818d09550295ce15949").unwrap()[..])
        )
    }

    // ToP2wpkhAddress NYI for PubkeyHash
    // #[test]
    // fn correct_address_from_pubkey_hash() {
    //     let pubkey_hash =
    // PubkeyHash::from(&hex::decode("c021f17be99c6adfbcba5d38ee0d292c0399d2f5").
    // unwrap()[..]);     let address =
    // pubkey_hash.to_p2wpkh_address(Network::Regtest);

    //     assert_eq!(
    //         address,
    //         Address::from_str("bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap").
    // unwrap(),     )
    // }

    #[test]
    fn generates_same_address_from_private_key_as_btc_address_generator() {
        // https://kimbatt.github.io/btc-address-generator/
        let private_key =
            PrivateKey::from_str("L4nZrdzNnawCtaEcYGWuPqagQA3dJxVPgN8ARTXaMLCxiYCy89wm").unwrap();
        let keypair: KeyPair = private_key.secret_key().clone().into();
        let address = keypair.public_key().into_p2wpkh_address(Network::Bitcoin);

        assert_eq!(
            address,
            Address::from_str("bc1qmxq0cu0jktxyy2tz3je7675eca0ydcevgqlpgh").unwrap()
        );
    }

    #[test]
    fn roudtrip_serialization_of_pubkeyhash() {
        let public_key = PublicKey::from_hex(
            "02c2a8efce029526d364c2cf39d89e3cdda05e5df7b2cbfc098b4e3d02b70b5275",
        )
        .unwrap();
        let pubkey_hash: PubkeyHash = public_key.into();
        let serialized = serde_json::to_string(&pubkey_hash).unwrap();
        assert_eq!(serialized, "\"ac2db2f2615c81b83fe9366450799b4992931575\"");
        let deserialized = serde_json::from_str::<PubkeyHash>(serialized.as_str()).unwrap();
        assert_eq!(deserialized, pubkey_hash);
    }

}
