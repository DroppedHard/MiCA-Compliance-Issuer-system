use crate::{
    application::{TokenReadError, TokenReader},
    domain::TokenSnapshot,
};
use alloy::{
    primitives::Address,
    providers::{DynProvider, Provider, ProviderBuilder},
    sol,
};
use async_trait::async_trait;

sol! {
    #[sol(rpc)]
    interface ResearchEuroEMT {
        function name() external view returns (string);
        function symbol() external view returns (string);
        function decimals() external view returns (uint8);
        function totalSupply() external view returns (uint256);
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
        let contract = ResearchEuroEMT::new(self.token_address, &self.provider);
        let name = contract.name().call().await.map_err(rpc_error)?;
        let symbol = contract.symbol().call().await.map_err(rpc_error)?;
        let decimals = contract.decimals().call().await.map_err(rpc_error)?;
        let total_supply = contract.totalSupply().call().await.map_err(rpc_error)?;
        Ok(TokenSnapshot {
            contract_address: self.token_address.to_checksum(None),
            name,
            symbol,
            decimals,
            total_supply_raw: total_supply.to_string(),
        })
    }
}

fn rpc_error(error: impl std::fmt::Display) -> TokenReadError {
    TokenReadError::Rpc(error.to_string())
}
