use std::borrow::{Borrow, Cow};

use crate::{
	filter::{Filter, WhereFilter},
	row::{FromRowOwned, NamedColumns, ToRow, ToRowStatic},
	Connection, Error,
};

#[derive(Debug, Clone)]
pub struct Table {
	name: Cow<'static, str>,
}

impl Table {
	pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
		Self { name: name.into() }
	}

	pub fn with_conn<'a>(&'a self, conn: Connection<'a>) -> TableWithConn<'a> {
		TableWithConn { table: &self, conn }
	}
}

#[derive(Debug, Clone)]
pub struct TableWithConn<'a> {
	table: &'a Table,
	conn: Connection<'a>,
}

impl TableWithConn<'_> {
	/// Get the name of the table
	pub fn name(&self) -> &str {
		self.table.name.as_ref()
	}

	pub async fn select<R>(
		&self,
		filter: impl Borrow<Filter<'_>>,
	) -> Result<Vec<R>, Error>
	where
		R: FromRowOwned + NamedColumns,
	{
		self.conn.select(self.name(), filter).await
	}

	pub async fn select_one<R>(
		&self,
		filter: impl Borrow<Filter<'_>>,
	) -> Result<R, Error>
	where
		R: FromRowOwned + NamedColumns,
	{
		self.conn.select_one(self.name(), filter).await
	}

	pub async fn select_opt<R>(
		&self,
		filter: impl Borrow<Filter<'_>>,
	) -> Result<Option<R>, Error>
	where
		R: FromRowOwned + NamedColumns,
	{
		self.conn.select_opt(self.name(), filter).await
	}

	pub async fn count(
		&self,
		column: &str,
		filter: impl Borrow<Filter<'_>>,
	) -> Result<u32, Error> {
		self.conn.count(self.name(), column, filter).await
	}

	pub async fn insert<U>(&self, item: &U) -> Result<(), Error>
	where
		U: ToRow,
	{
		self.conn.insert(self.name(), item).await
	}

	pub async fn insert_many<U, I>(&self, items: I) -> Result<(), Error>
	where
		U: ToRowStatic,
		I: IntoIterator,
		I::Item: Borrow<U>,
	{
		self.conn.insert_many(self.name(), items).await
	}

	pub async fn update<U>(
		&self,
		item: &U,
		filter: impl Borrow<WhereFilter<'_>>,
	) -> Result<(), Error>
	where
		U: ToRow,
	{
		self.conn.update(self.name(), item, filter).await
	}

	pub async fn delete(
		&self,
		filter: impl Borrow<WhereFilter<'_>>,
	) -> Result<(), Error> {
		self.conn.delete(self.name(), filter).await
	}
}
