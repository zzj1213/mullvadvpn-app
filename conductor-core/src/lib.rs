#[macro_use]
extern crate serde_derive;

mod database;
pub use database::Database;
pub use database::Error as DbError;
mod types;
pub use types::AccountInfo;
pub mod convention;