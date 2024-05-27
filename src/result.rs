use std::result;
use ethers::{contract::ContractError, providers::Middleware};
use near_jsonrpc_client::{errors::JsonRpcError, methods::{broadcast_tx_async::RpcBroadcastTxAsyncError, query::RpcQueryError, tx::RpcTransactionError}};
use near_jsonrpc_primitives::types::light_client::RpcLightClientProofError;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum NearRpcError {
    BroadcastTxError(JsonRpcError<RpcBroadcastTxAsyncError>),
    QueryError(JsonRpcError<RpcQueryError>),
    FailedToExtractNonce,
    FinalizationTimeoutError,
    RpcTransactionError(JsonRpcError<RpcTransactionError>),
    ProofError(JsonRpcError<RpcLightClientProofError>),
    Other,
}

#[derive(Debug)]
pub enum Error {
    ConfigError(String),
    EthRpcError(String),
    NearRpcError(NearRpcError),
    InvalidProof,
    UnknownError
}

impl From<config::ConfigError> for Error {
    fn from(error: config::ConfigError) -> Self {
        Error::ConfigError(error.to_string())
    }
}

impl<M: Middleware> From<ContractError<M>> for Error {
    fn from(error: ContractError<M>) -> Self {
        Error::EthRpcError(error.to_string())
    }
}

impl<E> From<JsonRpcError<E>> for Error {
    fn from(_: JsonRpcError<E>) -> Self {
        Error::NearRpcError(NearRpcError::Other)
    }
}
