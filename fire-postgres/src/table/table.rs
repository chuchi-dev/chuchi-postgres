use deadpool_postgres::{Object, Pool};

use super::util::{info_data_to_sql, quote};
use super::{Info, TableTemplate};

use crate::connection::OwnedConnection;
use crate::database::DatabaseError;
use crate::filter::{Filter, WhereFilter};
use crate::{filter, Database, Error, Result, Row};

use std::borrow::Borrow;
use std::marker::PhantomData;
use std::sync::Arc;
// use std::fmt::Write;

// use tokio_postgres::types::ToSql;
// use tokio_postgres::row::Row;

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
	// insert: String,
	// update_full: SqlBuilder,
	// names_for_select: String,
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
		let meta = TableMeta {
			// select: Self::create_select_sql(&info, name),
			// insert: Self::create_insert_sql(&info, name),
			// update_full: Self::create_update_full(&info),
			// names_for_select: Self::create_names_for_select(&info),
			info,
		};

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

	// /// ## Example Output
	// /// `"a", "b"`
	// pub fn names_for_select(&self) -> &str {
	// 	&self.meta.names_for_select
	// }

	pub fn info(&self) -> &Info {
		&self.meta.info
	}

	// fn create_names_for_select(info: &Info) -> String {
	// 	format!("\"{}\"", info.names().join("\", \""))
	// }

	// fn create_select_sql(info: &Info, name: &str) -> String {
	// 	let names = info.names();
	// 	format!("SELECT \"{}\" FROM \"{}\"", names.join("\", \""), name)
	// }

	// fn create_insert_sql(info: &Info, name: &str) -> String {
	// 	let mut names = vec![];
	// 	let mut vals = vec![];
	// 	for (i, col) in info.data().iter().enumerate() {
	// 		names.push(quote(col.name));
	// 		vals.push(format!("${}", i + 1));
	// 	}

	// 	// maybe could prepare basic sql already??
	// 	format!(
	// 		"INSERT INTO \"{}\" ({}) VALUES ({})",
	// 		name,
	// 		names.join(", "),
	// 		vals.join(", ")
	// 	)
	// }

	// // we need to return an SqlBuilder and not just a string is since
	// // the where clause could also contain some parameters which would reset
	// // the param counter
	// fn create_update_full(info: &Info) -> SqlBuilder {
	// 	let mut sql = SqlBuilder::new();

	// 	let last = info.data().len() - 1;
	// 	for (i, col) in info.data().iter().enumerate() {
	// 		sql.space_after(format!("\"{}\" =", col.name));
	// 		sql.param();

	// 		if i != last {
	// 			sql.space_after(",");
	// 		}
	// 	}

	// 	sql
	// }

	async fn get_conn(&self) -> Result<OwnedConnection> {
		self.db.get().await.map_err(|e| match e {
			DatabaseError::Other(e) => e.into(),
			e => Error::Unknown(e.into()),
		})
	}

	// Create
	pub async fn try_create(&self) -> Result<()> {
		let sql = info_data_to_sql(self.name, self.meta.info.data());

		debug_sql!("create", self.name, sql);

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

	/*pub async fn query_raw(
		&self,
		sql: &str,
		params: &[&(dyn ToSql + Sync)]
	) -> Result<Vec<Row>, PostgresError> {
		self.client.query(sql, params).await
	}

	pub async fn query_to_raw(
		&self,
		query: Query
	) -> Result<Vec<Row>, PostgresError> {
		let sql = query.sql().to_string();
		let data = query.to_sql_params();
		self.client.query(sql.as_str(), data.as_slice()).await
	}*/

	// find
	// maybe rename to insert
	// and store statement in table
	pub async fn insert_one(&self, input: &T) -> Result<()> {
		// let sql = &self.meta.insert;
		// debug_sql!("insert_one", self.name, sql);

		// let cl = self.get_client().await?;

		todo!()

		// let data = input.to_data();
		// let params = data_into_sql_params(&data);

		// don't use a prepare statement since this is executed only once
		// cl.execute(sql, params.as_slice()).await?;
		// Ok(())
	}

	// pub async fn insert_many<B, I>(&self, input: I) -> Result<()>
	// where
	// 	B: Borrow<T>,
	// 	I: Iterator<Item = B>,
	// {
	// 	let sql = &self.meta.insert;
	// 	debug_sql!("insert_many", self.name, sql);

	// 	// we make a transaction so if an error should occur
	// 	// we don't insert any data
	// 	let mut cl = self.get_client().await?;
	// 	let ts = cl.transaction().await?;

	// 	let stmt = ts.prepare(sql).await?;

	// 	for input in input {
	// 		let data = input.borrow().to_data();
	// 		let params = data_into_sql_params(&data);

	// 		ts.execute(&stmt, params.as_slice()).await?;
	// 	}

	// 	ts.commit().await?;

	// 	Ok(())
	// }

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

	/// expects the rows to be in the order which get's returned by
	/// names_for_select
	pub async fn find_many_raw(&self, sql: &str) -> Result<Vec<T>> {
		debug_sql!("find_many_raw", self.name, sql);

		self.get_conn().await?.connection().query(sql, &[]).await
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

	// // update one
	// pub async fn update<'a>(
	// 	&self,
	// 	where_query: Query<'a>,
	// 	update_query: UpdateParams<'a>,
	// ) -> Result<()> {
	// 	// UPDATE table SET column WHERE
	// 	let mut query = update_query.into_query();
	// 	query.sql.space("WHERE");
	// 	query.append(where_query);

	// 	self.meta.info.validate_params(query.params())?;

	// 	let sql = format!("UPDATE \"{}\" SET {}", self.name, query.sql());
	// 	debug_sql!("update", self.name, sql);
	// 	let params = query.to_sql_params();

	// 	let cl = self.get_client().await?;
	// 	cl.execute(&sql, params.as_slice()).await?;

	// 	Ok(())
	// }

	// pub async fn update_full<'a>(
	// 	&self,
	// 	where_query: Query<'a>,
	// 	input: &'a T,
	// ) -> Result<()> {
	// 	let mut sql = self.meta.update_full.clone();

	// 	self.meta.info.validate_params(where_query.params())?;

	// 	sql.space("WHERE");
	// 	sql.append(where_query.sql);

	// 	let sql = format!("UPDATE \"{}\" SET {}", self.name, sql);
	// 	debug_sql!("update_full", self.name, sql);

	// 	let mut data = input.to_data();
	// 	for param in where_query.params {
	// 		data.push(param.data);
	// 	}
	// 	let params = data_into_sql_params(&data);

	// 	let cl = self.get_client().await?;
	// 	cl.execute(&sql, params.as_slice()).await?;

	// 	Ok(())
	// }

	// delete one
	pub async fn delete(&self, where_query: WhereFilter<'_>) -> Result<()> {
		// self.meta.info.validate_params(where_query.params())?;

		let sql = format!("DELETE FROM \"{}\"{}", self.name, where_query);
		debug_sql!("delete_many", self.name, sql);

		self.get_conn()
			.await?
			.connection()
			.execute_raw(&sql, where_query.params.iter_to_sql())
			.await
			.map(|_| ())
	}

	// /// this does not verify the params
	// pub async fn execute_raw(
	// 	&self,
	// 	sql: SqlBuilder,
	// 	data: &[ColumnData<'_>],
	// ) -> Result<()> {
	// 	let sql = sql.to_string();
	// 	debug_sql!("execute_raw", self.name, sql);

	// 	let params = data_into_sql_params(data);

	// 	let cl = self.get_client().await?;
	// 	cl.execute(&sql, params.as_slice()).await?;

	// 	Ok(())
	// }
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
