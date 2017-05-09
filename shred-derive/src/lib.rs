#![recursion_limit="128"]

extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use syn::{Body, Field, Ident, MacroInput, VariantData};
use quote::Tokens;

/// Used to `#[derive]` the trait
/// `SystemData`.
#[proc_macro_derive(SystemData)]
pub fn system_data(input: TokenStream) -> TokenStream {
    let s = input.to_string();
    let ast = syn::parse_macro_input(&s).unwrap();

    let gen = impl_system_data(&ast);

    gen.parse().expect("Invalid")
}

fn impl_system_data(ast: &MacroInput) -> Tokens {
    let name = &ast.ident;
    let fields = get_fields(ast);

    let identifiers = gen_identifiers(fields);
    let methods = gen_methods(fields);
    let reads = collect_field_ty_params(fields, "Fetch");
    let writes = collect_field_ty_params(fields, "FetchMut");

    quote! {
        impl<'a> ::shred::SystemData<'a> for #name<'a> {
            fn fetch(res: &'a ::shred::Resources) -> #name<'a> {
                #name {
                    #( #identifiers: unsafe { res.#methods(()) }, )*
                }
            }

            unsafe fn reads() -> Vec<::shred::ResourceId> {
                #![allow(unused_imports)] // In case there is no read

                use std::any::TypeId;

                vec![ #( (TypeId::of::<#reads>(), 14695981039346656037) ),* ]
            }

            unsafe fn writes() -> Vec<::shred::ResourceId> {
                #![allow(unused_imports)] // In case there is no write

                use std::any::TypeId;

                vec![ #( (TypeId::of::<#writes>(), 14695981039346656037) ),* ]
            }
        }
    }
}

fn collect_field_ty_params(fields: &Vec<Field>, with_type: &str) -> Vec<Tokens> {
    fields
        .iter()
        .filter_map(|x| {
            use syn::{PathParameters, PathSegment, Ty};

            let field: &Field = x;

            if let Ty::Path(_, ref path) = field.ty {
                let last: PathSegment = path.segments.last().unwrap().clone();

                if last.ident.as_ref() == with_type {
                    if let PathParameters::AngleBracketed(x) = last.parameters {
                        let ref ty: Ty = x.types[0];

                        return Some(quote! { #ty });
                    }
                } else {
                    return None;
                }
            }

            panic!("Should have panicked already")
        })
        .collect()
}

fn gen_identifiers(fields: &Vec<Field>) -> Vec<Ident> {
    fields
        .iter()
        .map(|x| x.ident.clone().unwrap())
        .collect()
}

fn gen_methods(fields: &Vec<Field>) -> Vec<Ident> {
    fields
        .iter()
        .map(|x| {
            use syn::Ty;

            match x.ty {
                Ty::Path(_, ref path) => path.segments.last().unwrap().clone().ident,
                _ => panic!("Only Fetch and FetchMut types allowed"),
            }
        })
        .map(|x| {
            match x.as_ref() {
                "Fetch" => "fetch",
                "FetchMut" => "fetch_mut",
                _ => panic!("Only Fetch and FetchMut supported"),
            }
                .into()
        })
        .collect()
}

fn get_fields(ast: &MacroInput) -> &Vec<Field> {
    match ast.body {
        Body::Struct(VariantData::Struct(ref x)) => x,
        _ => panic!("Only structs with named fields supported"),
    }
}
