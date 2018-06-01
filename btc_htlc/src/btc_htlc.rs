use bitcoin::blockdata::opcodes::All::OP_NOP3 as OP_CHECKSEQUENCEVERIFY;
use bitcoin::blockdata::opcodes::All::*;
use bitcoin::blockdata::script::{Builder, Script};
use bitcoin::network::constants::Network;
use bitcoin::util::address::Address;
use bitcoin::util::address::Payload::WitnessProgram;
use bitcoin_rpc;
use bitcoin_rpc::BlockHeight;
use secret::SecretHash;
use std::str::FromStr;

// Create BTC HTLC
// Returns P2WSH address
// Input:
// - BTC address of the exchange to receive the funds (exchange_success_address)
// - BTC timeout
// - BTC amount
// - hashed secret

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BtcHtlc {
    recipient_success_address: bitcoin_rpc::Address,
    sender_refund_address: bitcoin_rpc::Address,
    secret_hash: SecretHash,
    relative_timelock: BlockHeight,
    pub htlc_address: bitcoin_rpc::Address,
}

//TODO: implement proper error handling
impl BtcHtlc {
    pub fn new(
        recipient_success_address: bitcoin_rpc::Address,
        sender_refund_address: bitcoin_rpc::Address,
        secret_hash: SecretHash,
        relative_timelock: BlockHeight,
        network: &Network,
    ) -> Option<BtcHtlc> {
        // TODO: the recipient is the exchange_service -> we actually should get the exchange pubkey hash directly instead of an address
        // to be addressed with the final product. Get leave as it for MVP
        let recipient_pubkey_hash = get_pubkey_hash_from_address(&recipient_success_address.0)
            .expect("Could not extract pubkey hash from recipient_success_address");
        let sender_pubkey_hash = get_pubkey_hash_from_address(&sender_refund_address.0)
            .expect("Could not extract pubkey hash from sender_refund_address");

        let script = create_htlc_redeem_script(
            &recipient_pubkey_hash,
            &sender_pubkey_hash,
            &secret_hash.0,
            &relative_timelock,
        );

        let htlc_address = Address::p2wsh(&script, network.clone());
        let htlc_address: String = htlc_address.to_string();
        let htlc_address = bitcoin_rpc::Address::from(htlc_address.as_str());

        Some(BtcHtlc {
            recipient_success_address,
            sender_refund_address,
            secret_hash,
            relative_timelock,
            htlc_address,
        })
    }
}

//TODO: implement proper error handling
pub fn get_pubkey_hash_from_address(address: &String) -> Option<Vec<u8>> {
    let addr = Address::from_str(address.as_str());
    let addr = match addr {
        Ok(a) => a,
        Err(e) => panic!(
            "Could not create address out of {}; Error: {:?}",
            address, e
        ),
    };
    match addr.payload {
        WitnessProgram(witness) => Some(witness.program().to_vec()),
        _ => None,
    }
}

