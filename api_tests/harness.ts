///<reference path="./lib/satoshi_bitcoin.d.ts"/>

import { ChildProcess, execSync, spawn } from "child_process";
import commander from "commander";
import * as fs from "fs";
import Mocha from "mocha";
import path from "path";
import * as toml from "toml";
import { BtsieveRunner } from "./lib/btsieve_runner";
import { MetaComitNodeConfig } from "./lib/comit";
import { LedgerRunner } from "./lib/ledger_runner";
import { HarnessGlobal, sleep } from "./lib/util";

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

    public async ensureComitNodesRunning(
        comit_nodes: Array<[string, MetaComitNodeConfig]>
    ) {
        console.log(
            "Starting comit node for " +
                comit_nodes.map(([name, _]) => name).join(", ")
        );
        for (const [name, comit_config] of comit_nodes) {
            if (this.running_nodes[name]) {
                continue;
            }

            this.running_nodes[name] = await spawn(
                project_root + "/target/debug/comit_node",
                ["--config", comit_config.config_file],
                {
                    cwd: project_root,
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

            await sleep(500);

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

        await sleep(2000);
    }

    public stopComitNodes() {
        const names = Object.keys(this.running_nodes);

        if (names.length > 0) {
            console.log("Stopping comit nodes: " + names.join(", "));
            for (const process of Object.values(this.running_nodes)) {
                process.kill();
            }
            this.running_nodes = {};
        }
    }
}

async function run_tests(test_files: string[]) {
    const ledger_runner = new LedgerRunner(
        project_root + "/api_tests/regtest/docker-compose.yml",
        project_root + "/api_tests/regtest/ledgers.toml",
        log_dir
    );
    global.ledgers_config = ledger_runner.getLedgersConfig();

    const node_runner = new ComitRunner();
    const btsieve_runner = new BtsieveRunner(
        project_root,
        project_root + "/target/debug/btsieve",
        log_dir
    );

    process.on("SIGINT", () => {
        console.log("SIGINT RECEIEVED");
        process.exit(0);
    });

    process.on("unhandledRejection", reason => {
        console.error(reason);
        process.exit(1);
    });

    process.on("exit", () => {
        console.log("cleaning up");
        btsieve_runner.stopBtsieves();
        node_runner.stopComitNodes();
        ledger_runner.stopLedgers();
        console.log("cleanup done");
    });

    for (const test_file of test_files) {
        const test_dir = path.dirname(test_file);
        const config = toml.parse(
            fs.readFileSync(test_dir + "/config.toml", "utf8")
        );
        global.config = config;

        if (config.ledgers) {
            await ledger_runner.ensureLedgersRunning(config.ledgers);
        }

        if (config.btsieve) {
            btsieve_runner.ensureBtsievesRunning(
                Object.entries(config.btsieve)
            );
        }

        if (config.comit_node) {
            await node_runner.ensureComitNodesRunning(
                Object.entries(config.comit_node)
            );
        }

        const runTests = new Promise(res => {
            new Mocha({ bail: true, ui: "bdd", delay: true })
                .addFile(test_file)
                .run((failures: number) => res(failures));
        });

        const failures = await runTests;

        if (failures) {
            if (commander.dumpLogs || process.env.CARGO_MAKE_CI === "TRUE") {
                execSync(`/bin/sh -c 'tail -n +1 ${test_root}/log/*.log'`, {
                    stdio: "inherit",
                });
            }
            process.exit(1);
        }

        node_runner.stopComitNodes();
        btsieve_runner.stopBtsieves();
    }

    process.exit(0);
}

function validTestFile(path: string): boolean {
    return (
        validTestPath(path) &&
        /^.*.ts$/.test(path) &&
        !/^.*harness.ts$/.test(path)
    );
}

function validTestPath(path: string): boolean {
    return (
        !/^.*lib\/.*$/.test(path) &&
        !/^.*node_modules\/.*$/.test(path) &&
        !/^.*gen\/.*$/.test(path) &&
        !/^.*log\/.*$/.test(path) &&
        !/^.*regtest\/.*$/.test(path)
    );
}

function expandPath(
    paths: string[],
    parentDir: string = "",
    depth: number = 5
): string[] {
    if (!depth) {
        return [];
    }
    if (!paths.length) {
        return expandPath(["./"]);
    }

    let result: string[] = [];
    for (let path of paths) {
        if (validTestPath(path)) {
            path = parentDir + path;
            const stats = fs.lstatSync(path);
            if (stats.isFile()) {
                if (validTestFile(path)) {
                    result.push(path);
                }
            } else if (stats.isDirectory()) {
                const subPaths = fs.readdirSync(path);
                const files = expandPath(subPaths, path + "/", depth - 1);
                const concat = result.concat(files);
                result = concat;
            }
        }
    }
    return result;
}

const args = commander.args;
const testFiles = expandPath(args);
console.log(testFiles);
run_tests(testFiles);
