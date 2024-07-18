//! Migrations
//!
//! How do migrations work
//!
//! A migration is an sql script which can be executed on the database
//! this script is only executed once and then stored in the database.

use std::borrow::Cow;

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
	pub(super) fn new(table_name: Option<String>) -> Self {
		Self {
			table: Table::new(
				table_name.map(Cow::Owned).unwrap_or("migrations".into()),
			),
		}
	}

	pub(super) async fn init(
		&self,
		db: &mut ConnectionOwned,
	) -> Result<(), Error> {
		let db = db.transaction().await?;
		let conn = db.connection();

		// replace migrations with the correct table name
		let table_exists =
			TABLE_EXISTS.replace("migrations", self.table.name());

		// check if the migrations table exists
		let [result] =
			conn.query_one::<[bool; 1], _>(&table_exists, &[]).await?;

		if !result {
			let create_table =
				CREATE_TABLE.replace("migrations", self.table.name());
			conn.batch_execute(&create_table).await?;
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
