use chuchi_postgres::enum_u16;
use chuchi_postgres::row;
use chuchi_postgres::row::NamedColumns;
use chuchi_postgres::row::ToRowStatic;
use chuchi_postgres::{FromRow, TableTempl, ToRow, UniqueId};

#[derive(Debug, TableTempl, FromRow, ToRow)]
pub struct Table {
	#[index(primary)]
	pub id: UniqueId,
	pub name: Option<String>,
	pub age: i32,
	pub ty: Type,
}

#[derive(Debug, FromRow)]
#[allow(dead_code)]
pub struct Count(u32);

enum_u16! {
	#[derive(Debug)]
	pub enum Type {
		One = 1,
		Two = 2,
		Three = 3
	}
}

#[test]
fn test_select_columns() {
	assert_eq!(Table::select_columns(), r#""id", "name", "age", "ty""#);
}

#[test]
fn test_insert_columns() {
	assert_eq!(Table::insert_columns(), r#""id", "name", "age", "ty""#);
}

#[test]
fn test_create_row() {
	let s = "";

	let _ = row! {
		"test": "123",
		s
	};
}
