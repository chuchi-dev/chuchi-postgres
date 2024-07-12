use ::quote::{quote, ToTokens};

use syn::{DeriveInput, Error};
use syn::{Field, Fields, FieldsNamed, Type};

use proc_macro2::TokenStream;

type Result<T> = std::result::Result<T, Error>;

macro_rules! err {
	($input:expr, $msg:expr) => {
		Error::new_spanned($input.into_token_stream(), $msg)
	};
}

pub fn expand_table_templ(
	input: &DeriveInput,
	name: &TokenStream,
) -> Result<proc_macro::TokenStream> {
	match &input.data {
		syn::Data::Struct(data) => match &data.fields {
			Fields::Named(fields) => {
				let ident = &input.ident;

				let info_block = parse_named_fields(fields, name)?;

				let table = quote!(#name::table);
				let toks = quote!(
					impl #table::TableTemplate for #ident {
						fn table_info() -> #table::Info {
							{ #info_block }
						}
					}
				);

				Ok(toks.into())
			}
			_ => Err(err!(input, "only named fields are supported")),
		},
		_ => Err(err!(input, "only structs with named fields are supported")),
	}
}

fn parse_named_fields(
	fields: &FieldsNamed,
	name: &TokenStream,
) -> Result<TokenStream> {
	let len = fields.named.len();
	let mut info_stream = quote!(
		let mut info = #name::table::Info::with_capacity(#len);
	);

	for field in fields.named.iter() {
		let col = parse_named_field(field, name)?;
		info_stream.extend(quote!(info.push(#col);));
	}

	info_stream.extend(quote!(info));

	Ok(info_stream)
}

fn parse_named_field(
	field: &Field,
	crate_name: &TokenStream,
) -> Result<TokenStream> {
	let name = field.ident.as_ref().unwrap().to_string(); // TODO

	let mut len = quote!(None);
	let table = quote!(#crate_name::table);
	let index_kind = quote!(#table::column::IndexKind);
	let mut index = quote!(#index_kind::None);

	// this is name: Type
	// should build Attributes
	// println!("parse named ident: {:?} attr: {:?}", name, field.attrs);
	for attr in &field.attrs {
		match &attr.path() {
			p if p.is_ident("len") => {
				let res: syn::LitInt = attr.parse_args()?;
				len = quote!(Some(#res));
			}
			p if p.is_ident("index") => {
				let res: syn::Ident = attr.parse_args()?;
				let index_str = res.to_string();
				index = match index_str.as_str() {
					"primary" => quote!(#index_kind::Primary),
					"unique" => quote!(#index_kind::Unique),
					"index" => quote!(#index_kind::Index),
					_ => return Err(err!(res, "not supported index type")),
				};
			}
			p if p.is_ident("unique") => {
				let res: syn::Ident = attr.parse_args()?;
				let index_str = res.to_string();
				index = quote!(#index_kind::NamedUnique(#index_str));
			}
			_ => {}
		}
	}

	let ty = match &field.ty {
		Type::Path(t) => t,
		t => return Err(err!(t, "only type path is supported")),
	};

	let col = quote!(#table::column::Column::new::<#ty>(#name, #len, #index));

	Ok(col)
}
