mod trace;

use anyhow::Context;
use nectar::{
    bitcoin,
    command::{balance, deposit, dump_config, trade, wallet_info, withdraw, Command, Options},
    config::{self, Settings},
    ethereum,
};

#[tokio::main]
async fn main() {
    let options = Options::from_args();

    let settings = read_config(&options)
        .and_then(Settings::from_config_file_and_defaults)
        .expect("Could not initialize configuration");

    if let Command::DumpConfig = options.cmd {
        dump_config(settings).unwrap();
        std::process::exit(0);
    }

    trace::init_tracing(settings.logging.level).unwrap();

    let seed = config::Seed::from_file_or_generate(&settings.data.dir)
        .expect("Could not retrieve/initialize seed")
        .into();

    let dai_contract_addr = settings.ethereum.dai_contract_address;

    let bitcoin_wallet = bitcoin::Wallet::new(
        seed,
        settings.bitcoin.bitcoind.node_url.clone(),
        settings.bitcoin.network,
    )
    .await
    .expect("can initialise bitcoin wallet");
    let ethereum_wallet = ethereum::Wallet::new(
        seed,
        settings.ethereum.node_url.clone(),
        dai_contract_addr.into(),
        settings.ethereum.chain_id,
    )
    .await
    .expect("can initialise ethereum wallet");

    match options.cmd {
        Command::Trade => {
            let runtime = tokio::runtime::Runtime::new().unwrap();

            trade(
                runtime.handle().clone(),
                &seed,
                settings,
                bitcoin_wallet,
                ethereum_wallet,
            )
            .await
            .expect("Start trading")
        }
        Command::WalletInfo => {
            let wallet_info = wallet_info(ethereum_wallet, bitcoin_wallet).await.unwrap();
            println!("{}", wallet_info);
        }
        Command::Balance => {
            let balance = balance(ethereum_wallet, bitcoin_wallet).await.unwrap();
            println!("{}", balance);
        }
        Command::Deposit => {
            let deposit = deposit(ethereum_wallet, bitcoin_wallet).await.unwrap();
            println!("{}", deposit);
        }
        Command::Withdraw(arguments) => {
            let tx_id = withdraw(ethereum_wallet, bitcoin_wallet, arguments)
                .await
                .unwrap();
            println!("Withdraw successful. Transaction Id: {}", tx_id);
        }
        Command::DumpConfig => unreachable!(),
    }
}

fn read_config(options: &Options) -> anyhow::Result<config::File> {
    // if the user specifies a config path, use it
    if let Some(path) = &options.config_file {
        eprintln!("Using config file {}", path.display());

        return config::File::read(&path)
            .with_context(|| format!("failed to read config file {}", path.display()));
    }

    // try to load default config
    let default_path = nectar::fs::default_config_path()?;

    if !default_path.exists() {
        return Ok(config::File::default());
    }

    eprintln!(
        "Using config file at default path: {}",
        default_path.display()
    );

    config::File::read(&default_path)
        .with_context(|| format!("failed to read config file {}", default_path.display()))
}
