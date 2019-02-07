const bitcoin = require("./lib/bitcoin.js");
const execSync = require("child_process").execSync;
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

const log4js = require("log4js");
log4js.configure({
    appenders: {
        test_suite: {
            type: "file",
            filename: log_dir + "/test-suite.log",
        },
    },
    categories: { default: { appenders: ["test_suite"], level: "ALL" } },
});
const logger = log4js.getLogger("test_suite");
global.harness.logger = logger;

const docker_cwd = project_root + "/api_tests/regtest";
const services_cwd = project_root + "/api_tests/";
process.chdir(services_cwd);

const docker_compose_options = {
    cwd: docker_cwd,
    encoding: "utf-8",
};

const config = Toml.parse(fs.readFileSync(test_dir + "/config.toml", "utf8"));
logger.debug("++ Config:\n", config, "\n++ --------------------");
global.harness.config = config;

const ledgers_config = Toml.parse(
    fs.readFileSync(project_root + "/api_tests/regtest/ledgers.toml", "utf8")
);
logger.debug(
    "++ Ledgers Config:\n",
    ledgers_config,
    "\n++ --------------------"
);
global.harness.ledgers_config = ledgers_config;

let ledger_up_time = 0;
let ledger_down_time = 0;
{
    if (config.ledgers) {
        config.ledgers.forEach(function(ledger) {
            const config_ledger = ledgers_config[ledger];

            const up_time = config_ledger.up_time;
            if (up_time && up_time > ledger_up_time) {
                ledger_up_time = up_time;
            }

            const down_time = parseInt(config_ledger.down_time);
            if (down_time && down_time > ledger_down_time) {
                ledger_down_time = down_time;
            }
        });
    }
}

// To be done once all global variables are set
const util = require("./lib/util.js");

// *********************************************** //
// Clean-up docker containers and processes helper //
// *********************************************** //

function cleanUp(subprocesses) {
    subprocesses.forEach(function(subprocess) {
        logger.info("++ Killing", subprocess.spawnfile, subprocess.pid);
        subprocess.kill();
    });
    logger.info("++ Stopping docker containers");
    execSync("docker-compose rm -sfv", docker_compose_options);
}

process.once("SIGINT", function() {
    logger.debug("++ SIGINT received");
    cleanUp();
});

// ********************** //
// Start services helpers //
// ********************** //

async function startDockerContainers(ledgers, ledgers_config) {
    if (ledgers.length === 0) {
        throw new Error("No ledgers to start");
    }

    let names = [];
    ledgers.forEach(function(ledger) {
        names.push(ledgers_config[ledger].docker);
    });
    await execSync(
        "docker-compose up -d " + names.join(" "),
        docker_compose_options
    );

    return await spawn("docker-compose", ["logs", "--tail=all", "-f"], {
        cwd: docker_cwd,
        encoding: "utf-8",
        stdio: [
            "ignore",
            fs.openSync(log_dir + "/docker-compose.log", "w"),
            fs.openSync(log_dir + "/docker-compose-err.log", "w"),
        ],
    });
}

async function generateBlock(ledgers) {
    if (ledgers && ledgers.includes("bitcoin")) {
        await bitcoin.btc_generate();
    }
}

async function startComitNode(name, comit_config) {
    logger.info("Starting", name + "'s COMIT node:", comit_config);

    return await spawn(project_root + "/target/debug/comit_node", [], {
        cwd: services_cwd,
        encoding: "utf-8",
        env: { COMIT_NODE_CONFIG_PATH: comit_config.config_dir },
        stdio: [
            "ignore",
            fs.openSync(log_dir + "/comit_node-" + name + ".log", "w"),
            fs.openSync(log_dir + "/comit_node-" + name + ".log", "w"),
        ],
    });
}

async function startLedgerQueryService(name, lqs_config) {
    logger.info("Starting", name, "Ledger Query Service:", lqs_config);

    return await spawn(
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
}

// ********************************** //
// Start services, run test, shutdown //
// ********************************** //

function run_tests(file) {
    describe("🏃" + file, async function() {
        let block_interval;
        let subprocesses = [];
        before(async function() {
            this.timeout(ledger_up_time + 4000);

            if (config.ledgers) {
                logger.info("++ Starting docker container(s)");
                const subprocess = await startDockerContainers(
                    config.ledgers,
                    ledgers_config
                );
                subprocesses.push(subprocess);
                logger.info("++ Docker containers started");
                await util.sleep(ledger_up_time);
            }

            if (config.ledger_query_service) {
                logger.info("++ Starting Ledger Query Service node(s)");
                Object.keys(config.ledger_query_service).forEach(async function(
                    name
                ) {
                    const subprocess = await startLedgerQueryService(
                        name,
                        config.ledger_query_service[name]
                    );
                    subprocesses.push(subprocess);
                });
            }

            if (config.comit_node) {
                logger.info("++ Starting COMIT node(s)");
                Object.keys(config.comit_node).forEach(async function(name) {
                    const subprocess = await startComitNode(
                        name,
                        config.comit_node[name]
                    );
                    subprocesses.push(subprocess);
                });
            }

            block_interval = setInterval(() => {
                generateBlock(config.ledgers, ledgers_config);
            }, 3000);

            await util.sleep(2000);
        });

        require(file);

        after(async function() {
            clearInterval(block_interval);
            this.timeout(ledger_down_time);
            await cleanUp(subprocesses);
        });
    });
}

let items = fs.readdirSync(test_dir);

for (let item of items) {
    if (item.endsWith(".js")) {
        run_tests(test_dir + "/" + item);
    }
}
