use clap::{Args, Parser, Subcommand, ValueEnum};
use eth_connector_command::EthConnectorSubCommand;
use nep141_connector_command::Nep141ConnectorSubCommand;
use std::env;

mod defaults;
mod eth_connector_command;
mod nep141_connector_command;

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
    #[arg(long)]
    eth_custodian_address: Option<String>,
    #[arg(long)]
    eth_connector_account_id: Option<String>,
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
            bridge_token_factory_address: self
                .bridge_token_factory_address
                .or(other.bridge_token_factory_address),
            near_light_client_eth_address: self
                .near_light_client_eth_address
                .or(other.near_light_client_eth_address),
            eth_custodian_address: self.eth_custodian_address.or(other.eth_custodian_address),
            eth_connector_account_id: self
                .eth_connector_account_id
                .or(other.eth_connector_account_id),
        }
    }
}

#[derive(Subcommand, Debug)]
enum SubCommand {
    Nep141Connector {
        #[clap(subcommand)]
        cmd: Nep141ConnectorSubCommand,
    },
    EthConnector {
        #[clap(subcommand)]
        cmd: EthConnectorSubCommand,
    },
}

#[derive(ValueEnum, Clone, Debug)]
enum Network {
    Mainnet,
    Testnet,
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
        eth_chain_id: env::var("ETH_CHAIN_ID")
            .ok()
            .and_then(|val| val.parse::<u64>().ok()),
        near_rpc: env::var("NEAR_RPC").ok(),
        near_signer: env::var("NEAR_SIGNER").ok(),
        near_private_key: env::var("NEAR_PRIVATE_KEY").ok(),
        eth_private_key: env::var("ETH_PRIVATE_KEY").ok(),
        token_locker_id: env::var("TOKEN_LOCKER_ID").ok(),
        bridge_token_factory_address: env::var("BRIDGE_TOKEN_FACTORY_ADDRESS").ok(),
        near_light_client_eth_address: env::var("NEAR_LIGHT_CLIENT_ADDRESS").ok(),
        eth_custodian_address: env::var("ETH_CUSTODIAN_ADDRESS").ok(),
        eth_connector_account_id: env::var("ETH_CONNECTOR_ACCOUNT_ID").ok(),
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
            bridge_token_factory_address: Some(
                defaults::BRIDGE_TOKEN_FACTORY_ADDRESS_MAINNET.to_owned(),
            ),
            near_light_client_eth_address: Some(
                defaults::NEAR_LIGHT_CLIENT_ETH_ADDRESS_MAINNET.to_owned(),
            ),
            eth_connector_account_id: Some(defaults::ETH_CONNECTOR_ACCOUNT_ID_MAINNET.to_owned()),
            eth_custodian_address: Some(defaults::ETH_CUSTODIAN_ADDRESS_MAINNET.to_owned()),
        },
        Network::Testnet => CliConfig {
            eth_rpc: Some(defaults::ETH_RPC_TESTNET.to_owned()),
            eth_chain_id: Some(defaults::ETH_CHAIN_ID_TESTNET),
            near_rpc: Some(defaults::NEAR_RPC_TESTNET.to_owned()),
            near_signer: None,
            near_private_key: None,
            eth_private_key: None,
            token_locker_id: Some(defaults::TOKEN_LOCKER_ID_TESTNET.to_owned()),
            bridge_token_factory_address: Some(
                defaults::BRIDGE_TOKEN_FACTORY_ADDRESS_TESTNET.to_owned(),
            ),
            near_light_client_eth_address: Some(
                defaults::NEAR_LIGHT_CLIENT_ETH_ADDRESS_TESTNET.to_owned(),
            ),
            eth_connector_account_id: Some(defaults::ETH_CONNECTOR_ACCOUNT_ID_TESTNET.to_owned()),
            eth_custodian_address: Some(defaults::ETH_CUSTODIAN_ADDRESS_TESTNET.to_owned()),
        },
    }
}

// TODO: Add file config
// fn file_config() -> CliConfig

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let args = Arguments::parse();

    match args.cmd {
        SubCommand::Nep141Connector { cmd } => {
            nep141_connector_command::match_subcommand(cmd, args.network).await
        }
        SubCommand::EthConnector { cmd } => {
            eth_connector_command::match_subcommand(cmd, args.network).await
        }
    }
}
