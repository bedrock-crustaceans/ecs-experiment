use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token, Attribute, ExprStruct, Field, Ident, ItemStruct, Path, Token,
};

#[proc_macro_derive(Component)]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as ItemStruct);

    let ItemStruct {
        attrs,
        vis,
        ident,
        fields,
        ..
    } = input;

    let expanded = quote! {
        // #(#attrs)*
        // #vis struct #ident {
        //     #fields
        // }

        impl Component for #ident {}
    };

    TokenStream::from(expanded)
}
