use std::{str::FromStr, sync::Arc};
use borsh::BorshSerialize;
use near_crypto::SecretKey;
use near_light_client_on_eth::NearOnEthClient;
use near_primitives::{hash::CryptoHash, types::{AccountId, TransactionOrReceiptId}};
use ethers::{abi::Address, prelude::*};
use crate::result::{EthConnectorError, EthConnectorResult};

abigen!(
    EthCustodian,
    r#"[
      function depositToEVM(string memory ethRecipientOnNear, uint256 fee) payable
      function depositToNear(string memory nearRecipientAccountId, uint256 fee) payable
      function withdraw(bytes calldata proofData, uint64 proofBlockHeight)
    ]"#
);

#[derive(BorshSerialize)]
pub struct WithdrawArgs {
    pub recipient_address: [u8; 20],
    pub amount: u128
}

/// Bridging ETH from Ethereum to Near and back
#[derive(Builder)]
pub struct EthConnector {
    #[doc = r"Ethereum RPC endpoint. Required for "]
    eth_endpoint: Option<String>,
    #[doc = r"Ethereum chain id. Required for "]
    eth_chain_id: Option<u64>,
    #[doc = r"Ethereum private key. Required for "]
    eth_private_key: Option<String>,
    #[doc = r"EthCustodian address on Ethereum. Required for "]
    eth_custodian_address: Option<String>,
    #[doc = r"NEAR RPC endpoint. Required for "]
    near_endpoint: Option<String>,
    #[doc = r"NEAR private key. Required for "]
    near_private_key: Option<String>,
    #[doc = r"NEAR account id of the transaction signer. Required for "]
    near_signer: Option<String>,
    #[doc = r"Eth connector account id on Near. Required for "]
    eth_connector_account_id: Option<String>,
    #[doc = r"NEAR light client address on Ethereum. Required for "]
    near_light_client_address: Option<String>,
}

impl EthConnector {
    /// Transfers ETH to the EthCustodian and sets recipient as a Near account. A proof from this transaction is then used to mint nETH on Near
    pub async fn deposit_to_near(&self, amount: u128, recipient_account_id: String) -> EthConnectorResult<TxHash> {
        let eth_custodian = self.eth_custodian()?;
        let call = eth_custodian.deposit_to_near(recipient_account_id, U256::zero())
            .value(amount);

        let tx = call.send().await?;
        Ok(tx.tx_hash())
    }

    /// Transfers ETH to the EthCustodian and sets recipient as an Aurora EVM account. A proof from this transaction is then used to mint nETH on Aurora
    pub async fn deposit_to_evm(&self, amount: u128, recipient_address: String) -> EthConnectorResult<TxHash> {
        let eth_custodian = self.eth_custodian()?;
        let call = eth_custodian.deposit_to_evm(recipient_address, U256::zero())
            .value(amount);

        let tx = call.send().await?;
        Ok(tx.tx_hash())
    }

    /// Generates a proof of the deposit transaction and uses it to mint nETH either on Near or Aurora, depending on the recipient field of the deposit transaction
    pub async fn finalize_deposit(&self, tx_hash: TxHash, log_index: u64) -> EthConnectorResult<CryptoHash> {
        let eth_endpoint = self.eth_endpoint()?;
        let near_endpoint = self.near_endpoint()?;

        let proof = eth_proof::get_proof_for_event(tx_hash, log_index, eth_endpoint)
            .await?;

        let mut args = Vec::new();
        proof.serialize(&mut args)
            .map_err(|_| EthConnectorError::EthProofError("Failed to serialize proof".to_string()))?;

        let tx_hash = near_rpc_client::change(
            near_endpoint,
            self.near_signer()?,
            self.eth_connector_account_id()?.to_string(),
            "deposit".to_string(),
            args,
            300_000_000_000_000,
            0
        ).await?;
        
        Ok(tx_hash)
    }

    /// Burns nNEAR on Near. A proof of this transaction is then used to unlock ETH on Ethereum
    pub async fn withdraw(&self, amount: u128, recipient_address: Address) -> EthConnectorResult<CryptoHash> {
        let near_endpoint = self.near_endpoint()?;
        let eth_connector_account_id = self.eth_connector_account_id()?.to_string();

        let mut args = Vec::new();
        let args_struct = WithdrawArgs {
            recipient_address: recipient_address.to_fixed_bytes(),
            amount
        };
        args_struct.serialize(&mut args)
            .map_err(|_| EthConnectorError::UnknownError)?;

        let tx_hash = near_rpc_client::change(
            near_endpoint,
            self.near_signer()?,
            eth_connector_account_id,
            "withdraw".to_string(),
            args,
            300_000_000_000_000,
            1
        ).await?;
        
        Ok(tx_hash)
    }

