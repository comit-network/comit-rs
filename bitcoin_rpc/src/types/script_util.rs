use bitcoin::blockdata::opcodes::All;
use bitcoin::blockdata::opcodes::All::*;
use bitcoin::blockdata::script::Builder;
use bitcoin::blockdata::script::Script;
use regex::Regex;
use std::num;
use std_hex;

#[derive(Debug)]
pub enum Error {
    UnknownOpCode,
    IntConversionFail(num::ParseIntError),
    HexConversionFail(std_hex::FromHexError),
}

pub fn script_from_str(str: &str) -> Result<Script, Error> {
    let iter = str.split(" ");
    let mut script = Builder::new();
    for s in iter {
        script = push_from_str(script, &s)?;
    }
    Ok(script.into_script())
}

pub fn push_from_str(builder: Builder, s: &str) -> Result<Builder, Error> {
    let op_re = Regex::new(r"^OP_.+$").unwrap();
    if op_re.is_match(s) {
        let builder = builder.push_opcode(opcode_from_str(s)?);
        return Ok(builder);
    }

    let int_re = Regex::new(r"^[0-9]+$").unwrap();
    if int_re.is_match(s) {
        let int = s.parse::<i64>()
            .map_err(|err| Error::IntConversionFail(err))?; // happens if bigger than i64
        let builder = builder.push_int(int);
        return Ok(builder);
    }
    let hex_re = Regex::new(r"^[A-Fa-f0-9]+$").unwrap();
    if hex_re.is_match(s) {
        let hex = std_hex::decode(s).map_err(|err| Error::HexConversionFail(err))?;
        let builder = builder.push_slice(hex.as_ref());
        return Ok(builder);
    }
    Ok(builder)
}

