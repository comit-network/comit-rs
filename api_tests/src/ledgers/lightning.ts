import LedgerInstance from "./ledger_instance";
import { Logger } from "log4js";
import path from "path";
import { existsAsync, readFileAsync, writeFileAsync } from "../utils";
import ledgerLock from "./ledger_lock";

/**
 * An instance of the Lightning ledger for use in the e2e tests.
 *
 * This class is compatible with anything that implements {@link LightningInstance}.
 *
 * Compared to {@link BitcoinLedger} and {@link EthereumLedger}, there is nothing
 * to be done after a {@link LightningInstance} is started. If this ever changes,
 * this class is the place where to put this information.
 */
export default class LightningLedger implements LedgerInstance {
    public static async start(
        instance: LightningInstance,
        logger: Logger,
        lockDir: string
    ) {
        const release = await ledgerLock(lockDir);
        const configFile = path.join(lockDir, "config.json");

        logger.info(
            "File-lock for Lightning ledger acquired, checking for config file at",
            configFile
        );
        const configFileExists = await existsAsync(configFile);

        const lightningLedger = configFileExists
            ? await LightningLedger.reuseExisting(configFile, logger)
            : await LightningLedger.startNew(instance, configFile, logger);

        await release();

        return lightningLedger;
    }

    private static async reuseExisting(configFile: string, logger: Logger) {
        logger.info(
            "Found config file, we'll be using that configuration instead of starting another instance"
        );

        const config: LightningNodeConfig = JSON.parse(
            await readFileAsync(configFile, {
                encoding: "utf-8",
            })
        );
        const proxy = new LightningInstanceProxy(config);

        return new LightningLedger(proxy);
    }

    private static async startNew(
        instance: LightningInstance,
        configFile: string,
        logger: Logger
    ) {
        logger.info("No config file found, starting Lightning ledger");

        await instance.start();

        const { grpcSocket } = instance.config;

        logger.info("Lightning node started at", grpcSocket);

        await writeFileAsync(configFile, JSON.stringify(instance.config), {
            encoding: "utf-8",
        });

        logger.info("Lightning ledger config file written to", configFile);

        return new LightningLedger(instance);
    }

    constructor(private readonly instance: LightningInstance) {}

    async stop(): Promise<void> {
        return this.instance.stop();
    }

    get config(): LightningNodeConfig {
        return this.instance.config;
    }
}

class LightningInstanceProxy implements LightningInstance {
    constructor(public readonly config: LightningNodeConfig) {}

    async start(): Promise<void> {
        return Promise.resolve();
    }

    async stop(): Promise<void> {
        return Promise.resolve();
    }
}

export interface LightningInstance {
    config: LightningNodeConfig;

    start(): Promise<void>;
    stop(): Promise<void>;
}

export interface LightningNodeConfig {
    p2pSocket: string;
    grpcSocket: string;
    tlsCertPath: string;
    macaroonPath: string;
}
