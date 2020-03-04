import { JsonMap, stringify } from "@iarna/toml";
import { ChildProcess, spawn } from "child_process";
import * as fs from "fs";
import tempWrite from "temp-write";
import { promisify } from "util";
import { CndConfigFile, E2ETestActorConfig } from "../config";
import { LedgerConfig } from "../ledgers/ledger_runner";
import { HarnessGlobal, sleep } from "../utils";
import path from "path";
import { LogReader } from "../ledgers/log_reader";

declare var global: HarnessGlobal;

const openAsync = promisify(fs.open);

export class CndInstance {
    private process: ChildProcess;
    private configFile?: CndConfigFile;

    constructor(
        private readonly projectRoot: string,
        private readonly logDir: string,
        private readonly actorConfig: E2ETestActorConfig,
        private readonly ledgerConfig: LedgerConfig
    ) {}

    public getConfigFile() {
        return this.configFile;
    }

    public async start() {
        const bin = process.env.CND_BIN
            ? process.env.CND_BIN
            : this.projectRoot + "/target/debug/cnd";

        if (global.verbose) {
            console.log(`[${this.actorConfig.name}] using binary ${bin}`);
        }

        this.configFile = this.actorConfig.generateCndConfigFile(
            this.ledgerConfig
        );

        const configFile = await tempWrite(
            stringify((this.configFile as unknown) as JsonMap),
            "config.toml"
        );

        const logFile = path.join(
            this.logDir,
            `cnd-${this.actorConfig.name}.log`
        );

        this.process = spawn(bin, ["--config", configFile], {
            cwd: this.projectRoot,
            stdio: [
                "ignore", // stdin
                await openAsync(logFile, "w"), // stdout
                await openAsync(logFile, "w"), // stderr
            ],
        });

        if (global.verbose) {
            console.log(
                `[${this.actorConfig.name}] process spawned with PID ${this.process.pid}`
            );
        }

        this.process.on("exit", (code: number, signal: number) => {
            if (global.verbose) {
                console.log(
                    `cnd ${this.actorConfig.name} exited with ${code ||
                        `signal ${signal}`}`
                );
            }
        });

        const logReader = new LogReader(logFile);
        await logReader.waitForLogMessage("Starting HTTP server on");

        // we emit the log _before_ we start the http server, let's make sure it actually starts up
        await sleep(1000);
    }

    public stop() {
        this.process.kill("SIGINT");
        this.process = null;
    }

    public isRunning() {
        return this.process != null;
    }
}
