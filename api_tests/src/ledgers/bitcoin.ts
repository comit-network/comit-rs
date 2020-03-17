import BitcoinRpcClient from "bitcoin-core";
import { Logger } from "log4js";
import { existsAsync, readFileAsync, writeFileAsync } from "../utils";

/**
 * An instance of the Bitcoin ledger for use in the e2e tests.
 *
 * This class is compatible with anything that implements {@link BitcoinInstance}
 * and takes care of correctly initializing the Bitcoin ledger. Concretely,
 * this means mining a 101 blocks so we can spend coins from the mining reward.
 */
export default class BitcoinLedger {
    public static async start(
        instance: BitcoinInstance,
        logger: Logger,
        configFile: string
    ) {
        logger.info(
            "File-lock for Bitcoin ledger acquired, checking for config file at",
            configFile
        );
        const configFileExists = await existsAsync(configFile);

        const bitcoinLedger = configFileExists
            ? await BitcoinLedger.reuseExisting(configFile, logger)
            : await BitcoinLedger.startNew(instance, configFile, logger);

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

        await writeFileAsync(configFile, JSON.stringify(instance.config), {
            encoding: "utf-8",
        });

        logger.info("Bitcoin ledger config file written to", configFile);

        return new BitcoinLedger(instance);
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

        return new BitcoinLedger(proxy);
    }

    constructor(private readonly instance: BitcoinInstance) {}

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
