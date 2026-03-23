use serde::Deserialize;
use uuid::Uuid;

use crate::error::LedgerError;

#[derive(Debug, Deserialize)]
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
    pub from_account: AccountId,
    pub to_account: AccountId,
    pub amount: Money,
    #[serde(default = "default_transfer_id")]
    pub transfer_id: TransferId,
}
fn default_transfer_id() -> TransferId {
    TransferId(Uuid::new_v4())
}

pub struct TransferResult {
    pub from_account: AccountId,
    pub to_account: AccountId,
    pub amount: Money,
}

#[derive(Debug, Deserialize)]
pub struct AccountId(pub Uuid);

#[derive(Debug, Deserialize)]
pub struct TransferId(pub Uuid);
