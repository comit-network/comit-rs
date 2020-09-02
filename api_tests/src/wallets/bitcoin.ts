import crypto from "crypto";
import { bip32, networks } from "bitcoinjs-lib";
import { Logger } from "log4js";
import BitcoinRpcClient from "bitcoin-core";
import { toBitcoin, toSatoshi } from "satoshi-bitcoin";
import { pollUntilMinted, Wallet } from "./index";
import { BitcoinNodeConfig } from "../ledgers";
import { Asset } from "../asset";
import axios, { AxiosInstance, Method } from "axios";

export interface BitcoinWallet extends Wallet {
    mintToAddress(
        minimumExpectedBalance: bigint,
        toAddress: string
    ): Promise<void>;

    getAddress(): Promise<string>;

    getBalance(): Promise<number>;

    sendToAddress(
        address: string,
        satoshis: number,
        network: Network
    ): Promise<string>;

    broadcastTransaction(
        transactionHex: string,
        network: Network
    ): Promise<string>;

    getFee(): string;

    getTransaction(transactionId: string): Promise<BitcoinTransaction>;
}

export class BitcoindWallet implements BitcoinWallet {
    public static async newInstance(config: BitcoinNodeConfig, logger: Logger) {
        const hdKey = bip32.fromSeed(crypto.randomBytes(32), networks.regtest);
        const derivationPath = "44h/1h/0h/0/*";
        let walletDescriptor = `wpkh(${hdKey.toBase58()}/${derivationPath})`;

        const walletName = hdKey.fingerprint.toString("hex");
        const url = config.rpcUrl;
        const auth = {
            username: config.username,
            password: config.password,
        };

        const client = axios.create({
            baseURL: url,
            method: "post" as Method,
            auth,
        });

        const walletExists = await client
            .request({
                data: {
                    jsonrpc: "1.0",
                    method: "listwallets",
                },
            })
            .then((res) => res.data.result.includes(walletName));

        if (!walletExists) {
            await client.request({
                data: {
                    jsonrpc: "1.0",
                    method: "createwallet",
                    params: [walletName],
                },
            });
        }

        // Ask bitcoind for a checksum if none was provided with the descriptor
        if (!hasChecksum(walletDescriptor)) {
            const checksum = await client
                .request({
                    data: {
                        jsonrpc: "1.0",
                        method: "getdescriptorinfo",
                        params: [walletDescriptor],
                    },
                })
                .then((res) => res.data.result.checksum);
            walletDescriptor = `${walletDescriptor}#${checksum}`;
        }

        const walletClient = axios.create({
            baseURL: `${url}/wallet/${walletName}`,
            method: "post" as Method,
            auth,
        });

        await walletClient.request({
            data: {
                jsonrpc: "1.0",
                method: "importmulti",
                params: [
                    [{ desc: walletDescriptor, timestamp: 0, range: 0 }],
                    { rescan: true },
                ],
            },
        });

        const rpcClientArgs = {
            network: config.network,
            port: config.rpcPort,
            host: config.host,
            username: config.username,
            password: config.password,
        };

        const minerClient = new BitcoinRpcClient({
            ...rpcClientArgs,
            wallet: config.minerWallet,
        });

        return new BitcoindWallet(minerClient, logger, client);
    }

    public MaximumFee = 100000;

    private constructor(
        private readonly minerClient: BitcoinRpcClient,
        private readonly logger: Logger,
        private readonly rpcClient: AxiosInstance
    ) {}

    public async mintToAddress(
        minimumExpectedBalance: bigint,
        toAddress: string
    ): Promise<void> {
        const res = await this.rpcClient.request({
            data: { jsonrpc: "1.0", method: "getblockcount", params: [] },
        });

        const blockHeight = res.data.result;
        if (blockHeight < 101) {
            throw new Error(
                "unable to mint bitcoin, coinbase transactions are not yet spendable"
            );
        }

        // make sure we have at least twice as much
        const amount = toBitcoin(
            (minimumExpectedBalance * BigInt(2)).toString()
        );

        await this.minerClient.sendToAddress(toAddress, amount);

        this.logger.info("Minted", amount, "bitcoin for", toAddress);
    }

    public async mint(asset: Asset): Promise<void> {
        if (asset.name !== "bitcoin") {
            throw new Error(
                `Cannot mint asset ${asset.name} with BitcoinWallet`
            );
        }

        const startingBalance = await this.getBalanceByAsset(asset);

        const minimumExpectedBalance = BigInt(asset.quantity);

        await this.mintToAddress(
            minimumExpectedBalance,
            await this.getAddress()
        );

        await pollUntilMinted(
            this,
            startingBalance + minimumExpectedBalance,
            asset
        );
    }

    public async getBalanceByAsset(asset: Asset): Promise<bigint> {
        if (asset.name !== "bitcoin") {
            throw new Error(
                `Cannot read balance for asset ${asset.name} with BitcoinWallet`
            );
        }
        return BigInt(toSatoshi(await this.getBalance()));
    }

    public async getBlockchainTime(): Promise<number> {
        const res = await this.rpcClient.request({
            data: { jsonrpc: "1.0", method: "getblockchaininfo", params: [] },
        });

        return res.data.result.mediantime;
    }

    public async getBalance(): Promise<number> {
        const res = await this.rpcClient.request({
            data: { jsonrpc: "1.0", method: "getbalance", params: [] },
        });

        return res.data.result;
    }

    public async getAddress(): Promise<string> {
        const res = await this.rpcClient.request({
            data: {
                jsonrpc: "1.0",
                method: "getnewaddress",
                params: ["", "bech32"],
            },
        });

        return res.data.result;
    }

    public async sendToAddress(
        address: string,
        satoshis: number,
        network: Network
    ): Promise<string> {
        await this.assertNetwork(network);

        const res = await this.rpcClient.request({
            data: {
                jsonrpc: "1.0",
                method: "sendtoaddress",
                params: [address, toBitcoin(satoshis)],
            },
        });

        return res.data.result;
    }

    public async broadcastTransaction(
        transactionHex: string,
        network: Network
    ): Promise<string> {
        await this.assertNetwork(network);

        const res = await this.rpcClient.request({
            data: {
                jsonrpc: "1.0",
                method: "sendrawtransaction",
                params: [transactionHex],
            },
        });

        return res.data.result;
    }

    public getFee(): string {
        // should be dynamic in a real application or use `estimatesmartfee`
        return "150";
    }

    public async getTransaction(
        transactionId: string
    ): Promise<BitcoinTransaction> {
        const res = await this.rpcClient.request({
            data: {
                jsonrpc: "1.0",
                method: "getrawtransaction",
                params: [transactionId, true],
            },
        });
        return res.data.result;
    }

    public async close(): Promise<void> {
        await this.rpcClient.request({
            data: {
                jsonrpc: "1.0",
                method: "unloadwallet",
                params: [],
            },
        });
    }

    private async assertNetwork(network: Network): Promise<void> {
        const res = await this.rpcClient.request({
            data: { jsonrpc: "1.0", method: "getblockchaininfo", params: [] },
        });

        if (res.data.result.chain !== network) {
            return Promise.reject(
                `This wallet is only connected to the ${network} network and cannot perform actions on the ${network} network`
            );
        }
    }
}

export type Network = "main" | "test" | "regtest";

function hasChecksum(descriptor: string): boolean {
    const [, checksum] = descriptor.split("#", 2);

    return !!checksum && checksum.length === 8;
}

/**
 * A simplied representation of a Bitcoin transaction
 */
export interface BitcoinTransaction {
    hex: string;
    txid: string;
    confirmations: number;
}
