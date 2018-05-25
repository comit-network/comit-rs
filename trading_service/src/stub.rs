// Types and things that are TODO

use bitcoin_rpc;
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EthAddress(pub String);
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EthTimeDelta(pub u32); // Measured in seconds
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct BtcBlockHeight(pub u32);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BtcHtlc {
    success_address: bitcoin_rpc::Address,
    refund_address: bitcoin_rpc::Address,
    timelock: BtcBlockHeight,
    address: Option<bitcoin_rpc::Address>,
}

// The actual implemtation will have to go in a common library
impl BtcHtlc {
    pub fn new(
        success_address: bitcoin_rpc::Address,
        refund_address: bitcoin_rpc::Address,
        timelock: BtcBlockHeight,
    ) -> BtcHtlc {
        BtcHtlc {
            success_address,
            refund_address,
            timelock,
            address: None,
        }
    }

    pub fn address(&self) -> bitcoin_rpc::Address {
        bitcoin_rpc::Address::from("TODO")
    }
}
