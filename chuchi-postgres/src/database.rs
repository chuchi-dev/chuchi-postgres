use deadpool_postgres::{CreatePoolError, Pool, PoolError, Runtime};

use tokio_postgres::Error as PgError;
use tokio_postgres::NoTls;

pub use deadpool::managed::TimeoutType;
pub use deadpool_postgres::{Config as PgConfig, ConfigError};

use crate::connection::ConnectionOwned;
use crate::migrations::Migrations;
use crate::table::TableOwned;
use crate::table::TableTemplate;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DatabaseError {
	#[error("The database configuration is invalid {0}")]
	Config(ConfigError),

	#[error("Getting a connection timed out {0:?}")]
	Timeout(TimeoutType),

	#[error("Connection error {0}")]
	Connection(#[from] crate::Error),

	#[error("Postgres error {0}")]
	Other(#[from] PgError),
}

#[derive(Clone, Debug, Default)]
pub struct Config {
	pg_config: PgConfig,
	migration_table: Option<String>,
}

impl Config {
	pub fn from_pg_config(pg_config: PgConfig) -> Self {
		Self {
			pg_config,
			..Default::default()
		}
	}

	pub fn host(mut self, host: impl Into<String>) -> Self {
		self.pg_config.host = Some(host.into());
		self
	}

	pub fn dbname(mut self, dbname: impl Into<String>) -> Self {
		self.pg_config.dbname = Some(dbname.into());
		self
	}

	pub fn user(mut self, user: impl Into<String>) -> Self {
		self.pg_config.user = Some(user.into());
		self
	}

	pub fn password(mut self, password: impl Into<String>) -> Self {
		self.pg_config.password = Some(password.into());
		self
	}

	pub fn migration_table(mut self, table: impl Into<String>) -> Self {
		self.migration_table = Some(table.into());
		self
	}

	pub fn pg_config(&self) -> &PgConfig {
		&self.pg_config
	}

	pub fn pg_config_mut(&mut self) -> &mut PgConfig {
		&mut self.pg_config
	}
}

#[derive(Debug, Clone)]
pub struct Database {
	pool: Pool,
	migrations: Migrations,
}

impl Database {
	/// Create a new database
	pub async fn new(
		name: impl Into<String>,
		user: impl Into<String>,
		password: impl Into<String>,
	) -> Result<Self, DatabaseError> {
		Self::with_host("localhost", name, user, password).await
	}

	/// Create a new database with a host
	pub async fn with_host(
		host: impl Into<String>,
		name: impl Into<String>,
		user: impl Into<String>,
		password: impl Into<String>,
	) -> Result<Self, DatabaseError> {
		Self::with_cfg(
			Config::default()
				.host(host)
				.dbname(name)
				.user(user)
				.password(password),
		)
		.await
	}

	pub async fn with_cfg(cfg: Config) -> Result<Self, DatabaseError> {
		let pool = cfg
			.pg_config
			.create_pool(Some(Runtime::Tokio1), NoTls)
			.map_err(|e| match e {
				CreatePoolError::Config(e) => DatabaseError::Config(e),
				CreatePoolError::Build(_) => unreachable!(
					"since we provide a runtime this should never happen"
				),
			})?;

		let this = Self {
			pool,
			migrations: Migrations::new(cfg.migration_table),
		};

		// just make sure the connection worked
		let mut db = this.get().await?;

		this.migrations.init(&mut db).await?;

		Ok(this)
	}

	pub async fn get(&self) -> Result<ConnectionOwned, DatabaseError> {
		self.pool
			.get()
			.await
			.map_err(|e| match e {
				PoolError::Timeout(tim) => DatabaseError::Timeout(tim),
				PoolError::Backend(e) => e.into(),
				PoolError::Closed => todo!("when can a pool be closed?"),
				PoolError::NoRuntimeSpecified => unreachable!(),
				PoolError::PostCreateHook(e) => {
					todo!("what is this error {e:?}?")
				}
			})
			.map(ConnectionOwned)
	}

	pub fn migrations(&self) -> Migrations {
		self.migrations.clone()
	}

	/// Get a table from the database
	pub fn table_owned<T>(&self, name: &'static str) -> TableOwned<T>
	where
		T: TableTemplate,
	{
		TableOwned::new(self.clone(), name)
	}
}
