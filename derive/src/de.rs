use alloc::vec::Vec;
use proc_macro2::TokenStream;
use quote::quote;

pub fn impl_decodable(ast: &syn::DeriveInput) -> TokenStream {
    let body = if let syn::Data::Struct(s) = &ast.data {
        s
    } else {
        panic!("#[derive(Decodable)] is only defined for structs.");
    };

    let stmts: Vec<_> = body
        .fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let id = if let Some(ident) = &field.ident {
                quote! { #ident }
            } else {
                let index = syn::Index::from(index);
                quote! { #index }
            };

            quote! { #id: cser::Decodable::decode(input)?, }
        })
        .collect();
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let impl_block = quote! {
        impl #impl_generics cser::Decodable for #name #ty_generics #where_clause {
            type Error = anyhow::Error;

            fn decode(mut input: &mut cser::Reader) -> Result<Self, anyhow::Error> {
                let this = Self {
                    #(#stmts)*
                };
                ::core::result::Result::Ok(this)
            }
        }
    };

    quote! {
        const _: () = {
            extern crate cser;
            #impl_block
        };
    }
}

pub fn impl_decodable_wrapper(ast: &syn::DeriveInput) -> TokenStream {
    let body = if let syn::Data::Struct(s) = &ast.data {
        s
    } else {
        panic!("#[derive(DecodableWrapper)] is only defined for structs.");
    };

    assert_eq!(
        body.fields.iter().count(),
        1,
        "#[derive(DecodableWrapper)] is only defined for structs with one field."
    );

    let wrapped_ty = &body.fields.iter().next().unwrap().ty;

    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let impl_block = quote! {
        impl #impl_generics cser::Decodable for #name #ty_generics #where_clause {
            type Error = <#wrapped_ty as cser::Decodable>::Error;

            fn decode(buf: &mut cser::Reader<'_>) -> Result<Self, Self::Error> {
                <#wrapped_ty as cser::Decodable>::decode(buf).map(Self)
            }
        }
    };

    quote! {
        const _: () = {
            extern crate cser;
            #impl_block
        };
    }
}
