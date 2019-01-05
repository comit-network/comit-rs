const bitcoin = require("bitcoinjs-lib");
const bitcoin_rpc_client_conf = require("./bitcoin_rpc_client_conf.js");
const test_lib = require("./test_lib.js");

const bitcoin_rpc_client = bitcoin_rpc_client_conf.create_client();

//FIXME: Remove this whenever this change:
// https://github.com/bitcoinjs/bitcoinjs-lib/commit/44a98c0fa6487eaf81500427366787a953ff890d#diff-9e60abeb4e2333a5d2f02de53b4edfac
// Hits npm!
const regtest = {
    messagePrefix: "\x18Bitcoin Signed Message:\n",
    bech32: "bcrt",
    bip32: {
        public: 0x043587cf,
        private: 0x04358394,
    },
    pubKeyHash: 0x6f,
    scriptHash: 0xc4,
    wif: 0xef,
};

class BitcoinWallet {
    constructor() {
        this.keypair = bitcoin.ECPair.makeRandom({ rng: test_lib.test_rng });
        this.bitcoin_utxos = [];
        this._identity = bitcoin.payments.p2wpkh({
            pubkey: this.keypair.publicKey,
            network: regtest,
        });
    }

    identity() {
        return this._identity;
    }

    async fund(btc_value) {
        let txid = await bitcoin_rpc_client.sendToAddress(
            this.identity().address,
            btc_value
        );
        let raw_transaction = await bitcoin_rpc_client.getRawTransaction(txid);
        let transaction = bitcoin.Transaction.fromHex(raw_transaction);
        for (let [i, out] of transaction.outs.entries()) {
            if (out.script.equals(this.identity().output)) {
                out.txid = txid;
                out.vout = i;
                this.bitcoin_utxos.push(out);
            }
        }
    }

    async send_btc_to_address(to, value) {
        const txb = new bitcoin.TransactionBuilder();
        const utxo = this.bitcoin_utxos.shift();
        const to_address = bitcoin.address.fromBech32(to);
        const input_amount = utxo.value;
        const key_pair = this.keypair;
        const fee = 2500;
        const change = input_amount - value - fee;
        txb.addInput(utxo.txid, utxo.vout, null, this.identity().output);
        //TODO: Add it back to UTXOs after transaction is successful
        txb.addOutput(this.identity().output, change);
        txb.addOutput(bitcoin.address.toOutputScript(to, regtest), value);
        txb.sign(0, key_pair, null, null, input_amount);

        return bitcoin_rpc_client.sendRawTransaction(txb.build().toHex());
    }
}

module.exports.create = () => {
    return new BitcoinWallet();
};
