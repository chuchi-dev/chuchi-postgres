use std::fmt::Write;

use proc_macro2::{Span, TokenStream};
use syn::{
	parse::{Parse, ParseStream},
	punctuated::Punctuated,
	Error, Expr, ExprPath, ExprReference, Ident, LitStr, Token,
};

use ::quote::quote;

/*
row! {
	abc,
	&bcd,
	"key": val,
	"key": &val,
}
*/

type Result<T> = std::result::Result<T, Error>;

pub struct RowInput {
	pub fields: Punctuated<RowField, Token![,]>,
}

impl Parse for RowInput {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		Ok(Self {
			fields: input.parse_terminated(RowField::parse, Token![,])?,
		})
	}
}

pub struct RowField {
	pub name: String,
	pub name_span: Span,
	pub value: Expr,
}

impl Parse for RowField {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		// possible values &ident, ident, "key": val, "key": &val

		// check if we have an ident with a reference or without
		// or we have a string
		let lookhead = input.lookahead1();

		if lookhead.peek(LitStr) {
			let name: LitStr = input.parse()?;
			input.parse::<Token![:]>()?;
			let value = input.parse()?;

			Ok(Self {
				name: name.value(),
				name_span: name.span(),
				value,
			})
		} else if lookhead.peek(Token![&]) {
			let and_token = input.parse::<Token![&]>()?;
			let name: Ident = input.parse()?;

			Ok(Self {
				name: name.to_string(),
				name_span: name.span(),
				// no an expr with a reference which holds the name has we just parsed
				value: Expr::Reference(ExprReference {
					attrs: vec![],
					and_token,
					mutability: None,
					expr: Box::new(Expr::Path(ExprPath {
						attrs: vec![],
						qself: None,
						path: name.into(),
					})),
				}),
			})
		} else if lookhead.peek(Ident) {
			let name: Ident = input.parse()?;

			Ok(Self {
				name: name.to_string(),
				name_span: name.span(),
				// no an expr with a reference which holds the name has we just parsed
				value: Expr::Path(ExprPath {
					attrs: vec![],
					qself: None,
					path: name.into(),
				}),
			})
		} else {
			Err(lookhead.error())
		}
	}
}

pub fn expand_row(
	input: &RowInput,
	name: &TokenStream,
) -> Result<proc_macro::TokenStream> {
	// let's build a local_struct with the fields
	let row = quote!(#name::row);
	let macros = quote!(#name::macros);

	let mut insert_columns = String::new();
	let mut insert_values = String::new();
	let mut update_columns = String::new();
	let mut struct_fields = quote!();
	let mut params = quote!();
	let mut struct_init = quote!();

	for (i, field) in input.fields.iter().enumerate() {
		let name = &field.name;
		let ident = Ident::new(&name, field.name_span);
		let value = &field.value;

		if !insert_columns.is_empty() {
			insert_columns.push_str(", ");
			insert_values.push_str(", ");
			update_columns.push_str(", ");
		}
		let i = i + 1;
		write!(&mut insert_columns, "\"{name}\"").unwrap();
		write!(&mut insert_values, "${i}").unwrap();
		write!(&mut update_columns, "\"{name}\" = ${i}").unwrap();

		struct_fields.extend(quote!(
			#ident: &'a (dyn #macros::ToSql + std::marker::Sync),
		));
		params.extend(quote!(
			self.#ident,
		));
		//  as &(dyn #macros::ToSql + std::marker::Sync)
		struct_init.extend(quote!(
			#ident: &#value,
		));
	}

	let fields_len = input.fields.len();
	let toks = quote!(
		{
			struct __RowStruct<'a> {
				#struct_fields
			}

			impl #row::ToRowStatic for __RowStruct<'_> {
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

			&__RowStruct {
				#struct_init
			}
		}
	);

	Ok(toks.into())
}
