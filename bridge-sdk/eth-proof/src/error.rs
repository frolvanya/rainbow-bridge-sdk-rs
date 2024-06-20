use cita_trie::TrieError;
use crate::eth_rpc_client::EthRpcError;

#[derive(thiserror::Error, Debug)]
pub enum EthProofError {
    #[error("Could not build a merkle trie for the proof: {0}")]
    TrieError(#[from] TrieError),
    #[error("Could not fetch data for Ethereum proof: {0}")]
    EthRpcError(#[from] EthRpcError),
    #[error("Could not generate Ethereum proof: {0}")]
    Other(String),
}