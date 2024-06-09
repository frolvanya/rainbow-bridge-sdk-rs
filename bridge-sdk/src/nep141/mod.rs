use std::{str::FromStr, sync::Arc};
use borsh::BorshSerialize;
use ethers::{abi::Address, prelude::*};
use near_crypto::SecretKey;
use near_primitives::{hash::CryptoHash, types::{AccountId, TransactionOrReceiptId}};
use crate::{common::{Result, SdkError}, eth_proof_generator, near_on_eth_client::NearOnEthClient, near_rpc_client};
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
#[derive(Builder)]
pub struct Nep141Bridging {
    #[doc = r"Ethereum RPC endpoint. Required for `deploy_token`, `mint`, `burn`, `withdraw`"]
    eth_endpoint: Option<String>,
    #[doc = r"Ethereum chain id. Required for `deploy_token`, `mint`, `burn`, `withdraw`"]
    eth_chain_id: Option<u64>,
    #[doc = r"Ethereum private key. Required for `deploy_token`, `mint`, `burn`"]
    eth_private_key: Option<String>,
    #[doc = r"Bridged token factory address on Ethereum. Required for `deploy_token`, `mint`, `burn`"]
    bridge_token_factory_address: Option<String>,
    #[doc = r"NEAR RPC endpoint. Required for `log_token_metadata`, `storage_deposit_for_token`, `deploy_token`, `deposit`, `mint`, `withdraw`"]
    near_endpoint: Option<String>,
    #[doc = r"NEAR private key. Required for `log_token_metadata`, `storage_deposit_for_token`, `deploy_token`, `deposit`, `withdraw`"]
    near_private_key: Option<String>,
    #[doc = r"NEAR account id of the transaction signer. Required for `log_token_metadata`, `storage_deposit_for_token`, `deploy_token`, `deposit`, `withdraw`"]
    near_signer: Option<String>,
    #[doc = r"Token locker account id on Near. Required for `log_token_metadata`, `storage_deposit_for_token`, `deploy_token`, `deposit`, `mint`, `withdraw`"]
    token_locker_id: Option<String>,
    #[doc = r"NEAR light client address on Ethereum. Required for `deploy_token`, `mint`"]
    near_light_client_address: Option<String>,
}

impl Nep141Bridging {
    /// Creates an empty instance of the bridging client. Property values can be set separately depending on the required use case.
    pub fn new() -> Self {
        Self {
            eth_chain_id: None,
            bridge_token_factory_address: None,
            eth_endpoint: None,
            eth_private_key: None,
            near_endpoint: None,
            near_private_key: None,
            near_signer: None,
            token_locker_id: None,
            near_light_client_address: None,
        }
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
            self.token_locker_id()?.to_string(),
            "log_metadata".to_string(),
            args,
            300_000_000_000_000,
            0
        ).await?)
    }

    /// Performs a storage deposit on behalf of the token_locker so that the tokens can be transferred to the locker. To be called once for each NEP-141
    pub async fn storage_deposit_for_token(&self, near_token_id: String, amount: u128) -> Result<CryptoHash> {
        let near_endpoint = self.near_endpoint()?;
        let token_locker = self.token_locker_id()?.to_string();

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

        let near_on_eth_client = NearOnEthClient::new(self.near_light_client_address()?, eth_endpoint.to_string());

        let proof_block_height = near_on_eth_client.get_sync_height().await?;
        let block_hash = near_on_eth_client.get_block_hash(proof_block_height).await?;

        let receipt_id = TransactionOrReceiptId::Receipt {
            receipt_id,
            receiver_id: AccountId::from_str(&self.token_locker_id()?)
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
        let token_locker = self.token_locker_id()?.to_string();

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

        let near_on_eth_client = NearOnEthClient::new(self.near_light_client_address()?, eth_endpoint.to_string());

        let proof_block_height = near_on_eth_client.get_sync_height().await?;
        let block_hash = near_on_eth_client.get_block_hash(proof_block_height).await?;

        let receipt_id = TransactionOrReceiptId::Receipt {
            receipt_id,
            receiver_id: AccountId::from_str(&self.token_locker_id()?)
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
            self.token_locker_id()?.to_string(),
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
            .ok_or(SdkError::ConfigError("Ethereum rpc endpoint is not set".to_string()))?)
    }

    fn near_endpoint(&self) -> Result<&str> {
        Ok(self.near_endpoint
            .as_ref()
            .ok_or(SdkError::ConfigError("Near rpc endpoint is not set".to_string()))?)
    }

    fn token_locker_id(&self) -> Result<&str> {
        Ok(self.token_locker_id
            .as_ref()
            .ok_or(SdkError::ConfigError("Token locker account id is not set".to_string()))?)
    }

    fn near_light_client_address(&self) -> Result<Address> {
        self.near_light_client_address
            .as_ref()
            .ok_or(SdkError::ConfigError("Near on Eth light client address is not set".to_string()))
            .and_then(|addr| Address::from_str(addr)
                .map_err(|_| SdkError::ConfigError("near_light_client_address is not a valid Ethereum address".to_string()))
            )
    }

    fn bridge_token_factory_address(&self) -> Result<Address> {
        self.bridge_token_factory_address
            .as_ref()
            .ok_or(SdkError::ConfigError("Bridge token factory address is not set".to_string()))
            .and_then(|addr| Address::from_str(addr)
                .map_err(|_| SdkError::ConfigError("bridge_token_factory_address is not a valid Ethereum address".to_string()))
            )
    }

    fn near_signer(&self) -> Result<near_crypto::InMemorySigner> {
        let near_private_key = self.near_private_key
            .as_ref()
            .ok_or(SdkError::ConfigError("Near account private key is not set".to_string()))?;
        let near_signer = self.near_signer
            .as_ref()
            .ok_or(SdkError::ConfigError("Near signer account id is not set".to_string()))?;

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
            .ok_or(SdkError::ConfigError("Ethereum rpc endpoint is not set".to_string()))?;

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
            .ok_or(SdkError::ConfigError("Ethereum rpc endpoint is not set".to_string()))?;

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
            .ok_or(SdkError::ConfigError("Ethereum private key is not set".to_string()))?;

        let eth_chain_id = self.eth_chain_id
            .as_ref()
            .ok_or(SdkError::ConfigError("Ethereum chain id is not set".to_string()))?
            .clone();

        let private_key_bytes = hex::decode(eth_private_key)
            .map_err(|_| SdkError::ConfigError("Ethereum private key is not a valid hex string".to_string()))?;

        if private_key_bytes.len() != 32 {
            return Err(SdkError::ConfigError("Ethereum private key is of invalid length".to_string()));
        }

        Ok(LocalWallet::from_bytes(&private_key_bytes)
            .map_err(|_| SdkError::ConfigError("Invalid ethereum private key".to_string()))?
            .with_chain_id(eth_chain_id))
    }
}
