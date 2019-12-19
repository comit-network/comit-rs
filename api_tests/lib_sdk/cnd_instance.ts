import { JsonMap, stringify } from "@iarna/toml";
import { ChildProcess, spawn } from "child_process";
import * as fs from "fs";
import tempWrite from "temp-write";
import { promisify } from "util";
import { CndConfigFile, E2ETestActorConfig } from "../lib/config";
import { LedgerConfig } from "../lib/ledger_runner";
import { sleep } from "../lib/util";
import { HarnessGlobal } from "../lib/util";

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

        this.configFile = this.actorConfig.generateCndConfigFile(
            this.ledgerConfig
        );

        const configFile = await tempWrite(
            stringify((this.configFile as unknown) as JsonMap),
            "config.toml"
        );

        this.process = spawn(bin, ["--config", configFile], {
            cwd: this.projectRoot,
            stdio: [
                "ignore", // stdin
                await openAsync(
                    this.logDir + "/cnd-" + this.actorConfig.name + ".log",
                    "w"
                ), // stdout
                await openAsync(
                    this.logDir + "/cnd-" + this.actorConfig.name + ".log",
                    "w"
                ), // stderr
            ],
        });

        this.process.on("exit", (code: number, signal: number) => {
            if (global.verbose) {
                console.log(
                    `cnd ${this.actorConfig.name} exited with ${code ||
                        "signal " + signal}`
                );
            }
        });

        await sleep(200); // allow the nodes to start up
    }

    public stop() {
        this.process.kill("SIGINT");
        this.configFile = null;
    }
}
