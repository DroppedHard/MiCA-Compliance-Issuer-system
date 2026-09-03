use crate::application::{
    ActivityIssuanceController, AddressRestrictionChain, AddressRestrictionError,
    CaspReportingError, IssuanceError, MintResult, RedemptionError, RedemptionToken, ReserveError,
    ReserveStateController, TokenIssuer, TokenLifecycle, WindDownError,
};
use crate::domain::{AssetState, AssetStateCode, QuarterlyTransactionAssessment};
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
        function blockIssuance(bytes32 evidenceHash) external;
        function unblockIssuance(bytes32 evidenceHash) external;
        function issuanceBlocked() external view returns (bool);
        function issuanceBlockEvidence() external view returns (bytes32);
        function setReserveState(uint8 newState, bytes32 evidenceHash) external;
        function reserveState() external view returns (uint8);
        function freeze(address account) external;
        function unfreeze(address account) external;
        function isFrozen(address account) external view returns (bool);
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
impl AddressRestrictionChain for AlloyTokenIssuer {
    async fn set_frozen(
        &self,
        address: Address,
        frozen: bool,
    ) -> Result<Option<String>, AddressRestrictionError> {
        let contract = IssuanceToken::new(self.token_address, &self.provider);
        let current = contract
            .isFrozen(address)
            .call()
            .await
            .map_err(restriction_chain)?;
        if current == frozen {
            return Ok(None);
        }
        let pending = if frozen {
            contract.freeze(address).send().await
        } else {
            contract.unfreeze(address).send().await
        }
        .map_err(restriction_chain)?;
        let receipt = pending.get_receipt().await.map_err(restriction_chain)?;
        Ok(Some(receipt.transaction_hash.to_string()))
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
#[async_trait]
impl ActivityIssuanceController for AlloyTokenIssuer {
    async fn synchronize_issuance_restriction(
        &self,
        assessment: &QuarterlyTransactionAssessment,
    ) -> Result<(), CaspReportingError> {
        let contract = IssuanceToken::new(self.token_address, &self.provider);
        let current = contract
            .issuanceBlocked()
            .call()
            .await
            .map_err(enforcement_chain)?;
        let desired = assessment.threshold_enforceable;
        let evidence = serde_json::to_vec(assessment)
            .map_err(|error| CaspReportingError::Enforcement(error.to_string()))?;
        let evidence_hash = keccak256(evidence);
        let current_evidence = contract
            .issuanceBlockEvidence()
            .call()
            .await
            .map_err(enforcement_chain)?;
        if current == desired && current_evidence == evidence_hash {
            return Ok(());
        }
        if desired {
            contract.blockIssuance(evidence_hash).send().await
        } else {
            contract.unblockIssuance(evidence_hash).send().await
        }
        .map_err(enforcement_chain)?
        .get_receipt()
        .await
        .map_err(enforcement_chain)?;
        Ok(())
    }
}
#[async_trait]
impl ReserveStateController for AlloyTokenIssuer {
    async fn synchronize_reserve_state(&self, state: &AssetState) -> Result<(), ReserveError> {
        if state.state == AssetStateCode::WindDown {
            return Ok(());
        }
        let desired = contract_reserve_state(state.state);
        let contract = IssuanceToken::new(self.token_address, &self.provider);
        if contract
            .reserveState()
            .call()
            .await
            .map_err(reserve_chain)?
            == desired
        {
            return Ok(());
        }
        let evidence = serde_json::to_vec(state)
            .map_err(|error| ReserveError::Blockchain(error.to_string()))?;
        contract
            .setReserveState(desired, keccak256(evidence))
            .send()
            .await
            .map_err(reserve_chain)?
            .get_receipt()
            .await
            .map_err(reserve_chain)?;
        Ok(())
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
fn enforcement_chain(error: impl std::fmt::Display) -> CaspReportingError {
    CaspReportingError::Enforcement(error.to_string())
}
fn reserve_chain(error: impl std::fmt::Display) -> ReserveError {
    ReserveError::Blockchain(error.to_string())
}
fn restriction_chain(error: impl std::fmt::Display) -> AddressRestrictionError {
    AddressRestrictionError::Blockchain(error.to_string())
}

fn contract_reserve_state(state: AssetStateCode) -> u8 {
    match state {
        AssetStateCode::Active => 0,
        AssetStateCode::Warning => 1,
        AssetStateCode::MintBlocked | AssetStateCode::DataUnavailable => 2,
        AssetStateCode::WindDown => unreachable!("wind-down is synchronized by TokenLifecycle"),
    }
}

#[cfg(test)]
mod reserve_state_tests {
    use super::*;

    #[test]
    fn maps_missing_evidence_to_fail_closed_contract_state() {
        assert_eq!(contract_reserve_state(AssetStateCode::Active), 0);
        assert_eq!(contract_reserve_state(AssetStateCode::Warning), 1);
        assert_eq!(contract_reserve_state(AssetStateCode::MintBlocked), 2);
        assert_eq!(contract_reserve_state(AssetStateCode::DataUnavailable), 2);
    }
}
