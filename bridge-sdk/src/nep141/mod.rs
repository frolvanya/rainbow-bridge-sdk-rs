use std::{str::FromStr, sync::Arc};
use borsh::BorshSerialize;
use ethers::{abi::Address, prelude::*};
use near_crypto::SecretKey;
use near_primitives::{hash::CryptoHash, types::{AccountId, TransactionOrReceiptId}};
use crate::{common::{Env, SdkError, Result}, eth_proof_generator, near_on_eth_client::NearOnEthClient, near_rpc_client};
use light_client_proof::LightClientExecutionProof;

mod light_client_proof;

abigen!(
    BridgeTokenFactory,
    r#"[
      function newBridgeToken(bytes memory proofData, uint64 proofBlockHeight) external returns (address)
      function deposit(bytes memory proofData, uint64 proofBlockHeight) external
      function withdraw(string memory token, uint256 amount, string memory recipient) external
      function nearToEthToken(string calldata nearTokenId) external view returns (address)
    ]"#
);

abigen!(
    ERC20,
    r#"[
      function allowance(address _owner, address _spender) public view returns (uint256 remaining)
      function approve(address spender, uint256 amount) external returns (bool)
    ]"#
);

/// Bridging NEAR-originated NEP-141 tokens to Ethereum and back
pub struct Nep141Bridging {
    eth_endpoint: Option<String>,
    eth_chain_id: u64,
    eth_private_key: Option<String>,
    bridge_token_factory_address: String,
    near_endpoint: Option<String>,
    near_private_key: Option<String>,
    near_signer: Option<String>,
    token_locker_id: String,
    environment: Env,
}

impl Nep141Bridging {
    /// Creates a new instance of the bridging client. Constant values are set here, while the others can be provided separately depending on the use case.
    pub fn new(env: Env) -> Self {
        match env {
            Env::Testnet => Self {
                eth_chain_id: 11155111,
                bridge_token_factory_address: "0xa9108f7F83Fb661e611991116D526fCa1a9585ab".to_string(),
                eth_endpoint: None,
                eth_private_key: None,
                near_endpoint: None,
                near_private_key: None,
                near_signer: None,
                token_locker_id: "ft-locker.sepolia.testnet".to_string(),
                environment: env,
            },
            Env::Mainnet => Self {
                eth_chain_id: 1,
                bridge_token_factory_address: "0x252e87862A3A720287E7fd527cE6e8d0738427A2".to_string(),
                eth_endpoint: None,
                eth_private_key: None,
                near_endpoint: None,
                near_private_key: None,
                near_signer: None,
                token_locker_id: "ft-locker.bridge.near".to_string(),
                environment: env,
            }
        }
    }

    /// Set Ethereum RPC endpoint
    /// 
    /// Required for `deploy_token`, `mint`, `burn`, `withdraw`
    pub fn with_eth_endpoint(mut self, endpoint: String) -> Self {
        self.eth_endpoint = Some(endpoint);
        self
    }

    /// Set Ethereum wallet private key
    /// 
    /// Required for `deploy_token`, `mint`, `burn`
    pub fn with_eth_private_key(mut self, private_key: String) -> Self {
        self.eth_private_key = Some(private_key);
        self
    }

    /// Set Near RPC endpoint
    /// 
    /// Required for `log_token_metadata`, `storage_deposit_for_token`, `deploy_token`, `deposit`, `mint`, `withdraw`
    pub fn with_near_endpoint(mut self, endpoint: String) -> Self {
        self.near_endpoint = Some(endpoint);
        self
    }

    /// Set near transaction signer
    /// 
    /// Required for `log_token_metadata`, `storage_deposit_for_token`, `deploy_token`, `deposit`, `withdraw`
    pub fn with_near_signer(mut self, signer: String, private_key: String) -> Self {
        self.near_private_key = Some(private_key);
        self.near_signer = Some(signer);
        self
    }

