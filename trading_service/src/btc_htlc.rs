use bitcoin::blockdata::script::{Builder, Script};
use bitcoin::util;
use bitcoin::util::address::Payload::WitnessProgram;
use std::str::FromStr;
// All opcodes
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

pub fn create_redeem_script(pubkey_hash: &Vec<u8>) -> Script {
    Builder::new()
        .push_opcode(OP_DUP)
        .push_opcode(OP_HASH160)
        .push_slice(pubkey_hash)
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
    fn given_a_vec_u8_pubkey_hash_return_string_smart_contract() {
        let pubkey_hash: Vec<u8> = vec![
            192, 33, 241, 123, 233, 156, 106, 223, 188, 186, 93, 56, 238, 13, 41, 44, 3, 153, 210,
            245,
        ];
        let script = create_redeem_script(&pubkey_hash);

        assert_eq!(
            script.into_vec(),
            hex::decode("76a914c021f17be99c6adfbcba5d38ee0d292c0399d2f588ac").unwrap()
        );
    }

    #[test]
    fn given_a_script_return_p2wsh() {
        let pubkey_hash: Vec<u8> = vec![
            192, 33, 241, 123, 233, 156, 106, 223, 188, 186, 93, 56, 238, 13, 41, 44, 3, 153, 210,
            245,
        ];
        let script = create_redeem_script(&pubkey_hash);

        let address = bitcoin::util::address::Address::p2wsh(
            &script,
            network::constants::Network::BitcoinCoreRegtest,
        );
        assert_eq!(
            address.to_string(),
            "bcrt1q5072z6s48j5s8rkz2amujxplty8fn5tgletyam4hct7rgzf75j3sdvdcjs"
        );
    }
}
