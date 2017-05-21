#![recursion_limit="128"]

extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use syn::{Body, Field, Ident, Lifetime, LifetimeDef, MacroInput, TyParam, VariantData};
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
    // Assumes that the first lifetime is the fetch lt
    let fetch_lt = lifetime_defs
        .iter()
        .next()
        .expect("There has to be at least one lifetime");
    let def_lt_tokens = gen_def_lt_tokens(lifetime_defs);
    let impl_lt_tokens = gen_impl_lt_tokens(lifetime_defs);
    let def_ty_params = gen_def_ty_params(ty_params);
    let impl_ty_params = gen_impl_ty_params(ty_params);
    // Reads and writes are taken from the same types,
    // but need to be cloned before.
    let reads = collect_field_types(fields);
    let writes = reads.clone();

    quote! {
        impl< #def_lt_tokens , #def_ty_params >
            ::shred::SystemData< #fetch_lt >
            for #name< #impl_lt_tokens , #impl_ty_params >
        {
            fn fetch(res: & #fetch_lt ::shred::Resources) -> Self {
                #name {
                    #( #identifiers: ::shred::SystemData::fetch(res), )*
                }
            }

            unsafe fn reads() -> Vec<::shred::ResourceId> {
                let mut r = Vec::new();

                #( {
                        let mut reads = <#reads as ::shred::SystemData> :: reads();
                        r.append(&mut reads);
                    } )*

                r
            }

            unsafe fn writes() -> Vec<::shred::ResourceId> {
                let mut r = Vec::new();

                #( {
                        let mut writes = <#writes as ::shred::SystemData> :: writes();
                        r.append(&mut writes);
                    } )*

                r
            }
        }
    }
}

fn collect_field_types(fields: &Vec<Field>) -> Vec<Tokens> {
    fields
        .iter()
        .map(|x| x.ty.clone())
        .map(|x| quote! { #x })
        .collect()
}

fn gen_identifiers(fields: &Vec<Field>) -> Vec<Ident> {
    fields.iter().map(|x| x.ident.clone().unwrap()).collect()
}

fn gen_def_lt_tokens(lifetime_defs: &Vec<LifetimeDef>) -> Tokens {
    let lts: Vec<Tokens> = lifetime_defs
        .iter()
        .map(|x| {
            let ref lt = x.lifetime;
            let ref bounds = x.bounds;

            if bounds.is_empty() {
                quote! { #lt }
            } else {
                quote! { #lt: #( #bounds )+* }
            }
        })
        .collect();

    quote! { #( #lts ),* }
}

fn gen_impl_lt_tokens(lifetime_defs: &Vec<LifetimeDef>) -> Tokens {
    let lts: Vec<Lifetime> = lifetime_defs.iter().map(|x| x.lifetime.clone()).collect();

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

fn get_fields(ast: &MacroInput) -> &Vec<Field> {
    match ast.body {
        Body::Struct(VariantData::Struct(ref x)) => x,
        _ => panic!("Only structs with named fields supported"),
    }
}
