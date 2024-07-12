use deadpool_postgres::{CreatePoolError, Pool, PoolError, Runtime};

use tokio_postgres::Error as PgError;
use tokio_postgres::NoTls;

pub use deadpool::managed::TimeoutType;
pub use deadpool_postgres::{Config, ConfigError};

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
		Self::with_cfg(Config {
			host: Some(host.into()),
			dbname: Some(name.into()),
			user: Some(user.into()),
			password: Some(password.into()),
			..Default::default()
		})
		.await
	}

	pub async fn with_cfg(cfg: Config) -> Result<Self, DatabaseError> {
		// cfg.manager = Some(ManagerConfig {
		// 	recycling_method: deadpool_postgres::RecyclingMethod::Clean,
		// });

		let pool = cfg.create_pool(Some(Runtime::Tokio1), NoTls).map_err(
			|e| match e {
				CreatePoolError::Config(e) => DatabaseError::Config(e),
				CreatePoolError::Build(_) => unreachable!(
					"since we provide a runtime this should never happen"
				),
			},
		)?;

		let this = Self {
			pool,
			migrations: Migrations::new(),
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
