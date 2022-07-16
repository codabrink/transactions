mod account;
mod transaction;

use account::{Accounts, AccountsExt};
use std::io;

fn main() {
  let mut accounts = Accounts::new();
  let args: Vec<String> = std::env::args().collect();
  for file_name in args {
    let file =
      std::fs::File::open(&file_name).expect(&format!("Could not find file {}", &file_name));
    transaction::process(&mut accounts, &file)
      .expect(&format!("Coult not process file {}", file_name));
  }

  let mut writer = io::stdout();
  accounts.export(&mut writer);
}
