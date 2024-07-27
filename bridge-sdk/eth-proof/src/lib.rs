mod error;
mod proof_generator;

pub use error::EthProofError;
pub use proof_generator::get_event_proof;
pub use proof_generator::get_storage_proof;
