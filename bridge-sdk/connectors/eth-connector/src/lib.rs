#[macro_use]
extern crate derive_builder;

mod eth_connector;
mod result;

pub use eth_connector::{EthConnector, EthConnectorBuilder};