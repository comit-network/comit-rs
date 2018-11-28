use bitcoin_support::{
    serialize::serialize_hex, Address, BitcoinQuantity, Network, OutPoint, PrivateKey, PubkeyHash,
};
use bitcoin_witness::{PrimedInput, PrimedTransaction};
use secp256k1_support::KeyPair;
use swap_protocols::rfc003::{bitcoin::Htlc, Secret};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SendToAddress {
    pub address: Address,
    pub value: BitcoinQuantity,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpendOutput {
    //Remember: One man's input is another man's output!
    //TODO: decide whether we want to serialize this directly
    pub output: PrimedInput,
}

impl SpendOutput {
    pub fn spend_to(self, to_address: Address) -> PrimedTransaction {
        PrimedTransaction {
            inputs: vec![self.output],
            locktime: 0,
            output_address: to_address,
        }
    }
}

impl SpendOutput {
    pub fn serialize(&self, to: String) -> Result<String, ()> {
        unimplemented!()
    }
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct BitcoinRedeem {
    pub outpoint: OutPoint,
    pub htlc: Htlc,
    pub value: BitcoinQuantity,
    pub transient_keypair: KeyPair,
    pub secret: Secret,
}

impl BitcoinRedeem {
    pub fn to_transaction(&self, to_address: Address) -> PrimedTransaction {
        PrimedTransaction {
            inputs: vec![PrimedInput::new(
                self.outpoint,
                self.value,
                self.htlc
                    .unlock_with_secret(self.transient_keypair, &self.secret),
            )],
            locktime: 0,
            output_address: to_address,
        }
    }

    pub fn serialize(&self, to: String) -> Result<String, ()> {
        //TODO return error here
        //TODO `to` should be an address
        let fee = BitcoinQuantity::from_satoshi(1000); //TODO set correct fee
        let address = to.parse().unwrap(); //TODO don't unwrap here

        let transaction = self.to_transaction(address).sign_with_fee(fee);
        let redeem_tx_hex = serialize_hex(&transaction).unwrap(); //TODO don't unwrap here

        Ok(redeem_tx_hex)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bitcoin_support::Sha256dHash;
    use spectral::prelude::*;
    use std::str::{self, FromStr};

    #[test]
    fn serialize_redeem_btc_correctly() {
        let success_privkey =
            PrivateKey::from_str("cSrWvMrWE3biZinxPZc1hSwMMEdYgYsFpB6iEoh8KraLqYZUUCtt").unwrap();
        let success_keypair: KeyPair = success_privkey.secret_key().clone().into();
        let success_pubkey_hash: PubkeyHash = success_keypair.public_key().clone().into();
        let refund_privkey =
            PrivateKey::from_str("cNZUJxVXghSri4dUaNW8ES3KiFyDoWVffLYDz7KMcHmKhLdFyZPx").unwrap();
        let refund_keypair: KeyPair = refund_privkey.secret_key().clone().into();
        let refund_pubkey_hash: PubkeyHash = refund_keypair.public_key().clone().into();

        let secret = Secret::from(*b"hello world, you are beautiful!!");
        let secret_encoded = hex::encode(secret.raw_secret());

        let sequence_lock = 10;

        let amount = BitcoinQuantity::from_satoshi(100_000_001);

        let htlc = Htlc::new(
            success_pubkey_hash,
            refund_pubkey_hash,
            secret.hash(),
            sequence_lock,
        );

        let redeem_btc = BitcoinRedeem {
            outpoint: OutPoint {
                txid: Sha256dHash::from_hex(
                    "02b082113e35d5386285094c2829e7e2963fa0b5369fb7f4b79c4c90877dcd3d",
                )
                .unwrap(),
                vout: 0u32,
            },
            htlc,
            value: amount,
            secret,
            transient_keypair: success_keypair,
        };
        let to_address = Address::from_str("bc1qcqslz7lfn34dl096t5uwurff9spen5h4y9r93m"); //equals 0014c021f17be99c6adfbcba5d38ee0d292c0399d2f5
        let result =
            redeem_btc.serialize(String::from("bc1qcqslz7lfn34dl096t5uwurff9spen5h4y9r93m"));

        assert_that(&result).is_ok();

        let result = result.unwrap();

        assert_that(&result).is_equal_to(String::from("020000000001013dcd7d87904c9cb7f4b79f36b5a03f96e2e729284c09856238d5353e1182b0020000000000feffffff0119ddf50500000000160014c021f17be99c6adfbcba5d38ee0d292c0399d2f505483045022100ef04903959ad34ab6a315d8d39f7add5dcb3291bed2b8d9c637389ad4dc797fc022064d3be2309bd95a999cfec0f82b87e0dc84197a06d0f18fe01cd34ece0b32f90012103dee6abec27acac2d1bee121d6346e20bedf932af864be05d86dca5541eb474672068656c6c6f20776f726c642c20796f75206172652062656175746966756c212101015963a82068d627971643a6f97f27c58957826fcba853ec2077fd10ec6b93d8e61deb4cec8876a9142e90d7ea212ad448ea0fa118c7975af9fca9a995675ab27576a914cef2b9c276e2553f86acffaea33a1cb66f1a8a8b6888ac00000000"));

        assert_that(&result).contains("0014c021f17be99c6adfbcba5d38ee0d292c0399d2f5"); //contains to address

        assert_that(&result).contains(secret_encoded.as_str()); //contains secret
    }
}
