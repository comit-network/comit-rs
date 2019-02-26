const bitcoin = require("bitcoinjs-lib");
const BitcoinRpcClient = require("bitcoin-core");
const sb = require("satoshi-bitcoin");
const util = require("./util.js");

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

async function getSatoshiTransferredTo(tx_id, address) {
    let satoshi = 0;
    let tx = await _bitcoin_rpc_client.getRawTransaction(tx_id, 1);
    let vout = tx.vout[0];

    if (
        vout.scriptPubKey.addresses.length === 1 &&
        vout.scriptPubKey.addresses[0] === address
    ) {
        satoshi = sb.toSatoshi(vout.value);
    }

    return satoshi;
}

module.exports.getSatoshiTransferredTo = async function(tx_id, address) {
    return getSatoshiTransferredTo(tx_id, address);
};

class BitcoinWallet {
    constructor() {
        this.keypair = bitcoin.ECPair.makeRandom({ rng: util.test_rng });
        this.bitcoin_utxos = [];
        this._identity = bitcoin.payments.p2wpkh({
            pubkey: this.keypair.publicKey,
            network: bitcoin.networks.regtest,
        });
    }

    identity() {
        return this._identity;
    }

    async fund(btc_value) {
        let txid = await _bitcoin_rpc_client.sendToAddress(
            this.identity().address,
            btc_value
        );
        let raw_transaction = await _bitcoin_rpc_client.getRawTransaction(txid);
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
        // The remains of the utxo are NOT added back to the wallet
        txb.addOutput(this.identity().output, change);
        txb.addOutput(
            bitcoin.address.toOutputScript(to, bitcoin.networks.regtest),
            value
        );
        txb.sign(0, key_pair, null, null, input_amount);

        return _bitcoin_rpc_client.sendRawTransaction(txb.build().toHex());
    }
}

module.exports.create_wallet = () => {
    return new BitcoinWallet();
};
