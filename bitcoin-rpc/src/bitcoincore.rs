use jsonrpc::HTTPClient;
use jsonrpc::{JsonRpcVersion, RpcClient, RpcRequest, RpcResponse};
use jsonrpc::header::{Authorization, Basic, Headers};
use types::Address;
use jsonrpc::HTTPError;
use types::*;

struct BitcoinCoreClient {
    client: RpcClient,
}

#[allow(dead_code)]
impl BitcoinCoreClient {
    pub fn new() -> Self {
        let mut headers = Headers::new();
        headers.set(Authorization(Basic {
            username: "user".to_string(),
            password: Some("password".to_string()),
        }));

        let client = HTTPClient::builder()
            .default_headers(headers)
            .build()
            .unwrap();

        let rpc_client = RpcClient::new(client, "http://127.0.0.1:8332");

        BitcoinCoreClient { client: rpc_client }
    }

    // Order as per: https://bitcoin.org/en/developer-reference#rpcs

    fn add_multisig_address(
        &self,
        number_of_required_signatures: u32,
        participants: Vec<Address>,
    ) -> Result<RpcResponse<MultiSigAddress>, HTTPError> {
        self.client.send(RpcRequest::new2(
            JsonRpcVersion::V1,
            "test",
            "addmultisigaddress",
            number_of_required_signatures,
            participants,
        ))
    }

    // TODO: abandontransaction
    // TODO: addmultisigaddress
    // TODO: addnode
    // TODO: addwitnessaddress
    // TODO: backupwallet
    // TODO: bumpfee
    // TODO: clearbanned
    // TODO: createmultisig
    // TODO: createrawtransaction
    // TODO: decoderawtransaction

    fn decode_script(&self, script: RedeemScript) -> Result<RpcResponse<DecodedScript>, HTTPError> {
        self.client.send(RpcRequest::new1(
            JsonRpcVersion::V1,
            "test",
            "decodescript",
            script,
        ))
    }

    // TODO: disconnectnode
    // TODO: dumpprivkey
    // TODO: dumpwallet
    // TODO: encryptwallet
    // TODO: estimatefee
    // TODO: estimatepriority
    // TODO: fundrawtransaction

    fn generate(&self, number_of_blocks: i32) -> Result<RpcResponse<Vec<BlockHash>>, HTTPError> {
        self.client.send(RpcRequest::new1(
            JsonRpcVersion::V1,
            "test",
            "generate",
            number_of_blocks,
        ))
    }

    // TODO: generatetoaddress
    // TODO: getaccountaddress

    fn get_account(&self, address: Address) -> Result<RpcResponse<Account>, HTTPError> {
        self.client.send(RpcRequest::new1(
            JsonRpcVersion::V1,
            "test",
            "getaccount",
            address,
        ))
    }

    // TODO: getaddednodeinfo
    // TODO: getaddressesbyaccount
    // TODO: getbalance
    // TODO: getbestblockhash

    fn get_block(&self, header_hash: &BlockHash) -> Result<RpcResponse<Block>, HTTPError> {
        self.client.send(RpcRequest::new1(
            JsonRpcVersion::V1,
            "test",
            "getblock",
            header_hash,
        ))
    }

    // TODO: getblockchaininfo

    fn get_block_count(&self) -> Result<RpcResponse<i32>, HTTPError> {
        self.client.send(RpcRequest::new0(
            JsonRpcVersion::V1,
            "test",
            "getblockcount",
        ))
    }

    // TODO: getblockhash
    // TODO: getblockheader
    // TODO: getblocktemplate
    // TODO: getchaintips
    // TODO: getconnectioncount
    // TODO: getdifficulty
    // TODO: getgenerate
    // TODO: gethashespersec
    // TODO: getinfo
    // TODO: getmemoryinfo
    // TODO: getmempoolancestors
    // TODO: getmempooldescendants
    // TODO: getmempoolentry
    // TODO: getmempoolinfo
    // TODO: getmininginfo
    // TODO: getnettotals
    // TODO: getnetworkhashesps
    // TODO: getnetworkinfo

    fn get_new_address(&self) -> Result<RpcResponse<Address>, HTTPError> {
        self.client.send(RpcRequest::new0(
            JsonRpcVersion::V1,
            "test",
            "getnewaddress",
        ))
    }

    // TODO: getpeerinfo
    // TODO: getrawchangeaddress
    // TODO: getrawmempool
    // TODO: getrawtransaction
    // TODO: getreceivedbyaccount
    // TODO: getreceivedbyaddress

    fn get_transaction(&self, tx: TransactionId) -> Result<RpcResponse<Transaction>, HTTPError> {
        self.client.send(RpcRequest::new1(
            JsonRpcVersion::V1,
            "test",
            "gettransaction",
            tx,
        ))
    }

