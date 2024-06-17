mod proof_generator;
mod eth_rpc_client;
mod error;

pub use error::EthProofError;
pub use proof_generator::get_proof_for_event;