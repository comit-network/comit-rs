import { JsonMap, stringify } from "@iarna/toml";
import { ChildProcess, spawn } from "child_process";
import * as fs from "fs";
import tempWrite from "temp-write";
import { BtsieveConfigFile } from "./config";

export class BtsieveRunner {
    private process?: ChildProcess;
    private readonly logDir: string;
    private readonly bin: string;
    private readonly projectRoot: string;

    constructor(projectRoot: string, bin: string, logDir: string) {
        this.logDir = logDir;
        this.bin = bin;
        this.projectRoot = projectRoot;
    }

    public async ensureBtsieveRunningWithConfig(
        ledgerConfig: BtsieveConfigFile
    ) {
        if (this.process) {
            return;
        }

        console.log("Starting btsieve");

        const configFile = await tempWrite(
            stringify((ledgerConfig as unknown) as JsonMap),
            "config.toml"
        );

        this.process = spawn(this.bin, ["--config", configFile], {
            cwd: this.projectRoot,
            env: {
                RUST_LOG: "warn,btsieve=debug,warp=info",
            },
            stdio: [
                "ignore", // stdin
                fs.openSync(this.logDir + "/btsieve.log", "w"), // stdout
                fs.openSync(this.logDir + "/btsieve.log", "w"), // stderr
            ],
        });
    }

    public stopBtsieve() {
        if (this.process) {
            console.log("Stopping btsieve");

            this.process.kill();
            this.process = undefined;
        }
    }
}
