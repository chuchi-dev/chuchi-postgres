use types::{
	time::{Date, DateTime, Timeout},
	uid::UniqueId,
};

use super::ColumnKind;

pub trait ColumnType: Sized {
	fn column_kind() -> ColumnKind;
}

#[macro_export]
macro_rules! impl_column_type {
	($type:ty, $kind:ident) => {
		impl $crate::table::column::ColumnType for $type {
			#[inline(always)]
			fn column_kind() -> $crate::table::column::ColumnKind {
				$crate::table::column::ColumnKind::$kind
			}
		}
	};
}

impl_column_type!(String, Text);
impl_column_type!(&str, Text);

impl_column_type!(bool, Boolean);

impl_column_type!(f64, F64);
impl_column_type!(f32, F32);

impl_column_type!(i64, I64);
impl_column_type!(i32, I32);
impl_column_type!(i16, I16);

impl<T> ColumnType for Option<T>
where
	T: ColumnType,
{
	#[inline(always)]
	fn column_kind() -> ColumnKind {
		ColumnKind::Option(Box::new(T::column_kind()))
	}
}

impl ColumnType for Vec<String> {
	#[inline(always)]
	fn column_kind() -> ColumnKind {
		ColumnKind::TextArray
	}
}

impl ColumnType for UniqueId {
	fn column_kind() -> ColumnKind {
		ColumnKind::FixedText(14)
	}
}

impl ColumnType for Date {
	fn column_kind() -> ColumnKind {
		ColumnKind::Date
	}
}

impl ColumnType for DateTime {
	fn column_kind() -> ColumnKind {
		ColumnKind::Timestamp
	}
}

impl ColumnType for Timeout {
	fn column_kind() -> ColumnKind {
		ColumnKind::I64
	}
}

#[cfg(feature = "json")]
impl ColumnType for serde_json::Value {
	fn column_kind() -> ColumnKind {
		ColumnKind::Json
	}
}

#[cfg(feature = "email")]
impl ColumnType for email_address::EmailAddress {
	fn column_kind() -> ColumnKind {
		ColumnKind::Text
	}
}

#[cfg(feature = "json")]
mod impl_serde {
	use super::*;

	use postgres_types::Json;
	use serde::{de::DeserializeOwned, Serialize};

	impl<T> ColumnType for Json<T>
	where
		T: Serialize + DeserializeOwned,
	{
		fn column_kind() -> ColumnKind {
			ColumnKind::Json
		}
	}
}
