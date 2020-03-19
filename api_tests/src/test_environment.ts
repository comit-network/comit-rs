import { Config } from "@jest/types";
import {
    existsAsync,
    HarnessGlobal,
    mkdirAsync,
    readFileAsync,
    writeFileAsync,
} from "./utils";
import NodeEnvironment from "jest-environment-node";
import path from "path";
import { LightningWallet } from "./wallets/lightning";
import { BitcoinWallet } from "./wallets/bitcoin";
import { AssetKind } from "./asset";
import { LedgerKind } from "./ledgers/ledger";
import { BitcoindInstance } from "./ledgers/bitcoind_instance";
import { configure, Logger, shutdown as loggerShutdown } from "log4js";
import { EnvironmentContext } from "@jest/environment";
import ledgerLock from "./ledgers/ledger_lock";
import BitcoinMinerInstance from "./ledgers/bitcoin_miner_instance";
import { EthereumWallet } from "./wallets/ethereum";
import { LedgerInstance, LightningNodeConfig } from "./ledgers";
import { ParityInstance } from "./ledgers/parity_instance";
import { LndInstance } from "./ledgers/lnd_instance";

export default class TestEnvironment extends NodeEnvironment {
    private readonly projectRoot: string;
    private readonly testSuite: string;
    private readonly ledgers: string[];

    public global: HarnessGlobal;

    private logger: Logger;
    private logDir: string;

    constructor(config: Config.ProjectConfig, context: EnvironmentContext) {
        super(config);

        this.ledgers = TestEnvironment.extractLedgersToBeStarted(
            context.docblockPragmas
        );
        this.projectRoot = path.resolve(config.rootDir, "..");
        this.testSuite = path.parse(context.testPath).name;
    }

    async setup() {
        await super.setup();

        // setup global variables
        this.global.projectRoot = this.projectRoot;
        this.global.ledgerConfigs = {};
        this.global.lndWallets = {};

        this.logDir = path.join(this.projectRoot, "api_tests", "log");

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
        await mkdirAsync(testLogDir, { recursive: true });

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
            await mkdirAsync(dir, { recursive: true });

            return dir;
        };
        this.global.parityLockDir = await this.getLockDirectory("parity");

        this.logger.info("Starting up test environment");

