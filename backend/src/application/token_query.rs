use super::{TokenReadError, TokenReader};
use crate::domain::TokenSnapshot;
use std::sync::Arc;

/// Application use case for retrieving the current token state.
#[derive(Clone)]
pub struct TokenQueryService {
    reader: Arc<dyn TokenReader>,
}

impl TokenQueryService {
    pub fn new(reader: Arc<dyn TokenReader>) -> Self {
        Self { reader }
    }
    pub async fn get_snapshot(&self) -> Result<TokenSnapshot, TokenReadError> {
        self.reader.read_snapshot().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct StubTokenReader;
    #[async_trait]
    impl TokenReader for StubTokenReader {
        async fn read_snapshot(&self) -> Result<TokenSnapshot, TokenReadError> {
            Ok(TokenSnapshot {
                contract_address: "0x1234".to_owned(),
                name: "Research Euro EMT".to_owned(),
                symbol: "rEUR".to_owned(),
                decimals: 6,
                total_supply_raw: "1000000".to_owned(),
            })
        }
    }

    #[tokio::test]
    async fn returns_snapshot_from_reader() {
        let service = TokenQueryService::new(Arc::new(StubTokenReader));
        let snapshot = service.get_snapshot().await.expect("snapshot should load");
        assert_eq!(snapshot.symbol, "rEUR");
        assert_eq!(snapshot.total_supply_raw, "1000000");
    }
}
