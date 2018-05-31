use bitcoin::blockdata::script::{Builder, Script};
use bitcoin::util;
use bitcoin::util::address::Payload::WitnessProgram;
use std::str::FromStr;
// All opcodes
use bitcoin::blockdata::opcodes::All::OP_NOP2 as OP_CHECKLOCKTIMEVERIFY;
use bitcoin::blockdata::opcodes::All::*;

// Create BTC HTLC
// Returns P2WSH address
// Input:
// - BTC address of the exchange to receive the funds (exchange_success_address)
// - BTC timeout
// - BTC amount
// - hashed secret

//TODO: move to nice error handling
pub fn get_pub_key_from_address(address: String) -> Option<Vec<u8>> {
    let address = util::address::Address::from_str(address.as_str());
    let address = match address {
        Ok(a) => a,
        Err(e) => panic!("{:?}", e),
    };
    match address.payload {
        WitnessProgram(witness) => Some(witness.program().to_vec()),
        _ => None,
    }
}

pub fn u8_to_hex(vec: &Vec<u8>) -> String {
    let mut s = String::new();
    for i in vec {
        // 02x -> always output 2 chars, left pad with zero if needed
        s.push_str(&format!("{:02x}", i));
    }
    s
}

//TODO: Example function, can be deleted
pub fn create_simple_redeem_script(pubkey_hash: &Vec<u8>) -> Script {
    Builder::new()
        .push_opcode(OP_DUP)
        .push_opcode(OP_HASH160)
        .push_slice(pubkey_hash)
        .push_opcode(OP_EQUALVERIFY)
        .push_opcode(OP_CHECKSIG)
        .into_script()
}

