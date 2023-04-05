use alloc::vec::Vec;
use proc_macro2::TokenStream;
use quote::quote;

pub fn impl_encodable(ast: &syn::DeriveInput) -> TokenStream {
    let body = if let syn::Data::Struct(s) = &ast.data {
        s
    } else {
        panic!("#[derive(Encodable)] is only defined for structs.");
    };

    let stmts: Vec<_> = body
        .fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let ident = field_ident(index, field);

            let id = quote! { self.#ident };

            quote! { cser::Encodable::encode(&#id, out); }
        })
        .collect();
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let impl_block = quote! {
        impl #impl_generics cser::Encodable for #name #ty_generics #where_clause {
            fn encode(&self, out: &mut cser::Writer) {
                #(#stmts)*
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

pub fn impl_encodable_wrapper(ast: &syn::DeriveInput) -> TokenStream {
    let body = if let syn::Data::Struct(s) = &ast.data {
        s
    } else {
        panic!("#[derive(EncodableWrapper)] is only defined for structs.");
    };

    let ident = {
        let fields: Vec<_> = body.fields.iter().collect();
        if fields.len() == 1 {
            let field = fields.first().expect("fields.len() == 1; qed");
            field_ident(0, field)
        } else {
            panic!("#[derive(EncodableWrapper)] is only defined for structs with one field.")
        }
    };

    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let impl_block = quote! {
        impl #impl_generics cser::Encodable for #name #ty_generics #where_clause {
            fn encode(&self, out: &mut cser::Writer) {
                cser::Encodable::encode(&self.#ident, out)
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

fn field_ident(index: usize, field: &syn::Field) -> TokenStream {
    if let Some(ident) = &field.ident {
        quote! { #ident }
    } else {
        let index = syn::Index::from(index);
        quote! { #index }
    }
}
