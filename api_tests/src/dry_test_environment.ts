import { Config } from "@jest/types";
import { execAsync, HarnessGlobal, mkdirAsync, rimrafAsync } from "./utils";
import NodeEnvironment from "jest-environment-node";
import { Mutex } from "async-mutex";
import path from "path";
import { configure, shutdown as loggerShutdown } from "log4js";

// ************************ //
// Setting global variables //
// ************************ //

export default class DryTestEnvironment extends NodeEnvironment {
    private docblockPragmas: Record<string, string>;
    private projectRoot: string;
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

        // setup global variables
        this.global.projectRoot = this.projectRoot;
        this.global.ledgerConfigs = {};
        this.global.parityAccountMutex = new Mutex();

        const suiteConfig = this.extractDocblockPragmas(this.docblockPragmas);
        const logDir = path.join(
            this.projectRoot,
            "api_tests",
            "log",
            suiteConfig.logDir
        );

        await DryTestEnvironment.cleanLogDir(logDir);

        const log4js = configure({
            appenders: {
                multi: {
                    type: "multiFile",
                    base: logDir,
                    property: "categoryName",
                    extension: ".log",
                    layout: {
                        type: "pattern",
                        pattern: "%d %5.10p: %m",
                    },
                    timeout: 5000,
                },
            },
            categories: {
                default: { appenders: ["multi"], level: "debug" },
            },
        });

        const logger = log4js.getLogger("test_environment");
        logger.info("Starting up test environment");

        this.global.getLogFile = (pathElements) =>
            path.join(logDir, ...pathElements);
        this.global.getDataDir = async (program) => {
            const dir = path.join(logDir, program);
            await mkdirAsync(dir, { recursive: true });

            return dir;
        };
        this.global.getLogger = (category) => log4js.getLogger(category);
    }

    private static async cleanLogDir(logDir: string) {
        await rimrafAsync(logDir);
        await mkdirAsync(logDir, { recursive: true });
    }

    async teardown() {
        await super.teardown();

        loggerShutdown();
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