    /// Generates a proof of the withdraw transaction and uses it to unlock ETH on Ethereum
    pub async fn finalize_withdraw(&self, receipt_id: CryptoHash) -> EthConnectorResult<TxHash> {
        let eth_endpoint = self.eth_endpoint()?;
        let near_endpoint = self.near_endpoint()?;

        let near_on_eth_client = NearOnEthClient::new(self.near_light_client_address()?, eth_endpoint.to_string());

        let proof_block_height = near_on_eth_client.get_sync_height().await?;
        let block_hash = near_on_eth_client.get_block_hash(proof_block_height).await?;

        let receipt_id = TransactionOrReceiptId::Receipt {
            receipt_id,
            receiver_id: AccountId::from_str(&self.eth_connector_account_id()?)
                .map_err(|_| EthConnectorError::ConfigError("Invalid ETH connector account id".to_string()))?
        };

        let proof_data = near_rpc_client::get_light_client_proof(
            near_endpoint,
            receipt_id,
            CryptoHash(block_hash)
        ).await?;

        let mut buffer: Vec<u8> = Vec::new();
        proof_data.serialize(&mut buffer)
            .map_err(|_| EthConnectorError::NearProofError("Falied to deserialize proof".to_string()))?;
            
        let eth_custodian = self.eth_custodian()?;
        let call = eth_custodian.withdraw(buffer.into(), proof_block_height);

        let tx = call.send().await?;
        Ok(tx.tx_hash())
    }

    fn near_signer(&self) -> EthConnectorResult<near_crypto::InMemorySigner> {
        let near_private_key = self.near_private_key
            .as_ref()
            .ok_or(EthConnectorError::ConfigError("Near account private key is not set".to_string()))?;
        let near_signer = self.near_signer
            .as_ref()
            .ok_or(EthConnectorError::ConfigError("Near signer account id is not set".to_string()))?;

        Ok(near_crypto::InMemorySigner::from_secret_key(
            AccountId::from_str(near_signer)
                .map_err(|_| EthConnectorError::ConfigError("Invalid near signer account id".to_string()))?,
            SecretKey::from_str(near_private_key)
                .map_err(|_| EthConnectorError::ConfigError("Invalid near private key".to_string()))?
        ))
    }

    fn eth_custodian(&self) -> EthConnectorResult<EthCustodian<SignerMiddleware<Provider<Http>,LocalWallet>>> {
        let eth_provider = Provider::<Http>::try_from(self.eth_endpoint()?)
            .map_err(|_| EthConnectorError::ConfigError("Invalid ethereum rpc endpoint url".to_string()))?;

        let wallet = self.eth_signer()?;

        let signer = SignerMiddleware::new(eth_provider, wallet);
        let client = Arc::new(signer);

        Ok(EthCustodian::new(
            self.eth_custodian_address()?,
            client
        ))
    }

    fn eth_signer(&self) -> EthConnectorResult<LocalWallet> {
        let eth_private_key = self.eth_private_key
            .as_ref()
            .ok_or(EthConnectorError::ConfigError("Ethereum private key is not set".to_string()))?;

        let eth_chain_id = self.eth_chain_id
            .as_ref()
            .ok_or(EthConnectorError::ConfigError("Ethereum chain id is not set".to_string()))?
            .clone();

        let private_key_bytes = hex::decode(eth_private_key)
            .map_err(|_| EthConnectorError::ConfigError("Ethereum private key is not a valid hex string".to_string()))?;

        if private_key_bytes.len() != 32 {
            return Err(EthConnectorError::ConfigError("Ethereum private key is of invalid length".to_string()));
        }

        Ok(LocalWallet::from_bytes(&private_key_bytes)
            .map_err(|_| EthConnectorError::ConfigError("Invalid ethereum private key".to_string()))?
            .with_chain_id(eth_chain_id))
    }

    fn near_light_client_address(&self) -> EthConnectorResult<Address> {
        self.near_light_client_address
            .as_ref()
            .ok_or(EthConnectorError::ConfigError("Near on Eth light client address is not set".to_string()))
            .and_then(|addr| Address::from_str(addr)
                .map_err(|_| EthConnectorError::ConfigError("near_light_client_address is not a valid Ethereum address".to_string()))
            )
    }

    fn eth_connector_account_id(&self) -> EthConnectorResult<&str> {
        Ok(self.eth_connector_account_id
            .as_ref()
            .ok_or(EthConnectorError::ConfigError("Token locker account id is not set".to_string()))?)
    }

    fn eth_custodian_address(&self) -> EthConnectorResult<Address> {
        self.eth_custodian_address
            .as_ref()
            .ok_or(EthConnectorError::ConfigError("EthCustodian address is not set".to_string()))?
            .parse()
            .map_err(|_| EthConnectorError::ConfigError("eth_custodian_address is not a valid Ethereum address".to_string()))
    }

    fn eth_endpoint(&self) -> EthConnectorResult<&str> {
        Ok(self.eth_endpoint
            .as_ref()
            .ok_or(EthConnectorError::ConfigError("Ethereum rpc endpoint is not set".to_string()))?)
    }

    fn near_endpoint(&self) -> EthConnectorResult<&str> {
        Ok(self.near_endpoint
            .as_ref()
            .ok_or(EthConnectorError::ConfigError("Near rpc endpoint is not set".to_string()))?)
    }
}