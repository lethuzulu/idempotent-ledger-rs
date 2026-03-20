use sqlx::{PgPool, postgres::PgPoolOptions};
use uuid::Uuid;

use crate::{error::LedgerError, types::Money};

#[derive(Debug)]
pub struct Db {
    pool: PgPool
}

impl Db {

    pub async fn new(database_url: &str) -> Result<Self, LedgerError> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }

    // Run pending migrations from the ./migrations directory.
    pub async fn migrate(&self) -> Result<(), LedgerError> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|e| LedgerError::Database(e.into()))?;
        Ok(())
    }
}



// Account Queries
impl Db {
    pub async fn get_balance(&self, account_id: Uuid) -> Result<i64, LedgerError>{
        let row = sqlx::query!("SELECT balance FROM accounts WHERE id = $1", account_id).fetch_optional(&self.pool).await?;

        row.map(|r| r.balance)
        .ok_or(LedgerError::AccountNotFound(account_id))

    }

    pub async fn apply_entry(&self,account_id: Uuid, transfer_id: Uuid, amount: i64, ) -> Result<(), LedgerError> {

        sqlx::query!("UPDATE accounts SET balance = balance + $1 WHERE id = $2", amount, account_id).execute(&self.pool).await?;
        sqlx::query!("INSERT INTO ledger_entries (account_id, amount, transfer_id) VALUES ($1, $2, $3)", account_id, amount, transfer_id).execute(&self.pool).await?;
        Ok(())
    }
}