#![deny(missing_docs)]
//! Procedural macros for bevy_editor_derive.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// Derives the Message trait automatically.
#[proc_macro_derive(Message)]
pub fn derive_message(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, .. } = parse_macro_input!(input);
    quote! {
        impl Message for #ident {}
    }
    .into()
}

/// Derives and reflects all necessary traits to use a type as a message.
#[proc_macro_attribute]
pub fn message(_params: TokenStream, item: TokenStream) -> TokenStream {
    let item: proc_macro2::TokenStream = item.into();
    TokenStream::from(quote! {
        #[derive(Reflect, FromReflect, Message)]
        #[reflect(Message, MessageFromReflect)]
        #item
    })
}
