use super::DateTime;
// use crate::table::column::{ColumnData, ColumnKind, ColumnType, FromDataError};

use std::fmt;
use std::ops::{Add, Sub};
use std::str::FromStr;
use std::time::Duration as StdDuration;

use chrono::format::ParseError;
use chrono::Duration;
use chrono::{TimeZone, Utc};

/// A date in utc
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
// graphql
#[cfg_attr(feature = "juniper", derive(juniper::GraphQLScalar))]
#[cfg_attr(feature = "juniper", graphql(with = graphql))]
pub struct Date(chrono::NaiveDate);

impl Date {
	/// ## Panics
	/// if the date is invalid
	pub fn new(year: i32, month: u32, day: u32) -> Self {
		let naive = chrono::NaiveDate::from_ymd_opt(year, month, day)
			.expect("year, month or day out of range");
		Self(naive)
	}

	pub fn now() -> Self {
		DateTime::now().to_date()
	}

	pub fn raw(&self) -> &chrono::NaiveDate {
		&self.0
	}

	pub fn raw_mut(&mut self) -> &mut chrono::NaiveDate {
		&mut self.0
	}

	pub fn to_datetime(&self) -> DateTime {
		let naive = self.0.and_hms_opt(0, 0, 0).unwrap();
		Utc.from_utc_datetime(&naive).into()
	}

	pub fn to_days_since_1970(&self) -> i32 {
		let pg_epoch = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();

		// Calculate the difference in days
		(self.0 - pg_epoch)
			.num_days()
			.try_into()
			.expect("to many days")
	}

	pub fn from_days_since_1970(days: i32) -> Self {
		let pg_epoch = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
		Self(pg_epoch + Duration::days(days as i64))
	}

	pub fn try_sub(&self, other: &Date) -> Option<StdDuration> {
		(self.0 - other.0).to_std().ok()
	}

	pub fn into_raw(self) -> chrono::NaiveDate {
		self.0
	}
}

impl From<chrono::NaiveDate> for Date {
	fn from(d: chrono::NaiveDate) -> Self {
		Self(d)
	}
}

impl From<Date> for chrono::NaiveDate {
	fn from(d: Date) -> Self {
		d.0
	}
}

impl Add<StdDuration> for Date {
	type Output = Self;

	/// ## Panic
	/// May panic if the duration is to big
	fn add(self, rhs: StdDuration) -> Self {
		Self(self.0 + Duration::from_std(rhs).unwrap())
	}
}

impl Sub<StdDuration> for Date {
	type Output = Self;

	fn sub(self, rhs: StdDuration) -> Self {
		Self(self.0 - Duration::from_std(rhs).unwrap())
	}
}

// TABLE INFO

// impl ColumnType for Date {
// 	fn column_kind() -> ColumnKind {
// 		ColumnKind::Date
// 	}
// 	fn to_data(&self) -> ColumnData<'_> {
// 		ColumnData::Date(self.to_days_since_1970())
// 	}
// 	fn from_data(data: ColumnData) -> Result<Self, FromDataError> {
// 		match data {
// 			ColumnData::Date(m) => Ok(Self::from_days_since_1970(m)),
// 			_ => Err(FromDataError::ExpectedType("Date")),
// 		}
// 	}
// }

// DISPLAY
impl fmt::Display for Date {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.0.fmt(f)
	}
}

impl FromStr for Date {
	type Err = ParseError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Self(s.parse()?))
	}
}

// SERDE

#[cfg(feature = "serde")]
mod impl_serde {
	use super::*;

	use std::borrow::Cow;

	use serde::de::{Deserializer, Error};
	use serde::ser::Serializer;
	use serde::{Deserialize, Serialize};

	impl Serialize for Date {
		fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer,
		{
			serializer.serialize_str(&self.to_string())
		}
	}

	impl<'de> Deserialize<'de> for Date {
		fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
		where
			D: Deserializer<'de>,
		{
			let s: Cow<'_, str> = Deserialize::deserialize(deserializer)?;
			Date::from_str(s.as_ref()).map_err(D::Error::custom)
		}
	}
}

