use std::result;
use eth_proof::EthProofError;
use ethers::{contract::ContractError, providers::Middleware};
use near_light_client_on_eth::NearLightClientOnEthError;
use near_rpc_client::NearRpcError;

pub type Result<T> = result::Result<T, SdkError>;

#[derive(thiserror::Error, Debug)]
pub enum SdkError {
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Error communicating with Ethereum: {0}")]
    EthRpcError(String),
    #[error("Error communicating with Near")]
    NearRpcError(#[from] NearRpcError),
    #[error("Near transaction has been sent but its result couldn't be obtained")]
    NearTxFinalizationError,
    #[error("Error retrieving Near proof: {0}")]
    NearProofError(String),
    #[error("Error retrieving Ethereum proof: {0}")]
    EthProofError(String),
    #[error("Unexpected error occured")]
    UnknownError
}

impl From<EthProofError> for SdkError {
    fn from(error: EthProofError) -> Self {
        SdkError::EthProofError(error.to_string())
    }
}

impl<M: Middleware> From<ContractError<M>> for SdkError {
    fn from(error: ContractError<M>) -> Self {
        SdkError::EthRpcError(error.to_string())
    }
}

impl From<NearLightClientOnEthError> for SdkError {
    fn from(error: NearLightClientOnEthError) -> Self {
        match error {
            NearLightClientOnEthError::ConfigError(e) => SdkError::ConfigError(e),
            NearLightClientOnEthError::EthRpcError(e) => SdkError::EthRpcError(e),
        }
    }
}
