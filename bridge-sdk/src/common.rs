use std::result;
use ethers::{contract::ContractError, providers::Middleware};
use near_jsonrpc_client::errors::JsonRpcError;

pub type Result<T> = result::Result<T, SdkError>;

#[derive(thiserror::Error, Debug)]
pub enum SdkError {
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Error communicating with Ethereum: {0}")]
    EthRpcError(String),
    #[error("Error communicating with Near")]
    NearRpcError(#[source] Box<dyn std::error::Error>),
    #[error("Near transaction has been sent but its result couldn't be obtained")]
    NearTxFinalizationError,
    #[error("Error retrieving Near proof: {0}")]
    NearProofError(String),
    #[error("Error retrieving Ethereum proof: {0}")]
    EthProofError(String),
    #[error("Unexpected error occured")]
    UnknownError
}

impl From<config::ConfigError> for SdkError {
    fn from(error: config::ConfigError) -> Self {
        SdkError::ConfigError(error.to_string())
    }
}

impl<M: Middleware> From<ContractError<M>> for SdkError {
    fn from(error: ContractError<M>) -> Self {
        SdkError::EthRpcError(error.to_string())
    }
}

impl<E: std::fmt::Debug + std::fmt::Display + 'static> From<JsonRpcError<E>> for SdkError {
    fn from(error: JsonRpcError<E>) -> Self {
        SdkError::NearRpcError(Box::new(error))
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Env {
    Mainnet,
    Testnet,
}