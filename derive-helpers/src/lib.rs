use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{
	parse::{Parse, ParseStream}, punctuated::Punctuated,
	Token, braced, parse_macro_input, Expr, Pat, Result
};

#[proc_macro]
pub fn iter_over_fields(input: TokenStream) -> TokenStream {
	let IterOverFields {
		pattern,
		iter,
		binds,
		iter_definitions
	} = parse_macro_input!(input as IterOverFields);

	let iter_name = Ident::new("iter", Span::call_site());

	let binds = binds.into_iter()
		.map(|Bind {name, expr}| quote! {
			#[allow(unused_variables)]
			let #name = #expr;
		});

	let body = iter_definitions.iter()
		.map(|IterDefinition {name, expr}| {
			let binds = binds.clone();
			quote! {
				let #name = #iter_name.clone()
					.map(|#pattern| {
						#(#binds)*

						#expr
					});
			}
		});

	quote! {
		let #iter_name = #iter;
		#(#body)*
	}.into()
}

struct IterOverFields {
	pattern: Pat,
	iter: Expr,
	binds: Punctuated<Bind, Token![,]>,
	iter_definitions: Punctuated<IterDefinition, Token![;]>
}

impl Parse for IterOverFields {
	fn parse(input: ParseStream) -> Result<Self> {
		input.parse::<Token![for]>()?;
		let pattern = input.parse()?;
		input.parse::<Token![in]>()?;
		let iter = input.parse()?;

		let binds = if let Some(_) = input.parse::<Option<Token![where]>>()? {
			Punctuated::parse_separated_nonempty(input)?
		} else {
			Punctuated::new()
		};

		let braces;
		braced!(braces in input);
		let iter_definitions =
			Punctuated::parse_terminated(&braces)?;

		Ok(Self {
			pattern,
			iter,
			binds,
			iter_definitions
		})
	}
}

#[derive(Clone)]
struct Bind {
	name: Pat,
	expr: Expr
}

impl Parse for Bind {
	fn parse(input: ParseStream) -> Result<Self> {
		let name = input.parse()?;
		input.parse::<Token![=]>()?;
		let expr = input.parse()?;

		Ok(Self {name, expr})
	}
}

struct IterDefinition {
	name: Pat,
	expr: Expr
}

impl Parse for IterDefinition {
	fn parse(input: ParseStream) -> Result<Self> {
		input.parse::<Token![let]>()?;
		let name = input.parse()?;
		input.parse::<Token![=]>()?;
		let expr = input.parse()?;

		Ok(Self {name, expr})
	}
}