pub fn opcode_from_str(s: &str) -> Result<All, Error> {
    match s {
        // 0x00 An empty array of bytes is pushed onto the stack. (This is not a no-op: an item is added to the stack.)
        "OP_0" => Ok(OP_PUSHBYTES_0),
        "OP_FALSE" => Ok(OP_PUSHBYTES_0),
        // 0x4c The next byte contains the number of bytes to be pushed onto the stack.
        "OP_PUSHDATA1" => Ok(OP_PUSHDATA1),
        // 0x4d The next two bytes contain the number of bytes to be pushed onto the stack in little endian order.
        "OP_PUSHDATA2" => Ok(OP_PUSHDATA2),
        // 0x4e The next four bytes contain the number of bytes to be pushed onto the stack in little endian order.
        "OP_PUSHDATA4" => Ok(OP_PUSHDATA4),
        // 0x4f The number -1 is pushed onto the stack.
        "OP_1NEGATE" => Ok(OP_PUSHNUM_NEG1),
        // 0x51 The number 1 is pushed onto the stack.
        "OP_TRUE" => Ok(OP_PUSHNUM_1),
        "OP_1" => Ok(OP_PUSHNUM_1),
        // 0x52 Push 0x02 onto the stack.
        "OP_2" => Ok(OP_PUSHNUM_2),
        // 0x53 Push 0x03 onto the stack.
        "OP_3" => Ok(OP_PUSHNUM_3),
        // 0x54 Push 0x04 onto the stack.
        "OP_4" => Ok(OP_PUSHNUM_4),
        // 0x55 Push 0x05] onto the stack.
        "OP_5" => Ok(OP_PUSHNUM_5),
        // 0x56 Push 0x06 onto the stack.
        "OP_6" => Ok(OP_PUSHNUM_6),
        // 0x57 Push 0x07 onto the stack.
        "OP_7" => Ok(OP_PUSHNUM_7),
        // 0x58 Push 0x08 onto the stack.
        "OP_8" => Ok(OP_PUSHNUM_8),
        // 0x59 Push 0x09 onto the stack.
        "OP_9" => Ok(OP_PUSHNUM_9),
        // 0x5a Push 0x10 onto the stack.
        "OP_10" => Ok(OP_PUSHNUM_10),
        // 0x5b Push 0x11 onto the stack.
        "OP_11" => Ok(OP_PUSHNUM_11),
        // 0x5c Push 0x12 onto the stack.
        "OP_12" => Ok(OP_PUSHNUM_12),
        // 0x5d Push 0x13 onto the stack.
        "OP_13" => Ok(OP_PUSHNUM_13),
        // 0x5e Push 0x14 onto the stack.
        "OP_14" => Ok(OP_PUSHNUM_14),
        // 0x5f Push 0x15 onto the stack.
        "OP_15" => Ok(OP_PUSHNUM_15),
        // 0x60 Push 0x16 onto the stack.
        "OP_16" => Ok(OP_PUSHNUM_16),
        // 0x61 Does nothing.
        "OP_NOP" => Ok(OP_NOP),
        // 0x63 If the top stack value is not False, the statements are executed. The top stack value is removed.
        "OP_IF" => Ok(OP_IF),
        // 0x64 If the top stack value is False, the statements are executed. The top stack value is removed.
        "OP_NOTIF" => Ok(OP_NOTIF),
        // 0x65 Transaction is invalid even when occuring in an unexecuted OP_IF branch
        "OP_VERIF" => Ok(OP_VERIF),
        // 0x66 Transaction is invalid even when occuring in an unexecuted OP_IF branch
        "OP_VERNOTIF" => Ok(OP_VERNOTIF),
        // 0x67 If the preceding OP_IF or OP_NOTIF or OP_ELSE was not executed then these statements are and if the preceding OP_IF or OP_NOTIF or OP_ELSE was executed then these statements are not.
        "OP_ELSE" => Ok(OP_ELSE),
        // 0x68 Ends an if/else block. All blocks must end, or the transaction is invalid. An OP_ENDIF without OP_IF earlier is also invalid.
        "OP_ENDIF" => Ok(OP_ENDIF),
        // 0x69 Marks transaction as invalid if top stack value is not true. The top stack value is removed.
        "OP_VERIFY" => Ok(OP_VERIFY),
        // 0x6a Marks transaction as invalid.
        "OP_RETURN" => Ok(OP_RETURN),
        // 0x6b Puts the input onto the top of the alt stack. Removes it from the main stack.
        "OP_TOALTSTACK" => Ok(OP_TOALTSTACK),
        /*
        0x6b,
        /// Pop one element from the alt stack onto the main stack
        OP_FROMALTSTACK =
        0x6c,
        /// Drops the top two stack items
        OP_2DROP =
        0x6d,
        /// Duplicates the top two stack items as AB -> ABAB
        OP_2DUP =
        0x6e,
        /// Duplicates the two three stack items as ABC -> ABCABC
        OP_3DUP =
        0x6f,
        /// Copies the two stack items of items two spaces back to
    /// the front, as xxAB -> ABxxAB
        OP_2OVER =
        0x70,
        /// Moves the two stack items four spaces back to the front,
    /// as xxxxAB -> ABxxxx
        OP_2ROT =
        0x71,
        /// Swaps the top two pairs, as ABCD -> CDAB
        OP_2SWAP =
        0x72,
        /// Duplicate the top stack element unless it is zero
        OP_IFDUP =
        0x73,
        /// Push the current number of stack items onto te stack
        OP_DEPTH =
        0x74,
        /// Drops the top stack item
        OP_DROP =
        0x75,
        /// Duplicates the top stack item
        OP_DUP =
        0x76,
        /// Drops the second-to-top stack item
        OP_NIP =
        0x77,
        /// Copies the second-to-top stack item, as xA -> AxA
        OP_OVER =
        0x78,
        /// Pop the top stack element as N. Copy the Nth stack element to the top
        OP_PICK =
        0x79,
        /// Pop the top stack element as N. Move the Nth stack element to the top
        OP_ROLL =
        0x7a,
        /// Rotate the top three stack items, as [top next1 next2] -> [next2 top next1]
        OP_ROT =
        0x7b,
        /// Swap the top two stack items
        OP_SWAP =
        0x7c,
        /// Copy the top stack item to before the second item, as [top next] -> [top next top]
        OP_TUCK =
        0x7d,
        /// Fail the script unconditionally, does not even need to be executed
        OP_CAT =
        0x7e,
        /// Fail the script unconditionally, does not even need to be executed
        OP_SUBSTR =
        0x7f,
        /// Fail the script unconditionally, does not even need to be executed
        OP_LEFT =
        0x80,
        /// Fail the script unconditionally, does not even need to be executed
        OP_RIGHT =
        0x81,
        /// Pushes the length of the top stack item onto the stack
        OP_SIZE =
        0x82,
        /// Fail the script unconditionally, does not even need to be executed
        OP_INVERT =
        0x83,
        /// Fail the script unconditionally, does not even need to be executed
        OP_AND =
        0x84,
        /// Fail the script unconditionally, does not even need to be executed
        OP_OR =
        0x85,
        /// Fail the script unconditionally, does not even need to be executed
        OP_XOR =
        0x86,
        /// Pushes 1 if the inputs are exactly equal, 0 otherwise
        OP_EQUAL =
        0x87,
        /// Returns success if the inputs are exactly equal, failure otherwise
        OP_EQUALVERIFY =
        0x88,
        /// Synonym for OP_RETURN
        OP_RESERVED1 =
        0x89,
        /// Synonym for OP_RETURN
        OP_RESERVED2 =
        0x8a,
        /// Increment the top stack element in place
        OP_1ADD =
        0x8b,
        /// Decrement the top stack element in place
        OP_1SUB =
        0x8c,
        /// Fail the script unconditionally, does not even need to be executed
        OP_2MUL =
        0x8d,
        /// Fail the script unconditionally, does not even need to be executed
        OP_2DIV =
        0x8e,
        /// Multiply the top stack item by -1 in place
        OP_NEGATE =
        0x8f,
        /// Absolute value the top stack item in place
        OP_ABS =
        0x90,
        /// Map 0 to 1 and everything else to 0, in place
        OP_NOT =
        0x91,
        /// Map 0 to 0 and everything else to 1, in place
        OP_0NOTEQUAL =
        0x92,
        /// Pop two stack items and push their sum
        OP_ADD =
        0x93,
        /// Pop two stack items and push the second minus the top
        OP_SUB =
        0x94,
        /// Fail the script unconditionally, does not even need to be executed
        OP_MUL =
        0x95,
        /// Fail the script unconditionally, does not even need to be executed
        OP_DIV =
        0x96,
        /// Fail the script unconditionally, does not even need to be executed
        OP_MOD =
        0x97,
        /// Fail the script unconditionally, does not even need to be executed
        OP_LSHIFT =
        0x98,
        /// Fail the script unconditionally, does not even need to be executed
        OP_RSHIFT =
        0x99,
        /// Pop the top two stack items and push 1 if both are nonzero, else push 0
        OP_BOOLAND =
        0x9a,
        /// Pop the top two stack items and push 1 if either is nonzero, else push 0
        OP_BOOLOR =
        0x9b,
        /// Pop the top two stack items and push 1 if both are numerically equal, else push 0
        OP_NUMEQUAL =
        0x9c,
        /// Pop the top two stack items and return success if both are numerically equal, else return failure
        OP_NUMEQUALVERIFY =
        0x9d,
        /// Pop the top two stack items and push 0 if both are numerically equal, else push 1
        OP_NUMNOTEQUAL =
        0x9e,
        /// Pop the top two items; push 1 if the second is less than the top, 0 otherwise
        OP_LESSTHAN =
        0x9f,
        /// Pop the top two items; push 1 if the second is greater than the top, 0 otherwise
        OP_GREATERTHAN =
        0xa0,
        /// Pop the top two items; push 1 if the second is <= the top, 0 otherwise
        OP_LESSTHANOREQUAL =
        0xa1,
        /// Pop the top two items; push 1 if the second is >= the top, 0 otherwise
        OP_GREATERTHANOREQUAL =
        0xa2,
        /// Pop the top two items; push the smaller
        OP_MIN =
        0xa3,
        /// Pop the top two items; push the larger
        OP_MAX =
        0xa4,
        /// Pop the top three items; if the top is >= the second and < the third, push 1, otherwise push 0
        OP_WITHIN =
        0xa5,
        /// Pop the top stack item and push its RIPEMD160 hash
        OP_RIPEMD160 =
        0xa6,
        /// Pop the top stack item and push its SHA1 hash
        OP_SHA1 =
        0xa7,
        /// Pop the top stack item and push its SHA256 hash
        OP_SHA256 =
        0xa8,
        /// Pop the top stack item and push its RIPEMD(SHA256) hash
        OP_HASH160 =
        0xa9,
        /// Pop the top stack item and push its SHA256(SHA256) hash
        OP_HASH256 =
        0xaa,
        /// Ignore this and everything preceding when deciding what to sign when signature-checking
        OP_CODESEPARATOR =
        0xab,
        /// https://en.bitcoin.it/wiki/OP_CHECKSIG pushing 1/0 for success/failure
        OP_CHECKSIG =
        0xac,
        /// https://en.bitcoin.it/wiki/OP_CHECKSIG returning success/failure
        OP_CHECKSIGVERIFY =
        0xad,
        /// Pop N, N pubkeys, M, M signatures, a dummy (due to bug in reference code), and verify that all M signatures are valid.
        */
        // 0xae Compares the first signature against each public key until it finds an ECDSA match.
        // Starting with the subsequent public key, it compares the second signature against each
        // remaining public key until it finds an ECDSA match. The process is repeated until all
        // signatures have been checked or not enough public keys remain to produce a successful result.
        // All signatures need to match a public key. Because public keys are not checked again if
        // they fail any signature comparison, signatures must be placed in the scriptSig using the
        // same order as their corresponding public keys were placed in the scriptPubKey or redeemScript.
        // If all signatures are valid, 1 is returned, 0 otherwise. Due to a bug, one extra unused value is removed from the stack.
        "OP_CHECKMULTISIG" => Ok(OP_CHECKMULTISIG),
        /*
        /// Like the above but return success/failure
        OP_CHECKMULTISIGVERIFY =
        0xaf,
        /// Does nothing
        OP_NOP1 =
        0xb0,
        /// Does nothing
        OP_NOP2 =
        0xb1,
        /// Does nothing
        OP_NOP3 =
        0xb2,
        /// Does nothing
        OP_NOP4 =
        0xb3,
        /// Does nothing
        OP_NOP5 =
        0xb4,
        /// Does nothing
        OP_NOP6 =
        0xb5,
        /// Does nothing
        OP_NOP7 =
        0xb6,
        /// Does nothing
        OP_NOP8 =
        0xb7,
        /// Does nothing
        OP_NOP9 =
        0xb8,
        /// Does nothing
        OP_NOP10 =
        0xb9,*/
        _ => Err(Error::UnknownOpCode),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opcode_from_str() {
        let str = "OP_1NEGATE";
        let opcode = opcode_from_str(&str).unwrap();

        assert_eq!(opcode, All::OP_PUSHNUM_NEG1);
    }

    #[test]
    fn test_simple_script_from_str() {
        let str = "OP_0 OP_1NEGATE OP_TRUE";
        let script = script_from_str(&str).unwrap();

        assert_eq!(
            script,
            Builder::new()
                .push_opcode(OP_PUSHBYTES_0)
                .push_opcode(OP_PUSHNUM_NEG1)
                .push_opcode(OP_PUSHNUM_1)
                .into_script()
        )
    }

    #[test]
    fn test_multisig_script_from_str() {
        let str = "2 03ede722780d27b05f0b1169efc90fa15a601a32fc6c3295114500c586831b6aaf 02ecd2d250a76d204011de6bc365a56033b9b3a149f679bc17205555d3c2b2854f 022d609d2f0d359e5bc0e5d0ea20ff9f5d3396cb5b1906aa9c56a0e7b5edc0c5d5 3 OP_CHECKMULTISIG";
        let script = script_from_str(&str).unwrap();

        assert_eq!(
            script,
            Builder::new()
                .push_opcode(OP_PUSHNUM_2)
                .push_slice(
                    std_hex::decode(
                        "03ede722780d27b05f0b1169efc90fa15a601a32fc6c3295114500c586831b6aaf"
                    ).unwrap()
                        .as_ref()
                )
                .push_slice(
                    std_hex::decode(
                        "02ecd2d250a76d204011de6bc365a56033b9b3a149f679bc17205555d3c2b2854f"
                    ).unwrap()
                        .as_ref()
                )
                .push_slice(
                    std_hex::decode(
                        "022d609d2f0d359e5bc0e5d0ea20ff9f5d3396cb5b1906aa9c56a0e7b5edc0c5d5"
                    ).unwrap()
                        .as_ref()
                )
                .push_opcode(OP_PUSHNUM_3)
                .push_opcode(OP_CHECKMULTISIG)
                .into_script()
        )
    }
}
