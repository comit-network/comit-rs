#!/usr/bin/env ./api_tests/node_modules/.bin/ts-node --project api_tests/tsconfig.json

import { ChildProcess, execSync } from "child_process";
import { spawn } from "child_process";
import { HarnessGlobal, sleep } from "./lib/util";
import { MetaComitNodeConfig } from "./lib/comit";
import * as toml from "toml";
import * as fs from "fs";
import { LedgerRunner } from "./lib/ledger_runner";
import { BtsieveRunner } from "./lib/btsieve_runner";

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

const test_root = project_root + "/api_tests";
global.test_root = test_root;

const log_dir = project_root + "/api_tests/log";

if (!fs.existsSync(log_dir)) {
    fs.mkdirSync(log_dir);
}

// ********************** //
// Start services helpers //
// ********************** //

class ComitRunner {
    private running_nodes: { [key: string]: ChildProcess };

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
                    cwd: project_root,
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

async function run_tests(test_files: string[]) {
    let ledger_runner = new LedgerRunner(
        project_root,
        project_root + "/api_tests/regtest/ledgers.toml",
        log_dir
    );
    global.ledgers_config = ledger_runner.getLedgersConfig();

    let node_runner = new ComitRunner();
    let btsieve_runner = new BtsieveRunner(
        project_root,
        project_root + "/target/debug/btsieve",
        log_dir
    );

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
        await btsieve_runner.stopBtsieves();
    }

    clean_up();
}

let test_files = commander.args;

run_tests(test_files);
