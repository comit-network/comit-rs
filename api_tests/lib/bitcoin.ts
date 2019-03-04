import { ECPair, Out, Transaction } from "bitcoinjs-lib";
const bitcoin = require("bitcoinjs-lib");
const BitcoinRpcClient = require("bitcoin-core");
const sb = require("satoshi-bitcoin");
const util = require("./util.js");

interface IBitcoinConfig {
    // snake_case because it comes from TOML file
    rpc_username: string,
    rpc_password: string,
    rpc_host: string,
    rpc_port: number
}

interface IBitcoinRpcClient {
    //TODO: do not use 'any'
    generate(num: number): Promise<any>,
    getRawTransaction(txId: string, verbose?: boolean, blockHash?: string): Promise<any>,
    sendToAddress(address: string, amount: number|string): Promise<any>,
    sendRawTransaction(hexString: string): Promise<any>
}

interface IUtxo {
    // TODO: declare transaction Id type
    txId: string,
    value: number,
    vout: number,
}

let _bitcoinRpcClient: IBitcoinRpcClient;
let _bitcoinConfig: IBitcoinConfig;

function createBitcoinRpcClient(btcConfig: IBitcoinConfig) {
    if (!btcConfig && !_bitcoinConfig) {
        throw new Error("bitcoin configuration is needed");
    }

    if (!_bitcoinRpcClient || btcConfig !== _bitcoinConfig) {
        _bitcoinRpcClient = new BitcoinRpcClient({
            network: "regtest",
            port: btcConfig.rpc_port,
            host: btcConfig.rpc_host,
            username: btcConfig.rpc_username,
            password: btcConfig.rpc_password,
        });
        _bitcoinConfig = btcConfig;
    }
    return _bitcoinRpcClient;
}

module.exports.createClient = (btcConfig: IBitcoinConfig) => {
    return createBitcoinRpcClient(btcConfig);
};

module.exports.generate = async function(num: number = 1) {
    return createBitcoinRpcClient(_bitcoinConfig).generate(num);
};

module.exports.activateSegwit = async function() {
    return createBitcoinRpcClient(_bitcoinConfig).generate(432);
};

async function getFirstUtxoValueTransferredTo(txId: string, address: string) {
    let satoshi = 0;
    let tx = await _bitcoinRpcClient.getRawTransaction(txId, true);
    let vout = tx.vout[0];

    if (
        vout.scriptPubKey.addresses.length === 1 &&
        vout.scriptPubKey.addresses[0] === address
    ) {
        satoshi = sb.toSatoshi(vout.value);
    }

    return satoshi;
}

module.exports.get_first_utxo_value_transferred_to = getFirstUtxoValueTransferredTo;

class BitcoinWallet {
    keypair: ECPair;
    bitcoinUtxos: IUtxo[];
    _identity: { address: string, hash: Buffer, output: Buffer, pubkey: Buffer, signature: Buffer, input: Buffer, witness: Buffer[] };

    constructor() {
        this.keypair = ECPair.makeRandom({ rng: util.test_rng });
        // TODO: Use wallet instead of array to track Bitcoin UTXOs
        this.bitcoinUtxos = [];
        this._identity = bitcoin.payments.p2wpkh({
            pubkey: this.keypair.publicKey,
            network: bitcoin.networks.regtest,
        });
    }

    identity() {
        return this._identity;
    }

    async fund(btcValue: number) {
        let txId = await _bitcoinRpcClient.sendToAddress(
            this.identity().address,
            btcValue
        );
        let raw_transaction = await _bitcoinRpcClient.getRawTransaction(txId);
        let transaction = Transaction.fromHex(raw_transaction);
        let entries: Out[] = transaction.outs;
        for (let i in entries) {
            if (entries[i].script.equals(this.identity().output)) {
                let out: IUtxo = {
                    txId: txId,
                    vout: parseInt(i),
                    value: entries[i].value,
                };
                this.bitcoinUtxos.push(out);
            }
        }
    }

    async sendToAddress(to: string, value: number) {
        const txb = new bitcoin.TransactionBuilder();
        const utxo = this.bitcoinUtxos.shift();
        const input_amount = utxo.value;
        const key_pair = this.keypair;
        const fee = 2500;
        const change = input_amount - value - fee;
        txb.addInput(utxo.txId, utxo.vout, null, this.identity().output);
        txb.addOutput(this.identity().output, change);
        txb.addOutput(
            bitcoin.address.toOutputScript(to, bitcoin.networks.regtest),
            value
        );
        txb.sign(0, key_pair, null, null, input_amount);

        return _bitcoinRpcClient.sendRawTransaction(txb.build().toHex());
    }
}

module.exports.create_wallet = () => {
    return new BitcoinWallet();
};
