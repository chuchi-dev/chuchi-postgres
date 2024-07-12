use super::Row;

use std::error::Error as StdError;

use postgres_types::FromSql;

pub trait FromRowOwned: Sized {
	fn from_row_owned(
		row: Row,
	) -> Result<Self, Box<dyn StdError + Sync + Send>>;
}

impl<T> FromRowOwned for T
where
	T: for<'r> FromRow<'r>,
{
	fn from_row_owned(
		row: Row,
	) -> Result<Self, Box<dyn StdError + Sync + Send>> {
		T::from_row(&row)
	}
}

pub trait FromRow<'r>: Sized {
	fn from_row(row: &'r Row) -> Result<Self, Box<dyn StdError + Sync + Send>>;
}

// pub struct Single<T>(pub T);

impl<'r, T> FromRow<'r> for [T; 1]
where
	T: FromSql<'r>,
{
	fn from_row(row: &'r Row) -> Result<Self, Box<dyn StdError + Sync + Send>> {
		Ok([row.try_get(0)?])
	}
}

macro_rules! impl_tuple {
	($($name:ident),*) => {
		impl<'r, $($name),*> FromRow<'r> for ($($name),*)
		where
			$($name: FromSql<'r>),*
		{
			fn from_row(row: &'r Row) -> Result<Self, Box<dyn StdError + Sync + Send>> {
				Ok(($(row.try_get(stringify!($name))?),*))
			}
		}
	};
}

impl_tuple!(A, B);
impl_tuple!(A, B, C);
impl_tuple!(A, B, C, D);
impl_tuple!(A, B, C, D, E);
impl_tuple!(A, B, C, D, E, F);
impl_tuple!(A, B, C, D, E, F, G);
impl_tuple!(A, B, C, D, E, F, G, H);
impl_tuple!(A, B, C, D, E, F, G, H, I);
impl_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
