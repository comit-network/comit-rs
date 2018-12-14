const execSync = require("child_process").execSync;
const util = require("util");
const exec = util.promisify(require("child_process").exec);
const spawn = require("child_process").spawn;
const Toml = require("toml");
const fs = require("fs");

// ************************ //
// Setting global variables //
// ************************ //

// Used to pass down data to test.js
global.harness = {};

const project_root = execSync("git rev-parse --show-toplevel", {
    encoding: "utf-8",
}).trim();
global.harness.project_root = project_root;

const test_dir = process.env.TEST_DIR;
if (!test_dir) {
    throw new Error("env var $TEST_DIR env variable must be set");
}

const log_dir = test_dir + "/log";
global.harness.log_dir = log_dir;

const docker_cwd = project_root + "/api_tests/regtest";
const services_cwd = project_root + "/api_tests/";
process.chdir(services_cwd);

const docker_compose_options = {
    cwd: docker_cwd,
    encoding: "utf-8",
};

const config = Toml.parse(fs.readFileSync(test_dir + "/config.toml", "utf8"));
console.log("++ Config:\n", config, "\n++ --------------------");
global.harness.config = config;

let docker_container_names = "";
let ledger_up_time = 0;
let ledger_down_time = 0;
{
    if (config.ledger) {
        let docker_containers = [];
        Object.keys(config.ledger).forEach(function(ledger) {
            const config_ledger = config.ledger[ledger];

            if (config_ledger.docker) {
                docker_containers.push(config_ledger.docker);
            }

            const up_time = config_ledger.up_time;
            if (up_time && up_time > ledger_up_time) {
                ledger_up_time = up_time;
            }

            const down_time = parseInt(config_ledger.down_time);
            if (down_time && down_time > ledger_down_time) {
                ledger_down_time = down_time;
            }
        });
        docker_container_names = docker_containers.join(" ");

        console.log(
            "++ Extracted values:\n  ++ docker containers:",
            docker_container_names,
            "\n  ++ ledger_up_time:",
            ledger_up_time,
            "\n  ++ ledger_down_time:",
            ledger_down_time
        );
    }
}

// To be done once all global variables are set
const test_lib = require("./test_lib.js");

// *********************************************** //
// Clean-up docker containers and processes helper //
// *********************************************** //

let subprocesses = [];
function cleanUp() {
    subprocesses.forEach(function(subprocess) {
        console.log("++ Killing", subprocess.spawnfile, subprocess.pid);
        subprocess.kill();
    });
    console.log("++ Stopping docker containers");
    execSync("docker-compose rm -sfv", docker_compose_options);
}

process.once("SIGINT", function(code) {
    console.log("++ SIGINT received");
    cleanUp();
});

// ********************** //
// Start services helpers //
// ********************** //

async function startDockerContainers(names) {
    await execSync("docker-compose up -d " + names, docker_compose_options);

    const subprocess = await spawn(
        "docker-compose",
        ["logs", "--tail=all", "-f"],
        {
            cwd: docker_cwd,
            encoding: "utf-8",
            stdio: [
                "ignore",
                fs.openSync(log_dir + "/docker-compose.log", "w"),
                fs.openSync(log_dir + "/docker-compose-err.log", "w"),
            ],
        }
    );
    subprocesses.push(subprocess);
}

async function startComitNode(name, comit_config) {
    console.log("Starting", name + "'s COMIT node:", comit_config);

    const subprocess = await spawn(
        project_root + "/target/debug/comit_node",
        [],
        {
            cwd: services_cwd,
            encoding: "utf-8",
            env: { COMIT_NODE_CONFIG_PATH: comit_config.config_dir },
            stdio: [
                "ignore",
                fs.openSync(log_dir + "/comit_node-" + name + ".log", "w"),
                fs.openSync(log_dir + "/comit_node-" + name + ".log", "w"),
            ],
        }
    );
    subprocesses.push(subprocess);
}

async function startLedgerQueryService(name, lqs_config) {
    console.log("Starting", name, "Ledger Query Service:", lqs_config);

    const subprocess = await spawn(
        project_root + "/target/debug/ledger_query_service",
        [],
        {
            cwd: services_cwd,
            encoding: "utf-8",
            env: lqs_config.env,
            stdio: [
                "ignore",
                fs.openSync(
                    log_dir + "/ledger_query_service-" + name + ".log",
                    "w"
                ),
                fs.openSync(
                    log_dir + "/ledger_query_service-" + name + ".log",
                    "w"
                ),
            ],
        }
    );
    subprocesses.push(subprocess);
}

// ********************************** //
// Start services, run test, shutdown //
// ********************************** //

describe("Starting services", async function() {
    before(async function() {
        this.timeout(ledger_up_time + 4000);

        if (config.ledger) {
            console.log(
                "++ Starting docker container(s):",
                docker_container_names
            );
            await startDockerContainers(docker_container_names);
            console.log("++ Docker containers started");
            await test_lib.sleep(ledger_up_time);
        }

        if (config.ledger_query_service) {
            console.log("++ Starting Ledger Query Service node(s)");
            Object.keys(config.ledger_query_service).forEach(async function(
                name
            ) {
                await startLedgerQueryService(
                    name,
                    config.ledger_query_service[name]
                );
            });
        }

        if (config.comit_node) {
            console.log("++ Starting COMIT node(s)");
            Object.keys(config.comit_node).forEach(async function(name) {
                await startComitNode(name, config.comit_node[name]);
            });
        }

        await test_lib.sleep(2000);
    });

    describe("Running Test Suite", async function() {
        require(test_dir + "/test.js");
    });

    after(async function() {
        this.timeout(ledger_down_time);
        await cleanUp();
    });
});
