use std::borrow::Cow;
use std::fmt;

use tokio_postgres::types::ToSql;
use types::time::{Date, DateTime, Timeout};
use types::uid::UniqueId;

mod whr;

pub type SqlStr = Cow<'static, str>;

#[derive(Debug)]
#[non_exhaustive]
pub struct Filter<'a> {
	pub whr: Where,
	pub order_by: OrderBy,
	pub limit: Limit,
	pub offset: Offset,
	pub params: Params<'a>,
}

impl<'a> Filter<'a> {
	pub fn new() -> Self {
		Self {
			whr: Where::new(),
			order_by: OrderBy::new(),
			limit: Limit::new(),
			offset: Offset::new(),
			params: Params::new(),
		}
	}

	pub(crate) fn to_formatter(&'a self) -> FilterFormatter<'a> {
		FilterFormatter {
			whr: &self.whr,
			order_by: &self.order_by,
			limit: &self.limit,
			offset: &self.offset,
			params: &self.params,
		}
	}
}

impl fmt::Display for Filter<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.to_formatter())
	}
}

#[derive(Debug)]
#[non_exhaustive]
pub(crate) struct FilterFormatter<'a> {
	pub whr: &'a Where,
	pub order_by: &'a OrderBy,
	pub limit: &'a Limit,
	pub offset: &'a Offset,
	pub params: &'a Params<'a>,
}

impl fmt::Display for FilterFormatter<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.whr.fmt(f)?;

		self.order_by.fmt(f)?;

		let offset_has_param = matches!(self.offset, Offset::Param);
		let param_count =
			self.params.len() - if offset_has_param { 1 } else { 0 };

		match &self.limit {
			Limit::Fixed(value) => write!(f, " LIMIT {}", value)?,
			Limit::Param => {
				write!(f, " LIMIT ${}", param_count)?;
			}
			Limit::All => {}
		}

		match &self.offset {
			Offset::Zero => {}
			Offset::Fixed(value) => write!(f, " OFFSET {}", value)?,
			Offset::Param => {
				write!(f, " OFFSET ${}", self.params.len())?;
			}
		}

		Ok(())
	}
}

#[derive(Debug)]
#[non_exhaustive]
pub struct WhereFilter<'a> {
	pub whr: Where,
	pub params: Params<'a>,
}

impl<'a> WhereFilter<'a> {
	pub fn new() -> Self {
		Self {
			whr: Where::new(),
			params: Params::new(),
		}
	}
}

impl fmt::Display for WhereFilter<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.whr.fmt(f)
	}
}

#[derive(Debug)]
pub struct Where {
	inner: Vec<WherePart>,
}

#[derive(Debug)]
pub enum WherePart {
	Operation(WhereOperation),
	And,
	Or,
}

#[derive(Debug)]
pub struct WhereOperation {
	pub kind: Operator,
	pub column: Cow<'static, str>,
}

#[derive(Debug)]
pub enum Operator {
	Eq,
	Ne,
	Lt,
	Lte,
	Gt,
	Gte,
	Like,
	In { length: usize },

	// rhs will be ignored
	IsNull,
	// rhs will be ignored
	IsNotNull,
}

#[derive(Debug)]
pub enum WhereIdent {
	Param,
	Name(Cow<'static, str>),
}

impl Where {
	pub fn new() -> Self {
		Self { inner: vec![] }
	}

	pub fn push(&mut self, part: impl Into<WherePart>) {
		self.inner.push(part.into());
	}

	fn is_empty(&self) -> bool {
		self.inner.is_empty()
	}

	pub(crate) fn to_formatter<'a>(&'a self) -> WhereFormatter<'a> {
		WhereFormatter {
			whr: &self,
			param_start: 0,
		}
	}
}

impl<'a> fmt::Display for Where {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.to_formatter().fmt(f)
	}
}

pub(crate) struct WhereFormatter<'a> {
	pub whr: &'a Where,
	/// indexed by zero
	pub param_start: usize,
}

impl<'a> fmt::Display for WhereFormatter<'a> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		if self.whr.is_empty() {
			return Ok(());
		}

		f.write_str(" WHERE ")?;

		let mut param_num = self.param_start;

		for part in &self.whr.inner {
			match part {
				WherePart::And => f.write_str(" AND ")?,
				WherePart::Or => f.write_str(" OR ")?,
				WherePart::Operation(op) => match &op.kind {
					Operator::IsNull | Operator::IsNotNull => {
						write!(f, "\"{}\" {}", op.column, op.kind.as_str())?;
					}
					// handle in special if the length is zero
					// in this case we wan't the query to always return no results
					Operator::In { length } if *length == 0 => {
						write!(f, "1=0")?;
					}
					Operator::In { length } => {
						write!(f, "\"{}\" IN (", op.column)?;

						for i in 0..*length {
							if i != 0 {
								f.write_str(", ")?;
							}

							param_num += 1;
							write!(f, "${}", param_num)?;
						}

						f.write_str(")")?;
					}
					o => {
						param_num += 1;

						write!(
							f,
							"\"{}\" {} ${}",
							op.column,
							o.as_str(),
							param_num
						)?;
					}
				},
			}
		}

		Ok(())
	}
}

impl From<WhereOperation> for WherePart {
	fn from(op: WhereOperation) -> Self {
		Self::Operation(op)
	}
}

impl Operator {
	fn as_str(&self) -> &str {
		match self {
			Operator::Eq => "=",
			Operator::Ne => "!=",
			Operator::Lt => "<",
			Operator::Lte => "<=",
			Operator::Gt => ">",
			Operator::Gte => ">=",
			Operator::Like => "LIKE",
			Operator::In { .. } => "IN",
			Operator::IsNull => "IS NULL",
			Operator::IsNotNull => "IS NOT NULL",
		}
	}
}

