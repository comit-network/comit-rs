import * as path from "path";
import { CndInstance } from "./cnd_instance";
import { existsAsync, HarnessGlobal, unlinkAsync } from "../utils";
import { CND_CONFIGS } from "../config";
import { LedgerConfig } from "../ledgers/ledger_runner";

declare var global: HarnessGlobal;

export class CndRunner {
    private runningNodes: { [key: string]: CndInstance };

    constructor(
        private readonly projectRoot: string,
        private readonly logDir: string
    ) {
        this.runningNodes = {};
    }

    public async ensureCndsRunning(
        actors: string[],
        ledgerConfig: LedgerConfig
    ) {
        const actorsToBeStarted = actors.filter(
            actor => !Object.keys(this.runningNodes).includes(actor)
        );

        if (global.verbose) {
            console.log("Starting cnd for " + actorsToBeStarted.join(", "));
        }

        const promises = actorsToBeStarted.map(async name => {
            const cndconfig = CND_CONFIGS[name];

            if (!cndconfig) {
                throw new Error(
                    `Please define a cnd configuration for ${name}`
                );
            }

            const process = new CndInstance(
                this.projectRoot,
                this.logDir,
                cndconfig,
                ledgerConfig
            );

            const db = path.join(cndconfig.data, "cnd.sqlite");

            if (await existsAsync(db)) {
                await unlinkAsync(db); // delete the old database for the new test
            }

            await process.start();

            return {
                name,
                process,
            };
        });

        const startedNodes = await Promise.all(promises);

        for (const { name, process } of startedNodes) {
            this.runningNodes[name] = process;
        }

        if (global.verbose) {
            console.log("All nodes successfully started");
        }
    }

    public stopCnds() {
        const names = Object.keys(this.runningNodes);

        if (names.length > 0) {
            if (global.verbose) {
                console.log("Stopping cnds: " + names.join(", "));
            }
            for (const process of Object.values(this.runningNodes)) {
                process.stop();
            }
            this.runningNodes = {};
        }
    }
}
