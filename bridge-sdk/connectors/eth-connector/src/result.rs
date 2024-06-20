use std::result;
use eth_proof::EthProofError;
use ethers::{contract::ContractError, providers::Middleware};
use near_light_client_on_eth::NearLightClientOnEthError;
use near_rpc_client::NearRpcError;

#[derive(thiserror::Error, Debug)]
pub enum EthConnectorError {
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Error communicating with Ethereum: {0}")]
    EthRpcError(String),
    #[error("Error retrieving Ethereum proof: {0}")]
    EthProofError(String),
    #[error("Error retrieving Near proof: {0}")]
    NearProofError(String),
    #[error("Error communicating with Near")]
    NearRpcError(#[from] NearRpcError),
    #[error("Unexpected error occured")]
    UnknownError,
}

pub type EthConnectorResult<T> = result::Result<T, EthConnectorError>;

impl From<EthProofError> for EthConnectorError {
    fn from(error: EthProofError) -> Self {
        EthConnectorError::EthProofError(error.to_string())
    }
}

impl<M: Middleware> From<ContractError<M>> for EthConnectorError {
    fn from(error: ContractError<M>) -> Self {
        EthConnectorError::EthRpcError(error.to_string())
    }
}

impl From<NearLightClientOnEthError> for EthConnectorError {
    fn from(error: NearLightClientOnEthError) -> Self {
        match error {
            NearLightClientOnEthError::ConfigError(e) => EthConnectorError::ConfigError(e),
            NearLightClientOnEthError::EthRpcError(e) => EthConnectorError::EthRpcError(e),
        }
    }
}