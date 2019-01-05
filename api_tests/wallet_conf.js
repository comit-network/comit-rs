const bitcoin_rpc_client_conf = require("./bitcoin_rpc_client_conf.js");
const bitcoin_wallet = require("./bitcoin_wallet.js");
const ethereum_wallet = require("./ethereum_wallet.js");
const test_lib = require("./test_lib.js");

const logger = test_lib.logger();

const bitcoin_rpc_client = bitcoin_rpc_client_conf.create_client();

class WalletConf {
    constructor(owner) {
        this.owner = owner;
        this._eth_wallet = ethereum_wallet.create();
        this._btc_wallet = bitcoin_wallet.create();

        logger.trace(
            "Generated eth address for %s is %s",
            this.owner,
            this._eth_wallet.address()
        );
        logger.trace(
            "Generated btc address for %s is %s",
            this.owner,
            this._btc_wallet.identity.address
        );
    }

    eth() {
        return this._eth_wallet;
    }

    btc() {
        return this._btc_wallet;
    }

    async send_raw_tx(hex) {
        return bitcoin_rpc_client.sendRawTransaction(hex);
    }
}

module.exports.create = owner => {
    return new WalletConf(owner);
};
