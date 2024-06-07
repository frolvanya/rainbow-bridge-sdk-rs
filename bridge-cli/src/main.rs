use std::env;
use bridge_sdk::{common::Env, nep141::Nep141Bridging};
use clap::{Args, Parser, Subcommand, ValueEnum};

mod defaults;

#[derive(Args, Debug, Clone)]
struct CliConfig {
    #[arg(long)]
    eth_rpc: Option<String>,
    #[arg(long)]
    eth_chain_id: Option<u64>,
    #[arg(long)]
    near_rpc: Option<String>,
    #[arg(long)]
    near_signer: Option<String>,
    #[arg(long)]
    near_private_key: Option<String>,
    #[arg(long)]
    eth_private_key: Option<String>,
    #[arg(long)]
    token_locker_id: Option<String>,
    #[arg(long)]
    bridge_token_factory_address: Option<String>,
    #[arg(long)]
    near_light_client_eth_address: Option<String>,
}

#[derive(Subcommand, Debug)]
enum SubCommand {
    Nep141LogMetadata {
        #[clap(short, long)]
        token: String,
        #[command(flatten)]
        config_cli: CliConfig,
    },
    Nep141DeployToken {
        #[clap(short, long)]
        receipt_id: String,
        #[command(flatten)]
        config_cli: CliConfig,
    },
    Nep141FinTransfer {
        #[clap(short, long)]
        receipt_id: String,
        #[command(flatten)]
        config_cli: CliConfig,
    },
}

#[derive(ValueEnum, Clone, Debug)]
enum Network {
    Mainnet,
    Testnet,
}

impl From<Network> for Env {
    fn from(network: Network) -> Env {
        match network {
            Network::Mainnet => Env::Mainnet,
            Network::Testnet => Env::Testnet,
        }
    }
}

#[derive(Parser, Debug)]
#[clap(version)]
struct Arguments {
    network: Network,
    #[command(subcommand)]
    cmd: SubCommand,
}

fn env_config() -> CliConfig {
    CliConfig {
        eth_rpc: env::var("ETH_RPC").ok(),
        eth_chain_id: env::var("ETH_CHAIN_ID").ok()
            .and_then(|val| val.parse::<u64>().ok()),
        near_rpc: env::var("NEAR_RPC").ok(),
        near_signer: env::var("NEAR_SIGNER").ok(),
        near_private_key: env::var("NEAR_PRIVATE_KEY").ok(),
        eth_private_key: env::var("ETH_PRIVATE_KEY").ok(),
        token_locker_id: env::var("TOKEN_LOCKER_ID").ok(),
        bridge_token_factory_address: env::var("BRIDGE_TOKEN_FACTORY_ADDRESS").ok(),
        near_light_client_eth_address: env::var("NEAR_LIGHT_CLIENT_ADDRESS").ok(),
    }
}

fn default_config(network: Network) -> CliConfig {
    match network {
        Network::Mainnet => CliConfig {
            eth_rpc: Some(defaults::ETH_RPC_MAINNET.to_owned()),
            eth_chain_id: Some(defaults::ETH_CHAIN_ID_MAINNET),
            near_rpc: Some(defaults::NEAR_RPC_MAINNET.to_owned()),
            near_signer: None,
            near_private_key: None,
            eth_private_key: None,
            token_locker_id: Some(defaults::TOKEN_LOCKER_ID_MAINNET.to_owned()),
            bridge_token_factory_address: Some(defaults::BRIDGE_TOKEN_FACTORY_ADDRESS_MAINNET.to_owned()),
            near_light_client_eth_address: Some(defaults::NEAR_LIGHT_CLIENT_ETH_ADDRESS_MAINNET.to_owned()),
        },
        Network::Testnet => CliConfig {
            eth_rpc: Some(defaults::ETH_RPC_TESTNET.to_owned()),
            eth_chain_id: Some(defaults::ETH_CHAIN_ID_TESTNET),
            near_rpc: Some(defaults::NEAR_RPC_TESTNET.to_owned()),
            near_signer: None,
            near_private_key: None,
            eth_private_key: None,
            token_locker_id: Some(defaults::TOKEN_LOCKER_ID_TESTNET.to_owned()),
            bridge_token_factory_address: Some(defaults::BRIDGE_TOKEN_FACTORY_ADDRESS_TESTNET.to_owned()),
            near_light_client_eth_address: Some(defaults::NEAR_LIGHT_CLIENT_ETH_ADDRESS_TESTNET.to_owned()),
        },
    }
}

// TODO: Add file config
// fn file_config() -> CliConfig

fn nep141_bridging(network: Network, config: CliConfig) -> Nep141Bridging {
    // TODO: replace unwrap
    let env_config = env_config();
    let default_config = default_config(network);

    let mut bridging = Nep141Bridging::new()
        .with_eth_endpoint(
            config.eth_rpc.or(env_config.eth_rpc).or(default_config.eth_rpc).unwrap(),
            config.eth_chain_id.or(env_config.eth_chain_id).or(default_config.eth_chain_id).unwrap()
        )
        .with_near_endpoint(config.near_rpc.or(env_config.near_rpc).or(default_config.near_rpc).unwrap())
        .with_token_locker_id(
            config.token_locker_id.or(env_config.token_locker_id).or(default_config.token_locker_id).unwrap()
        )
        .with_bridge_token_factory_address(
            config.bridge_token_factory_address
                .or(env_config.bridge_token_factory_address)
                .or(default_config.bridge_token_factory_address)
                .unwrap()
        )
        .with_near_light_client_address(
            config.near_light_client_eth_address
                .or(env_config.near_light_client_eth_address)
                .or(default_config.near_light_client_eth_address)
                .unwrap()
        );

    let near_signer = config.near_signer.or(env_config.near_signer);
    let near_private_key = config.near_private_key.or(env_config.near_private_key);

    match (near_signer, near_private_key) {
        (Some(near_signer), Some(near_private_key)) => {
            bridging = bridging.with_near_signer(near_signer, near_private_key);
        },
        (Some(_), None) => {
            panic!("Near signer is provided but Near private key is missing");
        },
        (None, Some(_)) => {
            panic!("Near private key is provided but Near signer is missing");
        },
        (None, None) => {
        },
    }
    
    if let Some(eth_private_key) = config.eth_private_key.or(env_config.eth_private_key) {
        bridging = bridging.with_eth_private_key(eth_private_key);
    }
    
    bridging
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let args = Arguments::parse();

    match args.cmd {
        SubCommand::Nep141LogMetadata { token, config_cli } => {
            let tx_hash = nep141_bridging(args.network, config_cli)
                .log_token_metadata(token)
                .await
                .unwrap();
            println!("Tx hash: {tx_hash}")
        }
        SubCommand::Nep141DeployToken {
            receipt_id,
            config_cli,
        } => {
            // TODO: use tx hash instead receipt_id
            let tx_hash = nep141_bridging(args.network, config_cli)
                .deploy_token(receipt_id.parse().expect("Invalid receipt_id"))
                .await
                .unwrap();
            println!("Tx hash: {tx_hash}")
        }
        SubCommand::Nep141FinTransfer {
            receipt_id,
            config_cli,
        } => {
            // TODO: use tx hash instead receipt_id
            let tx_hash = nep141_bridging(args.network, config_cli)
                .mint(receipt_id.parse().expect("Invalid rreceipt_id"))
                .await
                .unwrap();
            println!("Tx hash: {tx_hash}")
        }
    }
}
