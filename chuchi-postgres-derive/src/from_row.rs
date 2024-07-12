use std::fmt::Write;

use ::quote::{quote, ToTokens};

use syn::{DeriveInput, Error};
use syn::{Fields, FieldsNamed, FieldsUnnamed};
use syn::{Lifetime, LifetimeParam};

use proc_macro2::{Span, TokenStream};

type Result<T> = std::result::Result<T, Error>;

macro_rules! err {
	($input:expr, $msg:expr) => {
		Error::new_spanned($input.into_token_stream(), $msg)
	};
}

pub fn expand_from_row(
	input: &DeriveInput,
	name: &TokenStream,
) -> Result<proc_macro::TokenStream> {
	match &input.data {
		syn::Data::Struct(data) => {
			let (input_impl_gens, ty_gens, where_clause) =
				input.generics.split_for_impl();

			let mut n_gens;

			// from lifetime
			let (impl_gens, from_lifetime) = {
				// get the first lifetime
				n_gens = input.generics.clone();
				let first_lifetime = n_gens.lifetimes().next();

				let lifetime = if let Some(first_lifetime) = first_lifetime {
					first_lifetime.lifetime.clone()
				} else {
					let lifetime = Lifetime::new("'r", Span::call_site());
					n_gens
						.params
						.push(LifetimeParam::new(lifetime.clone()).into());

					lifetime
				};

				let (impl_gens, _, _) = n_gens.split_for_impl();

				(impl_gens, lifetime)
			};

			let ident = &input.ident;

			match &data.fields {
				Fields::Named(fields) => {
					let (select_columns, from_named_fields) =
						parse_named_fields(fields)?;

					let row = quote!(#name::row);
					let toks = quote!(
						impl #impl_gens #row::FromRow<#from_lifetime> for #ident #ty_gens #where_clause {
							fn from_row(
								row: &#from_lifetime #row::Row
							) -> std::result::Result<Self, Box<dyn std::error::Error + Sync + Send>> {
								Ok(Self {
									#from_named_fields
								})
							}
						}

						impl #input_impl_gens #row::NamedColumns for #ident #ty_gens #where_clause {
							fn select_columns() -> &'static str {
								#select_columns
							}
						}
					);

					Ok(toks.into())
				}
				Fields::Unnamed(fields) => {
					let from_unnamed_fields = parse_unnamed_fields(fields)?;

					let row = quote!(#name::row);
					let toks = quote!(
						impl #impl_gens #row::FromRow<#from_lifetime> for #ident #ty_gens #where_clause {
							fn from_row(
								row: &#from_lifetime #row::Row
							) -> std::result::Result<Self, Box<dyn std::error::Error + Sync + Send>> {
								Ok(Self(
									#from_unnamed_fields
								))
							}
						}
					);

					Ok(toks.into())
				}
				f => Err(err!(f, "not supported")),
			}
		}
		_ => Err(err!(input, "is not supported")),
	}
}

fn parse_named_fields(
	fields: &FieldsNamed,
) -> Result<(TokenStream, TokenStream)> {
	let mut select_columns = String::new();
	let mut from_stream = quote!();

	for field in fields.named.iter() {
		let ident = &field.ident;
		let ident_str = ident.as_ref().unwrap().to_string();

		if !select_columns.is_empty() {
			select_columns.push_str(", ");
		}
		write!(&mut select_columns, "\"{ident_str}\"").unwrap();

		from_stream.extend(quote!(
			#ident: row.try_get(#ident_str)?,
		));
	}

	Ok((quote!(#select_columns), from_stream))
}

fn parse_unnamed_fields(fields: &FieldsUnnamed) -> Result<TokenStream> {
	let mut from_stream = quote!();

	for (i, _) in fields.unnamed.iter().enumerate() {
		from_stream.extend(quote!(
			row.try_get(#i)?,
		));
	}

	Ok(from_stream)
}
