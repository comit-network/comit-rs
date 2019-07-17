import { ChildProcess, spawn } from "child_process";
import * as fs from "fs";
import { MetaBtsieveConfig } from "./btsieve";

export class BtsieveRunner {
    public running_btsieves: { [key: string]: ChildProcess };
    private readonly log_dir: string;
    private readonly btsieve_bin: string;
    private readonly project_root: string;

    constructor(project_root: string, btsieve_bin: string, log_dir: string) {
        this.running_btsieves = {};
        this.log_dir = log_dir;
        this.btsieve_bin = btsieve_bin;
        this.project_root = project_root;
    }

    public ensureBtsievesRunning(btsieves: Array<[string, MetaBtsieveConfig]>) {
        for (const [name, btsieve_config] of btsieves) {
            console.log("Starting Btsieve: " + name);

            if (this.running_btsieves[name]) {
                continue;
            }

            this.running_btsieves[name] = spawn(
                this.btsieve_bin,
                ["--config", btsieve_config.config_file],
                {
                    cwd: this.project_root,
                    env: btsieve_config.env,
                    stdio: [
                        "ignore",
                        fs.openSync(
                            this.log_dir + "/btsieve-" + name + ".log",
                            "w"
                        ),
                        fs.openSync(
                            this.log_dir + "/btsieve-" + name + ".log",
                            "w"
                        ),
                    ],
                }
            );
        }
    }

    public stopBtsieves() {
        const names = Object.keys(this.running_btsieves);

        if (names.length > 0) {
            console.log("Stopping Btsieve(s): " + names.join(", "));
            for (const process of Object.values(this.running_btsieves)) {
                process.kill();
            }
        }

        this.running_btsieves = {};
    }
}
