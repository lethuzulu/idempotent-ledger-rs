use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};
use idempotent_ledger_rs::types::AccountId;
use idempotent_ledger_rs::{
    db::Db,
    error::LedgerError,
    ledger::LedgerService,
    types::{TransferRequest, TransferResult},
};
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // load .env if present
    dotenvy::dotenv().ok();

    // structured JSON logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let db = Db::new(&database_url).await?;
    db.migrate().await?;

    let ledger = LedgerService::new(db);

    let app = Router::new()
        .route("/transfers", post(transfer_handler))
        .route("/accounts/:id/balance", get(balance_handler))
        .with_state(ledger);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!(%addr, "ledger listening");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn transfer_handler(
    State(ledger): State<LedgerService>,
    Json(req): Json<TransferRequest>,
) -> Result<Json<TransferResult>, LedgerError> {
    let result = ledger.transfer(req).await?;
    Ok(Json(result))
}

async fn balance_handler(
    State(ledger): State<LedgerService>,
    axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
) -> Result<Json<serde_json::Value>, LedgerError> {
    let balance = ledger.get_balance(AccountId(id)).await?;
    Ok(Json(
        serde_json::json!({ "account_id": id, "balance_cents": balance }),
    ))
}
