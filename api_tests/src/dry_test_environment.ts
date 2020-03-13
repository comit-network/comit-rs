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

// ************************ //
// Setting global variables //
// ************************ //

export default class DryTestEnvironment extends NodeEnvironment {
    private docblockPragmas: Record<string, string>;
    private projectRoot: string;
    private testRoot: string;
    private logDir: string;
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
        this.global.verbose =
            this.global.process.argv.find(item => item.includes("verbose")) !==
            undefined;

        this.global.parityAccountMutex = new Mutex();

        if (this.global.verbose) {
            console.log(`Starting up test environment`);
        }

        const { logDir } = this.extractDocblockPragmas(this.docblockPragmas);

        this.logDir = path.join(this.projectRoot, "api_tests", "log", logDir);
        await DryTestEnvironment.cleanLogDir(this.logDir);

        this.global.logRoot = this.logDir;
    }

    private static async cleanLogDir(logDir: string) {
        await rimrafAsync(logDir);
        await mkdirAsync(logDir, { recursive: true });
    }

    async teardown() {
        await super.teardown();
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
