use crate::application::{
    IssuanceError, MintResult, RedemptionError, RedemptionToken, TokenIssuer, TokenLifecycle,
    WindDownError,
};
use alloy::{
    primitives::{Address, U256, keccak256},
    providers::{DynProvider, Provider, ProviderBuilder},
    signers::local::PrivateKeySigner,
    sol,
};
use async_trait::async_trait;

sol! {
    #[sol(rpc)]
    interface IssuanceToken {
        function mintForOperation(bytes32 operationId, address recipient, uint256 amount) external;
        function isMintOperationProcessed(bytes32 operationId) external view returns (bool);
        function burnForOperation(bytes32 operationId, address holder, uint256 amount) external;
        function isBurnOperationProcessed(bytes32 operationId) external view returns (bool);
        function enterWindDown() external;
        function windDown() external view returns (bool);
    }
}
#[async_trait]
impl RedemptionToken for AlloyTokenIssuer {
    async fn burn_for_operation(
        &self,
        id: &str,
        holder: Address,
        amount: u64,
    ) -> Result<Option<String>, RedemptionError> {
        let op = keccak256(id.as_bytes());
        let contract = IssuanceToken::new(self.token_address, &self.provider);
        if contract
            .isBurnOperationProcessed(op)
            .call()
            .await
            .map_err(redemption_chain)?
        {
            return Ok(None);
        }
        let receipt = contract
            .burnForOperation(op, holder, U256::from(amount))
            .send()
            .await
            .map_err(redemption_chain)?
            .get_receipt()
            .await
            .map_err(redemption_chain)?;
        Ok(Some(receipt.transaction_hash.to_string()))
    }
}

pub struct AlloyTokenIssuer {
    provider: DynProvider,
    token_address: Address,
}
impl AlloyTokenIssuer {
    pub async fn connect(
        rpc_url: &str,
        token_address: Address,
        private_key: &str,
    ) -> Result<Self, IssuanceError> {
        let signer: PrivateKeySigner = private_key
            .parse()
            .map_err(|e| IssuanceError::Blockchain(format!("invalid ISSUER_PRIVATE_KEY: {e}")))?;
        let provider = ProviderBuilder::new()
            .wallet(signer)
            .connect(rpc_url)
            .await
            .map_err(chain)?
            .erased();
        Ok(Self {
            provider,
            token_address,
        })
    }
}
#[async_trait]
impl TokenIssuer for AlloyTokenIssuer {
    async fn mint_for_operation(
        &self,
        operation_id: &str,
        recipient: Address,
        amount_raw: u64,
    ) -> Result<MintResult, IssuanceError> {
        let id = keccak256(operation_id.as_bytes());
        let contract = IssuanceToken::new(self.token_address, &self.provider);
        if contract
            .isMintOperationProcessed(id)
            .call()
            .await
            .map_err(chain)?
        {
            return Ok(MintResult {
                transaction_hash: None,
            });
        }
        let pending = contract
            .mintForOperation(id, recipient, U256::from(amount_raw))
            .send()
            .await
            .map_err(chain)?;
        let receipt = pending.get_receipt().await.map_err(chain)?;
        Ok(MintResult {
            transaction_hash: Some(receipt.transaction_hash.to_string()),
        })
    }
}

#[async_trait]
impl TokenLifecycle for AlloyTokenIssuer {
    async fn enter_wind_down(&self) -> Result<Option<String>, WindDownError> {
        let contract = IssuanceToken::new(self.token_address, &self.provider);
        if contract.windDown().call().await.map_err(wind_down_chain)? {
            return Ok(None);
        }
        let receipt = contract
            .enterWindDown()
            .send()
            .await
            .map_err(wind_down_chain)?
            .get_receipt()
            .await
            .map_err(wind_down_chain)?;
        Ok(Some(receipt.transaction_hash.to_string()))
    }
}
fn chain(error: impl std::fmt::Display) -> IssuanceError {
    IssuanceError::Blockchain(error.to_string())
}
fn redemption_chain(error: impl std::fmt::Display) -> RedemptionError {
    RedemptionError::Blockchain(error.to_string())
}
fn wind_down_chain(error: impl std::fmt::Display) -> WindDownError {
    WindDownError::Blockchain(error.to_string())
}
