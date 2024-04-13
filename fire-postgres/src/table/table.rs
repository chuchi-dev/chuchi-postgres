use super::util::info_data_to_sql;
use super::{Info, TableTemplate};

use crate::connection::OwnedConnection;
use crate::database::DatabaseError;
use crate::filter::{Filter, WhereFilter};
use crate::update::ToUpdate;
use crate::{filter, Database, Error, Result, Row};

use std::borrow::Borrow;
use std::marker::PhantomData;
use std::sync::Arc;

// is thread safe
// maybe should change to an inner?
macro_rules! debug_sql {
	($method:expr, $name:expr, $sql:expr) => {
		tracing::debug!("sql: {} {} with {}", $method, $name, $sql);
	};
}

#[derive(Debug)]
struct TableMeta {
	info: Info,
}

#[derive(Debug)]
pub struct Table<T>
where
	T: TableTemplate,
{
	db: Database,
	name: &'static str,
	meta: Arc<TableMeta>,
	phantom: PhantomData<T>,
}

impl<T> Table<T>
where
	T: TableTemplate,
{
	pub(crate) fn new(db: Database, name: &'static str) -> Self {
		let info = T::table_info();
		let meta = TableMeta { info };

		Self {
			db,
			name,
			meta: Arc::new(meta),
			phantom: PhantomData,
		}
	}

	pub fn name(&self) -> &'static str {
		self.name
	}

	pub fn info(&self) -> &Info {
		&self.meta.info
	}

	pub async fn get_conn(&self) -> Result<OwnedConnection> {
		self.db.get().await.map_err(|e| match e {
			DatabaseError::Other(e) => e.into(),
			e => Error::Unknown(e.into()),
		})
	}

	// Create
	pub async fn try_create(&self) -> Result<()> {
		let sql = info_data_to_sql(self.name, self.meta.info.data());

		self.get_conn()
			.await?
			.connection()
			.batch_execute(sql.as_str())
			.await
	}

	/// ## Panics
	/// if the table could not be created
	pub async fn create(self) -> Self {
		self.try_create().await.expect("could not create table");
		self
	}

	// find
	// maybe rename to insert
	// and store statement in table
	pub async fn insert_one(&self, input: &T) -> Result<()> {
		self.get_conn()
			.await?
			.connection()
			.insert(self.name, input)
			.await
	}

	pub async fn insert_many<'a, I>(&self, input: I) -> Result<()>
	where
		T: 'a,
		I: IntoIterator<Item = &'a T>,
	{
		let mut conn = self.get_conn().await?;
		let trans = conn.transaction().await?;
		let conn = trans.connection();

		conn.insert_many(self.name, input).await?;

		trans.commit().await?;

		Ok(())
	}

	/*
	SELECT id, name, FROM {}
	*/
	pub async fn find_all(&self) -> Result<Vec<T>> {
		self.get_conn()
			.await?
			.connection()
			.select(self.name, filter!())
			.await
	}

	pub async fn find_many(
		&self,
		filter: impl Borrow<Filter<'_>>,
	) -> Result<Vec<T>> {
		self.get_conn()
			.await?
			.connection()
			.select(self.name, filter)
			.await
	}

	pub async fn find_one(
		&self,
		filter: impl Borrow<Filter<'_>>,
	) -> Result<Option<T>> {
		self.get_conn()
			.await?
			.connection()
			.select_opt(self.name, filter)
			.await
	}

	pub async fn count_many<'a>(
		&self,
		filter: impl Borrow<Filter<'_>>,
	) -> Result<u32> {
		let sql = format!(
			"SELECT COUNT(*) FROM \"{}\"{}",
			self.name,
			filter.borrow()
		);

		debug_sql!("count_many", self.name, sql);

		let row: Option<Row> = {
			self.get_conn()
				.await?
				.connection()
				.query_raw_opt(
					sql.as_str(),
					filter.borrow().params.iter_to_sql(),
				)
				.await?
		};

		Ok(row.map(|row| row.get(0)).unwrap_or(0))
	}

	// update one
	pub async fn update<'a, U>(
		&self,
		item: &U,
		filter: impl Borrow<WhereFilter<'a>>,
	) -> Result<()>
	where
		U: ToUpdate,
	{
		self.get_conn()
			.await?
			.connection()
			.update(self.name, item, filter)
			.await
	}

	pub async fn update_full<'a>(
		&self,
		input: &'a T,
		filter: impl Borrow<WhereFilter<'a>>,
	) -> Result<()> {
		self.get_conn()
			.await?
			.connection()
			.update(self.name, input, filter)
			.await
	}

	// delete one
	pub async fn delete(
		&self,
		filter: impl Borrow<WhereFilter<'_>>,
	) -> Result<()> {
		self.get_conn()
			.await?
			.connection()
			.delete(self.name, filter)
			.await
	}
}

impl<T> Clone for Table<T>
where
	T: TableTemplate,
{
	fn clone(&self) -> Self {
		Self {
			db: self.db.clone(),
			name: self.name,
			meta: self.meta.clone(),
			phantom: PhantomData,
		}
	}
}
