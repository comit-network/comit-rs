import { EthereumWallet } from "../wallets/ethereum";
import LedgerInstance from "./ledger_instance";
import { Logger } from "log4js";
import { existsAsync, readFileAsync, writeFileAsync } from "../utils";
import path from "path";
import ledgerLock from "./ledger_lock";

/**
 * An instance of the Ethereum ledger for use in the e2e tests.
 *
 * This class is compatible with anything that implements {@link EthereumInstance}.
 *
 * Some of the e2e tests need an ERC20 token deployed to work properly.
 * This class takes care of deploying such contract after the Ethereum
 * blockchain is up and running.
 *
 * This class serves as an abstraction layer on top of Ethereum, regardless
 * of which implementation is used (Docker container, parity, geth, etc).
 */
export default class EthereumLedger implements LedgerInstance {
    public static async start(
        instance: EthereumInstance,
        logger: Logger,
        lockDir: string
    ) {
        const release = await ledgerLock(lockDir);
        const configFile = path.join(lockDir, "config.json");

        logger.info(
            "File-lock for Ethereum ledger acquired, checking for config file at",
            configFile
        );
        const configFileExists = await existsAsync(configFile);

        const ethereumLedger = configFileExists
            ? await EthereumLedger.reuseExisting(configFile, logger)
            : await EthereumLedger.startNew(
                  instance,
                  configFile,
                  logger,
                  lockDir
              );

        await release();

        return ethereumLedger;
    }

    private static async reuseExisting(configFile: string, logger: Logger) {
        logger.info(
            "Found config file, we'll be using that configuration instead of starting another instance"
        );

        const config: EthereumNodeConfig = JSON.parse(
            await readFileAsync(configFile, {
                encoding: "utf-8",
            })
        );

        const proxy = new EthereumInstanceProxy(config.rpc_url);

        return new EthereumLedger(proxy, config.tokenContract);
    }

    private static async startNew(
        instance: EthereumInstance,
        configFile: string,
        logger: Logger,
        lockDir: string
    ) {
        logger.info("No config file found, starting Ethereum ledger");

        await instance.start();

        const rpcUrl = instance.rpcUrl;

        logger.info("Ethereum node started at", rpcUrl);

        const erc20Wallet = new EthereumWallet(rpcUrl, logger, lockDir);
        const erc20TokenContract = await erc20Wallet.deployErc20TokenContract();

        logger.info("ERC20 token contract deployed at", erc20TokenContract);

        const config: EthereumNodeConfig = {
            rpc_url: rpcUrl,
            tokenContract: erc20TokenContract,
        };

        await writeFileAsync(configFile, JSON.stringify(config), {
            encoding: "utf-8",
        });

        logger.info("Ethereum ledger config file written to", configFile);

        return new EthereumLedger(instance, erc20TokenContract);
    }

    constructor(
        private readonly instance: EthereumInstance,
        private readonly erc20TokenContract: string
    ) {}

    public async stop(): Promise<void> {
        await this.instance.stop();
    }

    public get config(): EthereumNodeConfig {
        return {
            rpc_url: this.instance.rpcUrl,
            tokenContract: this.erc20TokenContract,
        };
    }
}

/**
 * A proxy implementation of `EthereumInstance` that will be used if
 * another test environment already started an actual instance of the
 * Ethereum ledger and we only have to provide the configuration values
 * of the one that was started.
 */
class EthereumInstanceProxy implements EthereumInstance {
    constructor(public readonly rpcUrl: string) {}

    public async start(): Promise<void> {
        return Promise.resolve();
    }

    public async stop(): Promise<void> {
        return Promise.resolve();
    }
}

export interface EthereumInstance {
    rpcUrl: string;

    start(): Promise<void>;
    stop(): Promise<void>;
}

export interface EthereumNodeConfig {
    rpc_url: string;
    tokenContract: string;
}
