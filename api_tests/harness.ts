#!/usr/bin/env ./api_tests/node_modules/.bin/ts-node --project api_tests/tsconfig.json

import * as bitcoin from "./lib/bitcoin";
import { ChildProcess, execSync } from "child_process";
import { spawn } from "child_process";
import { BtsieveConfig } from "./lib/btsieve";
import { HarnessGlobal, sleep } from "./lib/util";
import { MetaComitNodeConfig } from "./lib/comit";
import * as toml from "toml";
import * as fs from "fs";

const Mocha = require("mocha");
const path = require("path");
const commander = require("commander");

commander
    .option("--dump-logs", "Dump logs to stdout on failure")
    .parse(process.argv);

// ************************ //
// Setting global variables //
// ************************ //

declare const global: HarnessGlobal;

const project_root: string = execSync("git rev-parse --show-toplevel", {
    encoding: "utf8",
}).trim();
global.project_root = project_root;

const docker_cwd = project_root + "/api_tests/regtest";
const test_root = project_root + "/api_tests";
global.test_root = test_root;

const docker_compose_options = {
    cwd: docker_cwd,
    encoding: "utf8",
};

const ledgers_config = toml.parse(
    fs.readFileSync(project_root + "/api_tests/regtest/ledgers.toml", "utf8")
);
global.ledgers_config = ledgers_config;

const log_dir = project_root + "/api_tests/log";

if (!fs.existsSync(log_dir)) {
    fs.mkdirSync(log_dir);
}

// ********************** //
// Start services helpers //
// ********************** //
class LedgerRunner {
    running_ledgers: { [key: string]: boolean };
    block_timers: { [key: string]: NodeJS.Timeout };

    constructor() {
        this.running_ledgers = {};
        this.block_timers = {};
    }

    async ensureLedgersRunning(ledgers: string[]) {
        let running_ledgers = this.running_ledgers;
        let to_be_started = ledgers.filter(name => !running_ledgers[name]);

        if (to_be_started.length > 0) {
            let wait_times = [0];

            let images_to_start = to_be_started.map(
                name => ledgers_config[name].docker
            );

            await spawn("docker-compose", ["up", ...images_to_start], {
                cwd: docker_cwd,
                stdio: [
                    "ignore",
                    fs.openSync(`${log_dir}/docker-compose.log`, "w"),
                    "inherit",
                ],
            });

            for (let ledger of to_be_started) {
                let ledger_config = ledgers_config[ledger];
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
                this.block_timers["bitcoin"] = setInterval(async () => {
                    await bitcoin.generate();
                }, 3000);
                bitcoin.init(global.ledgers_config.bitcoin);
            }
        }
    }

    stopLedgers() {
        let names = Object.keys(this.running_ledgers);
        if (names.length > 0) {
            console.log("Stopping ledgers: " + names.join(", "));

            Object.values(this.block_timers).forEach(clearInterval);

            execSync("docker-compose rm -sfv", docker_compose_options);
        }
        this.running_ledgers = {};
    }
}

class ComitRunner {
    running_nodes: { [key: string]: ChildProcess };

    constructor() {
        this.running_nodes = {};
    }

    async ensureComitNodesRunning(
        comit_nodes: [string, MetaComitNodeConfig][]
    ) {
        console.log(
            "Starting comit node for " +
                comit_nodes.map(([name, _]) => name).join(", ")
        );
        for (let [name, comit_config] of comit_nodes) {
            if (this.running_nodes[name]) {
                continue;
            }

            this.running_nodes[name] = await spawn(
                project_root + "/target/debug/comit_node",
                [],
                {
                    cwd: test_root,
                    env: { COMIT_NODE_CONFIG_PATH: comit_config.config_dir },
                    stdio: [
                        "ignore",
                        fs.openSync(
                            log_dir + "/comit_node-" + name + ".log",
                            "w"
                        ),
                        fs.openSync(
                            log_dir + "/comit_node-" + name + ".log",
                            "w"
                        ),
                    ],
                }
            );

            this.running_nodes[name].on(
                "exit",
                (code: number, signal: number) => {
                    console.log(
                        `comit-node ${name} exited with ${code ||
                            "signal " + signal}`
                    );
                }
            );
        }

        await sleep(500);
    }

    stopComitNodes() {
        let names = Object.keys(this.running_nodes);

        if (names.length > 0) {
            console.log("Stopping comit nodes: " + names.join(", "));
            for (let process of Object.values(this.running_nodes)) {
                process.kill();
            }
            this.running_nodes = {};
        }
    }
}

class BtsieveRunner {
    running_btsieves: { [key: string]: ChildProcess };

    constructor() {
        this.running_btsieves = {};
    }

    async ensureBtsievesRunning(btsieves: [string, BtsieveConfig][]) {
        for (let [name, btsieve_config] of btsieves) {
            if (this.running_btsieves[name]) {
                continue;
            }

            this.running_btsieves[name] = await spawn(
                project_root + "/target/debug/btsieve",
                [],
                {
                    cwd: test_root,
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

async function run_tests(test_files: string[]) {
    let ledger_runner = new LedgerRunner();
    let node_runner = new ComitRunner();
    let btsieve_runner = new BtsieveRunner();

    let clean_up = () => {};

    process.once("SIGINT", () => {
        console.log("SIGINT RECEIEVED");
        clean_up();
    });

    clean_up = () => {
        console.log("cleaning up");
        btsieve_runner.stopBtsieves();
        node_runner.stopComitNodes();
        ledger_runner.stopLedgers();
        console.log("cleanup done");
        process.exit();
    };

    for (let test_file of test_files) {
        let test_dir = path.dirname(test_file);
        let config = toml.parse(
            fs.readFileSync(test_dir + "/config.toml", "utf8")
        );
        global.config = config;

        const mocha = new Mocha({
            bail: true,
            ui: "bdd",
            delay: true,
        });

        mocha.addFile(test_file);

        if (config.ledgers) {
            await ledger_runner.ensureLedgersRunning(config.ledgers);
        }

        if (config.btsieve) {
            await btsieve_runner.ensureBtsievesRunning(
                Object.entries(config.btsieve)
            );
        }

        if (config.comit_node) {
            await node_runner.ensureComitNodesRunning(
                Object.entries(config.comit_node)
            );
        }

        let test_finish = new Promise((res, rej) => {
            mocha.run(async (failures: number) => {
                res(failures);
            });
        });

        let failures = await test_finish;

        if (failures) {
            process.exitCode = 1;
            if (commander.dumpLogs || process.env.CARGO_MAKE_CI === "TRUE") {
                execSync(`/bin/sh -c 'tail -n +1 ${test_root}/log/*.log'`, {
                    stdio: "inherit",
                });
            }
            break;
        }
        node_runner.stopComitNodes();
    }

    clean_up();
}

let test_files = commander.args;

run_tests(test_files);
