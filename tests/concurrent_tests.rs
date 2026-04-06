use idempotent_ledger_rs::{
    error::LedgerError,
    ledger::LedgerService,
    types::{AccountId, IdempotencyKey, Money, TransferId, TransferRequest},
};
use tokio::task::JoinSet;
use uuid::Uuid;

async fn setup() -> (LedgerService, AccountId, AccountId) {
    dotenvy::dotenv().ok();
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL required for tests");
    let db = idempotent_ledger_rs::db::Db::new(&url).await.unwrap();
    db.migrate().await.unwrap();

    // create two test accounts with known starting balances
    let alice = sqlx::query_scalar!(
        "INSERT INTO accounts (owner, balance) VALUES ('alice', 10000) RETURNING id"
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();

    let bob =
        sqlx::query_scalar!("INSERT INTO accounts (owner, balance) VALUES ('bob', 0) RETURNING id")
            .fetch_one(&db.pool)
            .await
            .unwrap();

    (LedgerService::new(db), AccountId(alice), AccountId(bob))
}

#[tokio::test]
async fn same_key_concurrent_produces_one_transfer() {
    let (ledger, alice, bob) = setup().await;

    let key = IdempotencyKey::new(format!("test-{}", Uuid::new_v4())).unwrap();

    let mut set = JoinSet::new();
    for _ in 0..10 {
        let ledger = ledger.clone();
        let key = key.clone();
        set.spawn(async move {
            ledger
                .transfer(TransferRequest {
                    idempotency_key: key,
                    from_account: alice,
                    to_account: bob,
                    amount: Money::from_cents(1000).unwrap(),
                    transfer_id: TransferId(Uuid::new_v4()),
                })
                .await
        });
    }

    let mut results = Vec::new();
    while let Some(r) = set.join_next().await {
        results.push(r);
    }

    // all 10 calls should succeed - the idempotent ones return cached
    for r in &results {
        assert!(r.is_ok(), "task panicked: {:?}", r);
        assert!(r.as_ref().unwrap().is_ok(), "transfer failed: {:?}", r);
    }

    // all results should reference the same transfer_id
    let ids: std::collections::HashSet<_> = results
        .iter()
        .filter_map(|r| r.as_ref().ok())
        .filter_map(|r| r.as_ref().ok())
        .map(|t| t.transfer_id)
        .collect();
    assert_eq!(ids.len(), 1, "expected exactly one unique transfer_id");

    // alice should have lost exactly 1000 cents - not 10 000
    let alice_balance = ledger.get_balance(alice).await.unwrap();
    assert_eq!(alice_balance, 9000, "balance should be 10000 - 1000 = 9000");
}

// Test 2 - insufficient funds
// A transfer for more than the account balance should fail with
// InsufficientFunds, and the balance should be unchanged.

#[tokio::test]
async fn transfer_exceeding_balance_is_rejected() {
    let (ledger, alice, bob) = setup().await;

    let result = ledger
        .transfer(TransferRequest {
            idempotency_key: IdempotencyKey::new(format!("test-{}", Uuid::new_v4())).unwrap(),
            from_account: alice,
            to_account: bob,
            amount: Money::from_cents(99_999).unwrap(), // more than 10 000
            transfer_id: TransferId(Uuid::new_v4()),
        })
        .await;

    assert!(
        matches!(result, Err(LedgerError::InsufficientFunds { .. })),
        "expected InsufficientFunds, got {:?}",
        result
    );

    // balance must be unchanged
    let balance = ledger.get_balance(alice).await.unwrap();
    assert_eq!(balance, 10_000);
}

// Test 3 - balance invariant across N sequential transfers
// Run 5 sequential transfers of 1000 cents each.
// Total moved = 5000. Alice: 5000 remaining. Bob: 5000.

#[tokio::test]
async fn sequential_transfers_maintain_balance_invariant() {
    let (ledger, alice, bob) = setup().await;

    for i in 0..5 {
        ledger
            .transfer(TransferRequest {
                idempotency_key: IdempotencyKey::new(format!("seq-{}", i)).unwrap(),
                from_account: alice,
                to_account: bob,
                amount: Money::from_cents(1000).unwrap(),
                transfer_id: TransferId(Uuid::new_v4()),
            })
            .await
            .unwrap();
    }

    let alice_balance = ledger.get_balance(alice).await.unwrap();
    let bob_balance = ledger.get_balance(bob).await.unwrap();

    assert_eq!(alice_balance, 5_000);
    assert_eq!(bob_balance, 5_000);
    // the system is closed: total balance never changes
    assert_eq!(alice_balance + bob_balance, 10_000);
}
