//! Proc-macros for removing boilerplate within the grid engine.

#![warn(missing_docs)]

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// Implement the event trait on a structure.
#[proc_macro_derive(Event)]
pub fn hello_macro_derive(input: TokenStream) -> TokenStream {
    let structure = parse_macro_input!(input as DeriveInput);
    let name = structure.ident;

    // Build the trait implementation
    let gen = quote! {
        impl crate::world::Event for #name {
            fn type_name() -> String {
                String::from(stringify!(#name))
            }
        }
    };

    gen.into()
}
