use std::env;
use bridge_sdk::{common::Env, nep141::{Nep141Bridging, Nep141BridgingBuilder}};
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

impl CliConfig {
    fn or(self, other: Self) -> Self {
        Self {
            eth_rpc: self.eth_rpc.or(other.eth_rpc),
            eth_chain_id: self.eth_chain_id.or(other.eth_chain_id),
            near_rpc: self.near_rpc.or(other.near_rpc),
            near_signer: self.near_signer.or(other.near_signer),
            near_private_key: self.near_private_key.or(other.near_private_key),
            eth_private_key: self.eth_private_key.or(other.eth_private_key),
            token_locker_id: self.token_locker_id.or(other.token_locker_id),
            bridge_token_factory_address: self.bridge_token_factory_address.or(other.bridge_token_factory_address),
            near_light_client_eth_address: self.near_light_client_eth_address.or(other.near_light_client_eth_address),
        }
    }
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

fn nep141_bridging(network: Network, cli_config: CliConfig) -> Nep141Bridging {
    // TODO: replace unwrap
    let combined_config = cli_config
        .or(env_config())
        .or(default_config(network));

    Nep141BridgingBuilder::default()
        .eth_endpoint(combined_config.eth_rpc)
        .eth_chain_id(combined_config.eth_chain_id)
        .near_endpoint(combined_config.near_rpc)
        .token_locker_id(combined_config.token_locker_id)
        .bridge_token_factory_address(combined_config.bridge_token_factory_address)
        .near_light_client_address(combined_config.near_light_client_eth_address)
        .eth_private_key(combined_config.eth_private_key)
        .near_signer(combined_config.near_signer)
        .near_private_key(combined_config.near_private_key)
        .build()
        .unwrap()
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
