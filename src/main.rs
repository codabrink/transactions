mod account;
mod transaction;

use account::{Accounts, AccountsExt};

fn main() {
  let mut accounts = Accounts::new();
  let args: Vec<String> = std::env::args().collect();
  for file_name in args {
    transaction::process(&mut accounts, &file_name)
      .expect(&format!("Coult not process file {}", file_name));
  }
  accounts.export();
}
