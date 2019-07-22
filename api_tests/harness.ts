///<reference path="./lib/satoshi_bitcoin.d.ts"/>

import { ChildProcess, execSync, spawn } from "child_process";
import commander from "commander";
import * as fs from "fs";
import glob from "glob";
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

const projectRoot: string = execSync("git rev-parse --show-toplevel", {
    encoding: "utf8",
}).trim();
global.project_root = projectRoot;

const testRoot = projectRoot + "/api_tests";
global.test_root = testRoot;

const logDir = projectRoot + "/api_tests/log";

if (!fs.existsSync(logDir)) {
    fs.mkdirSync(logDir);
}

// ********************** //
// Start services helpers //
// ********************** //

class ComitRunner {
    private runningNodes: { [key: string]: ChildProcess };

    constructor() {
        this.runningNodes = {};
    }

    public async ensureComitNodesRunning(
        comitNodes: Array<[string, MetaComitNodeConfig]>
    ) {
        console.log(
            "Starting comit node for " +
                comitNodes.map(([name]) => name).join(", ")
        );
        for (const [name, comitConfig] of comitNodes) {
            if (this.runningNodes[name]) {
                continue;
            }

            this.runningNodes[name] = await spawn(
                projectRoot + "/target/debug/comit_node",
                ["--config", comitConfig.config_file],
                {
                    cwd: projectRoot,
                    stdio: [
                        "ignore",
                        fs.openSync(
                            logDir + "/comit_node-" + name + ".log",
                            "w"
                        ),
                        fs.openSync(
                            logDir + "/comit_node-" + name + ".log",
                            "w"
                        ),
                    ],
                }
            );

            await sleep(500);

            this.runningNodes[name].on(
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
        const names = Object.keys(this.runningNodes);

        if (names.length > 0) {
            console.log("Stopping comit nodes: " + names.join(", "));
            for (const process of Object.values(this.runningNodes)) {
                process.kill();
            }
            this.runningNodes = {};
        }
    }
}

async function runTests(testFiles: string[]) {
    const ledgerRunner = new LedgerRunner(
        projectRoot + "/api_tests/regtest/docker-compose.yml",
        projectRoot + "/api_tests/regtest/ledgers.toml",
        logDir
    );
    global.ledgers_config = ledgerRunner.getLedgersConfig();

    const nodeRunner = new ComitRunner();
    const btsieveRunner = new BtsieveRunner(
        projectRoot,
        projectRoot + "/target/debug/btsieve",
        logDir
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
        btsieveRunner.stopBtsieves();
        nodeRunner.stopComitNodes();
        ledgerRunner.stopLedgers();
        console.log("cleanup done");
    });

    for (const testFile of testFiles) {
        const testDir = path.dirname(testFile);
        const config = toml.parse(
            fs.readFileSync(testDir + "/config.toml", "utf8")
        );
        global.config = config;

        if (config.ledgers) {
            await ledgerRunner.ensureLedgersRunning(config.ledgers);
        }

        if (config.btsieve) {
            btsieveRunner.ensureBtsievesRunning(Object.entries(config.btsieve));
        }

        if (config.comit_node) {
            await nodeRunner.ensureComitNodesRunning(
                Object.entries(config.comit_node)
            );
        }

        const runTests = new Promise(res => {
            new Mocha({ bail: true, ui: "bdd", delay: true })
                .addFile(testFile)
                .run((failures: number) => res(failures));
        });

        const failures = await runTests;

        if (failures) {
            if (commander.dumpLogs || process.env.CARGO_MAKE_CI === "TRUE") {
                execSync(`/bin/sh -c 'tail -n +1 ${testRoot}/log/*.log'`, {
                    stdio: "inherit",
                });
            }
            process.exit(1);
        }

        nodeRunner.stopComitNodes();
        btsieveRunner.stopBtsieves();
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

function expandGlob(paths: string[]): string[] {
    if (!paths.length) {
        return expandGlob(["**/*.ts"]);
    }

    let result: string[] = [];
    for (const path of paths) {
        if (glob.hasMagic(path)) {
            const expandedPaths: string[] = glob.sync(path);
            for (const expandedPath of expandedPaths) {
                if (validTestFile(expandedPath)) {
                    result.push(expandedPath);
                }
            }
        } else if (fs.lstatSync(path).isDirectory()) {
            result = result.concat(expandGlob([path + "/**/*.ts"]));
        } else if (validTestFile(path)) {
            result.push(path);
        }
    }

    return result;
}

const args = commander.args;
const testFiles = expandGlob(args);
runTests(testFiles);
