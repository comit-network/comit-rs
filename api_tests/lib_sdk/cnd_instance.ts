import { JsonMap, stringify } from "@iarna/toml";
import { ChildProcess, spawn } from "child_process";
import * as fs from "fs";
import { PEMObject } from "pem-ts";
import tempWrite from "temp-write";
import { promisify } from "util";
import { CndConfigFile, E2ETestActorConfig } from "../lib/config";
import { LedgerConfig } from "../lib/ledger_runner";
import { sleep } from "../lib/util";

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

    public async start(withConfigFile?: CndConfigFile) {
        const bin = process.env.CND_BIN
            ? process.env.CND_BIN
            : this.projectRoot + "/target/debug/cnd";

        if (withConfigFile) {
            this.configFile = withConfigFile;
        } else {
            this.configFile = this.actorConfig.generateCndConfigFile(
                this.ledgerConfig
            );
        }

        const configFile = await tempWrite(
            stringify((this.configFile as unknown) as JsonMap),
            "config.toml"
        );

        const pemObject = new PEMObject("SEED", this.actorConfig.seed);
        const seedFile = await tempWrite(pemObject.encoded, "seed.pem");

        this.process = spawn(
            bin,
            ["--config", configFile, "--seed-file", seedFile],
            {
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
            }
        );

        this.process.on("exit", (code: number, signal: number) => {
            console.log(
                `cnd ${this.actorConfig.name} exited with ${code ||
                    "signal " + signal}`
            );
        });

        await sleep(1000); // allow the nodes to start up
    }

    public stop() {
        this.process.kill("SIGINT");
        this.configFile = null;
    }
}