pub fn create_htlc_redeem_script(
    recipient_pubkey_hash: &Vec<u8>,
    sender_pubkey_hash: &Vec<u8>,
    secret_hash: &Vec<u8>,
    redeem_block_height: i64,
) -> Script {
    /*
    OP_IF,
    OP_SHA256, h, OP_EQUALVERIFY,
    OP_DUP, OP_HASH160, recipientpubkey,
    OP_ELSE,
    redeemblocknum, OP_CHECKLOCKTIMEVERIFY, OP_DROP,
    OP_DUP, OP_HASH160,senderpubkey,
    OP_ENDIF,
    OP_EQUALVERIFY, OP_CHECKSIG
    */

    Builder::new()
        .push_opcode(OP_IF)
        .push_opcode(OP_SHA256)
        .push_slice(secret_hash)
        .push_opcode(OP_EQUALVERIFY)
        .push_opcode(OP_DUP)
        .push_opcode(OP_HASH160)
        .push_slice(recipient_pubkey_hash)
        .push_opcode(OP_ELSE)
        .push_int(redeem_block_height)
        .push_opcode(OP_CHECKLOCKTIMEVERIFY)
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
    use bitcoin;
    use bitcoin::network;
    use hex;

    #[test]
    fn given_an_address_return_pubkey_hash() {
        let address = String::from_str("bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap");
        let pubkey_hash = get_pub_key_from_address(address.unwrap()).unwrap();

        assert_eq!(
            pubkey_hash,
            hex::decode("c021f17be99c6adfbcba5d38ee0d292c0399d2f5").unwrap()
        );
    }

    #[test]
    fn given_a_vec_u8_return_hex_string() {
        let vec: Vec<u8> = vec![
            192, 33, 241, 123, 233, 156, 106, 223, 188, 186, 93, 56, 238, 13, 41, 44, 3, 153, 210,
            245,
        ];
        let str = u8_to_hex(&vec);

        assert_eq!(str, "c021f17be99c6adfbcba5d38ee0d292c0399d2f5");
    }

    #[test]
    fn given_a_vec_u8_pubkey_hash_return_string_simple_redeem_script() {
        let pubkey_hash: Vec<u8> = vec![
            192, 33, 241, 123, 233, 156, 106, 223, 188, 186, 93, 56, 238, 13, 41, 44, 3, 153, 210,
            245,
        ];
        let script = create_simple_redeem_script(&pubkey_hash);

        assert_eq!(
            script.into_vec(),
            hex::decode("76a914c021f17be99c6adfbcba5d38ee0d292c0399d2f588ac").unwrap()
        );
    }

    #[test]
    fn given_a_simple_redeem_script_return_p2wsh() {
        let pubkey_hash: Vec<u8> = vec![
            192, 33, 241, 123, 233, 156, 106, 223, 188, 186, 93, 56, 238, 13, 41, 44, 3, 153, 210,
            245,
        ];
        let script = create_simple_redeem_script(&pubkey_hash);

        let address = bitcoin::util::address::Address::p2wsh(
            &script,
            network::constants::Network::BitcoinCoreRegtest,
        );
        assert_eq!(
            address.to_string(),
            "bcrt1q5072z6s48j5s8rkz2amujxplty8fn5tgletyam4hct7rgzf75j3sdvdcjs"
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

    // htlc script: 63a82051a488e06e9c69c555b8ad5e2c4629bb3135b96accd1f23451af75e06d3aee9c8876a914c021f17be99c6adfbcba5d38ee0d292c0399d2f567028403b17576a9141925a274ac004373bb5429553bdb55c40e57b1246888ac
    // sha256 of htlc script: e6877a670b46b9913bdaed47084f2db8983c2a22c473f0aea1fa5c2ebc4fd8d4

    #[test]
    fn given_a_vec_u8_pubkey_hash_return_string_htlc_script() {
        let recipient_pubkey_hash: Vec<u8> = vec![
            192, 33, 241, 123, 233, 156, 106, 223, 188, 186, 93, 56, 238, 13, 41, 44, 3, 153, 210,
            245,
        ];
        let sender_pubkey_hash: Vec<u8> = vec![
            25, 37, 162, 116, 172, 0, 67, 115, 187, 84, 41, 85, 59, 219, 85, 196, 14, 87, 177, 36
        ];

        let secret_hash: Vec<u8> = vec![
            81, 164, 136, 224, 110, 156, 105, 197, 85, 184, 173, 94, 44, 70, 41, 187, 49, 53, 185,
            106, 204, 209, 242, 52, 81, 175, 117, 224, 109, 58, 238, 156,
        ];

        let script = create_htlc_redeem_script(
            &recipient_pubkey_hash,
            &sender_pubkey_hash,
            &secret_hash,
            900,
        );

        assert_eq!(
            script.into_vec(),
            hex::decode(
                "63a82051a488e06e9c69c555b8ad5e2c4629bb3135b96accd1f23451af75e06d3aee\
                 9c8876a914c021f17be99c6adfbcba5d38ee0d292c0399d2f567028403b17576a914\
                 1925a274ac004373bb5429553bdb55c40e57b1246888ac"
            ).unwrap()
        );
    }

    #[test]
    fn given_an_htlc_redeem_script_return_p2wsh() {
        let recipient_pubkey_hash: Vec<u8> = vec![
            192, 33, 241, 123, 233, 156, 106, 223, 188, 186, 93, 56, 238, 13, 41, 44, 3, 153, 210,
            245,
        ];
        let sender_pubkey_hash: Vec<u8> = vec![
            25, 37, 162, 116, 172, 0, 67, 115, 187, 84, 41, 85, 59, 219, 85, 196, 14, 87, 177, 36
        ];

        let secret_hash: Vec<u8> = vec![
            81, 164, 136, 224, 110, 156, 105, 197, 85, 184, 173, 94, 44, 70, 41, 187, 49, 53, 185,
            106, 204, 209, 242, 52, 81, 175, 117, 224, 109, 58, 238, 156,
        ];

        let script = create_htlc_redeem_script(
            &recipient_pubkey_hash,
            &sender_pubkey_hash,
            &secret_hash,
            900,
        );

        let address = bitcoin::util::address::Address::p2wsh(
            &script,
            network::constants::Network::BitcoinCoreRegtest,
        );
        assert_eq!(
            address.to_string(),
            "bcrt1qu6rh5ectg6uezw76a4rssnedhzvrc23zc3elpt4plfwza0z0mr2qp8u39k"
        );
        // I did a bitcoin-rpc validateaddress
        // -> witness_program returned = sha256 of htlc script
        // Hence I guess it's correct!
    }
}
