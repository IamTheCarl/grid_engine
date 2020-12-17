// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

use proc_macro::TokenStream;
use quote::quote;
use syn::{export::Span, parse_macro_input, Abi, Ident, ItemFn, LitStr, ReturnType, Token, Visibility};

#[proc_macro_attribute]
pub fn entry_point(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut function = parse_macro_input!(input as ItemFn);

    // Fail if the user is weird and tries to give us arguments.
    if !args.is_empty() {
        panic!("This attribute accepts no arguments.");
    }

    // Check that the signature of whatever function we've been given is a valid one.
    let valid_signature = function.sig.constness.is_none()
        && function.vis == Visibility::Inherited
        && function.sig.abi.is_none()
        && function.sig.inputs.is_empty()
        && function.sig.generics.params.is_empty()
        && function.sig.generics.where_clause.is_none()
        && function.sig.variadic.is_none()
        && match function.sig.output {
            // Return type should be nothing.
            ReturnType::Default => true,
            ReturnType::Type(_, _) => false,
        };

    if valid_signature {
        // Valid function signature, cool.
    } else {
        // We had a bad function signature.
        panic!("Function must have signature #[entry_point] fn() -> ()")
    }

    // Set the function name.
    function.sig.ident = Ident::new("__user_entry_point", Span::call_site());

    // Set it to be an extern "C" function.
    function.sig.abi =
        Some(Abi { extern_token: Token![extern](Span::call_site()), name: Some(LitStr::new("C", Span::call_site())) });

    TokenStream::from(quote! {
        #[no_mangle]
        #function
    })
}
