// macro internal
#[doc(hidden)]
pub use postgres_types::ToSql;

pub trait ToRowStatic {
	/// should return something like "id", "name", "email"
	fn insert_columns() -> &'static str;
	/// should return something like $1, $2, $3
	fn insert_values() -> &'static str;
	/// should return  something like "id" = $1, "name" = $2
	fn update_columns() -> &'static str;

	fn params_len() -> usize;
	fn params(&self) -> impl ExactSizeIterator<Item = &(dyn ToSql + Sync)>;
}

pub trait ToRow {
	/// should return something like "id", "name", "email"
	fn insert_columns(&self, s: &mut String);
	/// should return something like $1, $2, $3
	fn insert_values(&self, s: &mut String);
	/// should return  something like "id" = $1, "name" = $2
	fn update_columns(&self, s: &mut String);

	fn params_len(&self) -> usize;
	fn params(&self) -> impl ExactSizeIterator<Item = &(dyn ToSql + Sync)>;
}

impl<S> ToRow for S
where
	S: ToRowStatic,
{
	fn insert_columns(&self, s: &mut String) {
		s.push_str(S::insert_columns());
	}

	fn insert_values(&self, s: &mut String) {
		s.push_str(S::insert_values());
	}

	fn update_columns(&self, s: &mut String) {
		s.push_str(S::update_columns());
	}

	fn params_len(&self) -> usize {
		S::params_len()
	}

	fn params(&self) -> impl ExactSizeIterator<Item = &(dyn ToSql + Sync)> {
		S::params(self)
	}
}
