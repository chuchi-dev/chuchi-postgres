use super::column::{Column, IndexKind};

pub fn info_data_to_sql(name: &str, data: &[Column]) -> String {
	let mut primary_indexes = vec![];
	let mut normal_indexes = vec![];
	let mut unique_indexes = vec![]; // (name, vec![])

	let mut cols_sql = vec![];

	for col in data {
		let kind = col.kind.to_string(col.name);
		let not_null = col.kind.not_null_str();
		let quoted_name = quote(col.name);

		cols_sql.push(format!("{} {} {}", quoted_name, kind, not_null));

		match col.index {
			IndexKind::Primary => primary_indexes.push(quoted_name),
			IndexKind::Unique => {
				unique_indexes.push((col.name, vec![quoted_name]))
			}
			IndexKind::NamedUnique(n) => 'match_arm: loop {
				for ind in unique_indexes.iter_mut() {
					if ind.0 == n {
						ind.1.push(quoted_name);
						break 'match_arm;
					}
				}
				unique_indexes.push((n, vec![quoted_name]));
				break;
			},
			IndexKind::Index => normal_indexes.push(col.name),
			IndexKind::None => {}
		}
	}

	cols_sql.push(format!("PRIMARY KEY ({})", primary_indexes.join(", ")));
	for ind in unique_indexes {
		cols_sql.push(format!("UNIQUE ({})", ind.1.join(", ")));
	}

	let mut sqls = vec![format!(
		"CREATE TABLE IF NOT EXISTS \"{}\" ({})",
		name,
		cols_sql.join(", ")
	)];

	for ind in normal_indexes {
		let index_name = format!("{}_{}_nidx", name, ind);
		sqls.push(format!(
			"CREATE INDEX IF NOT EXISTS {} ON \"{}\" (\"{}\")",
			index_name, name, ind
		));
	}

	sqls.join("; ")
}

pub fn quote(s: &str) -> String {
	format!("\"{}\"", s)
}

// maybe this is not important because table name can't be -
/*pub fn sanitize_index_name(name: String) -> String {
	name
		.replace("-", "_")
		.replace(" ", "_")
}*/
