import { Config } from "@jest/types";
import { LedgerRunner } from "../lib/ledgers/ledger_runner";
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

// ************************ //
// Setting global variables //
// ************************ //

export default class E2ETestEnvironment extends NodeEnvironment {
    private docblockPragmas: Record<string, string>;
    private projectRoot: string;
    private testRoot: string;
    private logDir: string;
    private ledgerRunner: LedgerRunner;
    public global: HarnessGlobal;

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

        if (ledgers.length > 0) {
            // setup ledgers
            this.ledgerRunner = new LedgerRunner(this.projectRoot, this.logDir);

            if (this.global.verbose) {
                console.log(`Initializing ledgers : ${ledgers}`);
            }
            const ledgerConfig = await this.ledgerRunner.ensureLedgersRunning(
                ledgers
            );

            const ethereum = ledgerConfig.ethereum;
            if (ethereum) {
                this.global.tokenContract = ethereum.tokenContract;
                this.global.ledgerConfigs.ethereum = ethereum;
            }

            const bitcoin = ledgerConfig.bitcoin;
            if (bitcoin) {
                this.global.ledgerConfigs.bitcoin = bitcoin;
            }

            const lndAlice = ledgerConfig.lndAlice;
            const lndBob = ledgerConfig.lndBob;

            if (lndAlice && lndBob && bitcoin) {
                this.global.ledgerConfigs.lndAlice = lndAlice;
                this.global.ledgerConfigs.lndBob = lndBob;

                const aliceWallet = await LightningWallet.newInstance(
                    await BitcoinWallet.newInstance(bitcoin),
                    lndAlice.lnd,
                    lndAlice.p2pSocket
                );
                const bobWallet = await LightningWallet.newInstance(
                    await BitcoinWallet.newInstance(bitcoin),
                    lndBob.lnd,
                    lndBob.p2pSocket
                );
                this.global.lndWallets = {
                    alice: aliceWallet,
                    bob: bobWallet,
                };

                await aliceWallet.connectPeer(bobWallet);

                await aliceWallet.mint({
                    name: AssetKind.Bitcoin,
                    ledger: LedgerKind.Lightning,
                    quantity: "15000000",
                });

                await aliceWallet.openChannel(bobWallet, 15000000);
            }
        }

        this.global.logRoot = this.logDir;
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
        this.cleanupAll();
        if (this.global.verbose) {
            console.log(`All teared down.`);
        }
    }

    cleanupAll() {
        try {
            if (this.ledgerRunner) {
                this.ledgerRunner.stopLedgers();
            }
        } catch (e) {
            console.error("Failed to clean up resources", e);
        }
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
