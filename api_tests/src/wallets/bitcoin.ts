import crypto from "crypto";
import { Logger } from "log4js";
import BitcoinRpcClient from "bitcoin-core";
import { toBitcoin, toSatoshi } from "satoshi-bitcoin";
import { pollUntilMinted, Wallet } from "./index";
import { Asset } from "../asset";
import axios, { AxiosError, AxiosInstance, Method } from "axios";
import { BitcoinNodeConfig } from "../environment";
import { pollUntilMinted } from "./index";

export interface BitcoinWallet extends Wallet {
    mintToAddress(
        minimumExpectedBalance: bigint,
        toAddress: string
    ): Promise<void>;
    getAddress(): Promise<string>;
    getBalance(): Promise<bigint>;
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
}

export class BitcoindWallet implements BitcoinWallet {
    public static async newInstance(config: BitcoinNodeConfig, logger: Logger) {
        const walletName = crypto.randomBytes(32).toString("hex");
        const auth = {
            username: config.username,
            password: config.password,
        };

        await newAxiosClient(config.rpcUrl, auth, logger).request({
            data: {
                jsonrpc: "1.0",
                method: "createwallet",
                params: [walletName],
            },
        });

        logger.info("Name of generated Bitcoin wallet:", walletName);

        const minerClient = new BitcoinRpcClient({
            network: config.network,
            port: config.rpcPort,
            host: config.host,
            username: config.username,
            password: config.password,
            wallet: config.minerWallet,
        });
        const walletClient = newAxiosClient(
            `${config.rpcUrl}/wallet/${walletName}`,
            auth,
            logger
        );

        return new BitcoindWallet(minerClient, logger, walletClient);
    }

    public MaximumFee = BigInt(100000);

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
        const expectedBalance = minimumExpectedBalance * BigInt(2);
        const amount = toBitcoin(expectedBalance.toString());

        await this.minerClient.sendToAddress(toAddress, amount);

        this.logger.info("Minted", amount, "BTC to", toAddress);

        await pollUntilMinted(
            async () => this.getBalance(),
            BigInt(expectedBalance)
        );
    }

    public async mint(asset: Asset): Promise<void> {
        if (asset.name !== "bitcoin") {
            throw new Error(
                `Cannot mint asset ${asset.name} with BitcoinWallet`
            );
        }

        const startingBalance = await this.getBalance();

        const minimumExpectedBalance = BigInt(asset.quantity);

        await this.mintToAddress(
            minimumExpectedBalance,
            await this.getAddress()
        );

        await pollUntilMinted(
            async () => this.getBalance(),
            startingBalance + minimumExpectedBalance
        );
    }

    public async getBalanceByAsset(asset: Asset): Promise<bigint> {
        if (asset.name !== "bitcoin") {
            throw new Error(
                `Cannot read balance for asset ${asset.name} with BitcoinWallet`
            );
        }
        return this.getBalance();
    }

    public async getBalance(): Promise<bigint> {
        const res = await this.rpcClient.request({
            data: { jsonrpc: "1.0", method: "getbalance", params: [] },
        });

        return BigInt(toSatoshi(res.data.result));
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

function newAxiosClient(
    baseUrl: string,
    auth: { password: string; username: string },
    logger: Logger
) {
    const client = axios.create({
        baseURL: baseUrl,
        method: "post" as Method,
        auth,
    });
    client.interceptors.response.use(
        (response) => response,
        (error) => jsonRpcResponseInterceptor(logger, error)
    );

    return client;
}

export type Network = "main" | "test" | "regtest";

/**
 * A simplied representation of a Bitcoin transaction
 */
export interface BitcoinTransaction {
    hex: string;
    txid: string;
    confirmations: number;
}

async function jsonRpcResponseInterceptor(
    logger: Logger,
    error: AxiosError
): Promise<AxiosError> {
    const response = error.response;

    if (!response) {
        return Promise.reject(error);
    }

    const body = response.data;

    if (!body.error) {
        return Promise.reject(error);
    }

    logger.error("JSON-RPC request failed. Original request:", error.config);

    return Promise.reject(
        `JSON-RPC request '${
            JSON.parse(error.config.data).method
        }' failed with '${body.error.message}'`
    );
}
