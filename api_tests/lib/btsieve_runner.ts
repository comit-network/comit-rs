import { ChildProcess, spawn } from "child_process";
import * as fs from "fs";
import { MetaBtsieveConfig } from "./btsieve";

export class BtsieveRunner {
    private runningBtsieves: { [key: string]: ChildProcess };
    private readonly logDir: string;
    private readonly btsieveBin: string;
    private readonly projectRoot: string;

    constructor(projectRoot: string, btsieveBin: string, logDir: string) {
        this.runningBtsieves = {};
        this.logDir = logDir;
        this.btsieveBin = btsieveBin;
        this.projectRoot = projectRoot;
    }

    public ensureBtsievesRunning(btsieves: Array<[string, MetaBtsieveConfig]>) {
        for (const [name, btsieveConfig] of btsieves) {
            console.log("Ensuring Btsieve: " + name + " is started");

            if (this.runningBtsieves[name]) {
                console.log("Btsieve is already started");
                continue;
            }

            console.log(
                "Starting Btsieve: " + name + "; path:",
                this.btsieveBin
            );
            this.runningBtsieves[name] = spawn(
                this.btsieveBin,
                ["--config", btsieveConfig.config_file],
                {
                    cwd: this.projectRoot,
                    env: btsieveConfig.env,
                    stdio: [
                        "ignore",
                        fs.openSync(
                            this.logDir + "/btsieve-" + name + ".log",
                            "w"
                        ),
                        fs.openSync(
                            this.logDir + "/btsieve-" + name + ".log",
                            "w"
                        ),
                    ],
                }
            );
        }
    }

    public stopBtsieves() {
        const names = Object.keys(this.runningBtsieves);

        if (names.length > 0) {
            console.log("Stopping Btsieve(s): " + names.join(", "));
            for (const process of Object.values(this.runningBtsieves)) {
                process.kill();
            }
        }

        this.runningBtsieves = {};
    }
}
