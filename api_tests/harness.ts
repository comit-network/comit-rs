///<reference path="./lib/satoshi_bitcoin.d.ts"/>

import { execSync } from "child_process";
import commander from "commander";
import * as fs from "fs";
import glob from "glob";
import Mocha from "mocha";
import path from "path";
import * as toml from "toml";
import { BtsieveRunner } from "./lib/btsieve_runner";
import { CndRunner } from "./lib/cnd_runner";
import { LedgerRunner } from "./lib/ledger_runner";
import { HarnessGlobal } from "./lib/util";

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

async function runTests(testFiles: string[]) {
    const ledgerRunner = new LedgerRunner(
        projectRoot + "/api_tests/regtest/docker-compose.yml",
        projectRoot + "/api_tests/regtest/ledgers.toml",
        logDir
    );
    global.ledgers_config = ledgerRunner.getLedgersConfig();

    let cndPath = projectRoot + "/target/debug/cnd";
    let btsievePath = projectRoot + "/target/debug/btsieve";

    if (!fs.existsSync(cndPath)) {
        cndPath = projectRoot + "/target/release/cnd";
        btsievePath = projectRoot + "/target/release/btsieve";
    }

    const nodeRunner = new CndRunner(projectRoot, cndPath, logDir);
    const btsieveRunner = new BtsieveRunner(projectRoot, btsievePath, logDir);

    process.on("SIGINT", () => {
        console.log("SIGINT RECEIVED");
        process.exit(0);
    });

    process.on("unhandledRejection", reason => {
        console.error(reason);
        process.exit(1);
    });

    process.on("exit", () => {
        console.log("cleaning up");
        btsieveRunner.stopBtsieves();
        nodeRunner.stopCnds();
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

        if (config.cnd) {
            await nodeRunner.ensureCndsRunning(Object.entries(config.cnd));
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

        nodeRunner.stopCnds();
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
