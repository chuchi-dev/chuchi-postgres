/// Possible operators
/// - = | != | < | <= | > | >= | LIKE | IN
/// - AND | OR
///
/// ~ | ~= | =~ are shortcuts for LIKE
/// ## Example
/// ```
/// use chuchi_postgres::filter;
/// let a = "val";
/// let b = "val2".to_string();
/// let c: Option<String> = None;
/// let query = filter!(&a AND "b" != &b OR &c ORDER "a" ASC "b" DESC);
///
/// assert_eq!(r#" WHERE "a" = $1 AND "b" != $2 OR "c" IS NULL ORDER BY "a" ASC, "b" DESC"#, query.to_string());
/// ```
#[macro_export]
macro_rules! filter {
	// order
	(cont; $f:ident, ORDER $value:tt $($tt:tt)*) => ({
		$crate::filter_order!($f, $value);
	});
	// limit
	(cont; $f:ident, LIMIT $value:tt $($tt:tt)*) => ({
		$crate::filter_limit!($f, $value);
	});
	// offset
	(cont; $f:ident, OFFSET $value:tt $($tt:tt)*) => ({
		$crate::filter_offset!($f, $value);
	});
	(cont; $f:ident, $($tt:tt)*) => ({
		$crate::filter_inner!($f, $($tt)*);
	});

	($($tt:tt)*) => ({
		#[allow(unused_mut)]
		let mut f = $crate::filter::Filter::new();
		$crate::filter!(cont; f, $($tt)*);

		f
	});
}

/// Possible operators
/// - = | != | < | <= | > | >= | LIKE | IN
/// - AND | OR
///
/// ~ | ~= | =~ are shortcuts for LIKE
/// ## Example
/// ```
/// use chuchi_postgres::whr;
/// let a = "val";
/// let b = "val2".to_string();
/// let c: Option<String> = None;
/// let query = whr!(&a AND "b" != &b OR &c);
///
/// assert_eq!(r#" WHERE "a" = $1 AND "b" != $2 OR "c" IS NULL"#, query.to_string());
/// ```
#[macro_export]
macro_rules! whr {
	($($tt:tt)*) => ({
		#[allow(unused_mut)]
		let mut f = $crate::filter::WhereFilter::new();
		$crate::filter_inner!(f, $($tt)*);

		f
	});
}

#[doc(hidden)]
#[macro_export]
macro_rules! filter_inner {
	($f:ident,) => ();

	// nested
	($f:ident, ($($tt:tt)+) $($rest:tt)*) => ({
		// we need to swap the where
		let mut prev_where = std::mem::take(&mut $f.whr);
		$crate::filter_inner!($f, $($tt)*);
		std::mem::swap(&mut $f.whr, &mut prev_where);
		$f.whr.push($crate::filter::WherePart::Nested(prev_where));
		$crate::whr_log!($f, $($rest)*);
	});

	// reference ident eq
	($f:ident, &$id:ident $($tt:tt)*) => (
		$crate::whr_comp!($f, stringify!($id), Eq, &$id $($tt)*);
	);
	// ident eq
	($f:ident, $id:ident $($tt:tt)*) => (
		$crate::whr_comp!($f, stringify!($id), Eq, $id $($tt)*);
	);

	// eq
	($f:ident, $name:literal = $($tt:tt)+) => (
		$crate::whr_comp!($f, $name, Eq, $($tt)*);
	);
	// ne
	($f:ident, $name:literal != $($tt:tt)+) => (
		$crate::whr_comp!($f, $name, Ne, $($tt)*);
	);
	// lt
	($f:ident, $name:literal < $($tt:tt)+) => (
		$crate::whr_comp!($f, $name, Lt, $($tt)*);
	);
	// lte
	($f:ident, $name:literal <= $($tt:tt)+) => (
		$crate::whr_comp!($f, $name, Lte, $($tt)*);
	);
	// gt
	($f:ident, $name:literal > $($tt:tt)+) => (
		$crate::whr_comp!($f, $name, Gt, $($tt)*);
	);
	// gte
	($f:ident, $name:literal >= $($tt:tt)+) => (
		$crate::whr_comp!($f, $name, Gte, $($tt)*);
	);
	// like
	($f:ident, $name:literal LIKE $($tt:tt)+) => (
		$crate::whr_comp!($f, $name, Like, $($tt)*);
	);
	// like %val%
	($f:ident, $name:literal ~ $($tt:tt)+) => (
		$crate::whr_comp!($f, $name, ~, $($tt)*);
	);
	// like %val
	($f:ident, $name:literal ~= $($tt:tt)+) => (
		$crate::whr_comp!($f, $name, ~=, $($tt)*);
	);
	// like val%
	($f:ident, $name:literal =~ $($tt:tt)+) => (
		$crate::whr_comp!($f, $name, =~, $($tt)*);
	);
	// in
	($f:ident, $name:literal IN $($tt:tt)+) => (
		$crate::whr_comp_in!($f, $name, $($tt)*);
	);
}

