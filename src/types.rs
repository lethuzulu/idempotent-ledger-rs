use uuid::Uuid;

use crate::error::LedgerError;




pub struct Money(i64);

impl Money {
    pub fn from_cents(cents: i64) -> Result<Self, LedgerError> {
        if cents <= 0 {
            return Err (LedgerError::InvalidAmount)
        }
        Ok(Self(cents))
    }

    pub fn cents(self) -> i64 {
        self.0
    }
}

pub struct AccountId (pub Uuid);
pub struct TranferId(pub Uuid);