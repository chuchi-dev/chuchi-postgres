use super::column::Column;

#[derive(Debug, Clone)]
pub struct Info {
	data: Vec<Column>,
}

impl Info {
	pub fn new(data: Vec<Column>) -> Self {
		Self { data }
	}

	pub fn with_capacity(cap: usize) -> Self {
		Self {
			data: Vec::with_capacity(cap),
		}
	}

	pub fn push(&mut self, col: Column) {
		self.data.push(col);
	}

	pub fn data(&self) -> &Vec<Column> {
		&self.data
	}

	pub fn names<'a>(
		&'a self,
	) -> impl ExactSizeIterator<Item = &'static str> + 'a {
		self.data.iter().map(|v| v.name)
	}
}

#[derive(Debug, Clone)]
pub struct ValidateParamsError(pub String);
