// use crate::table::{Table, TableTemplate};

use std::borrow::Borrow;

use deadpool_postgres::{ClientWrapper, Object};

use futures_util::pin_mut;
use futures_util::StreamExt;
use futures_util::TryStreamExt;
use postgres_types::{BorrowToSql, ToSql, Type};
use tokio_postgres::error::SqlState;
use tokio_postgres::Error as PgError;

pub use deadpool::managed::TimeoutType;
pub use deadpool_postgres::{Config, ConfigError};
use tokio_postgres::Statement;
use tokio_postgres::ToStatement;
use tracing::error;

use crate::filter::Filter;
use crate::filter::Limit;
use crate::filter::WhereFilter;
use crate::row::FromRowOwned;
use crate::row::NamedColumns;
use crate::row::RowStream;
use crate::try2;
use crate::update::ToUpdate;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
	#[error("Unique violation {0}")]
	UniqueViolation(PgError),

	#[error("Expected one row")]
	ExpectedOneRow,

	#[error("Other Postgres error {0}")]
	Other(PgError),

	#[error("Deserialization error {0}")]
	Deserialize(Box<dyn std::error::Error + Send + Sync>),

	#[error("Unknown error {0}")]
	Unknown(Box<dyn std::error::Error + Send + Sync>),
}

impl From<PgError> for Error {
	fn from(e: PgError) -> Self {
		let Some(state) = e.code() else {
			return Self::Other(e);
		};

		match state {
			&SqlState::UNIQUE_VIOLATION => Self::UniqueViolation(e),
			state => {
				error!("db error with state {:?}", state);
				Self::Other(e)
			}
		}
	}
}

#[derive(Debug)]
pub struct OwnedConnection(pub(crate) Object);

impl OwnedConnection {
	pub fn connection(&self) -> Connection {
		Connection {
			inner: ConnectionInner::Client(&self.0),
		}
	}

	pub async fn transaction(&mut self) -> Result<Transaction, Error> {
		Ok(Transaction {
			inner: self.0.transaction().await.map_err(Error::from)?,
		})
	}
}

#[derive(Debug)]
pub struct Transaction<'a> {
	inner: deadpool_postgres::Transaction<'a>,
}

impl<'a> Transaction<'a> {
	/// Returns a connection to the database
	pub fn connection(&self) -> Connection {
		Connection {
			inner: ConnectionInner::Transaction(&self.inner),
		}
	}

	/// See [`tokio_postgres::Transaction::commit()`]
	pub async fn commit(self) -> Result<(), Error> {
		self.inner.commit().await.map_err(Error::from)
	}

	/// See [`tokio_postgres::Transaction::rollback()`]
	pub async fn rollback(self) -> Result<(), Error> {
		self.inner.rollback().await.map_err(Error::from)
	}
}

#[derive(Debug, Clone, Copy)]
pub struct Connection<'a> {
	inner: ConnectionInner<'a>,
}

#[derive(Debug, Clone, Copy)]
enum ConnectionInner<'a> {
	Client(&'a ClientWrapper),
	Transaction(&'a deadpool_postgres::Transaction<'a>),
}