#[cfg(feature = "postgres")]
mod postgres {
	use super::*;
	use bytes::BytesMut;
	use postgres_protocol::types;
	use postgres_types::{
		accepts, to_sql_checked, FromSql, IsNull, ToSql, Type,
	};

	impl ToSql for Date {
		fn to_sql(
			&self,
			_ty: &Type,
			out: &mut BytesMut,
		) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
			types::date_to_sql(self.to_days_since_1970(), out);

			Ok(IsNull::No)
		}

		accepts!(DATE);

		to_sql_checked!();
	}

	impl<'a> FromSql<'a> for Date {
		fn from_sql(
			_ty: &Type,
			raw: &'a [u8],
		) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
			let jd = types::date_from_sql(raw)?;

			Ok(Date::from_days_since_1970(jd))
		}

		accepts!(DATE);
	}
}

#[cfg(feature = "protobuf")]
mod protobuf {
	use super::*;

	use protopuffer::{
		bytes::BytesWrite,
		decode::{DecodeError, DecodeMessage, FieldKind},
		encode::{
			EncodeError, EncodeMessage, FieldOpt, MessageEncoder, SizeBuilder,
		},
		WireType,
	};

	impl EncodeMessage for Date {
		const WIRE_TYPE: WireType = WireType::Varint;

		fn is_default(&self) -> bool {
			false
		}

		fn encoded_size(
			&mut self,
			field: Option<FieldOpt>,
			builder: &mut SizeBuilder,
		) -> Result<(), EncodeError> {
			self.to_days_since_1970().encoded_size(field, builder)
		}

		fn encode<B>(
			&mut self,
			field: Option<FieldOpt>,
			encoder: &mut MessageEncoder<B>,
		) -> Result<(), EncodeError>
		where
			B: BytesWrite,
		{
			self.to_days_since_1970().encode(field, encoder)
		}
	}

	impl<'m> DecodeMessage<'m> for Date {
		const WIRE_TYPE: WireType = WireType::Varint;

		fn decode_default() -> Self {
			Self::from_days_since_1970(0)
		}

		fn merge(
			&mut self,
			kind: FieldKind<'m>,
			is_field: bool,
		) -> Result<(), DecodeError> {
			let mut n = 0i32;
			n.merge(kind, is_field)?;

			*self = Self::from_days_since_1970(n);

			Ok(())
		}
	}
}

#[cfg(feature = "juniper")]
mod graphql {
	use super::*;

	use juniper::{
		InputValue, ParseScalarResult, ScalarToken, ScalarValue, Value,
	};

	pub(crate) fn to_output<S: ScalarValue>(v: &Date) -> Value<S> {
		Value::scalar(v.to_string())
	}

	pub(crate) fn from_input<S: ScalarValue>(
		v: &InputValue<S>,
	) -> Result<Date, String> {
		v.as_string_value()
			.and_then(|s| Date::from_str(s.as_ref()).ok())
			.ok_or_else(|| "Expected a date y-m-d".into())
	}

	pub(crate) fn parse_token<S: ScalarValue>(
		value: ScalarToken<'_>,
	) -> ParseScalarResult<S> {
		<String as juniper::ParseScalarValue<S>>::from_str(value)
	}
}

#[cfg(all(test, feature = "serde"))]
mod tests {
	use super::*;
	use serde_json::{from_str, from_value, Value};

	#[test]
	fn serde_test() {
		let s = "\"2023-08-12\"";
		let d: Date = from_str(s).unwrap();
		assert_eq!(d.to_string(), "2023-08-12");

		let v = Value::String("2023-08-12".into());
		let d: Date = from_value(v).unwrap();
		assert_eq!(d.to_string(), "2023-08-12");

		assert_eq!(d.to_days_since_1970(), 19581);
		assert_eq!(Date::from_days_since_1970(19581).to_string(), "2023-08-12");
	}
}
