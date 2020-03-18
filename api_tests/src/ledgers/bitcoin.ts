import BitcoinRpcClient from "bitcoin-core";
import LedgerInstance from "./ledger_instance";
import { Logger } from "log4js";
import path from "path";
import { existsAsync, readFileAsync, writeFileAsync } from "../utils";
import ledgerLock from "./ledger_lock";

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
    public static async start(
        instance: BitcoinInstance,
        logger: Logger,
        lockDir: string
    ) {
        const release = await ledgerLock(lockDir);

        const configFile = path.join(lockDir, "config.json");
        logger.info(
            "File-lock for Bitcoin ledger acquired, checking for config file at",
            configFile
        );
        const configFileExists = await existsAsync(configFile);

        const bitcoinLedger = configFileExists
            ? await BitcoinLedger.reuseExisting(configFile, logger)
            : await BitcoinLedger.startNew(instance, configFile, logger);

        await release();

        return bitcoinLedger;
    }

    private static async startNew(
        instance: BitcoinInstance,
        configFile: string,
        logger: Logger
    ) {
        logger.info("No config file found, starting Bitcoin ledger");

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

        // only coins after the first 101 are spendable
        await client.generateToAddress(101, await client.getNewAddress());

        const miner = setInterval(async () => {
            await client.generateToAddress(1, await client.getNewAddress());
        }, 1000);

        logger.info("Bitcoin miner initialized");

        await writeFileAsync(configFile, JSON.stringify(instance.config), {
            encoding: "utf-8",
        });

        logger.info("Bitcoin ledger config file written to", configFile);

        return new BitcoinLedger(instance, miner);
    }

    private static async reuseExisting(configFile: string, logger: Logger) {
        logger.info(
            "Found config file, we'll be using that configuration instead of starting another instance"
        );

        const config: BitcoinNodeConfig = JSON.parse(
            await readFileAsync(configFile, {
                encoding: "utf-8",
            })
        );

        const proxy = new BitcoinInstanceProxy(config);
        const dummyMiner = setInterval(async () => undefined, 10000);

        return new BitcoinLedger(proxy, dummyMiner);
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

class BitcoinInstanceProxy implements BitcoinInstance {
    constructor(public readonly config: BitcoinNodeConfig) {}

    public async start(): Promise<void> {
        return Promise.resolve();
    }

    public async stop(): Promise<void> {
        return Promise.resolve();
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
