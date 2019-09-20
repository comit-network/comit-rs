import { JsonMap, stringify } from "@iarna/toml";
import { ChildProcess, spawn } from "child_process";
import * as fs from "fs";
import tempWrite from "temp-write";
import { BtsieveConfigFile, CND_CONFIGS } from "./config";
import { sleep } from "./util";

export class CndRunner {
    private runningNodes: { [key: string]: ChildProcess };
    private readonly projectRoot: string;
    private readonly logDir: string;
    private readonly bin: string;

    constructor(projectRoot: string, bin: string, logDir: string) {
        this.runningNodes = {};
        this.logDir = logDir;
        this.bin = bin;
        this.projectRoot = projectRoot;
    }

    public async ensureCndsRunning(
        actors: string[],
        btsieveConfig: BtsieveConfigFile
    ) {
        const actorsToBeStarted = actors.filter(
            actor => !Object.keys(this.runningNodes).includes(actor)
        );

        console.log("Starting cnd for " + actorsToBeStarted.join(", "));

        const promises = actorsToBeStarted.map(async name => {
            const cndconfig = CND_CONFIGS[name];

            if (!cndconfig) {
                throw new Error(
                    `Please define a cnd configuration for ${name}`
                );
            }

            const configFile = await tempWrite(
                stringify((cndconfig.generateCndConfigFile(
                    btsieveConfig
                ) as unknown) as JsonMap),
                "config.toml"
            );

            const process = spawn(this.bin, ["--config", configFile], {
                cwd: this.projectRoot,
                stdio: [
                    "ignore", // stdin
                    fs.openSync(this.logDir + "/cnd-" + name + ".log", "w"), // stdout
                    fs.openSync(this.logDir + "/cnd-" + name + ".log", "w"), // stderr
                ],
            });

            process.on("exit", (code: number, signal: number) => {
                console.log(
                    `cnd ${name} exited with ${code || "signal " + signal}`
                );
            });

            await sleep(500); // allow the nodes to start up

            return {
                name,
                process,
            };
        });

        const startedNodes = await Promise.all(promises);

        for (const { name, process } of startedNodes) {
            this.runningNodes[name] = process;
        }
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
