//! Derive macro for `#[derive(Encodable, Decodable)]`.

#![no_std]

extern crate alloc;
extern crate proc_macro;

mod de;
mod en;

use de::*;
use en::*;
use proc_macro::TokenStream;

#[proc_macro_derive(Encodable)]
pub fn encodable(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    let gen = impl_encodable(&ast);
    gen.into()
}

#[proc_macro_derive(EncodableWrapper)]
pub fn encodable_wrapper(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    let gen = impl_encodable_wrapper(&ast);
    gen.into()
}

#[proc_macro_derive(Decodable)]
pub fn decodable(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    let gen = impl_decodable(&ast);
    gen.into()
}

#[proc_macro_derive(DecodableWrapper)]
pub fn decodable_wrapper(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    let gen = impl_decodable_wrapper(&ast);
    gen.into()
}
