import { ChildProcess, execSync, spawn } from "child_process";
import { MetaBtsieveConfig } from "./btsieve";
import * as fs from "fs";

const project_root: string = execSync("git rev-parse --show-toplevel", {
    encoding: "utf8",
}).trim();

const log_dir = project_root + "/api_tests/log";

export class BtsieveRunner {
    running_btsieves: { [key: string]: ChildProcess };

    constructor() {
        this.running_btsieves = {};
    }

    async ensureBtsievesRunning(btsieves: [string, MetaBtsieveConfig][]) {
        for (let [name, btsieve_config] of btsieves) {
            console.log("Starting Btsieve: " + name);

            if (this.running_btsieves[name]) {
                continue;
            }

            this.running_btsieves[name] = await spawn(
                project_root + "/target/debug/btsieve",
                [],
                {
                    cwd: project_root,
                    env: btsieve_config.env,
                    stdio: [
                        "ignore",
                        fs.openSync(log_dir + "/btsieve-" + name + ".log", "w"),
                        fs.openSync(log_dir + "/btsieve-" + name + ".log", "w"),
                    ],
                }
            );
        }
    }

    async stopBtsieves() {
        let names = Object.keys(this.running_btsieves);

        if (names.length > 0) {
            console.log("Stopping Btsieve(s): " + names.join(", "));
            for (let process of Object.values(this.running_btsieves)) {
                process.kill();
            }
        }

        this.running_btsieves = {};
    }
}
