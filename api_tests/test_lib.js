const ethutil = require("ethereumjs-util");
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

async function btc_balance(address) {
    let btc_balance = await bitcoin_rpc_client.getReceivedByAddress(address);
    return parseFloat(btc_balance) * 100000000;
}

module.exports.btc_balance = async function(address) {
    return btc_balance(address);
};

module.exports.btc_import_address = async function(address) {
    return bitcoin_rpc_client.importAddress(address);
};

async function eth_balance(address) {
    return web3.eth
        .getBalance(address)
        .then(balance => new ethutil.BN(balance, 10));
}

module.exports.eth_balance = async function(address) {
    return eth_balance(address);
};

module.exports.erc20_balance = async function(
    token_holder_address,
    contract_address
) {
    const function_identifier = "70a08231";

    const padded_address = token_holder_address
        .replace(/^0x/, "")
        .padStart(64, "0");
    const payload = "0x" + function_identifier + padded_address;

    const tx = {
        from: token_holder_address,
        to: contract_address,
        data: payload,
    };

    let hex_balance = await web3.eth.call(tx);
    return web3.utils.toBN(hex_balance);
};

module.exports.log_eth_balance = async function(
    when,
    player,
    address,
    address_type
) {
    logger.info(
        "%s the swap, %s has %s wei at the %s address %s",
        when,
        player,
        await eth_balance(address),
        address_type,
        address
    );
};

module.exports.log_btc_balance = async function(
    when,
    player,
    address,
    address_type
) {
    logger.info(
        "%s the swap, %s has %s satoshis at the %s address %s",
        when,
        player,
        await btc_balance(address),
        address_type,
        address
    );
};
