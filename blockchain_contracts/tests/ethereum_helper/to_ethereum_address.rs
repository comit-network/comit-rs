use rust_bitcoin::secp256k1::PublicKey;
use web3::types::Address;

// TODO: Should/Can this be contributed back?
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
