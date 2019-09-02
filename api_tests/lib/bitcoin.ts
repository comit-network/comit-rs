// @ts-ignore
import BitcoinRpcClient from "bitcoin-core";
import {
    address,
    ECPair,
    ECPairInterface,
    networks,
    Payment,
    payments,
    Transaction,
    TransactionBuilder,
} from "bitcoinjs-lib";
import sb from "satoshi-bitcoin";
import { test_rng } from "./util";

export interface BitcoinNodeConfig {
    username: string;
    password: string;
    host: string;
    rpcPort: number;
    zmqPort: number;
}

interface GetBlockchainInfoResponse {
    mediantime: number;
}

interface VerboseRawTransactionResponse {
    vout: Array<{
        scriptPubKey: {
            addresses: string[];
        };
        value: number;
    }>;
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

let bitcoinRpcClient: BitcoinRpcClient;
let bitcoinConfig: BitcoinNodeConfig;

export function init(btcConfig: BitcoinNodeConfig) {
    console.log("Initiating bitcoin");
    createBitcoinRpcClient(btcConfig);
}

function createBitcoinRpcClient(btcConfig?: BitcoinNodeConfig) {
    if (!btcConfig && !bitcoinConfig) {
        throw new Error("bitcoin configuration is needed");
    }

    if (!bitcoinRpcClient || btcConfig !== bitcoinConfig) {
        bitcoinRpcClient = new BitcoinRpcClient({
            network: "regtest",
            port: btcConfig.rpcPort,
            host: btcConfig.host,
            username: btcConfig.username,
            password: btcConfig.password,
        });
        bitcoinConfig = btcConfig;
    }
    return bitcoinRpcClient;
}

export async function generate(num: number = 1) {
    return createBitcoinRpcClient(bitcoinConfig).generate(num);
}

export async function getBlockchainInfo() {
    return createBitcoinRpcClient(bitcoinConfig).getBlockchainInfo();
}

export async function ensureFunding() {
    const blockHeight = await createBitcoinRpcClient(
        bitcoinConfig
    ).getBlockCount();
    if (blockHeight < 101) {
        await createBitcoinRpcClient(bitcoinConfig).generate(101 - blockHeight);
    }
}

export async function sendRawTransaction(hexString: string) {
    return createBitcoinRpcClient(bitcoinConfig).sendRawTransaction(hexString);
}

export class BitcoinWallet {
    private readonly identity: Payment;
    private readonly keypair: ECPairInterface;
    private readonly bitcoinUtxos: Utxo[];
    private readonly addressForIncomingPayments: string;

    constructor(
        btcConfig: BitcoinNodeConfig,
        addressForIncomingPayments: string
    ) {
        this.addressForIncomingPayments = addressForIncomingPayments;
        this.keypair = ECPair.makeRandom({ rng: test_rng });
        this.bitcoinUtxos = [];
        this.identity = payments.p2wpkh({
            pubkey: this.keypair.publicKey,
            network: networks.regtest,
        });

        createBitcoinRpcClient(btcConfig);
    }

    public getNewAddress() {
        return this.addressForIncomingPayments;
    }

    public satoshiReceivedInTx(redeemTxId: string) {
        return getFirstUtxoValueTransferredTo(
            redeemTxId,
            this.addressForIncomingPayments
        );
    }

    public async fund(bitcoin: number) {
        const txId = await bitcoinRpcClient.sendToAddress(
            this.identity.address,
            bitcoin
        );
        const rawTransaction = (await bitcoinRpcClient.getRawTransaction(
            txId
        )) as HexRawTransactionResponse;
        const transaction = Transaction.fromHex(rawTransaction);

        const entries = transaction.outs;
        this.bitcoinUtxos.push(
            ...transaction.outs
                .filter(entry => entry.script.equals(this.identity.output))
                .filter(entry => "value" in entry && entry.value > 0)
                .map(entry => {
                    return {
                        txId,
                        vout: entries.indexOf(entry),
                        // @ts-ignore: we filtered out all outputs that don't have a value
                        value: entry.value,
                    };
                })
        );
    }

    public async sendToAddress(to: string, value: number) {
        const txb = new TransactionBuilder();
        const utxo = this.bitcoinUtxos.shift();
        const inputAmount = utxo.value;
        const keyPair = this.keypair;
        const fee = 2500;
        const change = inputAmount - value - fee;
        txb.addInput(utxo.txId, utxo.vout, null, this.identity.output);
        txb.addOutput(this.identity.output, change);
        txb.addOutput(address.toOutputScript(to, networks.regtest), value);
        txb.sign(0, keyPair, null, null, inputAmount);

        return bitcoinRpcClient.sendRawTransaction(txb.build().toHex());
    }
}

async function getFirstUtxoValueTransferredTo(txId: string, address: string) {
    let satoshi = 0;
    const tx = (await bitcoinRpcClient.getRawTransaction(
        txId,
        true
    )) as VerboseRawTransactionResponse;
    const vout = tx.vout[0];

    if (
        vout.scriptPubKey.addresses.length === 1 &&
        vout.scriptPubKey.addresses[0] === address
    ) {
        satoshi = sb.toSatoshi(vout.value);
    }

    return satoshi;
}
