use crate::format::Parser;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum Node {
    Record(String),
    Namespace(HashMap<String, Node>),
}

fn generate_string_library_code(node: &Node) -> TokenStream {
    let mut prefix = vec![];
    generate_code_for(node, &mut prefix)
}

fn generate_code_for(node: &Node, prefix: &mut Vec<String>) -> TokenStream {
    let children = match node {
        Node::Record(_) => return TokenStream::new(),
        Node::Namespace(x) => x,
    };
    let struct_name = format_ident!(
        "Strings{}{}",
        if prefix.is_empty() { "" } else { "__" },
        prefix.join("__"),
    );
    let mut fields = TokenStream::new();
    let mut new_fields = TokenStream::new();
    let mut substructs = TokenStream::new();
    let mut impls = TokenStream::new();
    for (name, child) in children.iter() {
        check_name(name);
        let name_ident = format_ident!("{}", &name);
        match child {
            Node::Namespace(_) => {
                let type_ident = format_ident!("{}__{}", &struct_name, &name);
                fields.extend(quote! {
                    pub #name_ident: #type_ident,
                });
                new_fields.extend(quote! {
                    #name_ident: #type_ident::new(),
                });
                prefix.push(String::from(name));
                substructs.extend(generate_code_for(child, prefix));
                prefix.pop();
            }
            Node::Record(s) => {
                let (code, num_params) = Parser::new(s).parse().unwrap().generate_code();
                let mut params_code = TokenStream::new();
                for i in 0..num_params {
                    let param_ident = format_ident!("param_{}", i + 1);
                    params_code.extend(quote! {
                        #param_ident: &(impl ::std::fmt::Display + ?::std::marker::Sized),
                    });
                }
                impls.extend(quote! {
                    pub fn #name_ident(&self, #params_code) -> crate::message::FormattedText {
                        #code
                    }
                });
            }
        }
    }
    let code = quote! {
        #[allow(non_camel_case_names)]
        pub struct #struct_name {
            #fields
        }

        impl #struct_name {
            const fn new() -> Self {
                Self {
                    #new_fields
                }
            }
            #impls
        }

        #substructs
    };
    code.into()
}

fn check_name(name: &str) {
    if !name
        .chars()
        .all(|c| ('a'..='z').contains(&c) || c.is_ascii_digit() || c == '_')
    {
        panic!("Invalid char");
    }
    if name.contains("__") {
        panic!("Name contains double underscore (`__`)");
    }
}

pub fn generate_library_from_yaml(path: &(impl AsRef<Path> + ?Sized)) -> TokenStream {
    let reader = BufReader::new(File::open(path).unwrap());
    let node = serde_yaml::from_reader(reader).expect("here");
    generate_string_library_code(&node)
}
