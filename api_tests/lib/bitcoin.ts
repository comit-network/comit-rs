import {
    address,
    ECPair,
    networks,
    Out,
    payments,
    Transaction,
    TransactionBuilder,
} from "bitcoinjs-lib";
import { test_rng } from "./util";

const BitcoinRpcClient = require("bitcoin-core");
const sb = require("satoshi-bitcoin");

export interface BitcoinNodeConfig {
    // snake_case because it comes from TOML file
    rpc_username: string;
    rpc_password: string;
    rpc_host: string;
    rpc_port: number;
}

interface GetBlockchainInfoResponse {
    mediantime: number;
}

interface VerboseRawTransactionResponse {
    vout: {
        scriptPubKey: {
            addresses: string[];
        };
        value: number;
    }[];
}

type HexRawTransactionResponse = string;

type GetRawTransactionResponse =
    | null
    | HexRawTransactionResponse
    | VerboseRawTransactionResponse;

interface BitcoinRpcClient {
    generate(num: number): Promise<string[]>;
    getBlockchainInfo(): Promise<GetBlockchainInfoResponse>;

    getBlockCount(): Promise<number>;

    getRawTransaction(
        txId: string,
        verbose?: boolean,
        blockHash?: string
    ): Promise<GetRawTransactionResponse>;

    sendToAddress(address: string, amount: number | string): Promise<string>;

    sendRawTransaction(hexString: string): Promise<string>;
}

interface Utxo {
    txId: string;
    value: number;
    vout: number;
}

let _bitcoinRpcClient: BitcoinRpcClient;
let _bitcoinConfig: BitcoinNodeConfig;

export function init(btcConfig?: BitcoinNodeConfig) {
    console.log("Initiating bitcoin");
    createBitcoinRpcClient(btcConfig);
}

function createBitcoinRpcClient(btcConfig?: BitcoinNodeConfig) {
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

export async function getBlockchainInfo() {
    return createBitcoinRpcClient(_bitcoinConfig).getBlockchainInfo();
}

export async function ensureFunding() {
    const blockHeight = await createBitcoinRpcClient(
        _bitcoinConfig
    ).getBlockCount();
    if (blockHeight < 101) {
        await createBitcoinRpcClient(_bitcoinConfig).generate(
            101 - blockHeight
        );
    }
}

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

    addressForIncomingPayments: string;

    constructor(
        btcConfig: BitcoinNodeConfig,
        addressForIncomingPayments: string
    ) {
        this.addressForIncomingPayments = addressForIncomingPayments;
        this.keypair = ECPair.makeRandom({ rng: test_rng });
        this.bitcoinUtxos = [];
        this._identity = payments.p2wpkh({
            pubkey: this.keypair.publicKey,
            network: networks.regtest,
        });

        createBitcoinRpcClient(btcConfig);
    }

    getNewAddress() {
        return this.addressForIncomingPayments;
    }

    moneyReceivedInTx(redeemTxId: string) {
        return getFirstUtxoValueTransferredTo(
            redeemTxId,
            this.addressForIncomingPayments
        );
    }

    identity() {
        return this._identity;
    }

    async fund(bitcoin: number) {
        let txId = await _bitcoinRpcClient.sendToAddress(
            this.identity().address,
            bitcoin
        );
        let raw_transaction = (await _bitcoinRpcClient.getRawTransaction(
            txId
        )) as HexRawTransactionResponse;
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

async function getFirstUtxoValueTransferredTo(txId: string, address: string) {
    let satoshi = 0;
    let tx = (await _bitcoinRpcClient.getRawTransaction(
        txId,
        true
    )) as VerboseRawTransactionResponse;
    let vout = tx.vout[0];

    if (
        vout.scriptPubKey.addresses.length === 1 &&
        vout.scriptPubKey.addresses[0] === address
    ) {
        satoshi = sb.toSatoshi(vout.value);
    }

    return satoshi;
}
