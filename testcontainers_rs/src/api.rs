use std::collections::HashMap;
use std::io::Read;
use std::str::FromStr;

pub trait Docker {
    fn run_detached<I: Image>(&self, image: &I, run_args: RunArgs) -> String;
    fn logs(&self, id: &str) -> Box<Read>;
    fn inspect(&self, id: &str) -> ContainerInfo;
    fn rm(&self, id: &str);
}

pub trait Image
where
    Self: Sized,
    Self::Args: IntoIterator<Item = String>,
{
    type Args;

    fn descriptor(&self) -> String;
    fn exposed_ports(&self) -> ExposedPorts;
    fn wait_until_ready<D: Docker>(&self, id: &str, docker: &D);
    fn args(&self) -> Self::Args;
    fn with_args(self, arguments: Self::Args) -> Self;
    fn new(tag: &str) -> Self;
    fn latest() -> Self {
        Self::new("latest")
    }
}

pub trait Container {
    fn id(&self) -> &str;
    fn ports(&self) -> &Ports;
}

#[derive(Default)]
pub struct RunArgs {
    pub ports: ExposedPorts,
    pub rm: bool,
    pub interactive: bool,
}

#[derive(Default, Clone)]
pub struct ExposedPorts(Vec<u32>);

impl IntoIterator for ExposedPorts {
    type Item = u32;
    type IntoIter = ::std::vec::IntoIter<u32>;

    fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
        self.0.into_iter()
    }
}

impl ExposedPorts {
    pub fn new(ports: &[u32]) -> Self {
        ExposedPorts(ports.to_vec())
    }
}

#[derive(Deserialize, Debug)]
pub struct ContainerInfo {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "NetworkSettings")]
    network_settings: NetworkSettings,
}

#[derive(Deserialize, Debug)]
pub struct NetworkSettings {
    #[serde(rename = "Ports")]
    ports: Ports,
}

#[derive(Deserialize, Debug)]
pub struct Ports(HashMap<String, Option<Vec<PortMapping>>>);

#[derive(Deserialize, Debug)]
pub struct PortMapping {
    #[serde(rename = "HostIp")]
    ip: String,
    #[serde(rename = "HostPort")]
    port: String,
}

impl Container for ContainerInfo {
    fn id(&self) -> &str {
        &self.id
    }

    fn ports(&self) -> &Ports {
        &self.network_settings.ports
    }
}

impl Ports {
    pub fn map_to_external_port(&self, internal_port: u32) -> Option<u32> {
        for key in self.0.keys() {
            let internal_port = format!("{}", internal_port);
            if key.contains(&internal_port) {
                return self.0.get(key).and_then(|option| {
                    option
                        .as_ref()
                        .and_then(|mappings| mappings.get(0))
                        .map(|mapping| &mapping.port)
                        .map(|port| u32::from_str(port).unwrap())
                });
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    extern crate serde_json;

    #[test]
    fn can_deserialize_docker_inspect_response() {
        let response = r#"{
        "Id": "fd2e896b883052dae31202b065a06dc5374a214ae348b7a8f8da3734f690d010",
        "NetworkSettings": {
            "Ports": {
                "18332/tcp": [
                    {
                        "HostIp": "0.0.0.0",
                        "HostPort": "33076"
                    }
                ],
                "18333/tcp": [
                    {
                        "HostIp": "0.0.0.0",
                        "HostPort": "33075"
                    }
                ],
                "18443/tcp": null,
                "18444/tcp": null,
                "8332/tcp": [
                    {
                        "HostIp": "0.0.0.0",
                        "HostPort": "33078"
                    }
                ],
                "8333/tcp": [
                    {
                        "HostIp": "0.0.0.0",
                        "HostPort": "33077"
                    }
                ]
            }
        }
    }"#;

        let info = serde_json::from_str::<ContainerInfo>(response).unwrap();

        let ports = info.network_settings.ports;

        let external_port = ports.map_to_external_port(18332);

        assert_eq!(
            info.id,
            "fd2e896b883052dae31202b065a06dc5374a214ae348b7a8f8da3734f690d010"
        );
        assert_eq!(external_port, Some(33076));
    }
}
