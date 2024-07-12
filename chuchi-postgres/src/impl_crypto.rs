#[cfg(feature = "crypto-cipher")]
mod cipher {
	use crate::{
		param_not_null,
		table::column::{ColumnKind, ColumnType},
	};
	use crypto::cipher::{Keypair, PublicKey};

	impl ColumnType for Keypair {
		fn column_kind() -> ColumnKind {
			ColumnKind::FixedText(43)
		}
	}

	impl ColumnType for PublicKey {
		fn column_kind() -> ColumnKind {
			ColumnKind::FixedText(43)
		}
	}

	param_not_null!(Keypair, PublicKey);
}

#[cfg(feature = "crypto-signature")]
mod signature {
	use crate::{
		param_not_null,
		table::column::{ColumnKind, ColumnType},
	};
	use crypto::signature::{Keypair, PublicKey, Signature};

	impl ColumnType for Keypair {
		fn column_kind() -> ColumnKind {
			ColumnKind::FixedText(43)
		}
	}

	impl ColumnType for PublicKey {
		fn column_kind() -> ColumnKind {
			ColumnKind::FixedText(43)
		}
	}

	impl ColumnType for Signature {
		fn column_kind() -> ColumnKind {
			ColumnKind::FixedText(86)
		}
	}

	param_not_null!(Keypair, PublicKey, Signature);
}

#[cfg(feature = "crypto-token")]
mod token {
	use crate::{
		filter::ParamData,
		table::column::{ColumnKind, ColumnType},
	};
	use crypto::token::Token;

	impl<const S: usize> ColumnType for Token<S> {
		fn column_kind() -> ColumnKind {
			ColumnKind::FixedText(Self::STR_LEN)
		}
	}

	impl<const S: usize> ParamData for Token<S> {
		fn is_null(&self) -> bool {
			false
		}
	}
}
