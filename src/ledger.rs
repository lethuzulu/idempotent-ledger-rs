use crate::{db::Db, error::LedgerError, types::{TransferRequest, TransferResult}};



#[derive(Debug)]
pub struct Ledger {
    db: Db
}

impl Ledger {
    pub fn new(db: Db) -> Self {
        Self {db}
    }


    pub async fn transfer(&self, req: TransferRequest) -> Result<(), LedgerError> {

        self.db.apply_entry(req.from_account.0, req.transfer_id.0, -req.amount.cents()).await?;
        self.db.apply_entry(req.to_account.0, req.transfer_id.0, req.amount.cents()).await?; 

        todo!()
    }
}

