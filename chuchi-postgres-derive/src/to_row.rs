use std::fmt::Write;

use ::quote::{quote, ToTokens};

use syn::Fields;
use syn::{DeriveInput, Error};

use proc_macro2::TokenStream;

type Result<T> = std::result::Result<T, Error>;

macro_rules! err {
	($input:expr, $msg:expr) => {
		Error::new_spanned($input.into_token_stream(), $msg)
	};
}

pub fn expand_to_row(
	input: &DeriveInput,
	name: &TokenStream,
) -> Result<proc_macro::TokenStream> {
	match &input.data {
		syn::Data::Struct(data) => match &data.fields {
			Fields::Named(fields) => {
				let (impl_gens, ty_gens, where_clause) =
					input.generics.split_for_impl();

				let row = quote!(#name::row);
				let macros = quote!(#name::macros);
				let ident = &input.ident;

				let mut insert_columns = String::new();
				let mut insert_values = String::new();
				let mut update_columns = String::new();
				let mut params = quote!();

				for (i, field) in fields.named.iter().enumerate() {
					let ident = &field.ident;
					let ident_str = ident.as_ref().unwrap().to_string();

					if !insert_columns.is_empty() {
						insert_columns.push_str(", ");
						insert_values.push_str(", ");
						update_columns.push_str(", ");
					}
					let i = i + 1;
					write!(&mut insert_columns, "\"{ident_str}\"").unwrap();
					write!(&mut insert_values, "${i}").unwrap();
					write!(&mut update_columns, "\"{ident_str}\" = ${i}")
						.unwrap();

					params.extend(quote!(
						&self.#ident as &(dyn #macros::ToSql + std::marker::Sync),
					));
				}

				let fields_len = fields.named.len();
				let toks = quote!(
					impl #impl_gens #row::ToRowStatic for #ident #ty_gens #where_clause {
						fn insert_columns() -> &'static str {
							#insert_columns
						}

						fn insert_values() -> &'static str {
							#insert_values
						}

						fn update_columns() -> &'static str {
							#update_columns
						}

						fn params_len() -> usize {
							#fields_len
						}

						fn params(&self) -> impl std::iter::ExactSizeIterator<Item=&(dyn #macros::ToSql + std::marker::Sync)> {
							[
								#params
							].into_iter()
						}
					}
				);

				Ok(toks.into())
			}
			f => Err(err!(f, "not supported")),
		},
		_ => Err(err!(input, "is not supported")),
	}
}
