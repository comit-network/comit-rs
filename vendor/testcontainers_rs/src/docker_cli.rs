use api::*;
use serde_json;
use std::{
    io::{BufRead, BufReader, Read},
    process::{Command, Stdio},
};

#[derive(Copy, Clone)]
pub struct DockerCli;

impl Docker for DockerCli {
    fn new() -> Self {
        DockerCli
    }

    fn run<I: Image>(&self, image: &I) -> Container<DockerCli> {
        let mut docker = Command::new("docker");

        let command = docker
            .arg("run")
            .arg("-d") // Always run detached
            .arg("-P") // Always expose all ports
            .arg(&image.descriptor())
            .args(image.args())
            .stdout(Stdio::piped());

        info!("Executing command: {:?}", command);

        let child = command.spawn().expect("Failed to execute docker command");

        let stdout = child.stdout.unwrap();
        let reader = BufReader::new(stdout);

        let container_id = reader.lines().next().unwrap().unwrap();

        let container = Container::new(container_id, DockerCli {});

        debug!("Waiting for {} to be ready.", container);

        image.wait_until_ready(&container);

        debug!("{} is now ready!", container);

        container
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
            .arg("-v") // Also remove volumes
            .arg(id)
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to execute docker command");
    }

    fn stop(&self, id: &str) {
        info!("Stopping docker container: {}", id);

        Command::new("docker")
            .arg("stop")
            .arg(id)
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to execute docker command");
    }
}
