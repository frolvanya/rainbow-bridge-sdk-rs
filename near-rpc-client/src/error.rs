use near_jsonrpc_client::errors::JsonRpcError;

#[derive(thiserror::Error, Debug)]
#[error("Failed to generate Ethereum proof: {0}")]
pub struct NearRpcError(#[source] pub Box<dyn std::error::Error>);

impl<E: std::fmt::Debug + std::fmt::Display + 'static> From<JsonRpcError<E>> for NearRpcError {
    fn from(error: JsonRpcError<E>) -> Self {
        NearRpcError(Box::new(error))
    }
}