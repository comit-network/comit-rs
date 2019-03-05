const bitcoin = require("./bitcoin.js");
const ethereum = require("./ethereum.js");

class Wallet {
    constructor(owner, config) {
        this.owner = owner;
        // TODO: create lazily (think dry tests)
        this._eth_wallet = ethereum.create(config.ethConfig);
        this._btc_wallet = bitcoin.create_wallet();
    }

    eth() {
        return this._eth_wallet;
    }

    btc() {
        return this._btc_wallet;
    }
}

module.exports.create = (owner, config) => {
    return new Wallet(owner, config);
};
