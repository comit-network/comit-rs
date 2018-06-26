use bitcoin::network::constants::Network;
use bitcoin::util::address::Address;
use bitcoin::util::bip32::ChildNumber;
use bitcoin::util::bip32::Error;
use bitcoin::util::bip32::ExtendedPubKey;
use secp256k1::Secp256k1;

pub struct HdAddressGenerator {
    secp: Secp256k1,
    xpubkey: ExtendedPubKey,
    last_index: u32,
    // Do we want to remember already generated addresses or regenerate them?
    // Memory vs CPU -> could be a switch/option
    // Common practice for wallets is to pre-generate some addresses, hence:
    // TODO: pre-generate and remember addresses
}

impl HdAddressGenerator {
    pub fn new(xpubkey: ExtendedPubKey) -> HdAddressGenerator {
        HdAddressGenerator {
            secp: Secp256k1::new(),
            xpubkey,
            last_index: 0,
        }
    }

    pub fn new_pubkey(&mut self) -> Result<ExtendedPubKey, Error> {
        let res = self.xpubkey
            .ckd_pub(&self.secp, ChildNumber::Normal(self.last_index));
        if res.is_ok() {
            self.last_index += 1;
        }
        res
    }

    pub fn new_address(&mut self, network: Network) -> Result<Address, Error> {
        let pubkey = self.new_pubkey();
        match pubkey {
            Err(e) => return Err(e),
            Ok(pubkey) => {
                // Using P2SH-WPKH (Legacy address wrapping SegWit)
                // which is the most popular type of address at the moment
                return Ok(Address::p2shwpkh(&pubkey.public_key, network));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn given_bip32_vector1_m0h_pubkey_return_correct_m0h1_pubkey() {
        // See https://github.com/bitcoin/bips/blob/master/bip-0032.mediawiki Test vector 1
        // Chain m/0H
        let pub_key = ExtendedPubKey::from_str("xpub68Gmy5EdvgibQVfPdqkBBCHxA5htiqg55crXYuXoQRKfDBFA1WEjWgP6LHhwBZeNK1VTsfTFUHCdrfp1bgwQ9xv5ski8PX9rL2dZXvgGDnw").unwrap();
        // Chain m/0H/1
        let expected_pubkey = ExtendedPubKey::from_str("xpub6ASuArnXKPbfEwhqN6e3mwBcDTgzisQN1wXN9BJcM47sSikHjJf3UFHKkNAWbWMiGj7Wf5uMash7SyYq527Hqck2AxYysAA7xmALppuCkwQ").unwrap();

        let mut add_gen = HdAddressGenerator::new(pub_key);
        // Chain m/0H/0 (discard)
        let _ = add_gen.new_pubkey();

        assert_eq!(add_gen.new_pubkey(), Ok(expected_pubkey));
    }

}
