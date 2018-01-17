#![recursion_limit = "256"]

extern crate proc_macro;
#[macro_use]
extern crate quote;
extern crate syn;

use proc_macro::TokenStream;
use quote::Tokens;
use syn::{Data, DataStruct, DeriveInput, Fields, GenericParam, Generics, Ident, Lifetime,
          LifetimeDef, Type, TypeParam, WhereClause};

/// Used to `#[derive]` the trait
/// `SystemData`.
#[proc_macro_derive(SystemData)]
pub fn system_data(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();

    let gen = impl_system_data(&ast);

    gen.into()
}

fn impl_system_data(ast: &DeriveInput) -> Tokens {
    let name = &ast.ident;

    let (ty_params, lifetime_defs, where_clause) = parse_generic(&ast.generics);

    let (fetch_return, tys) = gen_from_body(&ast.data, name);
    let tys = &tys;
    // Assumes that the first lifetime is the fetch lt
    let def_fetch_lt = lifetime_defs
        .iter()
        .next()
        .expect("There has to be at least one lifetime");
    let ref impl_fetch_lt = def_fetch_lt.lifetime;
    let def_lt_tokens = gen_def_lt_tokens(&lifetime_defs);
    let impl_lt_tokens = gen_impl_lt_tokens(&lifetime_defs);
    let def_ty_params = gen_def_ty_params(&ty_params);
    let impl_ty_params = gen_impl_ty_params(&ty_params);
    let where_clause = gen_where_clause(where_clause, impl_fetch_lt, tys);
    // Reads and writes are taken from the same types,
    // but need to be cloned before.

    quote! {
        impl< #def_lt_tokens , #def_ty_params >
            ::shred::SystemData< #impl_fetch_lt >
            for #name< #impl_lt_tokens , #impl_ty_params >
            where #where_clause
        {
            fn fetch(res: & #impl_fetch_lt ::shred::Resources, id: usize) -> Self {
                #fetch_return
            }

            fn reads(id: usize) -> Vec<::shred::ResourceId> {
                let mut r = Vec::new();

                #( {
                        let mut reads = <#tys as ::shred::SystemData> :: reads(id);
                        r.append(&mut reads);
                    } )*

                r
            }

            fn writes(id: usize) -> Vec<::shred::ResourceId> {
                let mut r = Vec::new();

                #( {
                        let mut writes = <#tys as ::shred::SystemData> :: writes(id);
                        r.append(&mut writes);
                    } )*

                r
            }
        }
    }
}

fn parse_generic(gen: &Generics) -> (Vec<&TypeParam>, Vec<&LifetimeDef>, Option<&WhereClause>) {
    let mut type_params = Vec::new();
    let mut lifetime_defs = Vec::new();
    let where_clause = gen.where_clause.as_ref();

    for param in &gen.params {
        match *param {
            GenericParam::Type(ref ty) => type_params.push(ty),
            GenericParam::Lifetime(ref lt) => lifetime_defs.push(lt),
            GenericParam::Const(_) => {}
        }
    }

    (type_params, lifetime_defs, where_clause)
}

fn collect_field_types(fields: &Fields) -> (Vec<Type>, BodyType) {
    match *fields {
        Fields::Named(ref named) => (
            named.named.iter().map(|field| field.ty.clone()).collect(),
            BodyType::Struct,
        ),
        Fields::Unnamed(ref unnamed) => (
            unnamed
                .unnamed
                .iter()
                .map(|field| field.ty.clone())
                .collect(),
            BodyType::Tuple,
        ),
        Fields::Unit => panic!("Enums are not supported"),
    }
}

fn gen_identifiers(fields: &Fields) -> Vec<Ident> {
    if let &Fields::Named(ref fields) = fields {
        fields
            .named
            .iter()
            .map(|field| field.ident.unwrap())
            .collect()
    } else {
        panic!("tried to gen_identifiers on tuple-like struct")
    }
}

fn gen_def_lt_tokens(lifetime_defs: &[&LifetimeDef]) -> Tokens {
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

fn gen_impl_lt_tokens(lifetime_defs: &[&LifetimeDef]) -> Tokens {
    let lts: Vec<Lifetime> = lifetime_defs.iter().map(|x| x.lifetime.clone()).collect();

    quote! { #( #lts ),* }
}

fn gen_def_ty_params(ty_params: &[&TypeParam]) -> Tokens {
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

fn gen_impl_ty_params(ty_params: &[&TypeParam]) -> Tokens {
    let ty_params: Vec<Ident> = ty_params.iter().map(|x| x.ident.clone()).collect();

    quote! { #( #ty_params ),* }
}

fn gen_where_clause(clause: Option<&WhereClause>, fetch_lt: &Lifetime, tys: &[Type]) -> Tokens {
    let user_predicates =
        clause.map(|where_clause| where_clause.predicates.iter().map(|x| quote! { #x , }));

    let system_data_predicates = tys.iter().map(|ty| {
        quote! { #ty : ::shred::SystemData< #fetch_lt >,  }
    });

    let mut tokens = Tokens::new();

    if let Some(user_predicates) = user_predicates {
        tokens.append_all(user_predicates.chain(system_data_predicates));
    } else {
        tokens.append_all(system_data_predicates);
    }

    tokens
}

fn gen_from_body(data: &Data, name: &Ident) -> (Tokens, Vec<Type>) {
    let structdef: &DataStruct;

    if let &Data::Struct(ref struct_defenition) = data {
        structdef = struct_defenition;
    } else {
        panic!("Enums and unions are not supported");
    }

    let (tys, body_type) = collect_field_types(&structdef.fields);

    let fetch_return = match body_type {
        BodyType::Struct => {
            let identifiers = gen_identifiers(&structdef.fields);

            quote! {
                #name {
                    #( #identifiers: ::shred::SystemData::fetch(res, id) ),*
                }
            }
        }
        BodyType::Tuple => {
            let count = tys.len();
            let fetch = vec![quote! { ::shred::SystemData::fetch(res, id) }; count];

            quote! {
                #name ( #( #fetch ),* )
            }
        }
    };

    (fetch_return, tys)
}

enum BodyType {
    Struct,
    Tuple,
}