        await this.startLedgers();
    }

    async teardown() {
        await super.teardown();

        await this.cleanupAll();

        loggerShutdown();
    }

    async cleanupAll() {
        const tasks = [];

        for (const [, wallet] of Object.entries(this.global.lndWallets)) {
            tasks.push(wallet.close());
        }

        await Promise.all(tasks);
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
        const release = await ledgerLock(lockDir);

        const bitcoind = await BitcoindInstance.new(
            this.projectRoot,
            await this.global.getDataDir("bitcoind"),
            path.join(lockDir, "bitcoind.pid"),
            this.logger
        );
        const config = await this.startLedger(
            lockDir,
            bitcoind,
            async (bitcoind) => bitcoind.config
        );

        const minerPidFile = path.join(lockDir, "miner.pid");

        const minerAlreadyRunning = await existsAsync(minerPidFile);

        if (!minerAlreadyRunning) {
            const tsNode = path.join(
                this.projectRoot,
                "api_tests",
                "node_modules",
                ".bin",
                "ts-node"
            );
            const minerProgram = path.join(
                this.projectRoot,
                "api_tests",
                "src",
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

        this.global.ledgerConfigs.bitcoin = config;

        await release();
    }

    /**
     * Start the Ethereum Ledger
     *
     * Once this function returns, the necessary configuration values have been set inside the test environment.
     */
    private async startEthereum() {
        const lockDir = await this.getLockDirectory("parity");
        const release = await ledgerLock(lockDir);

        const parity = await ParityInstance.new(
            this.projectRoot,
            await this.global.getDataDir("parity"),
            path.join(lockDir, "parity.pid"),
            this.logger
        );
        const config = await this.startLedger(
            lockDir,
            parity,
            async (parity) => {
                const rpcUrl = parity.rpcUrl;

                const erc20Wallet = new EthereumWallet(
                    rpcUrl,
                    this.logger,
                    lockDir
                );
                const erc20TokenContract = await erc20Wallet.deployErc20TokenContract();

                this.logger.info(
                    "ERC20 token contract deployed at",
                    erc20TokenContract
                );

                return {
                    rpc_url: rpcUrl,
                    tokenContract: erc20TokenContract,
                };
            }
        );

        this.global.ledgerConfigs.ethereum = config;
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

        await this.setupLightningChannels();
    }

    private async setupLightningChannels() {
        const { alice, bob } = this.global.lndWallets;

        await alice.connectPeer(bob);

        await alice.mint({
            name: AssetKind.Bitcoin,
            ledger: LedgerKind.Lightning,
            quantity: "15000000",
        });

        await bob.mint({
            name: AssetKind.Bitcoin,
            ledger: LedgerKind.Lightning,
            quantity: "15000000",
        });

        await alice.openChannel(bob, 15000000);
        await bob.openChannel(alice, 15000000);
    }

    /**
     * Start the Lightning Ledger for Alice
     *
     * This function assumes that the Bitcoin ledger is initialized.
     * Once this function returns, the necessary configuration values have been set inside the test environment.
     */
    private async startAliceLightning() {
        const config = await this.initLightningLedger("lnd-alice");
        this.global.lndWallets.alice = await this.initLightningWallet(config);
    }

    /**
     * Start the Lightning Ledger for Bob
     *
     * This function assumes that the Bitcoin ledger is initialized.
     * Once this function returns, the necessary configuration values have been set inside the test environment.
     */
    private async startBobLightning() {
        const config = await this.initLightningLedger("lnd-bob");
        this.global.lndWallets.bob = await this.initLightningWallet(config);
    }

    private async initLightningWallet(config: LightningNodeConfig) {
        return LightningWallet.newInstance(
            await BitcoinWallet.newInstance(
                this.global.ledgerConfigs.bitcoin,
                this.logger
            ),
            this.logger,
            config
        );
    }

    private async initLightningLedger(
        role: string
    ): Promise<LightningNodeConfig> {
        const lockDir = await this.getLockDirectory(role);
        const release = await ledgerLock(lockDir);

        const lnd = await LndInstance.new(
            await this.global.getDataDir(role),
            this.logger,
            await this.global.getDataDir("bitcoind"),
            path.join(lockDir, "lnd.pid")
        );

        const config = await this.startLedger(
            lockDir,
            lnd,
            async (lnd) => lnd.config
        );

        await release();

        return config;
    }

    private async startLedger<C, S extends LedgerInstance>(
        lockDir: string,
        instance: S,
        makeConfig: (instance: S) => Promise<C>
    ): Promise<C> {
        const configFile = path.join(lockDir, "config.json");

        this.logger.info("Checking for config file ", configFile);
        const configFileExists = await existsAsync(configFile);

        if (configFileExists) {
            this.logger.info(
                "Found config file, we'll be using that configuration instead of starting another instance"
            );

            const config = await readFileAsync(configFile, {
                encoding: "utf-8",
            });

            return JSON.parse(config);
        } else {
            this.logger.info("No config file found, starting ledger");

            await instance.start();

            const config = await makeConfig(instance);

            await writeFileAsync(configFile, JSON.stringify(config), {
                encoding: "utf-8",
            });

            this.logger.info("Config file written to", configFile);

            return config;
        }
    }

    private async getLockDirectory(process: string): Promise<string> {
        const dir = path.join(this.projectRoot, "api_tests", "locks", process);

        await mkdirAsync(dir, {
            recursive: true,
        });

        return dir;
    }

    private static extractLedgersToBeStarted(
        docblockPragmas: Record<string, string | string[]>
    ): string[] {
        const ledgersToStart = docblockPragmas.ledger;

        if (!ledgersToStart) {
            return [];
        }

        if (typeof ledgersToStart === "string") {
            return [ledgersToStart];
        }

        return ledgersToStart;
    }
}
