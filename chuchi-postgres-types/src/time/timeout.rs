use std::time::{Duration, SystemTime};

#[derive(Debug, Clone)]
pub struct Timeout {
	inner: SystemTime,
}

impl Timeout {
	pub fn new(dur: Duration) -> Self {
		Self {
			inner: SystemTime::now() + dur,
		}
	}

	pub fn now() -> Self {
		Self {
			inner: SystemTime::now(),
		}
	}

	pub fn has_elapsed(&self) -> bool {
		SystemTime::now() > self.inner
	}

	/// Returns None if the Duration is negative
	pub fn remaining(&self) -> Option<Duration> {
		self.inner.duration_since(SystemTime::now()).ok()
	}

	/// returns the time from UNIX_EPOCH
	pub fn as_secs(&self) -> u64 {
		self.inner
			.duration_since(SystemTime::UNIX_EPOCH)
			.expect("Welcome to the past!")
			.as_secs()
	}

	pub fn from_secs(s: u64) -> Option<Self> {
		SystemTime::UNIX_EPOCH
			.checked_add(Duration::from_secs(s))
			.map(|c| Timeout { inner: c })
	}
}

#[cfg(feature = "serde")]
mod impl_serde {
	use super::*;

	use serde::de::{Deserializer, Error};
	use serde::ser::Serializer;
	use serde::{Deserialize, Serialize};

	impl Serialize for Timeout {
		fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer,
		{
			serializer.serialize_u64(self.as_secs())
		}
	}

	impl<'de> Deserialize<'de> for Timeout {
		fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
		where
			D: Deserializer<'de>,
		{
			let num: u64 = Deserialize::deserialize(deserializer)?;
			Self::from_secs(num).ok_or(D::Error::custom("timeout to big"))
		}
	}
}

#[cfg(feature = "postgres")]
mod postgres {
	use super::*;
	use bytes::BytesMut;
	use postgres_types::{to_sql_checked, FromSql, IsNull, ToSql, Type};

	impl ToSql for Timeout {
		fn to_sql(
			&self,
			ty: &Type,
			out: &mut BytesMut,
		) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
			let secs: i64 = self.as_secs().try_into()?;

			secs.to_sql(ty, out)
		}

		fn accepts(ty: &Type) -> bool
		where
			Self: Sized,
		{
			<i64 as ToSql>::accepts(ty)
		}

		to_sql_checked!();
	}

	impl<'a> FromSql<'a> for Timeout {
		fn from_sql(
			ty: &Type,
			raw: &'a [u8],
		) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
			let secs: u64 = <i64 as FromSql>::from_sql(ty, raw)?.try_into()?;

			Self::from_secs(secs).ok_or_else(|| "timeout to large".into())
		}

		fn accepts(ty: &Type) -> bool {
			<i64 as FromSql>::accepts(ty)
		}
	}
}
