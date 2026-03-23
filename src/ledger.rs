use crate::{db::Db, error::LedgerError, types::TransferRequest};

#[derive(Debug)]
pub struct Ledger {
    db: Db,
}

impl Ledger {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    pub async fn transfer(&self, req: TransferRequest) -> Result<(), LedgerError> {
        self.db.with_transaction(|mut tx| async {
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

            Ok(((), tx))
        });
        todo!()
    }
}
