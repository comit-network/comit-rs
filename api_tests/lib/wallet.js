const util = require("./util.js");
const bitcoin = require("./bitcoin.js");
const ethereum = require("./ethereum.js");

const logger = util.logger();

class Wallet {
    constructor(owner) {
        this.owner = owner;
        this._eth_wallet = ethereum.create();
        this._btc_wallet = bitcoin.create_wallet();

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
}

module.exports.create = owner => {
    return new Wallet(owner);
};