    /// Logs token metadata to token_locker contract. The proof from this transaction is then used to deploy a corresponding token on Ethereum
    pub async fn log_token_metadata(&self, near_token_id: String) -> Result<CryptoHash> {
        let near_endpoint = self.near_endpoint()?;

        let args = format!(r#"{{"token_id":"{near_token_id}"}}"#)
            .to_string()
            .into_bytes();

        Ok(near_rpc_client::methods::change(
            near_endpoint,
            self.near_signer()?,
            self.token_locker_id.to_string(),
            "log_metadata".to_string(),
            args,
            300_000_000_000_000,
            0
        ).await?)
    }

    /// Performs a storage deposit on behalf of the token_locker so that the tokens can be transferred to the locker. To be called once for each NEP-141
    pub async fn storage_deposit_for_token(&self, near_token_id: String, amount: u128) -> Result<CryptoHash> {
        let near_endpoint = self.near_endpoint()?;
        let token_locker = self.token_locker_id.clone();

        let args = format!(r#"{{"account_id":"{token_locker}"}}"#)
            .to_string()
            .into_bytes();

        Ok(near_rpc_client::methods::change(
            near_endpoint,
            self.near_signer()?,
            near_token_id,
            "storage_deposit".to_string(),
            args,
            300_000_000_000_000,
            amount
        ).await?)
    }

    /// Deploys an ERC-20 token that will be used when bridging NEP-141 tokens to Ethereum. Requires a receipt from log_metadata transaction on Near
    pub async fn deploy_token(
        &self,
        receipt_id: CryptoHash,
    ) -> Result<TxHash> {
        let eth_endpoint = self.eth_endpoint()?;
        let near_endpoint = self.near_endpoint()?;

        let near_on_eth_client = NearOnEthClient::new(self.environment, eth_endpoint.to_string());

        let proof_block_height = near_on_eth_client.get_sync_height().await?;
        let block_hash = near_on_eth_client.get_block_hash(proof_block_height).await?;

        let receipt_id = TransactionOrReceiptId::Receipt {
            receipt_id,
            receiver_id: AccountId::from_str(&self.token_locker_id)
                .map_err(|_| SdkError::UnknownError)?
        };

        let proof_data: LightClientExecutionProof = near_rpc_client::methods::get_light_client_proof(
            near_endpoint,
            receipt_id,
            CryptoHash(block_hash)
        ).await?.into();

        let mut buffer: Vec<u8> = Vec::new();
        proof_data.serialize(&mut buffer)
            .map_err(|_| SdkError::NearProofError("Failed to deserialize proof".to_string()))?;
    
        let factory = self.bridge_token_factory()?;
        let call = factory.new_bridge_token(buffer.into(), proof_block_height);

        let tx = call.send().await?;
        Ok(tx.tx_hash())
    }

    /// Transfers NEP-141 tokens to the token locker. The proof from this transaction is then used to mint the corresponding tokens on Ethereum
    pub async fn deposit(&self, near_token_id: String, amount: u128, eth_receiver: String) -> Result<CryptoHash> {
        let near_endpoint = self.near_endpoint()?;
        let token_locker = self.token_locker_id.clone();

        let args = format!(r#"{{"receiver_id":"{token_locker}","amount":"{amount}","msg":"{eth_receiver}"}}"#)
            .to_string()
            .into_bytes();

        let tx_hash = near_rpc_client::methods::change(
            near_endpoint,
            self.near_signer()?,
            near_token_id,
            "ft_transfer_call".to_string(),
            args,
            300_000_000_000_000,
            1
        ).await?;
        
        Ok(tx_hash)
    }

    /// Mints the corresponding bridged tokens on Ethereum. Requires a proof from the deposit transaction on Near
    pub async fn mint(&self, receipt_id: CryptoHash) -> Result<TxHash> {
        let eth_endpoint = self.eth_endpoint()?;
        let near_endpoint = self.near_endpoint()?;

        let near_on_eth_client = NearOnEthClient::new(self.environment, eth_endpoint.to_string());

        let proof_block_height = near_on_eth_client.get_sync_height().await?;
        let block_hash = near_on_eth_client.get_block_hash(proof_block_height).await?;

        let receipt_id = TransactionOrReceiptId::Receipt {
            receipt_id,
            receiver_id: AccountId::from_str(&self.token_locker_id)
                .map_err(|_| SdkError::UnknownError)?
        };

        let proof_data: LightClientExecutionProof = near_rpc_client::methods::get_light_client_proof(
            near_endpoint,
            receipt_id,
            CryptoHash(block_hash)
        ).await?.into();

        let mut buffer: Vec<u8> = Vec::new();
        proof_data.serialize(&mut buffer)
            .map_err(|_| SdkError::NearProofError("Falied to deserialize proof".to_string()))?;
            
        let factory = self.bridge_token_factory()?;
        let call = factory.deposit(buffer.into(), proof_block_height);

        let tx = call.send().await?;
        Ok(tx.tx_hash())
    }

    /// Burns bridged tokens on Ethereum. The proof from this transaction is then used to withdraw the corresponding tokens on Near
    pub async fn burn(
        &self,
        near_token_id: String,
        amount: U256,
        receiver: String
    ) -> Result<TxHash> {
        let factory = self.bridge_token_factory()?;

        let erc20_address = factory.near_to_eth_token(near_token_id.clone())
            .call()
            .await?;

        let bridge_token = &self.bridge_token(erc20_address)?;

        let signer = self.eth_signer()?;
        let bridge_token_factory_address = self.bridge_token_factory_address()?;
        let allowance = bridge_token.allowance(signer.address(), bridge_token_factory_address.clone())
            .call()
            .await?;

        if allowance < amount {
            bridge_token.approve(bridge_token_factory_address, amount - allowance)
                .send()
                .await?
                .await
                .map_err(|e| SdkError::EthRpcError(e.to_string()))?;

            println!("Approved token for spending");
        }

        let withdraw_call = factory.withdraw(near_token_id, amount, receiver);

        let tx = withdraw_call.send().await?;
        Ok(tx.tx_hash())
    }

    /// Withdraws NEP-141 tokens from the token locker. Requires a proof from the burn transaction on Ethereum
    pub async fn withdraw(&self, tx_hash: TxHash, log_index: u64) -> Result<CryptoHash> {
        let eth_endpoint = self.eth_endpoint()?;
        let near_endpoint = self.near_endpoint()?;

        let proof = eth_proof_generator::get_proof_for_event(tx_hash, log_index, eth_endpoint)
            .await?;

        let mut args = Vec::new();
        proof.serialize(&mut args)
            .map_err(|_| SdkError::EthProofError("Failed to serialize proof".to_string()))?;

        let tx_hash = near_rpc_client::methods::change(
            near_endpoint,
            self.near_signer()?,
            self.token_locker_id.to_string(),
            "withdraw".to_string(),
            args,
            300_000_000_000_000,
            60_000_000_000_000_000_000_000
        ).await?;
        
        Ok(tx_hash)
    }

    fn eth_endpoint(&self) -> Result<&str> {
        Ok(self.eth_endpoint
            .as_ref()
            .ok_or(SdkError::ConfigError("Ethereum rpc endpoint not set".to_string()))?)
    }

    fn near_endpoint(&self) -> Result<&str> {
        Ok(self.near_endpoint
            .as_ref()
            .ok_or(SdkError::ConfigError("Near rpc endpoint not set".to_string()))?)
    }

    fn near_signer(&self) -> Result<near_crypto::InMemorySigner> {
        let near_private_key = self.near_private_key
            .as_ref()
            .ok_or(SdkError::ConfigError("Near account private key not set".to_string()))?;
        let near_signer = self.near_signer
            .as_ref()
            .ok_or(SdkError::ConfigError("Near signer account id not set".to_string()))?;

        Ok(near_crypto::InMemorySigner::from_secret_key(
            AccountId::from_str(near_signer)
                .map_err(|_| SdkError::ConfigError("Invalid near signer account id".to_string()))?,
            SecretKey::from_str(near_private_key)
                .map_err(|_| SdkError::ConfigError("Invalid near private key".to_string()))?
        ))
    }

    fn bridge_token_factory(&self) -> Result<BridgeTokenFactory<SignerMiddleware<Provider<Http>,LocalWallet>>> {
        let eth_endpoint = self.eth_endpoint
            .as_ref()
            .ok_or(SdkError::ConfigError("Ethereum rpc endpoint not set".to_string()))?;

        let eth_provider = Provider::<Http>::try_from(eth_endpoint)
            .map_err(|_| SdkError::ConfigError("Invalid ethereum rpc endpoint url".to_string()))?;

        let wallet = self.eth_signer()?;

        let signer = SignerMiddleware::new(eth_provider, wallet);
        let client = Arc::new(signer);

        Ok(BridgeTokenFactory::new(
            self.bridge_token_factory_address()?,
            client
        ))
    }

    fn bridge_token(&self, address: Address) -> Result<ERC20<SignerMiddleware<Provider<Http>,LocalWallet>>> {
        let eth_endpoint = self.eth_endpoint
            .as_ref()
            .ok_or(SdkError::ConfigError("Ethereum rpc endpoint not set".to_string()))?;

        let eth_provider = Provider::<Http>::try_from(eth_endpoint)
            .map_err(|_| SdkError::ConfigError("Invalid ethereum rpc endpoint url".to_string()))?;

        let wallet = self.eth_signer()?;

        let signer = SignerMiddleware::new(eth_provider, wallet);
        let client = Arc::new(signer);

        Ok(ERC20::new(
            address,
            client
        ))
    }

    fn eth_signer(&self) -> Result<LocalWallet> {
        let eth_private_key = self.eth_private_key
            .as_ref()
            .ok_or(SdkError::ConfigError("Ethereum private key not set".to_string()))?;

        let private_key_bytes = hex::decode(eth_private_key)
            .map_err(|_| SdkError::ConfigError("Ethereum private key is not a valid hex string".to_string()))?;

        if private_key_bytes.len() != 32 {
            return Err(SdkError::ConfigError("Ethereum private key is of invalid length".to_string()));
        }

        Ok(LocalWallet::from_bytes(&private_key_bytes)
            .map_err(|_| SdkError::ConfigError("Invalid ethereum private key".to_string()))?
            .with_chain_id(self.eth_chain_id))
    }

    fn bridge_token_factory_address(&self) -> Result<Address> {
        Address::from_str(&self.bridge_token_factory_address)
            .map_err(|_| SdkError::ConfigError("Couldn't parse nep141_eth_token_factory".to_string()))
    }
}
