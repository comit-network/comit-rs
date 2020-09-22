import crypto from "crypto";
import { Logger } from "log4js";
import BitcoinRpcClient from "bitcoin-core";
import { toBitcoin, toSatoshi } from "satoshi-bitcoin";
import axios, { AxiosError, AxiosInstance, Method } from "axios";
import { BitcoinNode } from "../environment";
import { sleep } from "../utils";
import pTimeout from "p-timeout";

export interface BitcoinWallet {
    MaximumFee: bigint;
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

    mint(satoshis: bigint): Promise<void>;
}

export class BitcoinFaucet {
    private readonly minerClient: BitcoinRpcClient;

    constructor(config: BitcoinNode, private readonly logger: Logger) {
        this.minerClient = new BitcoinRpcClient({
            network: config.network,
            port: config.rpcPort,
            host: config.host,
            username: config.username,
            password: config.password,
            wallet: config.minerWallet,
        });
    }

    public async mint(
        satoshis: bigint,
        address: string,
        getBalance?: () => Promise<bigint>
    ): Promise<void> {
        const startingBalance = getBalance ? await getBalance() : 0n;

        const blockHeight = await this.minerClient.getBlockCount();

        if (blockHeight < 101) {
            throw new Error(
                "unable to mint bitcoin, coinbase transactions are not yet spendable"
            );
        }

        const btc = toBitcoin(satoshis.toString());

        await this.minerClient.sendToAddress(address, btc);
        this.logger.info("Minted", btc, "BTC to", address);

        if (getBalance) {
            await waitUntilBalanceReaches(
                getBalance,
                startingBalance + satoshis
            );
        }
    }
}

async function waitUntilBalanceReaches(
    getBalance: () => Promise<bigint>,
    expectedBalance: bigint
): Promise<void> {
    let currentBalance = await getBalance();

    const timeout = 10;
    const error = new Error(
        `Balance did not reach ${expectedBalance} after ${timeout} seconds, starting balance was ${currentBalance}`
    );
    Error.captureStackTrace(error);

    const poller = async () => {
        while (currentBalance < expectedBalance) {
            await sleep(500);
            currentBalance = await getBalance();
        }
    };

    await pTimeout(poller(), timeout * 1000, error);
}

export class BitcoindWallet implements BitcoinWallet {
    public static async newInstance(config: BitcoinNode, logger: Logger) {
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

        const walletClient = newAxiosClient(
            `${config.rpcUrl}/wallet/${walletName}`,
            auth,
            logger
        );
        const faucet = new BitcoinFaucet(config, logger);

        return new BitcoindWallet(faucet, walletClient);
    }

    public MaximumFee = 100000n;

    constructor(
        private readonly faucet: BitcoinFaucet,
        private readonly rpcClient: AxiosInstance
    ) {}

    public async mint(satoshis: bigint): Promise<void> {
        await this.faucet.mint(satoshis, await this.getAddress(), async () =>
            this.getBalance()
        );
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
