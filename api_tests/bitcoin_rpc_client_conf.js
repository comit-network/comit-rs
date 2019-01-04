const BitcoinRpcClient = require("bitcoin-core");

let _bitcoin_rpc_client;

function create_bitcoin_rpc_client() {
    const btc_config = global.harness.ledgers_config.bitcoin;
    if (!btc_config) {
        throw new Error("ledger.bitcoin configuration is needed");
    }
    return (_bitcoin_rpc_client =
        _bitcoin_rpc_client ||
        new BitcoinRpcClient({
            network: "regtest",
            port: btc_config.rpc_port,
            host: btc_config.rpc_host,
            username: btc_config.rpc_username,
            password: btc_config.rpc_password,
        }));
}

module.exports.create_client = () => {
    return create_bitcoin_rpc_client();
};

module.exports.btc_generate = async function(num = 1) {
    return create_bitcoin_rpc_client().generate(num);
};

module.exports.btc_activate_segwit = async function() {
    return create_bitcoin_rpc_client().generate(432);
};

module.exports.btc_import_address = async function(address) {
    return create_bitcoin_rpc_client().importAddress(address);
};