pub fn create_htlc_redeem_script(
    recipient_pubkey_hash: &Vec<u8>,
    sender_pubkey_hash: &Vec<u8>,
    secret_hash: &Vec<u8>,
    redeem_block_height: &BlockHeight,
) -> Script {
    Builder::new()
        .push_opcode(OP_IF)
        .push_opcode(OP_SHA256)
        .push_slice(secret_hash)
        .push_opcode(OP_EQUALVERIFY)
        .push_opcode(OP_DUP)
        .push_opcode(OP_HASH160)
        .push_slice(recipient_pubkey_hash)
        .push_opcode(OP_ELSE)
        .push_int(redeem_block_height.as_i64())
        .push_opcode(OP_CHECKSEQUENCEVERIFY)
        .push_opcode(OP_DROP)
        .push_opcode(OP_DUP)
        .push_opcode(OP_HASH160)
        .push_slice(sender_pubkey_hash)
        .push_opcode(OP_ENDIF)
        .push_opcode(OP_EQUALVERIFY)
        .push_opcode(OP_CHECKSIG)
        .into_script()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::network;
    use hex;

    #[test]
    fn given_an_address_return_pubkey_hash() {
        let address = String::from_str("bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap");
        let pubkey_hash = get_pubkey_hash_from_address(&address.unwrap()).unwrap();

        assert_eq!(
            pubkey_hash,
            hex::decode("c021f17be99c6adfbcba5d38ee0d292c0399d2f5").unwrap()
        );
    }

    // Secret: 12345678901234567890123456789012
    // Secret hash: 51a488e06e9c69c555b8ad5e2c4629bb3135b96accd1f23451af75e06d3aee9c

    // Sender address: bcrt1qryj6ya9vqpph8w65992nhk64cs890vfy0khsfg
    // Sender pubkey: 020c04eb8cb87485501e30b656f37439ea7866d7c58b3c38161e5793b68e712356
    // Sender pubkey hash: 1925a274ac004373bb5429553bdb55c40e57b124

    // Recipient address: bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap
    // Recipient pubkey: 0298e113cc06bc862ac205f2c0f27ee8c0de98d0716537bbf74e2ea6f38a84d5dc
    // Recipient pubkey hash: c021f17be99c6adfbcba5d38ee0d292c0399d2f5

    // htlc script: 63a82051a488e06e9c69c555b8ad5e2c4629bb3135b96accd1f23451af75e06d3aee9c8876a914c021f17be99c6adfbcba5d38ee0d292c0399d2f567028403b27576a9141925a274ac004373bb5429553bdb55c40e57b1246888ac
    // sha256 of htlc script: 82badc8d1175d1c7ecfceb67a6b8d24fa51718beb594002c7cd9ca1da706b4ef

    #[test]
    fn given_a_vec_u8_pubkey_hash_return_htlc_redeem_script() {
        let recipient_pubkey_hash: Vec<u8> =
            hex::decode("c021f17be99c6adfbcba5d38ee0d292c0399d2f5").unwrap();
        let sender_pubkey_hash: Vec<u8> =
            hex::decode("1925a274ac004373bb5429553bdb55c40e57b124").unwrap();
        let secret_hash: Vec<u8> = hex::decode(
            "51a488e06e9c69c555b8ad5e2c4629bb3135b96accd1f23451af75e06d3aee9c",
        ).unwrap();

        let script = create_htlc_redeem_script(
            &recipient_pubkey_hash,
            &sender_pubkey_hash,
            &secret_hash,
            &BlockHeight::new(900),
        );

        assert_eq!(
            script.into_vec(),
            hex::decode(
                "63a82051a488e06e9c69c555b8ad5e2c4629bb3135b96accd1f2345\
                 1af75e06d3aee9c8876a914c021f17be99c6adfbcba5d38ee0d292c0399d2f\
                 567028403b27576a9141925a274ac004373bb5429553bdb55c40e57b1246888ac"
            ).unwrap()
        );
    }

    #[test]
    fn given_an_htlc_redeem_script_return_p2wsh() {
        let recipient_pubkey_hash: Vec<u8> =
            hex::decode("c021f17be99c6adfbcba5d38ee0d292c0399d2f5").unwrap();
        let sender_pubkey_hash: Vec<u8> =
            hex::decode("1925a274ac004373bb5429553bdb55c40e57b124").unwrap();
        let secret_hash: Vec<u8> = hex::decode(
            "51a488e06e9c69c555b8ad5e2c4629bb3135b96accd1f23451af75e06d3aee9c",
        ).unwrap();

        let script = create_htlc_redeem_script(
            &recipient_pubkey_hash,
            &sender_pubkey_hash,
            &secret_hash,
            &BlockHeight::new(900),
        );

        let address = Address::p2wsh(&script, Network::BitcoinCoreRegtest);
        assert_eq!(
            address.to_string(),
            "bcrt1qs2aderg3whgu0m8uadn6dwxjf7j3wx97kk2qqtrum89pmfcxknhsf89pj0"
        );
        // I did a bitcoin-rpc validateaddress
        // -> witness_program returned = sha256 of htlc script
        // Hence I guess it's correct!
    }
}
