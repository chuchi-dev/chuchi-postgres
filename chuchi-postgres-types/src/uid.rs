use std::fmt;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::{general_purpose::URL_SAFE_NO_PAD, Engine};
use base64::DecodeError;
use rand::{rngs::OsRng, RngCore};

/// A UniqueId that can be used within a database.
/// Is not cryptographically secure and could be bruteforced.
///
/// Contains 10bytes
/// - 0..5 are seconds since the UNIX_EPOCH
/// - 5..10 are random
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
// graphql
#[cfg_attr(feature = "juniper", derive(juniper::GraphQLScalar))]
#[cfg_attr(feature = "juniper", graphql(with = graphql))]
pub struct UniqueId([u8; 10]);

impl UniqueId {
	pub fn new() -> Self {
		let secs_bytes = SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.expect("SystemTime before UNIX EPOCH!")
			.as_secs()
			.to_be_bytes();

		let mut bytes = [0u8; 10];
		bytes[..5].copy_from_slice(&secs_bytes[3..8]);

		OsRng.fill_bytes(&mut bytes[5..]);

		Self(bytes)
	}

	/// This creates a unique id with it's raw content
	/// making it able to be called in a const context.
	pub const fn from_raw(inner: [u8; 10]) -> Self {
		Self(inner)
	}

	pub fn from_slice_unchecked(slice: &[u8]) -> Self {
		let mut bytes = [0u8; 10];
		bytes.copy_from_slice(slice);
		Self(bytes)
	}

	pub fn to_b64(&self) -> String {
		URL_SAFE_NO_PAD.encode(self.0)
	}

	/// If the string is not 14 bytes long returns InvalidLength
	pub fn parse_from_b64<T>(b64: T) -> Result<Self, DecodeError>
	where
		T: AsRef<[u8]>,
	{
		if b64.as_ref().len() != 14 {
			return Err(DecodeError::InvalidLength(b64.as_ref().len()));
		}

		let mut bytes = [0u8; 10];
		URL_SAFE_NO_PAD
			.decode_slice_unchecked(b64, &mut bytes)
			.map(|n| assert_eq!(n, bytes.len()))
			.map(|_| Self(bytes))
	}

	pub fn from_bytes(bytes: [u8; 10]) -> Self {
		Self(bytes)
	}

	pub fn into_bytes(self) -> [u8; 10] {
		self.0
	}

	pub fn since_unix_secs(&self) -> u64 {
		let mut bytes = [0u8; 8];
		bytes[3..].copy_from_slice(&self.0[..5]);
		u64::from_be_bytes(bytes)
	}

	pub fn as_slice(&self) -> &[u8] {
		&self.0
	}
}

impl fmt::Debug for UniqueId {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_tuple("UniqueId").field(&self.to_b64()).finish()
	}
}

impl fmt::Display for UniqueId {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.to_b64().fmt(f)
	}
}

impl FromStr for UniqueId {
	type Err = DecodeError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Self::parse_from_b64(s)
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

	impl Serialize for UniqueId {
		fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer,
		{
			serializer.serialize_str(&self.to_b64())
		}
	}

	impl<'de> Deserialize<'de> for UniqueId {
		fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
		where
			D: Deserializer<'de>,
		{
			let s: Cow<'_, str> = Deserialize::deserialize(deserializer)?;
			let s = s.as_ref();
			if s.len() == 14 {
				UniqueId::parse_from_b64(s).map_err(D::Error::custom)
			} else {
				Err(D::Error::custom(
					"expected string with exactly 14 characters",
				))
			}
		}
	}
}

#[cfg(feature = "postgres")]
mod postgres {
	use bytes::BytesMut;
	use postgres_types::{to_sql_checked, FromSql, IsNull, ToSql, Type};

	use super::*;

	impl ToSql for UniqueId {
		fn to_sql(
			&self,
			ty: &Type,
			out: &mut BytesMut,
		) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>>
		where
			Self: Sized,
		{
			self.to_b64().to_sql(ty, out)
		}

		fn accepts(ty: &Type) -> bool
		where
			Self: Sized,
		{
			<&str as ToSql>::accepts(ty)
		}

		to_sql_checked!();
	}

	impl<'r> FromSql<'r> for UniqueId {
		fn from_sql(
			ty: &Type,
			raw: &'r [u8],
		) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
			let s = <&str as FromSql>::from_sql(ty, raw)?;
			UniqueId::parse_from_b64(s).map_err(Into::into)
		}

		fn accepts(ty: &Type) -> bool {
			<&str as FromSql>::accepts(ty)
		}
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

	impl EncodeMessage for UniqueId {
		const WIRE_TYPE: WireType = WireType::Len;

		fn is_default(&self) -> bool {
			false
		}

		fn encoded_size(
			&mut self,
			field: Option<FieldOpt>,
			builder: &mut SizeBuilder,
		) -> Result<(), EncodeError> {
			self.0.encoded_size(field, builder)
		}

		fn encode<B>(
			&mut self,
			field: Option<FieldOpt>,
			encoder: &mut MessageEncoder<B>,
		) -> Result<(), EncodeError>
		where
			B: BytesWrite,
		{
			self.0.encode(field, encoder)
		}
	}

	impl<'m> DecodeMessage<'m> for UniqueId {
		const WIRE_TYPE: WireType = WireType::Len;

		fn decode_default() -> Self {
			Self::from_raw([0; 10])
		}

		fn merge(
			&mut self,
			kind: FieldKind<'m>,
			is_field: bool,
		) -> Result<(), DecodeError> {
			self.0.merge(kind, is_field)
		}
	}
}

#[cfg(feature = "juniper")]
mod graphql {
	use super::*;

	use juniper::{
		InputValue, ParseScalarResult, ScalarToken, ScalarValue, Value,
	};

	pub(crate) fn to_output<S: ScalarValue>(v: &UniqueId) -> Value<S> {
		Value::scalar(v.to_string())
	}

	pub(crate) fn from_input<S: ScalarValue>(
		v: &InputValue<S>,
	) -> Result<UniqueId, String> {
		v.as_string_value()
			.ok_or("Expected a string")?
			.parse()
			.map_err(|e: DecodeError| e.to_string())
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

	// abcdefghijklmnopqrstuvwxyz

	#[test]
	fn serde_test() {
		let s = "\"AGCGeWIDTlipbg\"";
		let d: UniqueId = from_str(s).unwrap();
		assert_eq!(d.to_string(), "AGCGeWIDTlipbg");

		let v = Value::String("AGCGeWIDTlipbg".into());
		let d: UniqueId = from_value(v).unwrap();
		assert_eq!(d.to_string(), "AGCGeWIDTlipbg");
	}
}
