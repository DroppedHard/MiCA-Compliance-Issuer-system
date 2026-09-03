use crate::{
    application::{TokenReadError, TokenReader},
    domain::TokenSnapshot,
};
use alloy::sol_types::SolEvent;
use alloy::{
    primitives::Address,
    providers::{DynProvider, Provider, ProviderBuilder},
    rpc::types::Filter,
    sol,
};
use async_trait::async_trait;

sol! {
    #[sol(rpc)]
    interface ResearchUsdEMT {
        function name() external view returns (string);
        function symbol() external view returns (string);
        function decimals() external view returns (uint8);
        function totalSupply() external view returns (uint256);
        event Transfer(address indexed from, address indexed to, uint256 value);
    }
}

/// Alloy-based adapter for read-only calls to the deployed token contract.
pub struct AlloyTokenReader {
    provider: DynProvider,
    token_address: Address,
}

impl AlloyTokenReader {
    pub async fn connect(rpc_url: &str, token_address: Address) -> Result<Self, TokenReadError> {
        let provider = ProviderBuilder::new()
            .connect(rpc_url)
            .await
            .map_err(|error| TokenReadError::Rpc(error.to_string()))?
            .erased();
        Ok(Self {
            provider,
            token_address,
        })
    }
}

#[async_trait]
impl TokenReader for AlloyTokenReader {
    async fn read_snapshot(&self) -> Result<TokenSnapshot, TokenReadError> {
        let chain_id = self.provider.get_chain_id().await.map_err(rpc_error)?;
        let block_number = self.provider.get_block_number().await.map_err(rpc_error)?;
        let contract = ResearchUsdEMT::new(self.token_address, &self.provider);
        let name = contract.name().call().await.map_err(rpc_error)?;
        let symbol = contract.symbol().call().await.map_err(rpc_error)?;
        let decimals = contract.decimals().call().await.map_err(rpc_error)?;
        let total_supply = contract.totalSupply().call().await.map_err(rpc_error)?;
        Ok(TokenSnapshot {
            chain_id,
            block_number,
            contract_address: self.token_address.to_checksum(None),
            name,
            symbol,
            decimals,
            total_supply_raw: total_supply.to_string(),
        })
    }

    async fn count_transfer_transactions(
        &self,
        from_block: u64,
        to_block: u64,
    ) -> Result<u64, TokenReadError> {
        if from_block > to_block {
            return Ok(0);
        }
        let filter = Filter::new()
            .address(self.token_address)
            .event_signature(ResearchUsdEMT::Transfer::SIGNATURE_HASH)
            .from_block(from_block)
            .to_block(to_block);
        let logs = self.provider.get_logs(&filter).await.map_err(rpc_error)?;
        Ok(count_unique_transaction_hashes(
            logs.into_iter()
                .filter(|log| !log.removed)
                .map(|log| log.transaction_hash),
        ))
    }
}

fn count_unique_transaction_hashes(
    hashes: impl IntoIterator<Item = Option<alloy::primitives::B256>>,
) -> u64 {
    hashes
        .into_iter()
        .flatten()
        .collect::<std::collections::HashSet<_>>()
        .len() as u64
}

fn rpc_error(error: impl std::fmt::Display) -> TokenReadError {
    TokenReadError::Rpc(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_each_transaction_once_and_ignores_logs_without_a_hash() {
        let first = alloy::primitives::B256::repeat_byte(1);
        let second = alloy::primitives::B256::repeat_byte(2);
        assert_eq!(
            count_unique_transaction_hashes([Some(first), Some(first), None, Some(second)]),
            2
        );
    }
}
