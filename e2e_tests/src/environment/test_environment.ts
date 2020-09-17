import { Config } from "@jest/types";
import { promises as asyncFs } from "fs";
import NodeEnvironment from "jest-environment-node";
import path from "path";
import { BitcoindInstance } from "./bitcoind_instance";
import { configure, Logger, shutdown as loggerShutdown } from "log4js";
import { EnvironmentContext } from "@jest/environment";
import BitcoinMinerInstance from "./bitcoin_miner_instance";
import { EthereumFaucet } from "../wallets/ethereum";
import { GethInstance } from "./geth_instance";
import { LndInstance } from "./lnd_instance";
import BitcoinRpcClient from "bitcoin-core";
import { CndConfig } from "./cnd_config";
import { set } from "lodash";
import { HarnessGlobal, Startable, LightningNode } from "./index";
import { execAsync, existsAsync } from "./async_fs";
import { LndClient } from "../wallets/lightning";
import { BitcoinFaucet } from "../wallets/bitcoin";
import properLockfile from "proper-lockfile";

export default class TestEnvironment extends NodeEnvironment {
    private readonly testSuite: string;
    private readonly ledgers: string[];
    private readonly logDir: string;
    private readonly locksDir: string;
    private readonly nodeModulesBinDir: string;
    private readonly srcDir: string;
    private readonly cndConfigOverrides: Partial<CndConfig>;

    public global: HarnessGlobal;

    private logger: Logger;
    private bitcoinFaucet: BitcoinFaucet;

    constructor(config: Config.ProjectConfig, context: EnvironmentContext) {
        super(config);

        this.ledgers = extractLedgersToBeStarted(context.docblockPragmas);
        this.cndConfigOverrides = extractCndConfigOverrides(
            context.docblockPragmas
        );
        assertNoUnhandledPargmas(context.docblockPragmas);

        this.logDir = path.resolve(config.rootDir, "log");
        this.locksDir = path.resolve(config.rootDir, "locks");
        this.nodeModulesBinDir = path.resolve(
            config.rootDir,
            "node_modules",
            ".bin"
        );
        this.srcDir = path.resolve(config.rootDir, "src");
        this.testSuite = path.parse(context.testPath).name;
    }

    async setup() {
        await super.setup();

        const cargoTargetDir = await execAsync(
            "cargo metadata --format-version=1 --no-deps"
        )
            .then(({ stdout }) => JSON.parse(stdout))
            .then((metadata) => metadata.target_directory);

        // setup global variables
        this.global.environment = {};
        this.global.lndClients = {};
        this.global.cargoTargetDir = cargoTargetDir;
        this.global.cndConfigOverrides = this.cndConfigOverrides;

        const log4js = configure({
            appenders: {
                multi: {
                    type: "multiFile",
                    base: this.logDir,
                    property: "categoryName",
                    extension: ".log",
                    layout: {
                        type: "pattern",
                        pattern: "%d %5.10p: %m",
                    },
                    timeout: 2000,
                },
            },
            categories: {
                default: { appenders: ["multi"], level: "debug" },
            },
        });

        const testLogDir = path.join(this.logDir, "tests", this.testSuite);
        await asyncFs.mkdir(testLogDir, { recursive: true });

        this.global.getLogFile = (pathElements) =>
            path.join(testLogDir, ...pathElements);
        this.global.getLogger = (categories) => {
            return log4js.getLogger(
                path.join("tests", this.testSuite, ...categories)
            );
        };
        this.logger = this.global.getLogger(["test_environment"]);

        this.global.getDataDir = async (program) => {
            const dir = path.join(this.logDir, program);
            await asyncFs.mkdir(dir, { recursive: true });

            return dir;
        };

        this.logger.info("Starting up test environment");

        await this.startLedgers();

        this.logger.info("Test environment started");
    }

    async teardown() {
        await super.teardown();

        loggerShutdown();
    }

