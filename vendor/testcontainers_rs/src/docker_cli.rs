use api::*;
use serde_json;
use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};

pub struct DockerCli;

impl Docker for DockerCli {
    fn run_detached<I: Image>(&self, image: &I, run_args: RunArgs) -> String {
        let mut command = Command::new("docker");

        let command_builder = command.arg("run").arg("-d");

        if run_args.rm {
            command_builder.arg("--rm");
        }

        if run_args.interactive {
            command_builder.arg("-i");
        }

        let ports = run_args.ports;

        for port in ports {
            command_builder.arg("-p").arg(format!("{}", port));
        }

        let command = command_builder.arg(&image.descriptor()).args(image.args());

        info!("Executing command: {:?}", command);

        let child = command
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to execute docker command");

        let stdout = child.stdout.unwrap();
        let reader = BufReader::new(stdout);

        let container_id = reader.lines().next().unwrap().unwrap();

        debug!("Waiting for docker container {} to be ready.", container_id);

        image.wait_until_ready(&container_id, self);

        debug!("Docker container {} is now ready!", container_id);

        container_id
    }

    fn logs(&self, id: &str) -> Box<Read> {
        let child = Command::new("docker")
            .arg("logs")
            .arg("-f")
            .arg(id)
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to execute docker command");

        Box::new(child.stdout.unwrap())
    }

    fn inspect(&self, id: &str) -> ContainerInfo {
        let child = Command::new("docker")
            .arg("inspect")
            .arg(id)
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to execute docker command");

        let stdout = child.stdout.unwrap();

        let mut infos: Vec<ContainerInfo> = serde_json::from_reader(stdout).unwrap();

        let info = infos.remove(0);

        debug!("Fetched container info: {:#?}", info);

        info
    }

    fn rm(&self, id: &str) {
        info!("Killing docker container: {}", id);

        Command::new("docker")
            .arg("rm")
            .arg("-f")
            .arg(id)
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to execute docker command");
    }

    fn stop(&self, id: &str) {
        info!("Killing docker container: {}", id);

        Command::new("docker")
            .arg("stop")
            .arg(id)
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to execute docker command");
    }
}
