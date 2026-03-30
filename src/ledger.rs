use crate::{
    db::Db,
    error::LedgerError,
    types::{AccountId, TransferRequest, TransferResult},
};

#[derive(Debug, Clone)]
pub struct LedgerService {
    db: Db,
}

impl LedgerService {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    pub async fn transfer(&self, req: TransferRequest) -> Result<TransferResult, LedgerError> {
        if let Some(cached) = self.db.get_cached_result(&req.idempotency_key).await? {
            tracing::info!(key = %req.idempotency_key,"replaying cached transfer result");
            return Ok(cached);
        }

        self.db
            .with_transaction(|mut tx| async {
                // lock accounts in consustent uuid order to prevent deadlock
                Db::lock_accounts(&mut tx, req.from_account.0, req.to_account.0).await?;

                // debit sender
                Db::apply_entry(
                    &mut tx,
                    req.from_account.0,
                    req.transfer_id.0,
                    -req.amount.cents(),
                )
                .await?;

                // credit receiver
                Db::apply_entry(
                    &mut tx,
                    req.to_account.0,
                    req.transfer_id.0,
                    req.amount.cents(),
                )
                .await?;

                let result = TransferResult {
                    from_account: req.from_account,
                    to_account: req.to_account,
                    amount: req.amount,
                    transfer_id: req.transfer_id,
                };

                // strore idempotency key with the ledger entries atomically
                Db::cache_result(&mut tx, &req.idempotency_key, &result).await?;

                Ok((result, tx))
            })
            .await
    }

    pub async fn get_balance(&self, account_id: AccountId) -> Result<i64, LedgerError> {
        self.db.get_balance(account_id.0).await
    }
}