    /**
     * Initializes all required ledgers with as much parallelism as possible.
     */
    private async startLedgers() {
        const startEthereum = this.ledgers.includes("ethereum");
        const startBitcoin = this.ledgers.includes("bitcoin");
        const startLightning = this.ledgers.includes("lightning");

        const tasks = [];

        if (startEthereum) {
            tasks.push(this.startEthereum());
        }

        if (startBitcoin && !startLightning) {
            tasks.push(this.startBitcoin());
        }

        if (startLightning) {
            tasks.push(this.startBitcoinAndLightning());
        }

        await Promise.all(tasks);
    }

    /**
     * Start the Bitcoin Ledger
     *
     * Once this function returns, the necessary configuration values have been set inside the test environment.
     */
    private async startBitcoin() {
        const lockDir = await this.getLockDirectory("bitcoind");
        const release = await lock(lockDir).catch(() =>
            Promise.reject(
                new Error(`Failed to acquire lock for starting bitcoind`)
            )
        );

        const bitcoind = await BitcoindInstance.new(
            await this.global.getDataDir("bitcoind"),
            path.join(lockDir, "bitcoind.pid"),
            this.logger
        );
        const config = await this.start(lockDir, bitcoind, async (bitcoind) => {
            const config = bitcoind.config;
            const rpcClient = new BitcoinRpcClient({
                network: config.network,
                port: config.rpcPort,
                host: config.host,
                username: config.username,
                password: config.password,
            });

            const name = "miner";
            await rpcClient.createWallet(name);

            this.logger.info(`Created miner wallet with name ${name}`);

            return { ...bitcoind.config, minerWallet: name };
        });

        const minerPidFile = path.join(lockDir, "miner.pid");

        try {
            await existsAsync(minerPidFile);
        } catch (e) {
            // miner is not running
            const tsNode = path.join(this.nodeModulesBinDir, "ts-node");
            const minerProgram = path.join(
                this.srcDir,
                "environment",
                "bitcoin_miner.ts"
            );

            await BitcoinMinerInstance.start(
                tsNode,
                minerProgram,
                path.join(lockDir, "config.json"),
                minerPidFile,
                this.logger
            );
        }

        this.global.environment.bitcoin = config;
        this.bitcoinFaucet = new BitcoinFaucet(config, this.logger);

        await release();
    }

    /**
     * Start the Ethereum Ledger
     *
     * Once this function returns, the necessary configuration values have been set inside the test environment.
     */
    private async startEthereum() {
        const lockDir = await this.getLockDirectory("geth");
        const release = await lock(lockDir).catch(() =>
            Promise.reject(
                new Error(`Failed to acquire lock for starting geth`)
            )
        );

        const geth = await GethInstance.new(
            await this.global.getDataDir("geth"),
            path.join(lockDir, "geth.pid"),
            this.logger
        );
        const config = await this.start(lockDir, geth, async (geth) => {
            const rpcUrl = geth.rpcUrl;
            const faucet = new EthereumFaucet(
                geth.devAccount,
                this.logger,
                rpcUrl,
                geth.CHAIN_ID
            );
            const erc20TokenContract = await faucet.deployErc20TokenContract();

            this.logger.info(
                "ERC20 token contract deployed at",
                erc20TokenContract
            );

            return {
                rpc_url: rpcUrl,
                devAccount: geth.devAccount,
                tokenContract: erc20TokenContract,
                chain_id: geth.CHAIN_ID,
            };
        });

        this.global.environment.ethereum = config;
        this.global.tokenContract = config.tokenContract;

        await release();
    }

    /**
     * First starts the Bitcoin and then the Lightning ledgers.
     *
     * The Lightning ledgers depend on Bitcoin to be up and running.
     */
    private async startBitcoinAndLightning() {
        await this.startBitcoin();

        // Lightning nodes can be started in parallel
        await Promise.all([
            this.startAliceLightning(),
            this.startBobLightning(),
        ]);
    }

    /**
     * Start the Lightning Ledger for Alice
     *
     * This function assumes that the Bitcoin ledger is initialized.
     * Once this function returns, the necessary configuration values have been set inside the test environment.
     */
    private async startAliceLightning() {
        const config = await this.initLightningLedger("lnd-alice");
        this.global.lndClients.alice = await LndClient.newInstance(
            config,
            this.logger
        );
        this.global.environment.aliceLnd = config;
    }

