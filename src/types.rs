use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::LedgerError;

#[derive(Debug, Deserialize, Clone)]
#[serde(transparent)]
pub struct IdempotencyKey(String);

impl IdempotencyKey {
    pub fn new(s: impl Into<String>) -> Result<Self, LedgerError> {
        let s = s.into();
        if s.is_empty() || s.len() > 255 {
            return Err(LedgerError::InvalidIdempotencyKey);
        }
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for IdempotencyKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug,Clone, Copy, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Money(i64);

impl Money {
    pub fn from_cents(cents: i64) -> Result<Self, LedgerError> {
        if cents <= 0 {
            return Err(LedgerError::InvalidAmount);
        }
        Ok(Self(cents))
    }

    pub fn cents(&self) -> i64 {
        self.0
    }
}

#[derive(Debug, Deserialize)]
pub struct TransferRequest {
    pub idempotency_key: IdempotencyKey,
    pub from_account: AccountId,
    pub to_account: AccountId,
    pub amount: Money,
    #[serde(default = "default_transfer_id")]
    pub transfer_id: TransferId,
}
fn default_transfer_id() -> TransferId {
    TransferId(Uuid::new_v4())
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TransferResult {
    pub from_account: AccountId,
    pub to_account: AccountId,
    pub amount: Money,
    pub transfer_id: TransferId,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Hash, PartialEq, Eq)]
#[serde(transparent)]
pub struct AccountId(pub Uuid);

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Hash, PartialEq, Eq)]
#[serde(transparent)]
pub struct TransferId(pub Uuid);
