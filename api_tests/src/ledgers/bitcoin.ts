import BitcoinRpcClient from "bitcoin-core";
import LedgerInstance from "./ledger_instance";
import { Logger } from "log4js";

/**
 * An instance of the Bitcoin ledger for use in the e2e tests.
 *
 * This class is compatible with anything that implements {@link BitcoinInstance}.
 *
 * For the e2e tests to work properly, we need to continuously mine bitcoin blocks.
 * This class takes care of spawning a miner after the Bitcoin blockchain has
 * been setup, regardless of how that is achieved (Docker container, bitcoind instance, etc).
 */
export default class BitcoinLedger implements LedgerInstance {
    public static async start(instance: BitcoinInstance, logger: Logger) {
        await instance.start();

        const { rpcPort, username, password, rpcUrl } = instance.config;

        logger.info("Bitcoin instance started at", rpcUrl);

        const client = new BitcoinRpcClient({
            network: "regtest",
            host: "localhost",
            port: rpcPort,
            username,
            password,
        });

        await client.generateToAddress(101, await client.getNewAddress());

        const miner = setInterval(async () => {
            await client.generateToAddress(1, await client.getNewAddress());
        }, 1000);

        logger.info("Bitcoin miner initialized");

        return new BitcoinLedger(instance, miner);
    }

    constructor(
        private readonly instance: BitcoinInstance,
        private readonly miner: NodeJS.Timeout
    ) {}

    public async stop(): Promise<void> {
        await this.instance.stop();
        clearInterval(this.miner);
    }

    public get config(): BitcoinNodeConfig {
        return this.instance.config;
    }
}

export interface BitcoinInstance {
    config: BitcoinNodeConfig;

    start(): Promise<void>;
    stop(): Promise<void>;
}

export interface BitcoinNodeConfig {
    network: string;
    username: string;
    password: string;
    host: string;
    rpcPort: number;
    rpcUrl: string;
    p2pPort: number;
    dataDir: string;
}
