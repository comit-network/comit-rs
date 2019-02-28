const bitcoin = require("./bitcoin.js");
const ethereum = require("./ethereum.js");

class Wallet {
    constructor(owner) {
        this.owner = owner;
        this._eth_wallet = ethereum.create();
        this._btc_wallet = bitcoin.create_wallet();
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
