const bitcoin = require("bitcoinjs-lib");
const BitcoinRpcClient = require("bitcoin-core");
const util = require("./util.js");

let _rpc_client;

function create_bitcoin_rpc_client() {
    const btc_config = global.harness.ledgers_config.bitcoin;
    if (!btc_config) {
        throw new Error("ledger.bitcoin configuration is needed");
    }
    return (_rpc_client =
        _rpc_client ||
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

module.exports.btc_import_address = async function(address) {
    return create_bitcoin_rpc_client().importAddress(address);
};

async function btc_balance(address) {
    let btc_balance = await _rpc_client.getReceivedByAddress(address);
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
    global.harness.logger.info(
        "%s the swap, %s has %s satoshis at the %s address %s",
        when,
        player,
        await btc_balance(address),
        address_type,
        address
    );
};

class BitcoinWallet {
    constructor() {
        this.keypair = bitcoin.ECPair.makeRandom({ rng: util.test_rng, network: bitcoin.networks.regtest });
        this.bitcoin_utxos = [];
        this._identity = bitcoin.payments.p2wpkh({
            pubkey: this.keypair.publicKey,
            network: bitcoin.networks.regtest,
        });
        this._identityBase58 = bitcoin.payments.p2pkh({
            pubkey: this.keypair.publicKey,
            network: bitcoin.networks.regtest,
        });
    }

    identity(base58 = false) {
        if(base58) {
            return this._identityBase58;
        }
        else {
            return this._identity;
        }
    }


    async fund(btc_value, rpcClient = _rpc_client, base58 = false) {
        let txid = await rpcClient.sendToAddress(
            this.identity(base58).address,
            btc_value
        );
        let raw_transaction = await rpcClient.getRawTransaction(txid);
        let transaction = bitcoin.Transaction.fromHex(raw_transaction);
        for (let [i, out] of transaction.outs.entries()) {
            if (out.script.equals(this.identity(base58).output)) {
                out.txid = txid;
                out.vout = i;
                this.bitcoin_utxos.push(out);
            }
        }
    }

    async send_btc_to_address(to, value) {
        const txb = new bitcoin.TransactionBuilder(bitcoin.networks.regtest);
        const utxo = this.bitcoin_utxos.shift();
        const to_address = to;
        const input_amount = utxo.value;
        const key_pair = this.keypair;
        const fee = 2500;
        const change = input_amount - value - fee;
        txb.addInput(utxo.txid, utxo.vout, null, this.identity().output);
        //TODO: Add it back to UTXOs after transaction is successful
        txb.addOutput(this.identity().output, change);
        txb.addOutput(bitcoin.address.toOutputScript(to, bitcoin.networks.regtest), value);
        txb.sign(0, key_pair, null, null, input_amount);

        return _rpc_client.sendRawTransaction(txb.build().toHex());
    }
}

module.exports.create_wallet = () => {
    return new BitcoinWallet();
};

module.exports.BitcoinWallet = BitcoinWallet;
