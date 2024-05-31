use std::sync::Arc;
use ethereum_types::Address;
use ethers::{contract::abigen, providers::{Http, Provider}};
use crate::common::{Env, Error, Result};

abigen!(
    NearLightClient,
    r#"[
        function bridgeState() public view returns (uint,uint,uint,uint)
        function blockHashes(uint64) public view returns (bytes32)
    ]"#
);

pub struct NearOnEthClient {
    eth_endpoint: String,
    near_on_eth_client_address: Address
}

impl NearOnEthClient {
    pub fn new(env: Env, eth_rpc_endpoint: String) -> Self {
        match env {
            Env::Testnet => Self {
                eth_endpoint: eth_rpc_endpoint,
                near_on_eth_client_address: "0x202cdf10bfa45a3d2190901373edd864f071d707".parse().unwrap()
            },
            Env::Mainnet => Self {
                eth_endpoint: eth_rpc_endpoint,
                near_on_eth_client_address: "0x3FEFc5A4B1c02f21cBc8D3613643ba0635b9a873".parse().unwrap()
            }
        }
    }

    pub async fn get_sync_height(&self) -> Result<u64> {
        let eth_provider = self.eth_provider()?;
        let client = Arc::new(eth_provider);
        let contract = NearLightClient::new(self.near_on_eth_client_address, client);
        
        let state = contract.bridge_state().call().await?;

        Ok(state.0.as_u64())
    }

    pub async fn get_block_hash(&self, block_number: u64) -> Result<[u8; 32]> {
        let eth_provider = self.eth_provider()?;
        let client = Arc::new(eth_provider);
        let contract = NearLightClient::new(self.near_on_eth_client_address, client);
        
        let state = contract.block_hashes(block_number).call().await?;

        Ok(state)
    }

    fn eth_provider(&self) -> Result<Provider<Http>> {
        Ok(Provider::<Http>::try_from(self.eth_endpoint.clone())
            .map_err(|_| Error::ConfigError("Ethereum endpoint url is invalid".to_string()))?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::{Config, File, FileFormat};

    fn get_config() -> Config {
        let config_path = env!("CARGO_MANIFEST_DIR").to_owned() + "/src/testnet.config.json";
        Config::builder()
            .add_source(File::new(&config_path, FileFormat::Json))
            .build().unwrap()
    }

    #[tokio::test]
    async fn test_sync_height() {
        let eth_rpc_endpoint = get_config().get_string("eth_rpc_endpoint").unwrap();
        let client = NearOnEthClient::new(Env::Testnet, eth_rpc_endpoint);

        let sync_height = client.get_sync_height().await.unwrap();
        println!("Current sync height: {:?}", sync_height);
    }

    #[tokio::test]
    async fn test_block_hashes() {
        let eth_rpc_endpoint = get_config().get_string("eth_rpc_endpoint").unwrap();
        let client = NearOnEthClient::new(Env::Testnet, eth_rpc_endpoint);

        let block_hash = client.get_block_hash(164243835).await.unwrap();
        println!("Block hash: {:?}", block_hash);
    }
}