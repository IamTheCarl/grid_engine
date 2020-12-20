// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! Procedural macros for generating code in the grid_engine_wasm_api crate.

#![warn(missing_docs)]

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{export::Span, parse_macro_input, Abi, ExprArray, Ident, ItemFn, LitStr, ReturnType, Token, Visibility};

/// Tag the function you wish to be your entry point with this.
/// It is expected to take no arguments, and return no arguments. Use this entry point function to
/// load global assets and config.
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

/// Create a list of chunk entities provided by this mod.
#[proc_macro]
pub fn chunk_entities(input: TokenStream) -> TokenStream {
    let list = syn::parse::<ExprArray>(input).unwrap();

    let length = list.elems.len();
    let items = list.elems.to_token_stream();

    TokenStream::from(quote! {
        static __CHUNK_ENTITY_INITIALIZERS: [(&'static str, fn() -> Box<dyn ChunkEntity>); #length] = [#items];

        #[no_mangle]
        fn __get_chunk_entity_initializer(type_id: u32) -> fn() -> Box<dyn ChunkEntity> {
            assert!((type_id as usize) < __CHUNK_ENTITY_INITIALIZERS.len());
            __CHUNK_ENTITY_INITIALIZERS[type_id as usize].1
        }

        #[no_mangle]
        fn __register_chunk_entity_initializer_names() {

            // Functions provided by the host.
            #[link(wasm_import_module = "grid_api")]
            extern "C" {
                fn __register_chunk_entity_initializer_name(name: *const u8, name_len: usize);
            }

            for (name, _constructor) in &__CHUNK_ENTITY_INITIALIZERS {
                log::info!("Registering chunk entity: {}", name);
                unsafe {
                    __register_chunk_entity_initializer_name(name.as_bytes().as_ptr(), name.len());
                }
            }
        }
    })
}
