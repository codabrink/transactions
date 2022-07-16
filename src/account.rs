use crate::transaction::{Transaction, TransactionType, TxId};
use hashbrown::HashMap;
use rust_decimal::Decimal;
use std::io::Write;

pub type Accounts = HashMap<u16, Account>;
pub type AccountId = u16;

pub trait AccountsExt {
  fn export<W>(&self, writer: W)
  where
    W: Write;
  fn consume(&mut self, transaction: Transaction);
}
impl AccountsExt for Accounts {
  fn export<W>(&self, mut writer: W)
  where
    W: Write,
  {
    let _ = writeln!(&mut writer, "client,available,held,total,locked");
    for (id, account) in self {
      let _ = writeln!(
        &mut writer,
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
    // no way to unlock for now, code to unlock will go up here..
    if self.locked {
      return;
    }
    use TransactionType::*;
    match tx.r#type {
      Chargeback => self.chargeback(tx),
      Deposit => self.deposit(tx),
      Withdrawal => self.withdrawal(tx),
      Dispute => self.dispute(tx),
      Resolve => self.resolve(tx),
    };
  }

  fn total(&self) -> Decimal {
    self.available + self.held
  }

  #[inline]
  fn deposit(&mut self, tx: Transaction) {
    if let Some(amount) = tx.amount {
      self.available += amount;
      self.transactions.insert(tx.id, tx);
    }
  }

  #[inline]
  fn withdrawal(&mut self, tx: Transaction) {
    if let Some(amount) = tx.amount {
      if self.available >= amount {
        self.available -= amount;
        self.transactions.insert(tx.id, tx);
      }
    }
  }

  #[inline]
  fn dispute(&mut self, tx: Transaction) {
    if let Some(tx) = self.transactions.get_mut(&tx.id) {
      if tx.disputed || tx.r#type != TransactionType::Deposit {
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
  fn resolve(&mut self, tx: Transaction) {
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
  fn chargeback(&mut self, tx: Transaction) {
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

#[cfg(test)]
mod tests {
  use crate::account::AccountTester;
  use rust_decimal_macros::dec;

  #[test]
  fn transactions_work() {
    let mut at = AccountTester::new();
    let deposit_id = at.deposit(dec!(1000.5));
    assert_eq!(at.available(), dec!(1000.5));

    // dispute the deposited funds
    at.dispute(deposit_id);
    // check that the funds are held
    assert_eq!(at.held(), dec!(1000.5));
    assert_eq!(at.available(), dec!(0));
    assert_eq!(at.total(), dec!(1000.5));

    // try to dispute again
    at.dispute(deposit_id);
    // check that nothing happened
    assert_eq!(at.held(), dec!(1000.5));
    assert_eq!(at.available(), dec!(0));
    assert_eq!(at.total(), dec!(1000.5));

    // withdraw money we don't have
    at.withdrawal(dec!(500));
    // ensure nothing happened
    assert_eq!(at.held(), dec!(1000.5));
    assert_eq!(at.available(), dec!(0));
    assert_eq!(at.total(), dec!(1000.5));

    // resolve the dispute
    at.resolve(deposit_id);
    // money should no longer be held
    assert_eq!(at.held(), dec!(0));
    assert_eq!(at.available(), dec!(1000.5));
    assert_eq!(at.total(), dec!(1000.5));

    // withdraw some money
    let withdraw_id = at.withdrawal(dec!(20));
    assert_eq!(at.total(), dec!(980.5));

    // do not allow disputes of withdrawals, since those funds are out of our control
    at.dispute(withdraw_id);
    assert_eq!(at.held(), dec!(0));
    assert_eq!(at.available(), dec!(980.5));
    assert_eq!(at.total(), dec!(980.5));

    let deposit2_id = at.deposit(dec!(0.5));

    // fail to chargeback and undisputed deposit
    at.chargeback(deposit2_id);
    assert_eq!(at.held(), dec!(0));
    assert_eq!(at.available(), dec!(981));
    assert_eq!(at.total(), dec!(981));

    // dispute and chargeback a deposit
    at.dispute(deposit2_id);
    assert_eq!(at.held(), dec!(0.5));
    at.chargeback(deposit2_id);
    assert_eq!(at.held(), dec!(0));
    assert_eq!(at.available(), dec!(980.5));
    assert_eq!(at.total(), dec!(980.5));
    assert!(at.locked());
  }
}

#[allow(dead_code)]
pub struct AccountTester {
  pub accounts: Accounts,
  pub client: AccountId,
  pub tx: TxId,
}
#[allow(dead_code)]
impl AccountTester {
  pub fn new() -> Self {
    Self {
      accounts: Accounts::new(),
      client: 0,
      tx: 0,
    }
  }

  pub fn deposit(&mut self, amount: Decimal) -> TxId {
    self.tx += 1;
    self.accounts.consume(Transaction {
      r#type: TransactionType::Deposit,
      client: self.client,
      id: self.tx,
      amount: Some(amount),
      disputed: false,
    });
    self.tx
  }
  pub fn withdrawal(&mut self, amount: Decimal) -> TxId {
    self.tx += 1;
    self.accounts.consume(Transaction {
      r#type: TransactionType::Withdrawal,
      client: self.client,
      id: self.tx,
      amount: Some(amount),
      disputed: false,
    });
    self.tx
  }
  pub fn dispute(&mut self, tx: TxId) {
    self.accounts.consume(Transaction {
      r#type: TransactionType::Dispute,
      client: self.client,
      id: tx,
      amount: None,
      disputed: false,
    });
  }
  pub fn resolve(&mut self, tx: TxId) {
    self.accounts.consume(Transaction {
      r#type: TransactionType::Resolve,
      client: self.client,
      id: tx,
      amount: None,
      disputed: false,
    });
  }
  pub fn chargeback(&mut self, tx: TxId) {
    self.accounts.consume(Transaction {
      r#type: TransactionType::Chargeback,
      client: self.client,
      id: tx,
      amount: None,
      disputed: false,
    });
  }

  fn account(&self) -> &Account {
    self.accounts.get(&self.client).unwrap()
  }
  pub fn available(&self) -> Decimal {
    self.account().available
  }
  pub fn held(&self) -> Decimal {
    self.account().held
  }
  pub fn total(&self) -> Decimal {
    self.account().total()
  }
  pub fn locked(&self) -> bool {
    self.account().locked
  }
}
