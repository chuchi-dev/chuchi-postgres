#[doc(hidden)]
pub use bytes::BytesMut;
#[doc(hidden)]
pub use postgres_types::{to_sql_checked, FromSql, IsNull, ToSql, Type};

/// ## Example
/// ```
/// # use chuchi_postgres::try2;
///
/// fn mul(maybe_num: Option<i32>) -> Result<Option<i32>, &'static str> {
/// 	let x = try2!(maybe_num);
///
/// 	x.checked_mul(2)
/// 		.map(Some)
/// 		.ok_or("overflow")
/// }
/// ```
#[macro_export]
macro_rules! try2 {
	($exp:expr) => {
		match $exp {
			Some(o) => o,
			None => return Ok(None),
		}
	};
}

/// ## Example
/// ```
/// # use chuchi_postgres::try_vec;
///
/// fn add(maybe_vec: Option<Vec<i32>>) -> Result<Vec<i32>, &'static str> {
/// 	let v = try_vec!(maybe_vec);
///
/// 	Ok(v.into_iter().map(|x| x + 1).collect())
/// }
/// ```
#[macro_export]
macro_rules! try_vec {
	($exp:expr) => {
		match $exp {
			Some(o) => o,
			None => return Ok(vec![]),
		}
	};
}

/// ## Example
/// ```
/// use chuchi_postgres::enum_u16;
/// enum_u16! {
/// 	#[derive(Debug)]
/// 	pub enum SiteRaw {
/// 		FrameUser = 10,
/// 		Admin = 20,
/// 		App = 30
/// 	}
/// }
/// ```
#[macro_export]
macro_rules! enum_u16 {
	($(#[$metas:meta])* $($pub:ident)? enum $name:ident {
		$($opt:ident = $num:expr),*
	}) => {
		$(#[$metas])*
		#[repr(u16)]
		$($pub)? enum $name {
			$($opt = $num),*
		}

		impl $name {
			pub fn as_u16(&self) -> u16 {
				match self {
					$(Self::$opt => $num),*
				}
			}

			pub fn from_u16(
				num: u16
			) -> std::result::Result<Self, &'static str> {
				match num {
					$($num => Ok(Self::$opt)),*,
					_ => Err(stringify!(could not parse u16 to $name))
				}
			}
		}

		impl $crate::table::column::ColumnType for $name {
			fn column_kind() -> $crate::table::column::ColumnKind {
				$crate::table::column::ColumnKind::I32
			}
		}

		impl $crate::macros::ToSql for $name {
			fn to_sql(
				&self,
				ty: &$crate::macros::Type,
				buf: &mut $crate::macros::BytesMut
			) -> std::result::Result<$crate::macros::IsNull, Box<dyn std::error::Error + Sync + Send>> {
				let val = self.as_u16() as i32;

				val.to_sql(ty, buf)
			}

			fn accepts(ty: &$crate::macros::Type) -> bool {
				<i32 as $crate::macros::ToSql>::accepts(ty)
			}

			$crate::macros::to_sql_checked!();
		}

		impl<'a> $crate::macros::FromSql<'a> for $name {
			fn from_sql(
				ty: &$crate::macros::Type,
				buf: &'a [u8]
			) -> std::result::Result<Self, Box<dyn std::error::Error + Sync + Send>> {
				let num = <i32 as $crate::macros::FromSql>::from_sql(ty, buf)?;
				num.try_into()
					.map_err(|_| "i32 to u16 conversion failed")
					.and_then(Self::from_u16)
					.map_err(|m| m.into())
			}

			fn accepts(ty: &$crate::macros::Type) -> bool {
				<i32 as $crate::macros::FromSql>::accepts(ty)
			}
		}

		impl $crate::filter::ParamData for $name {
			fn is_null(&self) -> bool {
				false
			}
		}
	};
}
