const bitcoin_rpc_client_conf = require("./bitcoin_rpc_client_conf.js");
const web3_conf = require("./web3_conf.js");

const web3 = web3_conf.create();

const logger = global.harness.logger;
module.exports.logger = function() {
    return logger;
};

async function sleep(time) {
    return new Promise((res, rej) => {
        setTimeout(res, time);
    });
}

module.exports.sleep = sleep;

const bitcoin_rpc_client = bitcoin_rpc_client_conf.create_client();

{
    const function_identifier = "40c10f19";
    module.exports.mint_erc20_tokens = (
        owner_wallet,
        contract_address,
        to_address,
        amount
    ) => {
        to_address = to_address.replace(/^0x/, "").padStart(64, "0");
        amount = web3.utils
            .numberToHex(amount)
            .replace(/^0x/, "")
            .padStart(64, "0");
        const payload = "0x" + function_identifier + to_address + amount;

        return owner_wallet.send_eth_transaction_to(
            contract_address,
            payload,
            "0x0"
        );
    };
}

module.exports.btc_generate = async function(num = 1) {
    return bitcoin_rpc_client.generate(num);
};

module.exports.btc_activate_segwit = async function() {
    return bitcoin_rpc_client.generate(432);
};

module.exports.btc_import_address = async function(address) {
    return bitcoin_rpc_client.importAddress(address);
};
