const ethutil = require("ethereumjs-util");
const bitcoin_rpc_client_conf = require("./bitcoin_rpc_client_conf.js");
const test_lib = require("./test_lib.js");
const web3_conf = require("./web3_conf.js");

const web3 = web3_conf.create();

const bitcoin_rpc_client = bitcoin_rpc_client_conf.create_client();

async function btc_balance(address) {
    let btc_balance = await bitcoin_rpc_client.getReceivedByAddress(address);
    return parseFloat(btc_balance) * 100000000;
}

module.exports.btc_balance = async function(address) {
    return btc_balance(address);
};

module.exports.log_btc_balance = async function(
    when,
    player,
    address,
    address_type
) {
    test_lib
        .logger()
        .info(
            "%s the swap, %s has %s satoshis at the %s address %s",
            when,
            player,
            await btc_balance(address),
            address_type,
            address
        );
};

async function eth_balance(address) {
    return web3.eth
        .getBalance(address)
        .then(balance => new ethutil.BN(balance, 10));
}

module.exports.eth_balance = async function(address) {
    return eth_balance(address);
};

module.exports.log_eth_balance = async function(
    when,
    player,
    address,
    address_type
) {
    test_lib
        .logger()
        .info(
            "%s the swap, %s has %s wei at the %s address %s",
            when,
            player,
            await eth_balance(address),
            address_type,
            address
        );
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
