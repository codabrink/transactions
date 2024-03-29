use crate::account::{AccountId, AccountsExt};
use crate::Accounts;
use anyhow::Result;
use csv::ReaderBuilder;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::io;

pub type TxId = u32;
pub fn process(accounts: &mut Accounts, reader: impl io::Read) -> Result<()> {
  let mut reader = ReaderBuilder::new()
    .trim(csv::Trim::All)
    .from_reader(reader);

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
  #[serde(rename = "tx")]
  pub id: TxId,
  pub amount: Option<Decimal>,

  #[serde(skip_deserializing)]
  pub disputed: bool,
}

#[derive(Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TransactionType {
  Chargeback,
  Deposit,
  Dispute,
  Withdrawal,
  Resolve,
}

#[cfg(test)]
mod tests {
  use super::process;
  use crate::account::{Accounts, AccountsExt};

  #[test]
  fn parse_all_transaction_types() {
    let csv = "type,client , tx,amount\ndeposit,1,1, 10.0 \nwithdrawal,1,2,0.5\ndispute,1,1,\nresolve,1,1,\ndispute,1,1,\nchargeback,1,1,\n";

    let mut accounts = Accounts::new();
    let _ = process(&mut accounts, csv.as_bytes());

    let mut output = Vec::new();
    accounts.export(&mut output);
    let output = String::from_utf8(output).expect("Not UTF-8");
    let expected = "client,available,held,total,locked\n1,-0.5,0,-0.5,true\n";
    assert_eq!(expected, output);
  }
}
