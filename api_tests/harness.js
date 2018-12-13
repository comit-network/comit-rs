const execSync = require("child_process").execSync;
const util = require("util");
const exec = util.promisify(require("child_process").exec);
const spawn = require("child_process").spawn;
const Toml = require("toml");
const fs = require("fs");

const project_root = execSync("git rev-parse --show-toplevel", {
  encoding: "utf-8"
}).trim();
// FIXME: there is probably a cleaner way to pass this to test_lib.js
process.env.PROJECT_ROOT = project_root;

const test_dir = process.env.TEST_DIR;
if (!test_dir) {
  throw new Error("env var $TEST_DIR env variable must be set");
}

const log_dir = test_dir + "/log";
// FIXME: there is probably a cleaner way to pass this to test.js
process.env.LOG_DIR = log_dir;

const test_lib = require("./test_lib.js"); // needs env var PROJECT_ROOT and LOG_DIR

let pids = [];

const docker_cwd = project_root + "/api_tests/regtest";

const docker_compose_options = {
  cwd: docker_cwd,
  encoding: "utf-8"
};

function cleanUp() {
  pids.forEach(function(pid) {
    console.log("++ Killing ", pid);
    process.kill(pid);
  });
  console.log("++ Stopping docker containers");
  execSync("docker-compose rm -sfv", docker_compose_options);
}

process.once("SIGINT", function(code) {
  console.log("++ SIGINT received");
  cleanUp();
});

const config = Toml.parse(fs.readFileSync(test_dir + "/config.toml", "utf8"));
console.log("++ Config: \n", config, "\n++ --------------------");

let docker_container_names = "";
let ledger_up_time = 0;
let ledger_down_time = 0;
{
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
    "++ Extracted values:\n  ++ docker containers: ",
    docker_container_names,
    "\n  ++ ledger_up_time: ",
    ledger_up_time,
    "\n  ++ ledger_down_time: ",
    ledger_down_time
  );
}

describe("Starting services", async function() {
  before(async function() {
    this.timeout(50000);

    console.log("++ Starting docker container(s): ", docker_container_names);
    await execSync(
      "docker-compose up -d " + docker_container_names,
      docker_compose_options
    );
    console.log("++ Docker containers started");

    spawn("docker-compose", ["logs", "--tail=all", "-f"], {
      cwd: docker_cwd,
      encoding: "utf-8",
      stdio: [
        "ignore",
        fs.openSync(log_dir + "/docker-compose.log", "w"),
        fs.openSync(log_dir + "/docker-compose-err.log", "w")
      ]
    });

    await test_lib.sleep(ledger_up_time);
  });

  it("This is my test", async () => {
    return;
  });

  after(function() {
    this.timeout(ledger_down_time);
    cleanUp();
  });
});
