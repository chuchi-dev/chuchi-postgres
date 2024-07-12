mod column_type;
pub use column_type::ColumnType;

#[derive(Debug, Clone, PartialEq)]
pub struct Column {
	pub name: &'static str,
	pub kind: ColumnKind,
	pub index: IndexKind,
}

impl Column {
	pub fn new<T>(
		name: &'static str,
		len: Option<usize>,
		index: IndexKind,
	) -> Self
	where
		T: ColumnType,
	{
		let mut kind = T::column_kind();
		if let Some(len) = len {
			kind = match kind {
				ColumnKind::Text => ColumnKind::Varchar(len),
				_ => panic!(
					"column kind {:?} doenst support len attribute",
					kind
				),
			};
		}

		Self { name, kind, index }
	}
}

/*
ToColumnType
*/

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColumnKind {
	Boolean,
	// Char(usize),
	Varchar(usize),
	FixedText(usize),
	Text,
	Date,
	Timestamp,
	F64,
	F32,
	I64,
	I32,
	I16,
	Option(Box<ColumnKind>),
	TextArray,
	Bytea,
	Json,
}

impl ColumnKind {
	pub fn short(&self) -> &'static str {
		match self {
			Self::Boolean => "boolean",
			// Self::Char(_) => "char",
			Self::Varchar(_) => "varchar",
			Self::FixedText(_) => "text",
			Self::Text => "text",
			Self::Date => "date",
			Self::Timestamp => "timestamp",
			Self::F64 => "float8",
			Self::F32 => "float4",
			Self::I64 => "int8",
			Self::I32 => "int4",
			Self::I16 => "int2",
			Self::Option(t) => t.short(),
			Self::TextArray => "text []",
			Self::Bytea => "bytea",
			Self::Json => "json",
		}
	}

	pub fn value(&self, name: &str) -> String {
		match self {
			// Self::Char(v) => Some(v.to_string()),
			Self::Varchar(v) => format!("({})", v),
			Self::FixedText(v) => format!(" CHECK (length({})={})", name, v),
			Self::Option(t) => t.value(name),
			_ => String::new(),
		}
	}

	pub fn to_string(&self, name: &str) -> String {
		format!("{}{}", self.short(), self.value(name))
	}

	pub fn not_null_str(&self) -> &'static str {
		match self {
			Self::Option(_) => "null",
			_ => "not null",
		}
	}
}

#[derive(Debug, Clone, PartialEq)]
pub enum IndexKind {
	Primary,
	Unique,
	NamedUnique(&'static str),
	Index,
	None,
}

impl IndexKind {
	pub fn is_none(&self) -> bool {
		matches!(self, Self::None)
	}
}

/*
CREATE TABLE account(
   user_id serial PRIMARY KEY,
   username VARCHAR (50) UNIQUE NOT NULL,
   password VARCHAR (50) NOT NULL,
   email VARCHAR (355) UNIQUE NOT NULL,
   created_on TIMESTAMP NOT NULL,
   last_login TIMESTAMP
);

// UNIQUE

// INDEX

CREATE TABLE account_role
(
  user_id integer NOT NULL,
  role_id integer NOT NULL,
  grant_date timestamp without time zone,
  PRIMARY KEY (user_id, role_id),
  CONSTRAINT account_role_role_id_fkey FOREIGN KEY (role_id)
	  REFERENCES role (role_id) MATCH SIMPLE
	  ON UPDATE NO ACTION ON DELETE NO ACTION,
  CONSTRAINT account_role_user_id_fkey FOREIGN KEY (user_id)
	  REFERENCES account (user_id) MATCH SIMPLE
	  ON UPDATE NO ACTION ON DELETE NO ACTION
);
*/

/*
all types

bigint	int8	signed eight-byte integer
bigserial	serial8	autoincrementing eight-byte integer
bit [ (n) ]	 	fixed-length bit string
varbit [ (n) ]	variable-length bit string
boolean	bool	logical Boolean (true/false)
box	 	rectangular box on a plane
bytea	 	binary data ("byte array")
char [ (n) ]	fixed-length character string
varchar [ (n) ]	variable-length character string
cidr	 	IPv4 or IPv6 network address
circle	 	circle on a plane
date	 	calendar date (year, month, day)
float8	double precision floating-point number (8 bytes)
inet	 	IPv4 or IPv6 host address
integer	int, int4	signed four-byte integer
interval [ fields ] [ (p) ]	 	time span
json	 	textual JSON data
jsonb	 	binary JSON data, decomposed
line	 	infinite line on a plane
lseg	 	line segment on a plane
macaddr	 	MAC (Media Access Control) address
money	 	currency amount
numeric [ (p, s) ]	decimal [ (p, s) ]	exact numeric of selectable precision
path	 	geometric path on a plane
pg_lsn	 	PostgreSQL Log Sequence Number
point	 	geometric point on a plane
polygon	 	closed geometric path on a plane
real	float4	single precision floating-point number (4 bytes)
smallint	int2	signed two-byte integer
smallserial	serial2	autoincrementing two-byte integer
serial	serial4	autoincrementing four-byte integer
text	 	variable-length character string
time [ (p) ] [ without time zone ]	 	time of day (no time zone)
time [ (p) ] with time zone	timetz	time of day, including time zone
timestamp [ (p) ] [ without time zone ]	 	date and time (no time zone)
timestamp [ (p) ] with time zone	timestamptz	date and time, including time zone
tsquery	 	text search query
tsvector	 	text search document
txid_snapshot	 	user-level transaction ID snapshot
uuid	 	universally unique identifier
xml	 	XML data
*/