#[derive(Debug)]
pub struct OrderBy {
	inner: Vec<OrderByPart>,
}

#[derive(Debug)]
pub enum OrderByPart {
	Asc(Cow<'static, str>),
	Desc(Cow<'static, str>),
}

impl OrderBy {
	pub fn new() -> Self {
		Self { inner: vec![] }
	}

	pub fn push_asc(&mut self, column: impl Into<Cow<'static, str>>) {
		self.inner.push(OrderByPart::Asc(column.into()));
	}

	pub fn push_desc(&mut self, column: impl Into<Cow<'static, str>>) {
		self.inner.push(OrderByPart::Desc(column.into()));
	}

	fn is_empty(&self) -> bool {
		self.inner.is_empty()
	}
}

impl fmt::Display for OrderBy {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		if self.is_empty() {
			return Ok(());
		}

		f.write_str(" ORDER BY ")?;

		for (i, part) in self.inner.iter().enumerate() {
			if i != 0 {
				f.write_str(", ")?;
			}

			match part {
				OrderByPart::Asc(column) => write!(f, "\"{}\" ASC", column)?,
				OrderByPart::Desc(column) => write!(f, "\"{}\" DESC", column)?,
			}
		}

		Ok(())
	}
}

#[derive(Debug)]
pub enum Limit {
	Fixed(usize),
	Param,
	All,
}

impl Limit {
	pub fn new() -> Self {
		Self::All
	}

	pub fn set_param(&mut self) {
		*self = Self::Param;
	}

	pub fn set_fixed(&mut self, value: usize) {
		*self = Self::Fixed(value);
	}
}

#[derive(Debug)]
pub enum Offset {
	Zero,
	Fixed(usize),
	Param,
}

impl Offset {
	pub fn new() -> Self {
		Self::Zero
	}

	pub fn set_param(&mut self) {
		*self = Self::Param;
	}

	pub fn set_fixed(&mut self, value: usize) {
		*self = Self::Fixed(value);
	}
}

#[derive(Debug)]
pub struct Params<'a> {
	inner: Vec<Param<'a>>,
}

impl<'a> Params<'a> {
	pub fn new() -> Self {
		Self { inner: vec![] }
	}

	pub fn push(&mut self, param: Param<'a>) {
		self.inner.push(param);
	}

	pub fn len(&self) -> usize {
		self.inner.len()
	}

	pub fn is_empty(&self) -> bool {
		self.inner.is_empty()
	}

	pub fn iter_to_sql(
		&self,
	) -> impl ExactSizeIterator<Item = &(dyn ToSql + Sync)> {
		self.inner.iter().map(|p| p.data.as_ref())
	}
}

#[derive(Debug)]
#[non_exhaustive]
pub struct Param<'a> {
	// todo is the name still needed?
	pub name: &'static str,
	pub data: CowParamData<'a>,
	is_null: bool,
}

impl<'a> Param<'a> {
	pub fn new<T>(name: &'static str, data: &'a T) -> Self
	where
		T: ParamData + ToSql + Send + Sync,
	{
		Self {
			name,
			is_null: data.is_null(),
			data: CowParamData::Borrowed(data),
		}
	}

	pub fn new_owned<T>(name: &'static str, data: T) -> Self
	where
		T: ParamData + ToSql + Send + Sync + 'static,
	{
		Self {
			name,
			is_null: data.is_null(),
			data: CowParamData::Owned(Box::new(data)),
		}
	}

	pub fn is_null(&self) -> bool {
		self.is_null
	}
}

#[derive(Debug)]
pub enum CowParamData<'a> {
	Borrowed(&'a (dyn ToSql + Send + Sync)),
	Owned(Box<dyn ToSql + Send + Sync>),
}

impl<'a> CowParamData<'a> {
	pub fn as_ref(&self) -> &(dyn ToSql + Sync) {
		match self {
			CowParamData::Borrowed(data) => *data,
			CowParamData::Owned(data) => &**data,
		}
	}
}

pub trait ParamData {
	fn is_null(&self) -> bool;
}

impl<T> ParamData for &T
where
	T: ParamData + ?Sized,
{
	fn is_null(&self) -> bool {
		(**self).is_null()
	}
}

impl<T> ParamData for &mut T
where
	T: ParamData + ?Sized,
{
	fn is_null(&self) -> bool {
		(**self).is_null()
	}
}

impl<T> ParamData for Option<T> {
	fn is_null(&self) -> bool {
		self.is_none()
	}
}

impl<T> ParamData for Vec<T> {
	fn is_null(&self) -> bool {
		// todo should an empty array be considered null?
		false
	}
}

impl<T> ParamData for [T] {
	fn is_null(&self) -> bool {
		// todo should an empty array be considered null?
		false
	}
}

impl<'a, T> ParamData for Cow<'a, T>
where
	T: ToOwned + ParamData,
{
	fn is_null(&self) -> bool {
		(**self).is_null()
	}
}

#[macro_export]
macro_rules! param_not_null {
	($impl_for:ty) => {
		impl $crate::filter::ParamData for $impl_for {
			fn is_null(&self) -> bool {
				false
			}
		}
	};

	($( $impl_for:ty ),*) => {
		$(
			$crate::param_not_null!($impl_for);
		)*
	};
}

param_not_null!(
	String, str, bool, f64, f32, i64, i32, i16, i8, UniqueId, DateTime, Date,
	Timeout
);

#[cfg(feature = "email")]
impl ParamData for email_address::EmailAddress {
	fn is_null(&self) -> bool {
		false
	}
}
