use crate::account::{AccountId, AccountsExt};
use crate::Accounts;
use anyhow::Result;
use csv::Reader;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::fs::File;
use std::path::Path;

pub type TxId = u32;
pub fn process(accounts: &mut Accounts, path: impl AsRef<Path>) -> Result<()> {
  let file = File::open(path)?;
  let mut reader = Reader::from_reader(file);

  for tx in reader.deserialize() {
    let tx: Transaction = tx?;
    accounts.consume(tx);
  }

  Ok(())
}

#[derive(Deserialize)]
pub struct Transaction {
  pub r#type: TransactionType,
  pub client: AccountId,
  pub id: TxId,
  pub amount: Option<Decimal>,

  #[serde(skip_serializing)]
  pub disputed: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
  Chargeback,
  Deposit,
  Dispute,
  Withdrawal,
  Resolve,
}
