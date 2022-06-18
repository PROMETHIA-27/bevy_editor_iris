use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(ClientMessage)]
pub fn derive_client(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, .. } = parse_macro_input!(input);
    quote! {
        impl ClientMessage for #ident {
            fn any(self: Box<Self>) -> Box<dyn Any> {
                self
            }

            fn any_ref(&self) -> &dyn Any {
                self
            }

            fn any_mut(&mut self) -> &mut dyn Any {
                self
            }

            fn reflect(self: Box<Self>) -> Box<dyn Reflect> {
                self
            }

            fn borrow_reflect(&self) -> &dyn Reflect {
                self
            }

            fn borrow_reflect_mut(&mut self) -> &mut dyn Reflect {
                self
            }
        }
    }
    .into()
}

#[proc_macro_derive(EditorMessage)]
pub fn derive_editor(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, .. } = parse_macro_input!(input);
    quote! {
        impl EditorMessage for #ident {
            fn any(self: Box<Self>) -> Box<dyn Any> {
                self
            }

            fn any_ref(&self) -> &dyn Any {
                self
            }

            fn any_mut(&mut self) -> &mut dyn Any {
                self
            }

            fn reflect(self: Box<Self>) -> Box<dyn Reflect> {
                self
            }

            fn borrow_reflect(&self) -> &dyn Reflect {
                self
            }

            fn borrow_reflect_mut(&mut self) -> &mut dyn Reflect {
                self
            }
        }
    }
    .into()
}

#[proc_macro_attribute]
pub fn dual_message(_params: TokenStream, item: TokenStream) -> TokenStream {
    let item: proc_macro2::TokenStream = item.into();
    TokenStream::from(quote! {
        #[derive(Reflect, FromReflect, ClientMessage, EditorMessage)]
        #[reflect(ClientMessage, EditorMessage, MessageFromReflect)]
        #item
    })
}

#[proc_macro_attribute]
pub fn client_message(_params: TokenStream, item: TokenStream) -> TokenStream {
    let item: proc_macro2::TokenStream = item.into();
    TokenStream::from(quote! {
        #[derive(Reflect, FromReflect, ClientMessage)]
        #[reflect(ClientMessage, MessageFromReflect)]
        #item
    })
}

#[proc_macro_attribute]
pub fn editor_message(_params: TokenStream, item: TokenStream) -> TokenStream {
    let item: proc_macro2::TokenStream = item.into();
    TokenStream::from(quote! {
        #[derive(Reflect, FromReflect, EditorMessage)]
        #[reflect(EditorMessage, MessageFromReflect)]
        #item
    })
}
