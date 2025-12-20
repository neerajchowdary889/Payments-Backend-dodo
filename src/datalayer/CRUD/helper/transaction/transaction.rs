use sqlx::{Postgres, pool::PoolConnection};

use crate::{
    datalayer::CRUD::{
        accounts::AccountBuilder, money::money, transaction::TransactionBuilder, types::Transaction,
    },
    errors::errors::ServiceError,
};

pub struct TransactionHelper<'a> {
    txn: TransactionBuilder,
    db_conn: &'a mut PoolConnection<Postgres>,
}

impl<'a> TransactionHelper<'a> {
    pub fn new(txn: TransactionBuilder, db_conn: &'a mut PoolConnection<Postgres>) -> Self {
        Self { txn, db_conn }
    }

    pub async fn transfer(&mut self) -> Result<Transaction, ServiceError> {
        // TODO: Implement transfer logic
        todo!("Implement transfer logic")
    }

    pub async fn debit(&mut self, storage_units: i64) -> Result<bool, ServiceError> {
        // load the account balance and do the subtract by converting into usd, as account gives back usd
        // if balance is less than amount then return error
        // else update the account balance
        // create a transaction and return it
        let account = AccountBuilder::new()
            .id(self.txn.from_account_id.unwrap())
            .expect_balance()
            .read(Some(self.db_conn))
            .await?;

        let new_balance = account.balance - money::from_storage_units(storage_units);

        if new_balance < 0.0 {
            return Err(ServiceError::InsufficientBalance {
                account_id: self.txn.from_account_id.unwrap().to_string(),
                required: money::from_storage_units(storage_units),
            });
        }

        // convert balance to storage units
        let _ = AccountBuilder::new()
            .id(self.txn.from_account_id.unwrap())
            .balance(new_balance)
            .update(Some(self.db_conn))
            .await?;

        return Ok(true);
    }

    pub async fn credit(&mut self, storage_units: i64) -> Result<bool, ServiceError> {
        // load the account balance and do the add by converting into usd, as account gives back usd
        // if balance is less than amount then return error
        // else update the account balance
        // create a transaction and return it
        let account = AccountBuilder::new()
            .id(self.txn.to_account_id.unwrap())
            .expect_balance()
            .read(Some(self.db_conn))
            .await?;

        let new_balance = account.balance + money::from_storage_units(storage_units);

        // convert balance to storage units
        let _ = AccountBuilder::new()
            .id(self.txn.to_account_id.unwrap())
            .balance(new_balance)
            .update(Some(self.db_conn))
            .await?;

        return Ok(true);
    }
}
