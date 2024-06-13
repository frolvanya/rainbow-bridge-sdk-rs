use cita_trie::TrieError;
use crate::eth_rpc_client::EthRpcError;

#[derive(thiserror::Error, Debug)]
#[error("Failed to generate Ethereum proof: {0}")]
pub struct EthProofError(pub String);

impl From<TrieError> for EthProofError {
    fn from(error: TrieError) -> Self {
        EthProofError(error.to_string())
    }
}

impl From<EthRpcError> for EthProofError {
    fn from(error: EthRpcError) -> Self {
        EthProofError(error.to_string())
    }
}