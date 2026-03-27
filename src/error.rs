use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum LedgerError {
    #[error("invalid amount: must be a positive number of cents")]
    InvalidAmount,

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("account not found: {0}")]
    AccountNotFound(Uuid),

    #[error("insufficient funds in account {account_id}")]
    InsufficientFunds { account_id: Uuid },

    #[error("invalid idempotency key: must be 1-255 characters")]
    InvalidIdempotencyKey,

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
