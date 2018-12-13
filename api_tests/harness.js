const execSync = require("child_process").execSync;
const spawn = require("child_process").spawn;
const Toml = require("toml");
const fs = require("fs");

const project_root = execSync("git rev-parse --show-toplevel", {
  encoding: "utf-8"
}).trim();
console.log("project_root: ", project_root);

const test_dir = process.env.TEST_DIR;
console.log("test_dir: ", test_dir);

if (!test_dir) {
  throw new Error("env var $TEST_DIR env variable must be set");
}

let pids = [];
let docker_container_ids = [];

const docker_cwd = project_root + "/api_tests/regtest";
console.log("docker_cwd: ", docker_cwd);
const docker_compose_options = {
  cwd: docker_cwd,
  encoding: "utf-8"
};

function end() {
  pids.forEach(function(pid) {
    console.log("Killing ", pid);
    process.kill(pid);
  });

  execSync("docker-compose rm -sfv", docker_compose_options);
}

process.once("SIGINT", function(code) {
  console.log("SIGINT received...");
  end();
});

const config = Toml.parse(fs.readFileSync(test_dir + "/config.toml", "utf8"));
console.log("Config: ", config);

let docker_containers = [];
Object.keys(config.ledger).forEach(function(ledger) {
  const docker = config.ledger[ledger].docker;
  if (docker) {
    docker_containers.push(docker);
  }
});
const docker_container_names = docker_containers.join(" ");

console.log("Starting docker container(s): ", docker_container_names);
execSync("docker-compose up -d " + docker_container_names, docker_compose_options);
spawn("docker-compose logs --tail=all", {
  cwd: docker_cwd,
  encoding: "utf-8",
  stdio: ["ignore", fs.openSync("std.out", "w"), fs.openSync("std.err", "w")]
});

end();
