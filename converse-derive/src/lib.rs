#![recursion_limit="512"]
extern crate proc_macro;

use proc_macro2::TokenStream;
use quote::quote;
use syn;

mod common;
mod server;
mod client;
mod structure;

#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn Converse(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {

    let ast: syn::Item = syn::parse(item).unwrap();

    let item_impl = match ast {
        syn::Item::Impl(ref x) => { x },
        _ => panic!("Server attribute must be placed on an impl!")
    };

    let server = server::Server::new(&item_impl, attr.to_string()).tokens();
    let client = client::Client::new(&item_impl, attr.to_string()).tokens();;

    let tokens = quote! {
        #ast
        #server
        #client
    };

    println!("{}", tokens);

    tokens.into()
}

