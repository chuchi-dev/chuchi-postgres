#![allow(clippy::tabs_in_doc_comments)]
#![allow(clippy::never_loop)]
#![allow(clippy::new_without_default)]

pub mod database;
pub use database::Database;

pub mod connection;
pub use connection::Connection;
pub use connection::Error;

pub mod db;

pub mod row;
pub use row::Row;

pub mod table;
// pub use table::Table;

#[cfg(feature = "json")]
pub use types::json;
pub use types::time;
pub use types::uid::UniqueId;

pub mod filter;

pub mod migrations;

pub use chuchi_postgres_derive::{row, FromRow, TableTempl, ToRow};

pub type Result<T> = std::result::Result<T, Error>;

#[doc(hidden)]
pub mod macros;

mod impl_crypto;
