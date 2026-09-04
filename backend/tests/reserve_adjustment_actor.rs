//! Integracyjny test administracyjnej korekty rezerwy emitenta.
//!
//! Łączy produkcyjną usługę, klienta HTTP, router mockBanku i trwałą bazę SQLite
//! mockBanku. Nie wymaga uruchomienia osobnego procesu banku.

use crypto_asset_backend::{
    application::{
        AdjustReserve, ReserveAdjustmentDirection, ReserveAdjustmentService,
    },
    infrastructure::mock_bank_client::HttpBankTransactionReader,
    mock_bank::{self, MockBankStore},
};
use std::{
    fs,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::net::TcpListener;

static NEXT_DATABASE: AtomicU64 = AtomicU64::new(0);

async fn start_mock_bank(store: Arc<MockBankStore>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, mock_bank::router(store))
            .await
            .unwrap();
    });
    format!("http://{address}")
}

#[tokio::test]
async fn reserve_adjustment_uses_http_mock_bank_and_persists_idempotent_audit_record() {
    let sequence = NEXT_DATABASE.fetch_add(1, Ordering::SeqCst);
    let directory = std::env::temp_dir().join(format!(
        "rusd-issuer-reserve-adjustment-{}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        sequence
    ));
    fs::create_dir_all(&directory).unwrap();
    let database = directory
        .join("mock-bank.sqlite")
        .to_string_lossy()
        .to_string();
    let store = Arc::new(MockBankStore::open(&database, 10_000).unwrap());
    let base_url = start_mock_bank(store.clone()).await;
    let gateway = Arc::new(HttpBankTransactionReader::new(&base_url));
    let service = ReserveAdjustmentService::new(gateway);
    let command = AdjustReserve {
        operation_id: "admin-reserve-deposit-1".into(),
        direction: ReserveAdjustmentDirection::Deposit,
        amount_usd: "12.30".into(),
        reason: "uzupełnienie demonstracyjnej rezerwy".into(),
    };

    let first = service.execute(command.clone()).await.unwrap();
    let replay = service.execute(command).await.unwrap();

    assert_eq!(first.balance_minor, "11230");
    assert_eq!(replay.balance_minor, "11230");
    let transaction = store
        .transaction("issuer-admin-reserve-admin-reserve-deposit-1")
        .unwrap();
    assert_eq!(transaction.operation_type, "deposit");
    assert_eq!(transaction.amount_minor, "1230");
    assert_eq!(
        transaction.reference,
        "uzupełnienie demonstracyjnej rezerwy"
    );
    assert_eq!(store.reserve().unwrap().balance_minor, "11230");
}
