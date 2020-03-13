import { Config } from "@jest/types";
import {
    execAsync,
    HarnessGlobal,
    mkdirAsync,
    rimrafAsync,
} from "../lib/utils";
import NodeEnvironment from "jest-environment-node";
import { Mutex } from "async-mutex";
import path from "path";
import { LightningWallet } from "../lib/wallets/lightning";
import { BitcoinWallet } from "../lib/wallets/bitcoin";
import { AssetKind } from "../lib/asset";
import { LedgerKind } from "../lib/ledgers/ledger";
import BitcoinLedger from "../lib/ledgers/bitcoin";
import { BitcoindInstance } from "../lib/ledgers/bitcoind_instance";
import EthereumLedger from "../lib/ledgers/ethereum";
import LightningLedger from "../lib/ledgers/lightning";
import { ParityInstance } from "../lib/ledgers/parity_instance";
import { LndInstance } from "../lib/ledgers/lnd_instance";

// ************************ //
// Setting global variables //
// ************************ //

export default class E2ETestEnvironment extends NodeEnvironment {
    private docblockPragmas: Record<string, string>;
    private projectRoot: string;
    private testRoot: string;
    private logDir: string;
    public global: HarnessGlobal;

    private bitcoinLedger?: BitcoinLedger;
    private ethereumLedger?: EthereumLedger;
    private aliceLightning?: LightningLedger;
    private bobLightning?: LightningLedger;

    constructor(config: Config.ProjectConfig, context: any) {
        super(config);

        this.docblockPragmas = context.docblockPragmas;
    }

    async setup() {
        await super.setup();

        // retrieve project root by using git
        const { stdout } = await execAsync("git rev-parse --show-toplevel", {
            encoding: "utf8",
        });
        this.projectRoot = stdout.trim();
        this.testRoot = path.join(this.projectRoot, "api_tests");

        // setup global variables
        this.global.projectRoot = this.projectRoot;
        this.global.testRoot = this.testRoot;
        this.global.ledgerConfigs = {};
        this.global.lndWallets = {};
        this.global.verbose =
            this.global.process.argv.find(item => item.includes("verbose")) !==
            undefined;

        this.global.parityAccountMutex = new Mutex();

        if (this.global.verbose) {
            console.log(`Starting up test environment`);
        }

        const { ledgers, logDir } = this.extractDocblockPragmas(
            this.docblockPragmas
        );

        this.logDir = path.join(this.projectRoot, "api_tests", "log", logDir);
        await E2ETestEnvironment.cleanLogDir(this.logDir);

        await this.startLedgers(ledgers);

        this.global.logRoot = this.logDir;
    }

    /**
     * Initializes all required ledgers with as much parallelism as possible.
     *
     * @param ledgers The list of ledgers to initialize
     */
    private async startLedgers(ledgers: string[]) {
        const startEthereum = ledgers.includes("ethereum");
        const startBitcoin = ledgers.includes("bitcoin");
        const startLightning = ledgers.includes("lightning");

        const tasks = [];

        if (startEthereum) {
            tasks.push(this.startEthereum());
        }

        if (startBitcoin && !startLightning) {
            tasks.push(this.startBitcoin());
        }

        if (startBitcoin && startLightning) {
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
        this.bitcoinLedger = await BitcoinLedger.start(
            await BitcoindInstance.new(this.projectRoot, this.logDir)
        );
        this.global.ledgerConfigs.bitcoin = this.bitcoinLedger.config;
    }
    /**
     * Start the Ethereum Ledger
     *
     * Once this function returns, the necessary configuration values have been set inside the test environment.
     */
    private async startEthereum() {
        this.ethereumLedger = await EthereumLedger.start(
            await ParityInstance.new(this.projectRoot, this.logDir)
        );
        this.global.ledgerConfigs.ethereum = this.ethereumLedger.config;
        this.global.tokenContract = this.ethereumLedger.config.tokenContract;
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

        await alice.openChannel(bob, 15000000);
    }

    /**
     * Start the Lightning Ledger for Alice
     *
     * This function assumes that the Bitcoin ledger is initialized.
     * Once this function returns, the necessary configuration values have been set inside the test environment.
     */
    private async startAliceLightning() {
        this.aliceLightning = await LightningLedger.start(
            await LndInstance.new(
                this.logDir,
                "lnd-alice",
                path.join(this.logDir, "bitcoind")
            )
        );

        this.global.lndWallets.alice = await LightningWallet.newInstance(
            await BitcoinWallet.newInstance(this.bitcoinLedger.config),
            this.aliceLightning.config.lnd,
            this.aliceLightning.config.p2pSocket
        );
    }

    /**
     * Start the Lightning Ledger for Bob
     *
     * This function assumes that the Bitcoin ledger is initialized.
     * Once this function returns, the necessary configuration values have been set inside the test environment.
     */
    private async startBobLightning() {
        this.bobLightning = await LightningLedger.start(
            await LndInstance.new(
                this.logDir,
                "lnd-bob",
                path.join(this.logDir, "bitcoind")
            )
        );

        this.global.lndWallets.bob = await LightningWallet.newInstance(
            await BitcoinWallet.newInstance(this.bitcoinLedger.config),
            this.bobLightning.config.lnd,
            this.bobLightning.config.p2pSocket
        );
    }

    private static async cleanLogDir(logDir: string) {
        await rimrafAsync(logDir);
        await mkdirAsync(logDir, { recursive: true });
    }

    async teardown() {
        await super.teardown();
        if (this.global.verbose) {
            console.log(`Tearing down test environment.`);
        }
        await this.cleanupAll();
        if (this.global.verbose) {
            console.log(`All teared down.`);
        }
    }

    async cleanupAll() {
        const tasks = [];

        if (this.bitcoinLedger) {
            tasks.push(this.bitcoinLedger.stop);
        }

        if (this.ethereumLedger) {
            tasks.push(this.ethereumLedger.stop);
        }

        if (this.aliceLightning) {
            tasks.push(this.aliceLightning.stop);
        }

        if (this.bobLightning) {
            tasks.push(this.bobLightning.stop);
        }

        await Promise.all(tasks);
    }

    private extractDocblockPragmas(
        docblockPragmas: Record<string, string>
    ): { logDir: string; ledgers: string[] } {
        const docblockLedgers = docblockPragmas.ledgers!;
        const ledgers = docblockLedgers ? docblockLedgers.split(",") : [];

        const logDir = this.docblockPragmas.logDir!;
        if (!logDir) {
            throw new Error(
                "Test file did not specify a log directory. Did you miss adding @logDir"
            );
        }

        return { ledgers, logDir };
    }
}
