//! Integracyjne testy blokowania adresów przez emitenta.

use alloy::primitives::Address;
use async_trait::async_trait;
use crypto_asset_backend::{
    application::{AddressRestrictionChain, AddressRestrictionError, AddressRestrictionService},
    infrastructure::address_restriction_sqlite::SqliteAddressRestrictionStore,
};
use std::sync::{Arc, Mutex};

const ADDRESS: &str = "0x0000000000000000000000000000000000000007";

struct TestChain {
    calls: Mutex<Vec<(Address, bool)>>,
    fail: bool,
}

#[async_trait]
impl AddressRestrictionChain for TestChain {
    async fn set_frozen(
        &self,
        address: Address,
        frozen: bool,
    ) -> Result<Option<String>, AddressRestrictionError> {
        self.calls.lock().unwrap().push((address, frozen));
        if self.fail {
            Err(AddressRestrictionError::Blockchain("revert".into()))
        } else {
            Ok(Some(if frozen { "0xfreeze" } else { "0xunfreeze" }.into()))
        }
    }
}

#[tokio::test]
async fn block_and_unblock_are_sent_to_chain_and_persist_current_audit_state() {
    let store = Arc::new(SqliteAddressRestrictionStore::open(":memory:").unwrap());
    let chain = Arc::new(TestChain {
        calls: Mutex::new(Vec::new()),
        fail: false,
    });
    let service = AddressRestrictionService::new(store, chain.clone());

    let blocked = service.block(ADDRESS, "polecenie organu").await.unwrap();
    let unblocked = service.unblock(ADDRESS).await.unwrap();

    assert!(blocked.active);
    assert_eq!(blocked.transaction_hash.as_deref(), Some("0xfreeze"));
    assert!(!unblocked.active);
    assert_eq!(unblocked.transaction_hash.as_deref(), Some("0xunfreeze"));
    assert_eq!(
        chain.calls.lock().unwrap().as_slice(),
        &[
            (ADDRESS.parse().unwrap(), true),
            (ADDRESS.parse().unwrap(), false),
        ]
    );
    let persisted = service.list().unwrap();
    assert_eq!(persisted.len(), 1);
    assert!(!persisted[0].active);
    assert_eq!(
        persisted[0].reason,
        "Blokada usunięta przez administratora emitenta"
    );
}

#[tokio::test]
async fn rejected_chain_command_does_not_publish_a_false_restriction() {
    let store = Arc::new(SqliteAddressRestrictionStore::open(":memory:").unwrap());
    let chain = Arc::new(TestChain {
        calls: Mutex::new(Vec::new()),
        fail: true,
    });
    let service = AddressRestrictionService::new(store, chain);

    let result = service.block(ADDRESS, "polecenie organu").await;

    assert!(matches!(
        result,
        Err(AddressRestrictionError::Blockchain(_))
    ));
    assert!(service.list().unwrap().is_empty());
}