#[doc(hidden)]
#[macro_export]
macro_rules! whr_comp {
	($f:ident, $name:expr, $symb:tt, &$value:tt $($tt:tt)*) => (
		$crate::whr_comp!(symb; $f, $name, $symb, &$value, $($tt)*);
	);
	($f:ident, $name:expr, $symb:tt, $value:tt $($tt:tt)*) => (
		$crate::whr_comp!(symb; $f, $name, $symb, $value, $($tt)*);
	);

	(symb; $f:ident, $name:expr, ~, $value:expr, $($tt:tt)*) => (
		let param = $crate::filter::Param::new_owned($name, format!("%{}%", $value));
		$crate::whr_comp!(fin; $f, param, Like, $($tt)*);
	);
	(symb; $f:ident, $name:expr, ~=, $value:expr, $($tt:tt)*) => (
		let param = $crate::filter::Param::new_owned($name, format!("%{}", $value));
		$crate::whr_comp!(fin; $f, param, Like, $($tt)*);
	);
	(symb; $f:ident, $name:expr, =~, $value:expr, $($tt:tt)*) => (
		let param = $crate::filter::Param::new_owned($name, format!("{}%", $value));
		$crate::whr_comp!(fin; $f, param, Like, $($tt)*);
	);
	(symb; $f:ident, $name:expr, $symb:ident, $value:expr, $($tt:tt)*) => (
		let param = $crate::filter::Param::new($name, $value);
		$crate::whr_comp!(fin; $f, param, $symb, $($tt)*);
	);
	(fin; $f:ident, $param:expr, $symb:ident, $($tt:tt)*) => (
		let symb = $crate::filter::Operator::$symb;

		let mut cont = true;

		// todo: can this become a noop
		if $param.is_null() {
			match symb {
				$crate::filter::Operator::Eq => {
					$f.whr.push($crate::filter::WhereOperation {
						kind: $crate::filter::Operator::IsNull,
						column: $param.name.into()
					});
					cont = false;
				},
				$crate::filter::Operator::Ne => {
					$f.whr.push($crate::filter::WhereOperation {
						kind: $crate::filter::Operator::IsNotNull,
						column: $param.name.into()
					});
					cont = false;
				},
				_ => {}
			}
		}

		if cont {
			$f.whr.push($crate::filter::WhereOperation {
				kind: symb,
				column: $param.name.into()
			});
			$f.params.push($param);
		}

		$crate::whr_log!($f, $($tt)*);
	);
	// LIMIT
}

#[doc(hidden)]
#[macro_export]
macro_rules! whr_comp_in {
	($f:ident, $name:expr, &$value:tt $($tt:tt)*) => (
		$crate::whr_comp_in!(two; $f, $name, &$value, $($tt)*);
	);
	($f:ident, $name:expr, $value:tt $($tt:tt)*) => (
		$crate::whr_comp_in!(two; $f, $name, $value, $($tt)*);
	);

	(two; $f:ident, $name:expr, $value:expr, $($tt:tt)*) => (
		{
			let mut c = 0;
			for val in $value {
				c += 1;
				let param = $crate::filter::Param::new($name, val);
				$f.params.push(param);
			}

			$f.whr.push($crate::filter::WhereOperation {
				kind: $crate::filter::Operator::In { length: c },
				column: $name.into()
			});
		}

		$crate::whr_log!($f, $($tt)*);
	);
}

#[doc(hidden)]
#[macro_export]
macro_rules! whr_log {
	($f:ident, AND $($tt:tt)+) => (
		$f.whr.push($crate::filter::WherePart::And);
		$crate::filter_inner!($f, $($tt)+);
	);
	($f:ident, OR $($tt:tt)+) => (
		$f.whr.push($crate::filter::WherePart::Or);
		$crate::filter_inner!($f, $($tt)+);
	);

	($f:ident, ORDER $($tt:tt)+) => (
		$crate::filter_order!($f, $($tt)+);
	);
	($f:ident, LIMIT $($tt:tt)+) => (
		$crate::filter_limit!($f, $($tt)+);
	);
	($f:ident, OFFSET $($tt:tt)+) => (
		$crate::filter_offset!($f, $($tt)+);
	);
	($f:ident,) => ();
}

