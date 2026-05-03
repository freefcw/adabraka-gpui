use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

pub fn derive_render(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let type_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = ast.generics.split_for_impl();

    let r#gen = quote! {
        impl #impl_generics adabraka_gpui::Render for #type_name #type_generics
        #where_clause
        {
            fn render(&mut self, _window: &mut adabraka_gpui::Window, _cx: &mut adabraka_gpui::Context<Self>) -> impl adabraka_gpui::Element {
                adabraka_gpui::Empty
            }
        }
    };

    r#gen.into()
}
