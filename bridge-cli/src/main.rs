use std::env;

use bridge_sdk::{common::Env, nep141::Nep141Bridging};
use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Args, Debug, Clone)]
struct CliConfig {
    eth_rpc: Option<String>,
    near_rpc: Option<String>,
    near_signer: Option<String>,
    near_private_key: Option<String>,
    eth_private_key: Option<String>,
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
        near_rpc: env::var("NEAR_RPC").ok(),
        near_signer: env::var("NEAR_SIGNER").ok(),
        near_private_key: env::var("NEAR_PRIVATE_KEY").ok(),
        eth_private_key: env::var("ETH_PRIVATE_KEY").ok(),
    }
}

// TODO: Add file config
// fn file_config() -> CliConfig

fn nep141_bridging(network: Network, config: CliConfig) -> Nep141Bridging {
    // TODO: replace unwrap
    let env_config = env_config();
    Nep141Bridging::new(network.into())
        .with_eth_endpoint(config.eth_rpc.or(env_config.eth_rpc).unwrap())
        .with_near_endpoint(config.near_rpc.or(env_config.near_rpc).unwrap())
        .with_near_signer(
            config.near_signer.or(env_config.near_signer).unwrap(),
            config
                .near_private_key
                .or(env_config.near_private_key)
                .unwrap(),
        )
        .with_eth_private_key(
            config
                .eth_private_key
                .or(env_config.eth_private_key)
                .unwrap(),
        )
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
