mod deserialize;

use self::deserialize::derive;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

fn crate_name() -> Ident {
	Ident::new("qj", Span::call_site())
}

#[proc_macro_derive(Deserialize)]
pub fn derive_deserialize(item: TokenStream) -> TokenStream {
	let output = derive(parse_macro_input!(item as DeriveInput));

	TokenStream::from(quote! {
		const _: () = {
			#output

			()
		};
	})
}
