use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum LedgerError {
    #[error("insufficient funds in account {account_id}")]
    InsufficientFunds { account_id: Uuid },

    #[error("duplicate idempotency key - request already processed")]
    DuplicateKey,

    #[error("invalid amount: must be a positive number of cents")]
    InvalidAmount,

    #[error("invalid idempotency key: must be 1 to 255 characters")]
    InvalidIdempotencyKey,

    #[error("account not found: {0}")]
    AccountNotFound(Uuid),

    #[error("transfer not found: {0}")]
    TransferNotFound(Uuid),

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl IntoResponse for LedgerError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            LedgerError::InsufficientFunds { .. } => {
                (StatusCode::UNPROCESSABLE_ENTITY, self.to_string())
            }
            LedgerError::DuplicateKey => {
                // 200 rather than 4xx — idempotent replay is not an error
                return (StatusCode::OK, Json(json!({ "replayed": true }))).into_response();
            }
            LedgerError::InvalidAmount | LedgerError::InvalidIdempotencyKey => {
                (StatusCode::BAD_REQUEST, self.to_string())
            }
            LedgerError::AccountNotFound(_) | LedgerError::TransferNotFound(_) => {
                (StatusCode::NOT_FOUND, self.to_string())
            }
            LedgerError::Database(_) | LedgerError::Serialization(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal server error".to_string(),
            ),
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}
