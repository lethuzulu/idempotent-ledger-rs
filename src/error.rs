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

}
