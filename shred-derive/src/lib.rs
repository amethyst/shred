#![recursion_limit="128"]

extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use syn::{Body, Field, Ident, Lifetime, LifetimeDef, MacroInput, PathParameters, PathSegment, Ty,
          TyParam, VariantData};
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
    let lifetime_defs = &ast.generics.lifetimes;
    let ty_params = &ast.generics.ty_params;
    let fields = get_fields(ast);

    let identifiers = gen_identifiers(fields);
    let fetch_lt = gen_fetch_lifetime(fields);
    let def_lt_tokens = gen_def_lt_tokens(lifetime_defs);
    let impl_lt_tokens = gen_impl_lt_tokens(lifetime_defs);
    let def_ty_params = gen_def_ty_params(ty_params);
    let impl_ty_params = gen_impl_ty_params(ty_params);
    let methods = gen_methods(fields);
    let reads = collect_field_ty_params(fields, "Fetch");
    let writes = collect_field_ty_params(fields, "FetchMut");

    quote! {
        impl< #def_lt_tokens , #def_ty_params >
            ::shred::SystemData< #fetch_lt >
            for #name< #impl_lt_tokens , #impl_ty_params >
        {
            fn fetch(res: & #fetch_lt ::shred::Resources) -> Self {
                #name {
                    #( #identifiers: res.#methods(()), )*
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

fn gen_fetch_lifetime(fields: &Vec<Field>) -> Lifetime {
    let field: &Field = fields
        .iter()
        .next()
        .expect("There has to be at least one field");

    match field.ty {
        Ty::Path(_, ref path) => {
            let segment: &PathSegment = path.segments.last().unwrap();
            match segment.parameters {
                PathParameters::AngleBracketed(ref data) => {
                    let ref lifetimes = data.lifetimes;

                    assert!(lifetimes.len() == 1,
                            "Fetch / FetchMut must have exactly one lifetime");

                    lifetimes[0].clone()
                }
                _ => {
                    panic!("No parenthesized brackets supported");
                }
            }
        }
        _ => {
            panic!("Only paths supported");
        }
    }
}

fn gen_def_lt_tokens(lifetime_defs: &Vec<LifetimeDef>) -> Tokens {
    let lts: Vec<Tokens> = lifetime_defs
        .iter()
        .map(|x| {
                 let ref lt = x.lifetime;
                 let ref bounds = x.bounds;

                 quote! { #lt: #( #bounds )+* }
             })
        .collect();

    quote! { #( #lts ),* }
}

fn gen_impl_lt_tokens(lifetime_defs: &Vec<LifetimeDef>) -> Tokens {
    let lts: Vec<Lifetime> = lifetime_defs
        .iter()
        .map(|x| x.lifetime.clone())
        .collect();

    quote! { #( #lts ),* }
}

fn gen_def_ty_params(ty_params: &Vec<TyParam>) -> Tokens {
    let ty_params: Vec<Tokens> = ty_params
        .iter()
        .map(|x| {
                 let ref ty = x.ident;
                 let ref bounds = x.bounds;

                 quote! { #ty: #( #bounds )+* }
             })
        .collect();

    quote! { #( #ty_params ),* }
}

fn gen_impl_ty_params(ty_params: &Vec<TyParam>) -> Tokens {
    let ty_params: Vec<Ident> = ty_params.iter().map(|x| x.ident.clone()).collect();

    quote! { #( #ty_params ),* }
}

fn gen_methods(fields: &Vec<Field>) -> Vec<Ident> {
    fields
        .iter()
        .map(|x| match x.ty {
                 Ty::Path(_, ref path) => path.segments.last().unwrap().clone().ident,
                 _ => panic!("Only Fetch and FetchMut types allowed"),
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
