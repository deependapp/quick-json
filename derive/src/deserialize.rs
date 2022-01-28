use super::crate_name;
use proc_macro2::{TokenStream, Ident, Span};
use quote::quote;
use syn::{
	punctuated::Punctuated, Token,
	Data, DataEnum, DataStruct, DeriveInput, Field, Fields, FieldsNamed,
	FieldsUnnamed, Variant, LitStr
};
use qj_derive_helpers::iter_over_fields;

fn impl_struct(name: &Ident, fields: Punctuated<Field, Token![,]>)
		-> TokenStream {
	let crate_ = crate_name();

	iter_over_fields! {
		for (index, field) in fields.iter().enumerate() where
				variable = Ident::new(&format!("field_{}", index), Span::call_site()),
				ty = &field.ty,
				field = field.ident.as_ref().unwrap(),
				field_str = LitStr::new(&field.to_string(), field.span()) {
			let field_definitions = quote! {let mut #variable: Option<Option<#ty>> = None;};
			let field_name_match = quote! {#field_str => #variable = Some(::#crate_::deserialize::r#trait::Deserialize::deserialize(value, error_context)?)};
			let field_presence_tuple = quote! {#variable};
			let field_presence_partial_match = quote! {Some(_)};
			let field_presence_full_match = quote! {Some(Some(#variable))};
			let field_presence_action = quote! {#field: #variable};
		}
	}

	quote! {
		match value {
			ValueDeserializer::Object(mut object) => {
				#(#field_definitions)*

				while let Some(entry) = object.next_entry()? {
					let (name, value) = entry.accept()?;
					::#crate_::deserialize::error::ErrorContext::push_key(error_context, KeyKind::Object(name.clone()));
					match name.as_ref() {
						#(#field_name_match,)*
						_ => ()
					}
					::#crate_::deserialize::error::ErrorContext::pop_key(error_context);
				}

				match (#(#field_presence_tuple),*) {
					(#(#field_presence_full_match),*) => Ok(Some(#name {
						#(#field_presence_action),*
					})),
					(#(#field_presence_partial_match),*) => Ok(None),
					_ => {
						::#crate_::deserialize::error::ErrorContext::report_missing_fields(error_context);
						Ok(None)
					}
				}
			},
			unexpected => {
				::#crate_::deserialize::error::ErrorContext::report_unexpected_type(error_context, unexpected.kind(), &[JSONType::Object]);
				Ok(None)
			}
		}
	}
}

fn impl_tuple_struct(_fields: Punctuated<Field, Token![,]>) -> TokenStream {
	todo!()
}

fn impl_enum(_variants: Punctuated<Variant, Token![,]>) -> TokenStream {
	todo!()
}

pub fn derive(item: DeriveInput) -> TokenStream {
	let DeriveInput {ident: name, data, ..} = item;
	let crate_ = crate_name();

	let deserialize = match data {
		Data::Struct(DataStruct {fields, ..}) => match fields {
			Fields::Named(FieldsNamed {named, ..}) =>
				impl_struct(&name, named),
			Fields::Unnamed(FieldsUnnamed {unnamed, ..}) =>
				impl_tuple_struct(unnamed),
			Fields::Unit =>
				todo!()
		},
		Data::Enum(DataEnum {variants, ..}) =>
			impl_enum(variants),
		Data::Union(_) =>
			todo!()
	};

	quote! {
		type Result<T, E> =
			::core::result::Result<T, E>;
		type Option<T> =
			::core::option::Option<T>;

		type ValueDeserializer<'d, 's> =
			::#crate_::deserialize::ValueDeserializer<'d, 's>;

		type JSONType =
			::#crate_::deserialize::error::JSONType;
		type SyntaxError =
			::#crate_::deserialize::error::SyntaxError;
		type KeyKind<'s> =
			::#crate_::deserialize::error::KeyKind<'s>;

		#[automatically_derived]
		impl<'s, E> ::#crate_::deserialize::r#trait::Deserialize<'s, E> for #name
				where E: ::#crate_::deserialize::error::ErrorContext<'s> {
			fn deserialize<'d>(value: ValueDeserializer<'d, 's>,
					error_context: &mut E) -> Result<Option<Self>, SyntaxError> {
				#deserialize
			}
		}
	}
}
