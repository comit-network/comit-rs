pub mod swap;

#[cfg(all(test, test_docker))]
pub mod ledgers {
    use reqwest::Url;
    use testcontainers::{
        clients,
        images::coblox_bitcoincore::BitcoinCore,
        images::generic::{GenericImage, Stream, WaitFor},
        Container, Docker, Image,
    };

    pub struct BitcoinBlockchain<'c> {
        _container: Container<'c, clients::Cli, BitcoinCore>,
        pub node_url: Url,
    }

    impl<'c> BitcoinBlockchain<'c> {
        pub fn new(client: &'c clients::Cli) -> anyhow::Result<Self> {
            let container = client.run(BitcoinCore::default().with_tag("0.19.1"));
            let port = container.get_host_port(18443);

            let auth = container.image().auth();
            let url = format!(
                "http://{}:{}@localhost:{}",
                &auth.username,
                &auth.password,
                port.unwrap()
            );
            let url = Url::parse(&url)?;

            Ok(Self {
                _container: container,
                node_url: url,
            })
        }
    }

    pub struct EthereumBlockchain<'c> {
        _container: Container<'c, clients::Cli, GenericImage>,
        pub node_url: Url,
    }

    impl<'c> EthereumBlockchain<'c> {
        pub fn new(client: &'c clients::Cli) -> anyhow::Result<Self> {
            let geth_image = GenericImage::new("ethereum/client-go")
                .with_wait_for(WaitFor::LogMessage {
                    message: String::from("mined potential block"),
                    stream: Stream::StdErr,
                })
                .with_args(vec![
                    String::from("--dev"), // TODO: Most definitely missing arguments, see comit-rs geth_instance.ts
                    String::from("--dev.period=1"),
                    String::from("--rpc"),
                ]);

            let container = client.run(geth_image);
            let port = container.get_host_port(8545);

            let url = format!("http://localhost:{}", port.unwrap());
            let url = Url::parse(&url)?;

            Ok(Self {
                _container: container,
                node_url: url,
            })
        }
    }
}
