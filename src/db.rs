use sqlx::{PgPool, postgres::PgPoolOptions};
use uuid::Uuid;

use crate::error::LedgerError;


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
}