#[doc(hidden)]
#[macro_export]
macro_rules! filter_order {
	($f:ident, $name:literal DESC $($tt:tt)*) => (
		$f.order_by.push_desc($name);
		$crate::filter_order!($f, $($tt)*);
	);
	($f:ident, $name:literal ASC $($tt:tt)*) => (
		$f.order_by.push_asc($name);
		$crate::filter_order!($f, $($tt)*);
	);
	($f:ident, LIMIT $($tt:tt)+) => (
		$crate::filter_limit!($f, $($tt)+);
	);
	($f:ident, OFFSET $($tt:tt)+) => (
		$crate::filter_offset!($f, $($tt)+);
	);
	($f:ident,) => ();
}

#[doc(hidden)]
#[macro_export]
macro_rules! filter_limit {
	($f:ident, &$value:ident $($tt:tt)*) => (
		$crate::filter_limit!(val; $f, stringify!($value), &$value, $($tt)*);
	);
	($f:ident, $value:ident $($tt:tt)*) => (
		$crate::filter_limit!(val; $f, stringify!($value), $value, $($tt)*);
	);
	($f:ident, $value:literal $($tt:tt)*) => (
		$f.limit.set_fixed($value);

		$crate::filter_limit!(next; $f, $($tt)*);
	);

	(val; $f:ident, $name:expr, $value:expr, $($tt:tt)*) => (
		let param = $crate::filter::Param::new($name, $value);
		$f.limit.set_param();
		$f.params.push(param);

		$crate::filter_limit!(next; $f, $($tt)*);
	);

	(next; $f:ident, OFFSET $($tt:tt)+) => (
		$crate::filter_offset!($f, $($tt)*);
	);
	(next; $f:ident,) => ();
}

#[doc(hidden)]
#[macro_export]
macro_rules! filter_offset {
	($f:ident, &$value:ident $($tt:tt)*) => (
		$crate::filter_offset!(val; $f, stringify!($value), &$value, $($tt)*);
	);
	($f:ident, $value:ident $($tt:tt)*) => (
		$crate::filter_offset!(val; $f, stringify!($value), $value, $($tt)*);
	);
	($f:ident, $value:literal $($tt:tt)*) => (
		$f.offset.set_fixed($value);

		$crate::filter_offset!(next; $f, $($tt)*);
	);

	(val; $f:ident, $name:literal, $value:expr, $($tt:tt)*) => (
		let param = $crate::filter::Param::new($name, $value);
		$f.offset.set_param();
		$f.params.push(param);

		$crate::filter_offset!(next; $f, $($tt)*);
	);

	(next; $f:ident,) => ();
}

#[cfg(test)]
mod tests {
	use crate::UniqueId;

	#[test]
	fn test_simple_eq() {
		let id = &UniqueId::new();
		let id2 = &UniqueId::new();
		let query = filter!(id OR "id" != &id2);
		assert_eq!(query.to_string(), r#" WHERE "id" = $1 OR "id" != $2"#);
	}

	#[test]
	fn test_simple_like() {
		let id = "str";
		let query = filter!("id" ~ id);
		assert_eq!(query.to_string(), r#" WHERE "id" LIKE $1"#);
	}

	#[test]
	fn test_limit() {
		let id = &UniqueId::new();
		let limit = 10;
		let query = filter!(id LIMIT &limit);
		assert_eq!(query.to_string(), " WHERE \"id\" = $1 LIMIT $2");
	}

	#[test]
	fn test_and_or() {
		let user_id = &UniqueId::new();
		let id1 = &UniqueId::new();
		let id2 = &UniqueId::new();
		let id3 = &UniqueId::new();
		let query =
			filter!(user_id AND ("id" != &id1 OR "id" = &id2) OR "id" = &id3);
		assert_eq!(
			query.to_string(),
			r#" WHERE "user_id" = $1 AND ("id" != $2 OR "id" = $3) OR "id" = $4"#
		);
	}

	// #[test]
	// fn test_order() {
	// 	let id = &UniqueId::new();
	// 	let limit = 10;
	// 	let query = whr!(id ORDER "id" ASC LIMIT limit);
	// 	assert_eq!(
	// 		query.sql.to_string().trim(),
	// 		"\"id\" = $1 ORDER BY \"id\" ASC  LIMIT 10"
	// 	);
	// 	let query = whr!(ORDER "id" DESC);
	// 	assert_eq!(query.sql.to_string().trim(), "ORDER BY \"id\" DESC");
	// }
}
