use types::{BlockHeader, TransactionReceipt};
use reqwest::Client;
use ::serde::Deserialize;
use serde_json::{json, Value};
use ethereum_types::{H256, U64};

pub mod types;
mod serde;

#[derive(thiserror::Error, Debug)]
#[error("Ethereum RPC error: {0}")]
pub struct EthRpcError(String);

impl From<reqwest::Error> for EthRpcError {
    fn from(error: reqwest::Error) -> Self {
        EthRpcError(error.to_string())
    }
}

pub struct EthRPCClient {
    endpoint_url: String,
    client: Client,
}

impl EthRPCClient {
    pub fn new(endpoint_url: &str) -> Self {
        Self {
            endpoint_url: endpoint_url.to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub async fn get_transaction_receipt_by_hash(&self, tx_hash: &H256) -> Result<TransactionReceipt, EthRpcError> {
        let json_value = json!({
            "id": 1,
            "jsonrpc": "2.0",
            "method": "eth_getTransactionReceipt",
            "params": [format!("{tx_hash:#x}")]
        });

        let res = self
            .client
            .post(&self.endpoint_url)
            .json(&json_value)
            .send().await?
            .text().await?;

        let val: Value = serde_json::from_str(&res)
            .map_err(|_| EthRpcError("Couldn't deserialize transaction receipt".to_string()))?;
        let receipt = TransactionReceipt::deserialize(&val["result"])
            .map_err(|_| EthRpcError("Couldn't deserialize transaction receipt".to_string()))?;

        Ok(receipt)
    }

    pub async fn get_block_by_number(&self, block_number: U64) -> Result<BlockHeader, EthRpcError> {
        let json_value = json!({
            "id": 1,
            "jsonrpc": "2.0",
            "method": "eth_getBlockByNumber",
            "params": [format!("0x{:x}", block_number), false]
        });

        let res = self
            .client
            .post(&self.endpoint_url)
            .json(&json_value)
            .send().await?
            .text().await?;

        let val: Value = serde_json::from_str(&res)
            .map_err(|_| EthRpcError("Couldn't deserialize block number".to_string()))?;
        let header = BlockHeader::deserialize(&val["result"])
            .map_err(|_| EthRpcError("Couldn't deserialize block number".to_string()))?;

        Ok(header)
    }

    pub async fn get_block_receipts(
        &self,
        block_number: U64,
    ) -> Result<Vec<TransactionReceipt>, EthRpcError> {
        let json_value = json!({
            "id": 1,
            "jsonrpc": "2.0",
            "method": "eth_getBlockReceipts",
            "params": [format!("0x{:x}", block_number)]
        });

        let res = self
            .client
            .post(&self.endpoint_url)
            .json(&json_value)
            .send().await?
            .text().await?;

        let val: Value = serde_json::from_str(&res)
            .map_err(|_| EthRpcError("Couldn't deserialize block receipts".to_string()))?;
        let receipts = Vec::<TransactionReceipt>::deserialize(&val["result"])
            .map_err(|_| EthRpcError("Couldn't deserialize block receipts".to_string()))?;

        Ok(receipts)
    }
}
