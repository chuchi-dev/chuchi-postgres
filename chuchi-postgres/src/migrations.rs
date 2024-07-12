//! Migrations
//!
//! How do migrations work
//!
//! A migration is an sql script which can be executed on the database
//! this script is only executed once and then stored in the database.

use crate::{connection::ConnectionOwned, filter, table::Table, Error};

use chuchi_postgres_derive::{row, FromRow};
use tracing::debug;
use types::time::DateTime;

#[derive(Debug, FromRow)]
pub struct ExecutedMigration {
	datetime: DateTime,
}

/// Holds all migrations
///
/// and checks which migrations already ran, and runs the others
#[derive(Debug, Clone)]
pub struct Migrations {
	table: Table,
}

impl Migrations {
	/// Create a new Migrations
	pub(super) fn new() -> Self {
		Self {
			table: Table::new("migrations"),
		}
	}

	pub(super) async fn init(
		&self,
		db: &mut ConnectionOwned,
	) -> Result<(), Error> {
		let db = db.transaction().await?;
		let conn = db.connection();
		// check if the migrations table exists
		let [result] =
			conn.query_one::<[bool; 1], _>(TABLE_EXISTS, &[]).await?;

		if !result {
			conn.batch_execute(CREATE_TABLE).await?;
		}

		db.commit().await?;

		Ok(())
	}

	pub async fn add(
		&self,
		conn: &mut ConnectionOwned,
		name: &str,
		sql: &str,
	) -> Result<(), Error> {
		let trans = conn.transaction().await?;
		let conn = trans.connection();
		let table = self.table.with_conn(conn);

		// check if the migration was already executed
		let existing: Option<ExecutedMigration> =
			table.select_opt(filter!(&name)).await?;
		if let Some(mig) = existing {
			debug!("migration {} was executed at {}", name, mig.datetime);
			return Ok(());
		}

		// else execute it
		conn.batch_execute(&sql).await?;

		table
			.insert(row! {
				name,
				"datetime": DateTime::now(),
			})
			.await?;

		trans.commit().await?;

		Ok(())
	}
}

const TABLE_EXISTS: &str = "\
SELECT EXISTS (
	SELECT FROM information_schema.tables
	WHERE table_schema = 'public'
	AND table_name = 'migrations'
);";

const CREATE_TABLE: &str = "\
CREATE TABLE migrations (
    name text PRIMARY KEY,
    datetime timestamp
);

CREATE INDEX ON migrations (datetime);";
