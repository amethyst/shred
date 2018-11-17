#![recursion_limit = "256"]

extern crate proc_macro;
extern crate proc_macro2;
#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;

use proc_macro::TokenStream;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{
    Data, DataStruct, DeriveInput, Field, Fields, FieldsNamed, FieldsUnnamed, Ident, Type,
    WhereClause, WherePredicate,
};

/// Used to `#[derive]` the trait `SystemData`.
#[proc_macro_derive(SystemData)]
pub fn system_data(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();

    let gen = impl_system_data(&ast);

    gen.into()
}

fn impl_system_data(ast: &DeriveInput) -> proc_macro2::TokenStream {
    let name = &ast.ident;
    let mut generics = ast.generics.clone();

    let (fetch_return, tys) = gen_from_body(&ast.data, name);
    let tys = &tys;

    {
        let where_clause = generics.make_where_clause();
        constrain_system_data_types(where_clause, tys);
    }
    // Reads and writes are taken from the same types,
    // but need to be cloned before.

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics
            ::shred::SystemData
            for #name #ty_generics #where_clause
        {
            fn setup(res: &mut ::shred::Resources) {
                #(
                    <#tys as ::shred::SystemData> :: setup(res);
                )*
            }

            fn fetch<'r>(res: & 'r ::shred::Resources) -> ::shred::SystemFetch<'r, Self> {
                ::shred::SystemFetch::from(#fetch_return)
            }

            fn reads() -> Vec<::shred::ResourceId> {
                let mut r = Vec::new();

                #( {
                        let mut reads = <#tys as ::shred::SystemData> :: reads();
                        r.append(&mut reads);
                    } )*

                r
            }

            fn writes() -> Vec<::shred::ResourceId> {
                let mut r = Vec::new();

                #( {
                        let mut writes = <#tys as ::shred::SystemData> :: writes();
                        r.append(&mut writes);
                    } )*

                r
            }
        }
    }
}

fn collect_field_types(fields: &Punctuated<Field, Comma>) -> Vec<Type> {
    fields.iter().map(|x| x.ty.clone()).collect()
}

fn gen_identifiers(fields: &Punctuated<Field, Comma>) -> Vec<Ident> {
    fields.iter().map(|x| x.ident.clone().unwrap()).collect()
}

/// Adds a `::shred::SystemData<'lt>` bound on each of the system data types.
fn constrain_system_data_types(clause: &mut WhereClause, tys: &[Type]) {
    for ty in tys.iter() {
        let where_predicate: WherePredicate = parse_quote!(#ty : ::shred::SystemData);
        clause.predicates.push(where_predicate);
    }
}

fn gen_from_body(ast: &Data, name: &Ident) -> (proc_macro2::TokenStream, Vec<Type>) {
    enum DataType {
        Struct,
        Tuple,
    }

    let (body, fields) = match *ast {
        Data::Struct(DataStruct {
            fields: Fields::Named(FieldsNamed { named: ref x, .. }),
            ..
        }) => (DataType::Struct, x),
        Data::Struct(DataStruct {
            fields: Fields::Unnamed(FieldsUnnamed { unnamed: ref x, .. }),
            ..
        }) => (DataType::Tuple, x),
        _ => panic!("Enums are not supported"),
    };

    let tys = collect_field_types(fields);

    let fetch_return = match body {
        DataType::Struct => {
            let identifiers = gen_identifiers(fields);

            quote! {
                #name {
                    #( #identifiers: ::shred::SystemData::fetch(res).into_inner() ),*
                }
            }
        }
        DataType::Tuple => {
            let count = tys.len();
            let fetch = vec![quote! { ::shred::SystemData::fetch(res).into_inner() }; count];

            quote! {
                #name ( #( #fetch ),* )
            }
        }
    };

    (fetch_return, tys)
}
