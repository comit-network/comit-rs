use api::{Docker, ExposedPorts, Image};
use std::{env::var, thread::sleep, time::Duration};
use WaitForMessage;

pub struct Bitcoind {
    tag: String,
    arguments: BitcoindImageArgs,
}

#[derive(Clone)]
pub enum Network {
    Mainnet,
    Testnet,
    Regtest,
}

#[derive(Clone)]
pub struct BitcoindImageArgs {
    pub server: bool,
    pub network: Network,
    pub print_to_console: bool,
    pub rpc_auth: String,
    pub tx_index: bool,
    pub rpc_bind: String,
    pub rpc_allowip: String,
}

impl Default for BitcoindImageArgs {
    fn default() -> Self {
        BitcoindImageArgs {
            server: true,
            network: Network::Regtest,
            print_to_console: true,
            rpc_auth: String::new(),
            tx_index: true,
            rpc_bind: "0.0.0.0:18443".to_string(),
            rpc_allowip: "0.0.0.0/0".to_string(),
        }
    }
}

impl IntoIterator for BitcoindImageArgs {
    type Item = String;
    type IntoIter = ::std::vec::IntoIter<String>;

    fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
        let mut args = Vec::new();

        if self.server {
            args.push("-server".to_string())
        }

        match self.network {
            Network::Testnet => args.push("-testnet".to_string()),
            Network::Regtest => args.push("-regtest".to_string()),
            Network::Mainnet => {}
        }

        if self.tx_index {
            args.push("-txindex=1".to_string())
        }

        if !self.rpc_auth.is_empty() {
            args.push(format!("-rpcauth={}", self.rpc_auth));
        }

        if !self.rpc_allowip.is_empty() {
            args.push(format!("-rpcallowip={}", self.rpc_allowip));
        }

        if !self.rpc_bind.is_empty() {
            args.push(format!("-rpcbind={}", self.rpc_bind));
        }

        if self.print_to_console {
            args.push("-printtoconsole".to_string())
        }

        args.push("-debug".into()); // Needed for message "Flushed wallet.dat"

        args.to_vec().into_iter()
    }
}

impl Image for Bitcoind {
    type Args = BitcoindImageArgs;

    fn descriptor(&self) -> String {
        format!("ruimarinho/bitcoin-core:{}", self.tag)
    }

    fn exposed_ports(&self) -> ExposedPorts {
        ExposedPorts::new(&[18443])
    }

    fn wait_until_ready<D: Docker>(&self, id: &str, docker: &D) {
        let logs = docker.logs(id);

        logs.wait_for_message("Flushed wallet.dat").unwrap();

        let additional_sleep_period =
            var("BITCOIND_ADDITIONAL_SLEEP_PERIOD").map(|value| value.parse());

        if let Ok(Ok(sleep_period)) = additional_sleep_period {
            trace!(
                "Waiting for an additional {} ms for container {}",
                sleep_period,
                id
            );

            sleep(Duration::from_millis(sleep_period))
        }
    }

    fn args(&self) -> <Self as Image>::Args {
        self.arguments.clone()
    }

    fn with_args(self, arguments: <Self as Image>::Args) -> Self {
        Bitcoind { arguments, ..self }
    }

    fn new(tag: &str) -> Self {
        Bitcoind {
            tag: tag.to_string(),
            arguments: BitcoindImageArgs::default(),
        }
    }
}
