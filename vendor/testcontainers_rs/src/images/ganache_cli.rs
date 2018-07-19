use api::*;
use WaitForMessage;

pub struct GanacheCli {
    tag: String,
    arguments: GanacheCliArgs,
}

#[derive(Clone)]
pub struct GanacheCliArgs {
    pub network_id: u32,
    pub number_of_accounts: u32,
    pub mnemonic: String,
}

impl Default for GanacheCliArgs {
    fn default() -> Self {
        GanacheCliArgs {
            network_id: 42,
            number_of_accounts: 10,
            mnemonic: "".to_string(),
        }
    }
}

impl IntoIterator for GanacheCliArgs {
    type Item = String;
    type IntoIter = ::std::vec::IntoIter<String>;

    fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
        let mut args = Vec::new();

        if !self.mnemonic.is_empty() {
            args.push("-m".to_string());
            args.push(format!("{}", self.mnemonic));
        }

        args.push("-a".to_string());
        args.push(format!("{}", self.number_of_accounts));
        args.push("-i".to_string());
        args.push(format!("{}", self.network_id));

        args.into_iter()
    }
}

impl Image for GanacheCli {
    type Args = GanacheCliArgs;

    fn descriptor(&self) -> String {
        format!("trufflesuite/ganache-cli:{}", self.tag)
    }

    fn exposed_ports(&self) -> ExposedPorts {
        ExposedPorts::new(&[8545])
    }

    fn wait_until_ready<D: Docker>(&self, id: &str, docker: &D) {
        let logs = docker.logs(id);

        logs.wait_for_message("Listening on localhost:");
    }

    fn args(&self) -> <Self as Image>::Args {
        self.arguments.clone()
    }

    fn with_args(self, arguments: <Self as Image>::Args) -> Self {
        GanacheCli { arguments, ..self }
    }

    fn new(tag: &str) -> Self {
        GanacheCli {
            tag: tag.to_string(),
            arguments: GanacheCliArgs::default(),
        }
    }
}
