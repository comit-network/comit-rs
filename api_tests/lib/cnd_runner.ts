import { CndInstance } from "../lib_sdk/cnd_instance";
import { CND_CONFIGS } from "./config";
import { LedgerConfig } from "./ledger_runner";

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

        console.log("Starting cnd for " + actorsToBeStarted.join(", "));

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
    }

    public stopCnds() {
        const names = Object.keys(this.runningNodes);

        if (names.length > 0) {
            console.log("Stopping cnds: " + names.join(", "));
            for (const process of Object.values(this.runningNodes)) {
                process.stop();
            }
            this.runningNodes = {};
        }
    }
}
