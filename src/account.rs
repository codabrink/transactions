use crate::transaction::{Transaction, TransactionType, TxId};
use hashbrown::HashMap;
use rust_decimal::Decimal;

pub type Accounts = HashMap<u16, Account>;
pub type AccountId = u16;

pub trait AccountsExt {
  fn export(&self);
  fn consume(&mut self, transaction: Transaction);
}
impl AccountsExt for Accounts {
  fn export(&self) {
    println!("client,available,held,total,locked");
    for (id, account) in self {
      println!(
        "{},{},{},{},{}",
        id,
        account.available,
        account.held,
        account.total(),
        account.locked
      );
    }
  }
  fn consume(&mut self, tx: Transaction) {
    let account = self.entry(tx.client).or_default();
    account.consume(tx);
  }
}

#[derive(Default)]
pub struct Account {
  available: Decimal,
  held: Decimal,
  locked: bool,
  // defeats the purpose of streaming csv from file if we're just going to keep most of it in mem anyway
  // typically transactions would sit in database
  transactions: HashMap<TxId, Transaction>,
}

impl Account {
  fn consume(&mut self, tx: Transaction) {
    use TransactionType::*;
    match tx.r#type {
      Chargeback => self.chargeback(&tx),
      Deposit => self.deposit(&tx),
      Withdrawal => self.withdrawal(&tx),
      Dispute => self.dispute(&tx),
      Resolve => self.resolve(&tx),
    };

    // might want to check if tx.id exists already to avoid overwrites
    // assuming all tx id's are unique for now..
    self.transactions.insert(tx.id, tx);
  }

  fn total(&self) -> Decimal {
    self.available - self.held
  }

  #[inline]
  fn deposit(&mut self, tx: &Transaction) {
    if let Some(amount) = tx.amount {
      self.available += amount;
    }
  }

  #[inline]
  fn withdrawal(&mut self, tx: &Transaction) {
    if let Some(amount) = tx.amount {
      if self.available >= amount {
        self.available -= amount;
      }
    }
  }

  #[inline]
  fn dispute(&mut self, tx: &Transaction) {
    if let Some(tx) = self.transactions.get_mut(&tx.id) {
      if tx.disputed {
        return;
      }
      tx.disputed = true;

      if let Some(amount) = tx.amount {
        self.available -= amount;
        self.held += amount;
      }
    }
  }

  #[inline]
  fn resolve(&mut self, tx: &Transaction) {
    if let Some(tx) = self.transactions.get_mut(&tx.id) {
      if !tx.disputed {
        return;
      }
      tx.disputed = false;

      if let Some(amount) = tx.amount {
        self.available += amount;
        self.held -= amount;
      }
    }
  }

  #[inline]
  fn chargeback(&mut self, tx: &Transaction) {
    if let Some(tx) = self.transactions.remove(&tx.id) {
      if !tx.disputed {
        self.transactions.insert(tx.id, tx);
        return;
      }
      self.locked = true;

      if let Some(amount) = tx.amount {
        self.held -= amount;
      }
    }
  }
}
