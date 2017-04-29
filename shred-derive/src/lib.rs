#![recursion_limit="128"]

extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use syn::{Body, Field, Ident, VariantData};
use quote::Tokens;

#[proc_macro_derive(TaskData)]
pub fn task_data(input: TokenStream) -> TokenStream {
    let s = input.to_string();
    let ast = syn::parse_macro_input(&s).unwrap();

    let gen = impl_task_data(&ast);

    //panic!("Debug: {}", gen.into_string());
    gen.parse().expect("Invalid")
}

fn impl_task_data(ast: &syn::MacroInput) -> Tokens {
    let name = &ast.ident;

    let fields: &Vec<Field> = match ast.body {
        Body::Struct(VariantData::Struct(ref x)) => x,
        _ => panic!("Only structs with named fields supported"),
    };

    // TODO: CLEAN UP and better error messages

    let identifier: Vec<_> = fields
        .iter()
        .map(|x| x.ident.clone().unwrap())
        .collect();
    let method: Vec<Ident> = fields
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
        .collect();

    let reads: Vec<_> = fields
        .iter()
        .filter_map(|x| {
            use syn::{PathParameters, PathSegment, Ty};

            let field: &Field = x;

            if let Ty::Path(_, ref path) = field.ty {
                let last: PathSegment = path.segments.last().unwrap().clone();

                if last.ident.as_ref() == "Fetch" {
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
        .collect();

    let writes: Vec<_> = fields
        .iter()
        .filter_map(|x| {
            use syn::{PathParameters, PathSegment, Ty};

            let field: &Field = x;

            if let Ty::Path(_, ref path) = field.ty {
                let last: PathSegment = path.segments.last().unwrap().clone();

                if last.ident.as_ref() == "FetchMut" {
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
        .collect();

    quote! {
        impl<'a> ::shred::TaskData<'a> for #name<'a> {
            fn fetch(res: &'a ::shred::Resources) -> #name<'a> {
                #name {
                    #( #identifier: unsafe { res.#method() }, )*
                }
            }

            unsafe fn reads() -> Vec<::shred::ResourceId> {
                use std::any::TypeId;

                vec![ #( TypeId::of::<#reads>() ),* ]
            }

            unsafe fn writes() -> Vec<::shred::ResourceId> {
                use std::any::TypeId;

                vec![ #( TypeId::of::<#writes>() ),* ]
            }
        }
    }
}
