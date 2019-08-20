use crate::Address;
use secp256k1_keypair::PublicKey;

pub trait ToEthereumAddress {
    fn to_ethereum_address(&self) -> Address;
}

impl ToEthereumAddress for PublicKey {
    fn to_ethereum_address(&self) -> Address {
        let serialized_public_key = self.serialize_uncompressed();
        // Remove the silly openssl 0x04 byte from the front of the
        // serialized public key. This is a bitcoin thing that
        // ethereum doesn't want. Eth pubkey should be 32 + 32 = 64 bytes.
        let actual_public_key = &serialized_public_key[1..];
        let hash = tiny_keccak::keccak256(actual_public_key);
        // Ethereum address is the last twenty bytes of the keccak256 hash
        let ethereum_address_bytes = &hash[12..];
        let mut address = Address::default();
        address.assign_from_slice(ethereum_address_bytes);
        address
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use secp256k1_keypair::KeyPair;
    use std::str::FromStr;

    fn valid_pair(key: &str, address: &str) -> bool {
        let privkey_data = hex::decode(key).unwrap();
        let keypair = KeyPair::from_secret_key_slice(&privkey_data[..]).unwrap();
        let generated_address: Address = keypair.public_key().to_ethereum_address();
        Address::from_str(address).unwrap() == generated_address
    }
    #[test]
    fn test_known_valid_pairs() {
        assert!(valid_pair(
            "981679905857953c9a21e1807aab1b897a395ea0c5c96b32794ccb999a3cd781",
            "5fe3062B24033113fbf52b2b75882890D7d8CA54"
        ));
        assert!(valid_pair(
            "dd0d193e94ad1deb5a45214645ac3793b4f1283d74f354c7849225a43e9cadc5",
            "33DcA4Dfe91388307CF415537D659Fef2d13B71a"
        ));
    }
}