    /**
     * Start the Lightning Ledger for Bob
     *
     * This function assumes that the Bitcoin ledger is initialized.
     * Once this function returns, the necessary configuration values have been set inside the test environment.
     */
    private async startBobLightning() {
        const config = await this.initLightningLedger("lnd-bob");
        this.global.lndClients.bob = await LndClient.newInstance(
            config,
            this.logger
        );
        this.global.environment.bobLnd = config;
    }

    private async initLightningLedger(
        role: "lnd-alice" | "lnd-bob"
    ): Promise<LightningNode> {
        const lockDir = await this.getLockDirectory(role);
        const release = await lock(lockDir).catch(() =>
            Promise.reject(
                new Error(`Failed to acquire lock for starting ${role}`)
            )
        );

        const lnd = await LndInstance.new(
            await this.global.getDataDir(role),
            this.logger,
            this.bitcoinFaucet,
            await this.global.getDataDir("bitcoind"),
            path.join(lockDir, "lnd.pid")
        );

        const config = await this.start(
            lockDir,
            lnd,
            async (lnd) => lnd.config
        );

        await release();

        return config;
    }

    private async start<C, S extends Startable>(
        lockDir: string,
        instance: S,
        makeConfig: (instance: S) => Promise<C>
    ): Promise<C> {
        const configFile = path.join(lockDir, "config.json");

        this.logger.info("Checking for config file ", configFile);

        try {
            await existsAsync(configFile);

            this.logger.info(
                "Found config file, we'll be using that configuration instead of starting another instance"
            );

            const config = await asyncFs.readFile(configFile, {
                encoding: "utf-8",
            });

            return JSON.parse(config);
        } catch (e) {
            this.logger.info("No config file found, starting new instance");

            await instance.start();

            const config = await makeConfig(instance);

            await asyncFs.writeFile(configFile, JSON.stringify(config), {
                encoding: "utf-8",
            });

            this.logger.info("Config file written to", configFile);

            return config;
        }
    }

    private async getLockDirectory(
        process: "geth" | "bitcoind" | "lnd-alice" | "lnd-bob"
    ): Promise<string> {
        const dir = path.join(this.locksDir, process);

        await asyncFs.mkdir(dir, {
            recursive: true,
        });

        return dir;
    }
}

function extractLedgersToBeStarted(
    docblockPragmas: Record<string, string | string[]>
): string[] {
    const ledgersToStart = docblockPragmas.ledger;
    delete docblockPragmas.ledger;

    if (!ledgersToStart) {
        return [];
    }

    if (typeof ledgersToStart === "string") {
        return [ledgersToStart];
    }

    return ledgersToStart;
}

export function extractCndConfigOverrides(
    docblockPragmas: Record<string, string | string[]>
): Partial<CndConfig> {
    let configOverrides = docblockPragmas.cndConfigOverride;
    delete docblockPragmas.cndConfigOverride;

    if (!configOverrides) {
        return {};
    }

    // generalize single override to list of overrides
    if (typeof configOverrides === "string") {
        configOverrides = [configOverrides];
    }

    return configOverrides
        .map((override) => override.split(" = "))
        .filter(([key, _]) => key !== "")
        .reduce((config, [key, value]) => {
            set(config, key, value);

            return config;
        }, {});
}

export function assertNoUnhandledPargmas(
    docblockPragmas: Record<string, string | string[]>
) {
    for (const [pragma] of Object.entries(docblockPragmas)) {
        throw new Error(`Unhandled pragma '${pragma}'! Typo?`);
    }
}

/**
 * Locks the given directory for exclusive access.
 */
async function lock(lockDir: string): Promise<() => Promise<void>> {
    return properLockfile.lock(lockDir, {
        retries: {
            retries: 60,
            factor: 1,
            minTimeout: 500,
        },
    });
}