    // TODO: gettxout
    // TODO: gettxoutsetinfo
    // TODO: getunconfirmedbalance
    // TODO: getwalletinfo
    // TODO: getwork
    // TODO: importaddress
    // TODO: importmulti
    // TODO: importprivkey
    // TODO: importprunedfunds
    // TODO: importwallet
    // TODO: keypoolrefill
    // TODO: invalidateblock
    // TODO: keypoolrefill
    // TODO: listaccounts
    // TODO: listaddressgroupings
    // TODO: listbanned
    // TODO: listlockunspent
    // TODO: listreceivedbyaccount
    // TODO: listreceivedbyaddress
    // TODO: listsinceblock
    // TODO: listtransactions
    // TODO: listunspent
    // TODO: lockunspent
    // TODO: move
    // TODO: ping
    // TODO: preciousblock
    // TODO: prioritisetransaction
    // TODO: pruneblockchain
    // TODO: removeprunedfunds
    // TODO: sendfrom
    // TODO: sendmany

    fn send_raw_transaction(
        &self,
        tx_data: RawTransactionHex,
    ) -> Result<RpcResponse<Transaction>, HTTPError> {
        self.client.send(RpcRequest::new1(
            JsonRpcVersion::V1,
            "test",
            "sendrawtransaction",
            tx_data,
        ))
    }

    // TODO: sendtoaddress
    // TODO: setaccount
    // TODO: setban
    // TODO: setgenerate
    // TODO: setnetworkactive
    // TODO: settxfee
    // TODO: signmessage
    // TODO: signmessagewithprivkey
    // TODO: signrawtransaction
    // TODO: stop
    // TODO: submitblock
    // TODO: validateaddress
    // TODO: verifychain
    // TODO: verifymessage
    // TODO: verifytxoutproof
    // TODO: walletlock
    // TODO: walletpassphrase
    // TODO: walletpassphrasechange
}

#[cfg(test)]
mod tests {

    extern crate env_logger;

    use super::*;
    use jsonrpc::RpcError;
    use std::fmt::Debug;

    #[test]
    fn test_add_multisig_address() {
        let _ = env_logger::try_init();
        assert_successful_result(|client| {
            let address = client.get_new_address().unwrap().into_result().unwrap();

            client.add_multisig_address(1, vec![address])
        })
    }

    #[test]
    fn test_get_block_count() {
        let _ = env_logger::try_init();
        assert_successful_result(BitcoinCoreClient::get_block_count)
    }

    #[test]
    fn test_get_new_address() {
        let _ = env_logger::try_init();
        assert_successful_result(BitcoinCoreClient::get_new_address)
    }

    #[test]
    fn test_generate() {
        let _ = env_logger::try_init();
        assert_successful_result(|client| client.generate(1))
    }

    #[test]
    fn test_getaccount() {
        let _ = env_logger::try_init();
        assert_successful_result(|client| {
            client.get_account(Address::from("2N2PMtfaKc9knQYxmTZRg3dcC1SMZ7SC8PW"))
        })
    }

    #[test]
    fn test_gettransaction() {
        let _ = env_logger::try_init();
        assert_successful_result(|client| {
            client.get_transaction(TransactionId::from(
                "70935ecf77405bccda14ed73a7e2d79f0bb75e0b1c06b8f1c3c2e3f6b600ff46",
            ))
        })
    }

    #[test]
    fn test_getblock() {
        let _ = env_logger::try_init();
        let generated_blocks = BitcoinCoreClient::new()
            .generate(1)
            .unwrap()
            .into_result()
            .unwrap();
        let block_hash = generated_blocks.get(0).unwrap().to_owned();

        assert_successful_result(|client| client.get_block(block_hash))
    }

    #[test]
    fn test_decode_script() {
        let _ = env_logger::try_init();

        assert_successful_result(|client| {
            client.decode_script(RedeemScript::from("522103ede722780d27b05f0b1169efc90fa15a601a32fc6c3295114500c586831b6aaf2102ecd2d250a76d204011de6bc365a56033b9b3a149f679bc17205555d3c2b2854f21022d609d2f0d359e5bc0e5d0ea20ff9f5d3396cb5b1906aa9c56a0e7b5edc0c5d553ae"))
        })
    }

    fn assert_successful_result<R, I>(invocation: I)
    where
        R: Debug,
        I: Fn(&BitcoinCoreClient) -> Result<RpcResponse<R>, HTTPError>,
    {
        let client = BitcoinCoreClient::new();
        let result: Result<R, RpcError> = invocation(&client).unwrap().into();

        if result.is_err() {
            error!("{:?}", result.unwrap_err());
            panic!("Result should be successful")
        } else {
            // Having a successful result means:
            // - No HTTP Error occured
            // - No deserialization error occured
            debug!("{:?}", result.unwrap())
        }
    }
}
