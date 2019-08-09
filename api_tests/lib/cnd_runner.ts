import { ChildProcess, spawn } from "child_process";
import * as fs from "fs";
import { MetaCndConfig } from "./comit";
import { sleep } from "./util";

export class CndRunner {
    private runningNodes: { [key: string]: ChildProcess };
    private readonly projectRoot: string;
    private readonly logDir: string;
    private readonly cndBin: string;

    constructor(projectRoot: string, btsieveBin: string, logDir: string) {
        this.runningNodes = {};
        this.logDir = logDir;
        this.cndBin = btsieveBin;
        this.projectRoot = projectRoot;
    }

    public async ensureCndsRunning(cnds: Array<[string, MetaCndConfig]>) {
        console.log(
            "Starting cnd for " + cnds.map(([name]) => name).join(", ")
        );
        for (const [name, comitConfig] of cnds) {
            if (this.runningNodes[name]) {
                continue;
            }

            this.runningNodes[name] = await spawn(
                this.cndBin,
                ["--config", comitConfig.config_file],
                {
                    cwd: this.projectRoot,
                    stdio: [
                        "ignore",
                        fs.openSync(this.logDir + "/cnd-" + name + ".log", "w"),
                        fs.openSync(this.logDir + "/cnd-" + name + ".log", "w"),
                    ],
                }
            );

            await sleep(500);

            this.runningNodes[name].on(
                "exit",
                (code: number, signal: number) => {
                    console.log(
                        `cnd ${name} exited with ${code || "signal " + signal}`
                    );
                }
            );
        }

        await sleep(2000);
    }

    public stopCnds() {
        const names = Object.keys(this.runningNodes);

        if (names.length > 0) {
            console.log("Stopping cnds: " + names.join(", "));
            for (const process of Object.values(this.runningNodes)) {
                process.kill();
            }
            this.runningNodes = {};
        }
    }
}
