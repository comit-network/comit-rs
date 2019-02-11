const bitcoin = require("./bitcoin.js");
const ethereum = require("./ethereum.js");
const omnilayer = require("./omnilayer.js");

const logger = global.harness.logger;

class Wallet {
    constructor(owner) {
        this.owner = owner;
        this._eth_wallet = ethereum.create();
        this._btc_wallet = bitcoin.create_wallet();
        this._omni_wallet = omnilayer.create_wallet();

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

    omni() {
        return this._omni_wallet
    }
}

module.exports.create = owner => {
    return new Wallet(owner);
};
