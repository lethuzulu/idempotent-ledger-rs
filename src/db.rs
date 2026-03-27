use sqlx::{PgPool, Postgres, postgres::PgPoolOptions};
use uuid::Uuid;

use crate::{error::LedgerError, types::{IdempotencyKey, TransferResult}};

use sqlx::Transaction;

#[derive(Debug, Clone)]
pub struct Db {
    pool: PgPool,
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

// transaction helper
impl Db {
    pub async fn with_transaction<'a, F, Fut, T>(&self, f: F) -> Result<T, LedgerError>
    where
        F: FnOnce(Transaction<'a, Postgres>) -> Fut,
        Fut: Future<Output = Result<(T, Transaction<'a, Postgres>), LedgerError>>,
    {
        let mut tx = self.pool.begin().await?;

        // set SERIALIZABLE isolation for the transaction
        sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
            .execute(&mut *tx)
            .await?;

        let (result, tx) = f(tx).await?;

        tx.commit().await?;
        Ok(result)
    }
}

// Account Queries
impl Db {
    pub async fn get_balance(&self, account_id: Uuid) -> Result<i64, LedgerError> {
        let row = sqlx::query!("SELECT balance FROM accounts WHERE id = $1", account_id)
            .fetch_optional(&self.pool)
            .await?;

        row.map(|r| r.balance)
            .ok_or(LedgerError::AccountNotFound(account_id))
    }

    // lock both accounts for an update within a transaction.
    // always in uuid order to prevent deadlocks when tow concurrent
    // transfers touch the same pair of accounts in opposite directions.
    pub async fn lock_accounts(
        tx: &mut Transaction<'_, Postgres>,
        a: Uuid,
        b: Uuid,
    ) -> Result<(), LedgerError> {
        // sort to ensure consistent lock ordering
        let (first, second) = if a < b { (a, b) } else { (b, a) };

        sqlx::query!(
            "SELECT id FROM accounts WHERE  id = ANY($1) ORDER  BY id FOR    UPDATE",
            &[first, second] as &[Uuid]
        )
        .fetch_all(&mut **tx)
        .await?;

        Ok(())
    }

    pub async fn apply_entry(
        tx: &mut Transaction<'_, Postgres>,
        account_id: Uuid,
        transfer_id: Uuid,
        amount: i64,
    ) -> Result<(), LedgerError> {
        // update running balance, check constraint fires if balance is negative
        sqlx::query!(
            r#"
            UPDATE accounts
            SET    balance = balance + $1
            WHERE  id = $2
            "#,
            amount,
            account_id
        )
        .execute(&mut **tx)
        .await
        .map_err(|e| {
            // postgres error code 23514 - check_violation
            if let sqlx::Error::Database(ref db_err) = e {
                if db_err.code().as_deref() == Some("23514") {
                    return LedgerError::InsufficientFunds { account_id };
                }
            }
            LedgerError::Database(e)
        })?;

        // insert ledger entry
        sqlx::query!(
            "INSERT INTO ledger_entries (account_id, amount, transfer_id) VALUES ($1, $2, $3)",
            account_id,
            amount,
            transfer_id
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }
}

// Idempotent Key Queries
impl Db {
    pub async fn store_idempotency_key(
        tx: &mut Transaction<'_, Postgres>, key: &IdempotencyKey, result: &TransferResult
    ) -> Result<(), LedgerError> {
        let response = serde_json::to_value(result)?;

        sqlx::query!("INSERT INTO idempotency_keys (key, response) VALUES ($1, $2) ON CONFLICT (key) DO NOTHING", key.as_str(), response).execute(&mut **tx).await?;
        Ok(())
    }

    pub async fn get_idempotency_key(&self, key: &IdempotencyKey) -> Result<Option<TransferResult>, LedgerError> {
        let row = sqlx::query!("SELECT response FROM idempotency_keys WHERE key = $1", key.as_str()).fetch_optional(&self.pool).await?;

        match row {
            None => Ok(None),
            Some(r) => {
                let result : TransferResult = serde_json::from_value(r.response)?;
                Ok(Some(result))
            }
        }
    }
}
