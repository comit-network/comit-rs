import * as bitcoin from "./bitcoin";
import * as toml from "toml";
import * as fs from "fs";
import { sleep } from "./util";
import { execSync, spawn } from "child_process";

const project_root: string = execSync("git rev-parse --show-toplevel", {
    encoding: "utf8",
}).trim();
const docker_cwd = project_root + "/api_tests/regtest";

export class LedgerRunner {
    private running_ledgers: { [key: string]: boolean };
    private readonly block_timers: { [key: string]: NodeJS.Timeout };
    private readonly docker_compose_options: { cwd: string; encoding: string };
    private readonly docker_cwd: string;
    private readonly log_dir: string;
    private readonly ledgers_config: any;

    constructor(
        project_root: string,
        ledgers_config_path: string,
        log_dir: string
    ) {
        this.running_ledgers = {};
        this.block_timers = {};
        this.docker_cwd = project_root + "/api_tests/regtest";
        this.log_dir = log_dir;

        this.ledgers_config = toml.parse(
            fs.readFileSync(ledgers_config_path, "utf8")
        );

        this.docker_compose_options = {
            cwd: this.docker_cwd,
            encoding: "utf8",
        };
    }

    async ensureLedgersRunning(ledgers: string[]) {
        let running_ledgers = this.running_ledgers;
        let to_be_started = ledgers.filter(name => !running_ledgers[name]);

        if (to_be_started.length > 0) {
            let wait_times = [0];

            let images_to_start = to_be_started.map(
                name => this.ledgers_config[name].docker
            );

            await spawn("docker-compose", ["up", ...images_to_start], {
                cwd: this.docker_cwd,
                stdio: [
                    "ignore",
                    fs.openSync(`${this.log_dir}/docker-compose.log`, "w"),
                    "inherit",
                ],
            });

            for (let ledger of to_be_started) {
                let ledger_config = this.ledgers_config[ledger];
                this.running_ledgers[ledger] = true;
                wait_times.push(
                    process.env.CARGO_MAKE_CI === "TRUE"
                        ? ledger_config.ci_docker_wait
                        : ledger_config.local_docker_wait
                );
            }

            let wait_time = Math.max(...wait_times);
            console.log(
                `Waiting ${wait_time}ms for ${to_be_started.join(
                    ", "
                )} to start`
            );

            await sleep(wait_time);

            if (to_be_started.includes("bitcoin")) {
                bitcoin.init(this.ledgers_config.bitcoin);
                this.block_timers["bitcoin"] = setInterval(async () => {
                    await bitcoin.generate();
                }, 3000);
            }
        }
    }

    stopLedgers() {
        let names = Object.keys(this.running_ledgers);
        if (names.length > 0) {
            console.log("Stopping ledgers: " + names.join(", "));

            Object.values(this.block_timers).forEach(clearInterval);

            execSync("docker-compose rm -sfv ", this.docker_compose_options);
        }
        this.running_ledgers = {};
    }

    getLedgersConfig(): any {
        return this.ledgers_config;
    }
}