impl Connection<'_> {
	// select
	pub async fn select<R>(
		&self,
		table: &str,
		filter: impl Borrow<Filter<'_>>,
	) -> Result<Vec<R>, Error>
	where
		R: FromRowOwned + NamedColumns,
	{
		let sql = format!(
			"SELECT {} FROM \"{}\"{}",
			R::select_columns(),
			table,
			filter.borrow()
		);
		let stmt = self.prepare_cached(&sql).await?;

		self.query_raw(&stmt, filter.borrow().params.iter_to_sql())
			.await?
			.map(|row| {
				row.and_then(|row| {
					R::from_row_owned(row).map_err(Error::Deserialize)
				})
			})
			.try_collect()
			.await
	}

	// select_one
	pub async fn select_one<R>(
		&self,
		table: &str,
		filter: impl Borrow<Filter<'_>>,
	) -> Result<R, Error>
	where
		R: FromRowOwned + NamedColumns,
	{
		let mut formatter = filter.borrow().to_formatter();

		if matches!(formatter.limit, Limit::All) {
			formatter.limit = &Limit::Fixed(1);
		}

		let sql = format!(
			"SELECT {} FROM \"{}\"{}",
			R::select_columns(),
			table,
			filter.borrow()
		);
		let stmt = self.prepare_cached(&sql).await?;

		let row = self
			.query_raw_opt(&stmt, filter.borrow().params.iter_to_sql())
			.await
			.and_then(|opt| opt.ok_or(Error::ExpectedOneRow))?;

		R::from_row_owned(row).map_err(Error::Deserialize)
	}

	// select_opt
	pub async fn select_opt<R>(
		&self,
		table: &str,
		filter: impl Borrow<Filter<'_>>,
	) -> Result<Option<R>, Error>
	where
		R: FromRowOwned + NamedColumns,
	{
		let mut formatter = filter.borrow().to_formatter();

		if matches!(formatter.limit, Limit::All) {
			formatter.limit = &Limit::Fixed(1);
		}

		let sql = format!(
			"SELECT {} FROM \"{}\"{}",
			R::select_columns(),
			table,
			filter.borrow()
		);
		let stmt = self.prepare_cached(&sql).await?;

		self.query_raw_opt(&stmt, filter.borrow().params.iter_to_sql())
			.await
	}

	// insert one
	pub async fn insert<U>(&self, table: &str, item: &U) -> Result<(), Error>
	where
		U: ToUpdate,
	{
		let sql = format!(
			"INSERT INTO \"{}\" ({}) VALUES ({})",
			table,
			U::insert_columns(),
			U::insert_values()
		);
		let stmt = self.prepare_cached(&sql).await?;

		self.execute(&stmt, item.params()).await.map(|_| ())
	}

	// insert_many
	pub async fn insert_many<'a, U, I>(
		&self,
		table: &str,
		items: I,
	) -> Result<(), Error>
	where
		U: ToUpdate + 'a,
		I: IntoIterator<Item = &'a U>,
	{
		let sql = format!(
			"INSERT INTO \"{}\" ({}) VALUES ({})",
			table,
			U::insert_columns(),
			U::insert_values()
		);
		let stmt = self.prepare_cached(&sql).await?;

		for item in items {
			self.execute(&stmt, item.params()).await?;
		}

		Ok(())
	}

	// update
	pub async fn update<U>(
		&self,
		table: &str,
		item: &U,
		filter: impl Borrow<WhereFilter<'_>>,
	) -> Result<(), Error>
	where
		U: ToUpdate,
	{
		let filter = filter.borrow();
		let mut formatter = filter.whr.to_formatter();
		formatter.param_start = U::params_len();

		let sql = format!(
			"UPDATE \"{}\" SET {}{}",
			table,
			U::update_columns(),
			formatter
		);
		let stmt = self.prepare_cached(&sql).await?;

		// we need to merge both params

		self.execute_raw(
			&stmt,
			TwoExactSize(
				item.params().into_iter().map(|p| *p),
				filter.params.iter_to_sql(),
			),
		)
		.await
		.map(|_| ())
	}

	// delete

	/// Like [`tokio_postgres::Client::prepare_typed()`] but uses a cached
	/// statement if one exists.
	pub async fn prepare_cached(
		&self,
		query: &str,
	) -> Result<Statement, Error> {
		match &self.inner {
			ConnectionInner::Client(client) => {
				client.prepare_cached(query).await.map_err(Error::from)
			}
			ConnectionInner::Transaction(tr) => {
				tr.prepare_cached(query).await.map_err(Error::from)
			}
		}
	}

	/// See [`tokio_postgres::Client::prepare()`]
	pub async fn prepare(&self, query: &str) -> Result<Statement, Error> {
		match &self.inner {
			ConnectionInner::Client(client) => {
				client.prepare(query).await.map_err(Error::from)
			}
			ConnectionInner::Transaction(tr) => {
				tr.prepare(query).await.map_err(Error::from)
			}
		}
	}

	/// Like [`tokio_postgres::Client::prepare_typed()`] but uses a cached
	/// statement if one exists.
	pub async fn prepare_typed_cached(
		&self,
		query: &str,
		types: &[Type],
	) -> Result<Statement, Error> {
		match &self.inner {
			ConnectionInner::Client(client) => client
				.prepare_typed_cached(query, types)
				.await
				.map_err(Error::from),
			ConnectionInner::Transaction(tr) => tr
				.prepare_typed_cached(query, types)
				.await
				.map_err(Error::from),
		}
	}

	/// See [`tokio_postgres::Client::prepare_typed()`]
	pub async fn prepare_typed(
		&self,
		query: &str,
		parameter_types: &[Type],
	) -> Result<Statement, Error> {
		match &self.inner {
			ConnectionInner::Client(client) => client
				.prepare_typed(query, parameter_types)
				.await
				.map_err(Error::from),
			ConnectionInner::Transaction(tr) => tr
				.prepare_typed(query, parameter_types)
				.await
				.map_err(Error::from),
		}
	}

	/// See [`tokio_postgres::Client::query()`]
	pub async fn query<R, T>(
		&self,
		statement: &T,
		params: &[&(dyn ToSql + Sync)],
	) -> Result<Vec<R>, Error>
	where
		R: FromRowOwned,
		T: ?Sized + ToStatement,
	{
		self.query_raw(statement, slice_iter(params))
			.await?
			.map(|row| {
				row.and_then(|row| {
					R::from_row_owned(row).map_err(Error::Deserialize)
				})
			})
			.try_collect()
			.await
	}

	/// See [`tokio_postgres::Client::query_one()`]
	pub async fn query_one<R, T>(
		&self,
		statement: &T,
		params: &[&(dyn ToSql + Sync)],
	) -> Result<R, Error>
	where
		R: FromRowOwned,
		T: ?Sized + ToStatement,
	{
		let row = match &self.inner {
			ConnectionInner::Client(client) => {
				client.query_one(statement, params).await?
			}
			ConnectionInner::Transaction(tr) => {
				tr.query_one(statement, params).await?
			}
		};

		R::from_row_owned(row.into()).map_err(Error::Deserialize)
	}

	/// See [`tokio_postgres::Client::query_opt()`]
	pub async fn query_opt<R, T>(
		&self,
		statement: &T,
		params: &[&(dyn ToSql + Sync)],
	) -> Result<Option<R>, Error>
	where
		R: FromRowOwned,
		T: ?Sized + ToStatement,
	{
		let row = match &self.inner {
			ConnectionInner::Client(client) => {
				client.query_opt(statement, params).await?
			}
			ConnectionInner::Transaction(tr) => {
				tr.query_opt(statement, params).await?
			}
		};

		R::from_row_owned(try2!(row).into())
			.map(Some)
			.map_err(Error::Deserialize)
	}

	/// See [`tokio_postgres::Client::query_opt()`] and [`tokio_postgres::Client::query_raw()`]
	pub async fn query_raw_opt<R, T, P, I>(
		&self,
		statement: &T,
		params: I,
	) -> Result<Option<R>, Error>
	where
		R: FromRowOwned,
		T: ?Sized + ToStatement,
		P: BorrowToSql,
		I: IntoIterator<Item = P>,
		I::IntoIter: ExactSizeIterator,
	{
		let stream = self.query_raw(statement, params).await?;
		pin_mut!(stream);

		let row = stream.try_next().await?;

		if stream.try_next().await?.is_some() {
			return Err(Error::ExpectedOneRow);
		}

		row.map(|row| R::from_row_owned(row).map_err(Error::Deserialize))
			.transpose()
	}

	/// See [`tokio_postgres::Client::query_raw()`]
	pub async fn query_raw<T, P, I>(
		&self,
		statement: &T,
		params: I,
	) -> Result<RowStream, Error>
	where
		T: ?Sized + ToStatement,
		P: BorrowToSql,
		I: IntoIterator<Item = P>,
		I::IntoIter: ExactSizeIterator,
	{
		let row_stream = match &self.inner {
			ConnectionInner::Client(client) => {
				client.query_raw(statement, params).await?
			}
			ConnectionInner::Transaction(tr) => {
				tr.query_raw(statement, params).await?
			}
		};

		Ok(row_stream.into())
	}

	/// See [`tokio_postgres::Client::execute()`]
	pub async fn execute<T>(
		&self,
		statement: &T,
		params: &[&(dyn ToSql + Sync)],
	) -> Result<u64, Error>
	where
		T: ?Sized + ToStatement,
	{
		match &self.inner {
			ConnectionInner::Client(client) => {
				client.execute(statement, params).await.map_err(Error::from)
			}
			ConnectionInner::Transaction(tr) => {
				tr.execute(statement, params).await.map_err(Error::from)
			}
		}
	}

	/// See [`tokio_postgres::Client::execute_raw()`]
	pub async fn execute_raw<T, P, I>(
		&self,
		statement: &T,
		params: I,
	) -> Result<u64, Error>
	where
		T: ?Sized + ToStatement,
		P: BorrowToSql,
		I: IntoIterator<Item = P>,
		I::IntoIter: ExactSizeIterator,
	{
		match &self.inner {
			ConnectionInner::Client(client) => client
				.execute_raw(statement, params)
				.await
				.map_err(Error::from),
			ConnectionInner::Transaction(tr) => {
				tr.execute_raw(statement, params).await.map_err(Error::from)
			}
		}
	}

	/// See [`tokio_postgres::Client::batch_execute()`]
	pub async fn batch_execute(&self, query: &str) -> Result<(), Error> {
		match &self.inner {
			ConnectionInner::Client(client) => {
				client.batch_execute(query).await.map_err(Error::from)
			}
			ConnectionInner::Transaction(tr) => {
				tr.batch_execute(query).await.map_err(Error::from)
			}
		}
	}
}

fn slice_iter<'a>(
	s: &'a [&'a (dyn ToSql + Sync)],
) -> impl ExactSizeIterator<Item = &'a dyn ToSql> + 'a {
	s.iter().map(|s| *s as _)
}

struct TwoExactSize<I, J>(I, J);

impl<I, J, T> Iterator for TwoExactSize<I, J>
where
	I: ExactSizeIterator<Item = T>,
	J: ExactSizeIterator<Item = T>,
{
	type Item = T;

	fn next(&mut self) -> Option<Self::Item> {
		self.0.next().or_else(|| self.1.next())
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let (a, b) = (self.0.size_hint(), self.1.size_hint());
		(a.0 + b.0, a.1.and_then(|a| b.1.map(|b| a + b)))
	}
}

impl<I, J, T> ExactSizeIterator for TwoExactSize<I, J>
where
	I: ExactSizeIterator<Item = T>,
	J: ExactSizeIterator<Item = T>,
{
}
