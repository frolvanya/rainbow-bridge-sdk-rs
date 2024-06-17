use ethers::{contract::ContractError, providers::Middleware};

#[derive(thiserror::Error, Debug)]
pub enum NearLightClientOnEthError {
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Error communicating with Ethereum: {0}")]
    EthRpcError(String),
}

impl<M: Middleware> From<ContractError<M>> for NearLightClientOnEthError {
    fn from(error: ContractError<M>) -> Self {
        NearLightClientOnEthError::EthRpcError(error.to_string())
    }
}