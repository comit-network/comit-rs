import { execSync, spawn } from "child_process";
import * as fs from "fs";
import * as toml from "toml";
import * as bitcoin from "./bitcoin";
import { sleep } from "./util";

export class LedgerRunner {
    private runningLedgers: { [key: string]: boolean };
    private readonly blockTimers: { [key: string]: NodeJS.Timeout };
    private readonly dockerComposeFile: string;
    private readonly logDir: string;
    private readonly ledgersConfig: any;

    constructor(
        dockerComposeFile: string,
        ledgersConfigPath: string,
        logDir: string
    ) {
        this.runningLedgers = {};
        this.blockTimers = {};
        this.dockerComposeFile = dockerComposeFile;
        this.logDir = logDir;

        this.ledgersConfig = toml.parse(
            fs.readFileSync(ledgersConfigPath, "utf8")
        );
    }

    public async ensureLedgersRunning(ledgers: string[]) {
        const runningLedgers = this.runningLedgers;
        const toBeStarted = ledgers.filter(name => !runningLedgers[name]);

        if (toBeStarted.length > 0) {
            const waitTimes = [0];

            const imagesToStart = toBeStarted.map(
                name => this.ledgersConfig[name].docker
            );

            await spawn(
                "docker-compose",
                ["-f", this.dockerComposeFile, "up", ...imagesToStart],
                {
                    stdio: [
                        "ignore",
                        fs.openSync(`${this.logDir}/docker-compose.log`, "w"),
                        "inherit",
                    ],
                }
            );

            for (const ledger of toBeStarted) {
                const ledgerConfig = this.ledgersConfig[ledger];
                this.runningLedgers[ledger] = true;
                waitTimes.push(
                    process.env.CARGO_MAKE_CI === "TRUE"
                        ? ledgerConfig.ci_docker_wait
                        : ledgerConfig.local_docker_wait
                );
            }

            const waitTime = Math.max(...waitTimes);
            console.log(
                `Waiting ${waitTime}ms for ${toBeStarted.join(", ")} to start`
            );

            await sleep(waitTime);

            if (toBeStarted.includes("bitcoin")) {
                bitcoin.init(this.ledgersConfig.bitcoin);
                this.blockTimers.bitcoin = global.setInterval(async () => {
                    await bitcoin.generate();
                }, 1000);
            }
        }
    }

    public stopLedgers() {
        const names = Object.keys(this.runningLedgers);
        if (names.length > 0) {
            console.log("Stopping ledgers: " + names.join(", "));

            Object.values(this.blockTimers).forEach(clearInterval);

            execSync("docker-compose -f " + this.dockerComposeFile + " down");
        }
        this.runningLedgers = {};
    }

    public getLedgersConfig(): any {
        return this.ledgersConfig;
    }
}
