//! Database module
//!
//! ## Axum
//! If you wan't to use axum the best way is to make a small wrapper around
//! ConnOwned.
//!
//! ```rust
//! pub struct ConnOwned(pub pg::db::ConnOwned);
//!
//! impl ConnOwned {
//! 	pub fn conn(&self) -> Conn {
//! 		self.0.conn()
//! 	}
//!
//! 	pub async fn trans(&mut self) -> Result<Trans, pg::Error> {
//! 		self.0.trans().await
//! 	}
//! }
//!
//! impl<S> FromRequestParts<S> for ConnOwned
//! where
//! 	S: Send + Sync,
//! 	Db: FromRef<S>,
//! {
//! 	type Rejection = Error;
//!
//! 	async fn from_request_parts(
//! 		_parts: &mut Parts,
//! 		state: &S,
//! 	) -> Result<Self, Self::Rejection> {
//! 		let db = Db::from_ref(state);
//! 		db.get()
//! 			.await
//! 			.map(Self)
//! 			.map_err(|e| Error::Internal(e.to_string()))
//! 	}
//! }
//! ```

use crate::{
	Connection, Database, Error,
	connection::{ConnectionOwned, Transaction},
	database::DatabaseError,
};

/// This might contain a database or none.
///
/// This will be usefull to mocking the database with a memory
/// database.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "chuchi", derive(chuchi::Resource))]
pub struct Db {
	pg: Option<Database>,
}

impl Db {
	pub fn new_memory() -> Self {
		Self { pg: None }
	}

	pub async fn get(&self) -> Result<ConnOwned, DatabaseError> {
		match &self.pg {
			Some(pg) => Ok(ConnOwned {
				pg: Some(pg.get().await?),
			}),
			None => Ok(ConnOwned { pg: None }),
		}
	}
}

impl From<Database> for Db {
	fn from(pg: Database) -> Self {
		Self { pg: Some(pg) }
	}
}

#[derive(Debug)]
pub struct ConnOwned {
	pg: Option<ConnectionOwned>,
}

impl ConnOwned {
	// connection
	pub fn conn(&self) -> Conn {
		Conn {
			pg: self.pg.as_ref().map(|pg| pg.connection()),
		}
	}

	// or transaction
	pub async fn trans(&mut self) -> Result<Trans, Error> {
		match &mut self.pg {
			Some(pg) => Ok(Trans {
				pg: Some(pg.transaction().await?),
			}),
			None => Ok(Trans { pg: None }),
		}
	}
}

#[cfg(feature = "chuchi")]
mod impl_chuchi {
	use chuchi::{
		extractor::Extractor, extractor_extract, extractor_prepare,
		extractor_validate,
	};

	use super::*;

	impl<'a, R> Extractor<'a, R> for ConnOwned {
		type Error = DatabaseError;
		type Prepared = Self;

		extractor_validate!(|validate| {
			assert!(validate.resources.exists::<Db>(), "Db resource not found");
		});

		extractor_prepare!(|prepare| {
			let db = prepare.resources.get::<Db>().unwrap();
			db.get().await
		});

		extractor_extract!(|extract| { Ok(extract.prepared) });
	}
}

/// This might contain a connection or none.
#[derive(Debug, Clone, Copy)]
pub struct Conn<'a> {
	pg: Option<Connection<'a>>,
}

impl<'a> Conn<'a> {
	/// Create a new connection.
	pub fn new_memory() -> Self {
		Self { pg: None }
	}

	/// Get the connection.
	///
	/// ## Panics
	/// If the connection is not set.
	pub fn pg(self) -> Connection<'a> {
		self.pg.unwrap()
	}
}

/// This might contain a transaction or none.
#[derive(Debug)]
pub struct Trans<'a> {
	pg: Option<Transaction<'a>>,
}

impl<'a> Trans<'a> {
	/// Get the connection of the transaction.
	pub fn conn(&self) -> Conn {
		Conn {
			pg: self.pg.as_ref().map(|pg| pg.connection()),
		}
	}

	/// Commit the transaction.
	///
	/// ## Note
	/// Does nothing if it contains a memory Conn
	pub async fn commit(self) -> Result<(), Error> {
		match self.pg {
			Some(pg) => pg.commit().await,
			None => Ok(()),
		}
	}

	/// Rollback the transaction.
	///
	/// ## Panics
	/// If the transaction is not set / this is a memory Conn
	pub async fn rollback(self) -> Result<(), Error> {
		match self.pg {
			Some(pg) => pg.commit().await,
			None => panic!("rollback not supported"),
		}
	}
}
