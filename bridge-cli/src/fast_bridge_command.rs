use crate::{combined_config, CliConfig, Network};
use clap::Subcommand;
use ethers_core::types::{Address, TxHash};
use fast_bridge::{FastBridge, FastBridgeBuilder};
use near_primitives::types::AccountId;
use std::str::FromStr;

#[derive(Subcommand, Debug)]
pub enum FastBridgeSubCommand {
    Transfer {
        #[clap(short, long)]
        token: String,
        #[clap(short, long)]
        amount: u128,
        #[clap(short, long)]
        fee: u128,
        #[clap(short, long)]
        eth_token_address: String,
        #[clap(short, long)]
        recipient: String,
        #[clap(short, long)]
        valid_till: u64,
        #[command(flatten)]
        config_cli: CliConfig,
    },
    TransferOnEth {
        #[clap(short, long)]
        token: String,
        #[clap(short, long)]
        amount: u128,
        #[clap(short, long)]
        recipient: String,
        #[clap(short, long)]
        nonce: u128,
        #[clap(short, long)]
        unlock_recipient: String,
        #[clap(short, long)]
        valid_till_block_height: u128,
        #[command(flatten)]
        config_cli: CliConfig,
    },
    LpUnlock {
        #[clap(short, long)]
        tx_hash: String,
        #[clap(short, long)]
        log_index: u64,
        #[command(flatten)]
        config_cli: CliConfig,
    },
    Withdraw {
        #[clap(short, long)]
        token: String,
        #[clap(short, long)]
        recipient: Option<String>,
        #[command(flatten)]
        config_cli: CliConfig,
    },
}

pub async fn match_subcommand(cmd: FastBridgeSubCommand, network: Network) {
    match cmd {
        FastBridgeSubCommand::Transfer {
            token,
            amount,
            fee,
            eth_token_address,
            recipient,
            valid_till,
            config_cli,
        } => {
            fast_bridge(network, config_cli)
                .transfer(
                    AccountId::from_str(&token).expect("Invalid token"),
                    amount,
                    fee,
                    Address::from_str(&eth_token_address).expect("Invalid eth_token_address"),
                    Address::from_str(&recipient).expect("Invalid recipient"),
                    valid_till,
                )
                .await
                .unwrap();
        }
        FastBridgeSubCommand::TransferOnEth {
            token,
            amount,
            recipient,
            nonce,
            unlock_recipient,
            valid_till_block_height,
            config_cli,
        } => {
            fast_bridge(network, config_cli)
                .transfer_on_eth(
                    Address::from_str(&token).expect("Invalid token"),
                    Address::from_str(&recipient).expect("Invalid recipient"),
                    nonce.into(),
                    amount.into(),
                    unlock_recipient,
                    valid_till_block_height.into(),
                )
                .await
                .unwrap();
        }
        FastBridgeSubCommand::LpUnlock {
            tx_hash,
            log_index,
            config_cli,
        } => {
            fast_bridge(network, config_cli)
                .lp_unlock(
                    TxHash::from_str(&tx_hash).expect("Invalid tx_hash"),
                    log_index,
                )
                .await
                .unwrap();
        }
        FastBridgeSubCommand::Withdraw {
            token,
            recipient,
            config_cli,
        } => {
            fast_bridge(network, config_cli)
                .withdraw(
                    AccountId::from_str(&token).expect("Invalid token"),
                    None,
                    recipient.map(|recipient| {
                        AccountId::from_str(&recipient).expect("Invalid recipient")
                    }),
                    None,
                )
                .await
                .unwrap();
        }
    }
}

fn fast_bridge(network: Network, cli_config: CliConfig) -> FastBridge {
    let combined_config = combined_config(cli_config, network);

    FastBridgeBuilder::default()
        .eth_endpoint(combined_config.eth_rpc)
        .eth_chain_id(combined_config.eth_chain_id)
        .eth_private_key(combined_config.eth_private_key)
        .near_endpoint(combined_config.near_rpc)
        .near_private_key(combined_config.near_private_key)
        .near_signer(combined_config.near_signer)
        .fast_bridge_account_id(combined_config.fast_bridge_account_id)
        .fast_bridge_address(combined_config.fast_bridge_address)
        .build()
        .unwrap()
}
