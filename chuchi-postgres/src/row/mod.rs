mod from;
mod to;

use std::{
	error::Error as StdError,
	fmt::Write,
	pin::Pin,
	task::{Context, Poll},
};

use futures_util::Stream;
use pin_project_lite::pin_project;
use postgres_types::{FromSql, ToSql};
use tokio_postgres::row::RowIndex;
pub use tokio_postgres::Column;

use crate::connection::Error;

pub use from::{FromRow, FromRowOwned};
pub use to::{ToRow, ToRowStatic};

pub trait NamedColumns {
	/// should return something like "id", "name", "email"
	fn select_columns() -> &'static str;
}

#[derive(Debug)]
#[repr(transparent)]
pub struct Row {
	row: tokio_postgres::Row,
}

impl Row {
	/// Returns information about the columns of data in the row.
	pub fn columns(&self) -> &[Column] {
		self.row.columns()
	}

	/// Determines if the row contains no values.
	pub fn is_empty(&self) -> bool {
		self.row.is_empty()
	}

	/// Returns the number of values in the row.
	pub fn len(&self) -> usize {
		self.row.len()
	}

	/// Deserializes the row.
	pub fn deserialize<'a, T>(
		&'a self,
	) -> Result<T, Box<dyn StdError + Sync + Send>>
	where
		T: FromRow<'a>,
	{
		T::from_row(self)
	}

	/// Deserializes the row and consumes it.
	///
	/// todo or deserialize_into?
	pub fn deserialize_owned<T>(
		self,
	) -> Result<T, Box<dyn StdError + Sync + Send>>
	where
		T: FromRowOwned,
	{
		T::from_row_owned(self)
	}

	/// Deserializes a value from the row.
	///
	/// The value can be specified either by its numeric index in the row, or by its column name.
	///
	/// # Panics
	///
	/// Panics if the index is out of bounds or if the value cannot be converted to the specified type.
	pub fn get<'a, I, T>(&'a self, idx: I) -> T
	where
		I: RowIndex + std::fmt::Display,
		T: FromSql<'a>,
	{
		self.row.get(idx)
	}

	/// Like [`Row::get()`], but returns a [`Result`] rather than panicking.
	pub fn try_get<'a, I, T>(
		&'a self,
		idx: I,
	) -> Result<T, tokio_postgres::Error>
	where
		I: RowIndex + std::fmt::Display,
		T: FromSql<'a>,
	{
		self.row.try_get(idx)
	}
}

impl From<tokio_postgres::Row> for Row {
	fn from(row: tokio_postgres::Row) -> Self {
		Self { row }
	}
}

impl FromRowOwned for Row {
	fn from_row_owned(
		row: Row,
	) -> Result<Self, Box<dyn StdError + Sync + Send>> {
		Ok(row)
	}
}

pin_project! {
	pub struct RowStream {
		#[pin]
		inner: tokio_postgres::RowStream,
	}
}

impl Stream for RowStream {
	type Item = Result<Row, Error>;

	fn poll_next(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<Option<Self::Item>> {
		let this = self.project();

		match this.inner.poll_next(cx) {
			Poll::Ready(Some(Ok(row))) => Poll::Ready(Some(Ok(row.into()))),
			Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e.into()))),
			Poll::Ready(None) => Poll::Ready(None),
			Poll::Pending => Poll::Pending,
		}
	}
}

impl From<tokio_postgres::RowStream> for RowStream {
	fn from(inner: tokio_postgres::RowStream) -> Self {
		Self { inner }
	}
}

#[derive(Debug)]
pub struct RowBuilder<'a> {
	inner: Vec<(&'a str, &'a (dyn ToSql + Sync))>,
}

impl<'a> RowBuilder<'a> {
	pub fn new() -> Self {
		Self { inner: Vec::new() }
	}

	/// Push a new column to the row.
	///
	/// ## Note
	/// Do not use untrusted names this might lead to
	/// SQL injection.
	pub fn push(
		&mut self,
		name: &'a str,
		value: &'a (dyn ToSql + Sync),
	) -> &mut Self {
		self.inner.push((name, value));

		self
	}
}

impl ToRow for RowBuilder<'_> {
	fn insert_columns(&self, s: &mut String) {
		for (i, (k, _)) in self.inner.iter().enumerate() {
			if i != 0 {
				s.push_str(", ");
			}

			write!(s, "\"{k}\"").unwrap();
		}
	}

	fn insert_values(&self, s: &mut String) {
		for (i, _) in self.inner.iter().enumerate() {
			if i != 0 {
				s.push_str(", ");
			}

			write!(s, "${}", i + 1).unwrap();
		}
	}

	fn update_columns(&self, s: &mut String) {
		for (i, (k, _)) in self.inner.iter().enumerate() {
			if i != 0 {
				s.push_str(", ");
			}

			write!(s, "\"{k}\" = ${}", i + 1).unwrap();
		}
	}

	fn params_len(&self) -> usize {
		self.inner.len()
	}

	fn params(&self) -> impl ExactSizeIterator<Item = &(dyn ToSql + Sync)> {
		self.inner.iter().map(|(_, v)| *v)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_row_builder() {
		let mut row = RowBuilder::new();
		row.push("id", &1i32)
			.push("name", &"test")
			.push("email", &"test");

		let mut cols = String::new();
		row.insert_columns(&mut cols);
		assert_eq!(cols, r#""id", "name", "email""#);

		let mut values = String::new();
		row.insert_values(&mut values);
		assert_eq!(values, r#"$1, $2, $3"#);

		let mut update = String::from("UPDATE \"users\" SET ");
		row.update_columns(&mut update);
		assert_eq!(
			update,
			r#"UPDATE "users" SET "id" = $1, "name" = $2, "email" = $3"#
		);
	}
}
