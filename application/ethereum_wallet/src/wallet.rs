use ethereum_support::{Address, ToEthereumAddress, U256};
use rlp::RlpStream;
use secp256k1_support::{KeyPair, RecoverableSignature};
use tiny_keccak;
use SignedTransaction;
use UnsignedTransaction;

pub trait Wallet: Send + Sync {
    fn sign<'a>(&self, tx: &'a UnsignedTransaction) -> SignedTransaction<'a>;
    fn calculate_contract_address(&self, nonce: U256) -> Address;
}

pub struct InMemoryWallet {
    keypair: KeyPair,
    chain_id: u8,
}

impl InMemoryWallet {
    pub fn new(keypair: KeyPair, chain_id: u8) -> Self {
        InMemoryWallet { keypair, chain_id }
    }

    // https://github.com/ethereum/EIPs/blob/master/EIPS/eip-155.md#specification
    fn chain_replay_protection_offset(&self) -> u8 {
        35 + self.chain_id * 2
    }
}

impl Wallet for InMemoryWallet {
    fn sign<'a>(&self, tx: &'a UnsignedTransaction) -> SignedTransaction<'a> {
        let hash: [u8; 32] = tx.hash(self.chain_id).into();

        let signature = self.keypair.sign_ecdsa_recoverable(hash.into());

        let (rec_id, signature) = RecoverableSignature::serialize_compact(&signature);

        let v = rec_id.to_i32() as u8 + self.chain_replay_protection_offset();

        SignedTransaction::new(tx, v, signature)
    }

    fn calculate_contract_address(&self, nonce: U256) -> Address {
        let mut stream = RlpStream::new_list(2);
        let h160 = self.keypair.public_key().to_ethereum_address();
        let ethereum_address: &[u8] = h160.as_ref();

        stream.append(&ethereum_address);
        stream.append(&nonce);

        let value = tiny_keccak::keccak256(stream.as_raw());

        let mut address = Address::default();
        address.copy_from_slice(&value[12..]);
        address
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use hex::FromHex;

    #[test]
    fn given_an_address_and_nounce_should_give_contract_address() {
        let wallet = {
            let secret_key_data = &<[u8; 32]>::from_hex(
                "3f92cbc79aa7e29c7c5f3525749fd7d90aa21938de096f1b78710befe6d8ef59",
            ).unwrap();
            let keypair = KeyPair::from_secret_key_slice(secret_key_data).unwrap();
            InMemoryWallet::new(keypair, 42) // 42 is used in GanacheCliNode
        };

        let contract_address = wallet.calculate_contract_address(U256::from(0));
        assert_eq!(
            contract_address,
            "a00f2cac7bad9285ecfd59e8860f5b2d8622e099".into()
        );
        let contract_address = wallet.calculate_contract_address(U256::from(3));
        assert_eq!(
            contract_address,
            "1e637bb1935f820390d746b241df4f6a0347884f".into()
        );
    }
}
