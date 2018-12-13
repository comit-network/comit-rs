const execSync = require("child_process").execSync;
const util = require('util');
const exec = util.promisify(require('child_process').exec);
const spawn = require("child_process").spawn;
const Toml = require("toml");
const fs = require("fs");

const project_root = execSync("git rev-parse --show-toplevel", {
  encoding: "utf-8"
}).trim();
console.log("project_root: ", project_root);
// FIXME: there is probably a cleaner way to pass this to test_lib.js
process.env.PROJECT_ROOT = project_root;

const test_lib = require("./test_lib.js"); // needs env var PROJECT_ROOT

const test_dir = process.env.TEST_DIR;
console.log("test_dir: ", test_dir);

if (!test_dir) {
  throw new Error("env var $TEST_DIR env variable must be set");
}

const log_dir = test_dir + "/log";
console.log("log_dir: ", log_dir);
// FIXME: there is probably a cleaner way to pass this to test.js
process.env.LOG_DIR = log_dir;

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
  return;
}

process.once("SIGINT", function(code) {
  console.log("++ SIGINT received");
  cleanUp();
});

const config = Toml.parse(fs.readFileSync(test_dir + "/config.toml", "utf8"));
console.log("Config: ", config, "\n\n");

let docker_containers = [];
Object.keys(config.ledger).forEach(function(ledger) {
  const docker = config.ledger[ledger].docker;
  if (docker) {
    docker_containers.push(docker);
  }
});
const docker_container_names = docker_containers.join(" ");

describe("Harness", async function() {

  before(async function() {
    this.timeout(50000);

    console.log("++ Starting docker container(s): ", docker_container_names);
    await execSync(
      "docker-compose up -d " + docker_container_names,
      docker_compose_options
    );
    console.log("++ Docker containers started")

    spawn("docker-compose", ["logs", "--tail=all", "-f"], {
      cwd: docker_cwd,
      encoding: "utf-8",
      stdio: [
        "ignore",
        fs.openSync(log_dir + "/docker-compose.log", "w"),
        fs.openSync(log_dir + "/docker-compose-err.log", "w")
      ]
    });

    console.log("Start sleeping");
    await test_lib.sleep(10000);
    console.log("Well rested");
  });

  it("This is my test", async () => {
    return;
  });

  after(async function() {
    return cleanUp();
  });

});
