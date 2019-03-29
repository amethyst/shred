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
    Data, DataStruct, DeriveInput, Field, Fields, FieldsNamed, FieldsUnnamed, Ident, Lifetime,
    Type, WhereClause, WherePredicate,
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
    // Assumes that the first lifetime is the fetch lt
    let def_fetch_lt = ast
        .generics
        .lifetimes()
        .next()
        .expect("There has to be at least one lifetime");
    let ref impl_fetch_lt = def_fetch_lt.lifetime;

    {
        let where_clause = generics.make_where_clause();
        constrain_system_data_types(where_clause, impl_fetch_lt, tys);
    }
    // Reads and writes are taken from the same types,
    // but need to be cloned before.

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics
            SystemData< #impl_fetch_lt >
            for #name #ty_generics #where_clause
        {
            fn setup(world: &mut World) {
                #(
                    <#tys as SystemData> :: setup(world);
                )*
            }

            fn fetch(world: & #impl_fetch_lt World) -> Self {
                #fetch_return
            }

            fn reads() -> Vec<ResourceId> {
                let mut r = Vec::new();

                #( {
                        let mut reads = <#tys as SystemData> :: reads();
                        r.append(&mut reads);
                    } )*

                r
            }

            fn writes() -> Vec<ResourceId> {
                let mut r = Vec::new();

                #( {
                        let mut writes = <#tys as SystemData> :: writes();
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

/// Adds a `SystemData<'lt>` bound on each of the system data types.
fn constrain_system_data_types(clause: &mut WhereClause, fetch_lt: &Lifetime, tys: &[Type]) {
    for ty in tys.iter() {
        let where_predicate: WherePredicate = parse_quote!(#ty : SystemData< #fetch_lt >);
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
                    #( #identifiers: SystemData::fetch(world) ),*
                }
            }
        }
        DataType::Tuple => {
            let count = tys.len();
            let fetch = vec![quote! { SystemData::fetch(world) }; count];

            quote! {
                #name ( #( #fetch ),* )
            }
        }
    };

    (fetch_return, tys)
}
