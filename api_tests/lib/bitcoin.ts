import {
    Transaction,
    ECPair,
    Out,
    payments,
    networks,
    TransactionBuilder,
    address,
} from "bitcoinjs-lib";
import { test_rng } from "./util";

const BitcoinRpcClient = require("bitcoin-core");
const sb = require("satoshi-bitcoin");

export interface BtcConfig {
    // snake_case because it comes from TOML file
    rpc_username: string;
    rpc_password: string;
    rpc_host: string;
    rpc_port: number;
}

interface BitcoinRpcClient {
    // TODO: Create Interface for Promise returned by RPC calls
    // We should avoid to use `any` and instead create the interface
    // of what is returned by the RPC calls
    generate(num: number): Promise<any>;

    getBlockCount(): Promise<number>;

    getRawTransaction(
        txId: string,
        verbose?: boolean,
        blockHash?: string
    ): Promise<any>;

    sendToAddress(address: string, amount: number | string): Promise<any>;

    sendRawTransaction(hexString: string): Promise<any>;
}

interface Utxo {
    txId: string;
    value: number;
    vout: number;
}

let _bitcoinRpcClient: BitcoinRpcClient;
let _bitcoinConfig: BtcConfig;

export function init(btcConfig: BtcConfig) {
    console.log("Initiating bitcoin");
    createBitcoinRpcClient(btcConfig);
}

function createBitcoinRpcClient(btcConfig: BtcConfig) {
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

export async function generate(num: number = 1) {
    return createBitcoinRpcClient(_bitcoinConfig).generate(num);
}

module.exports.generate = generate;

export async function ensureSegwit() {
    const blockHeight = await createBitcoinRpcClient(
        _bitcoinConfig
    ).getBlockCount();
    if (blockHeight < 432) {
        await createBitcoinRpcClient(_bitcoinConfig).generate(432);
    }
}

module.exports.ensureSegwit = ensureSegwit;

export async function getFirstUtxoValueTransferredTo(
    txId: string,
    address: string
) {
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

module.exports.getFirstUtxoValueTransferredTo = getFirstUtxoValueTransferredTo;

export async function sendRawTransaction(hexString: string) {
    return createBitcoinRpcClient(_bitcoinConfig).sendRawTransaction(hexString);
}

export class BitcoinWallet {
    keypair: ECPair;
    bitcoinUtxos: Utxo[];
    _identity: {
        address: string;
        hash: Buffer;
        output: Buffer;
        pubkey: Buffer;
        signature: Buffer;
        input: Buffer;
        witness: Buffer[];
    };

    constructor(btcConfig: BtcConfig) {
        this.keypair = ECPair.makeRandom({ rng: test_rng });
        this.bitcoinUtxos = [];
        this._identity = payments.p2wpkh({
            pubkey: this.keypair.publicKey,
            network: networks.regtest,
        });

        createBitcoinRpcClient(btcConfig);
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
        this.bitcoinUtxos.push(
            ...entries
                .filter(entry => entry.script.equals(this.identity().output))
                .map(entry => {
                    return {
                        txId: txId,
                        vout: entries.indexOf(entry),
                        value: entry.value,
                    };
                })
        );
    }

    async sendToAddress(to: string, value: number) {
        const txb = new TransactionBuilder();
        const utxo = this.bitcoinUtxos.shift();
        const input_amount = utxo.value;
        const key_pair = this.keypair;
        const fee = 2500;
        const change = input_amount - value - fee;
        txb.addInput(utxo.txId, utxo.vout, null, this.identity().output);
        txb.addOutput(this.identity().output, change);
        txb.addOutput(address.toOutputScript(to, networks.regtest), value);
        txb.sign(0, key_pair, null, null, input_amount);

        return _bitcoinRpcClient.sendRawTransaction(txb.build().toHex());
    }
}
