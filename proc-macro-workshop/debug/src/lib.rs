extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, Fields};

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);

    let struct_ident = ast.ident;

    let attrs = ast.attrs;
    let debug_attr = get_attr("debug", attrs);
    let bound_res = get_bound_from_attr(debug_attr);
    if let Err(err) = bound_res {
        return err.to_compile_error().into();
    }
    let bound_attr = bound_res.unwrap();

    let named_fields = match ast.data {
        Data::Struct(s) => {
            if let Fields::Named(named_fields) = s.fields {
                named_fields.named
            } else {
                unimplemented!("derive(Builder) only supports named fields")
            }
        }
        // Enum / Union
        other => unimplemented!(
            "CustomDebug only supports Struct and is not implemented for {:?}",
            other
        ),
    };

    // List of generic type identse.g `[Ident("T",..)]`
    let generic_types = ast
        .generics
        .type_params()
        .map(|t| &t.ident)
        .collect::<Vec<_>>();

    // List types' idents used in PhantomData e.g `PhantomData<T>` => `Ident(T)`
    let phantom_types = named_fields
        .iter()
        .filter_map(|field| {
            let ty = &field.ty;
            let inner_ty = inner_type(ty, Some("PhantomData"))?;
            if let syn::Type::Path(type_path) = inner_ty {
                let type_ident = &type_path.path.segments.first()?.ident;
                if generic_types.contains(&type_ident) {
                    return Some(type_ident);
                }
            }
            None
        })
        .collect::<Vec<_>>();

    // List of associated types e.g `T::Value`
    let associated_types = named_fields
        .iter()
        .filter_map(|field| get_associated_types(&field.ty, &generic_types))
        .collect::<Vec<_>>();

    // See: https://docs.rs/syn/1.0.33/syn/macro.parse_quote.html#example
    let generics = add_trait_bounds(ast.generics, phantom_types, associated_types, bound_attr);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let field_names = named_fields.iter().map(|field| &field.ident);

    let field_format = named_fields.iter().map(|field| {
        let attrs = &field.attrs;
        for attr in attrs.iter() {
            match attr.parse_meta() {
                Ok(syn::Meta::NameValue(name_value)) if name_value.path.is_ident("debug") => {
                    if let syn::Lit::Str(s) = name_value.lit {
                        return s.value();
                    }
                }
                other => unimplemented!("Unimplemented attribute {:?}", other),
            }
        }
        return String::from("{:?}");
    });

    // See: https://docs.rs/syn/1.0.33/syn/struct.Generics.html#impl-1
    quote!(
        impl #impl_generics ::std::fmt::Debug for #struct_ident #ty_generics #where_clause {
            fn fmt(&self, fmt: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                fmt.debug_struct(stringify!(#struct_ident))
                   #(.field(stringify!(#field_names), &format_args!(#field_format, &self.#field_names)))*
                   .finish()
            }
        }
    ).into()
}

fn attr_error<T: quote::ToTokens>(tokens: T) -> syn::Error {
    syn::Error::new_spanned(tokens, "expected `debug(bound = \"...\")`")
}

fn get_attr<'a>(attr_ident: &str, attrs: Vec<syn::Attribute>) -> Option<syn::Attribute> {
    for attr in attrs {
        if attr.path.segments.len() == 1 && attr.path.segments[0].ident == attr_ident {
            return Some(attr);
        }
    }
    None
}

fn get_bound_from_attr(
    attr: Option<syn::Attribute>,
) -> Result<Option<syn::WherePredicate>, syn::Error> {
    if attr.is_none() {
        return Ok(None);
    }

    match attr.unwrap().parse_meta() {
        Ok(syn::Meta::List(meta_list)) => {
            // We expect only one expression
            if meta_list.nested.len() != 1 {
                return Err(attr_error(meta_list.nested));
            }

            // Expecting `debug = ..`
            match &meta_list.nested[0] {
                syn::NestedMeta::Meta(syn::Meta::NameValue(name_value)) => {
                    if !name_value.path.is_ident("bound") {
                        // `builder` argument name is not "bound"
                        return Err(attr_error(&name_value.path));
                    }
                    // Argument value: `..` of `bound = ..`
                    if let syn::Lit::Str(val) = &name_value.lit {
                        let bound = val.value();
                        // Parse the bound as a where clause predicate
                        match syn::parse_str::<syn::WherePredicate>(&bound) {
                            Ok(where_predicate) => Ok(Some(where_predicate)),
                            Err(e) => Err(syn::Error::new_spanned(&name_value.lit, e)),
                        }
                    } else {
                        Err(attr_error(&name_value.lit))
                    }
                }
                other => Err(attr_error(other)),
            }
        }
        _ => Ok(None),
    }
}

fn add_trait_bounds(
    mut generics: syn::Generics,
    phantom_types: Vec<&syn::Ident>,
    associated_types: Vec<&syn::TypePath>,
    bound_attr: Option<syn::WherePredicate>,
) -> syn::Generics {
    let associated_types_ident = associated_types
        .iter()
        .map(|ty| &ty.path.segments[0].ident)
        .collect::<Vec<_>>();

    if let Some(bound) = bound_attr {
        let where_clause = generics.make_where_clause();
        where_clause.predicates.push(bound);
    } else {
        for type_param in generics.type_params_mut() {
            // if let syn::GenericParam::Type(ref mut type_param) = *param {
            // Do not add bound for Phantom types
            if phantom_types.contains(&&type_param.ident) {
                continue;
            }
            // Do not add bound for associated types
            if associated_types_ident.contains(&&type_param.ident) {
                continue;
            }
            type_param.bounds.push(syn::parse_quote!(::std::fmt::Debug));
            // }
        }

        // Add where clause for associated types
        let where_clause = generics.make_where_clause();
        for associated_type in associated_types {
            where_clause
                .predicates
                .push(syn::parse_quote!(#associated_type : ::std::fmt::Debug))
        }
    }
    generics
}

// Extract the simple inner type of an optional outer type from a field
// If not outer type is specified, just returns the inner type.
// e.g x: Option<String> => Some(String)
// e.g x: String => None
fn inner_type<'a>(ty: &'a syn::Type, outer_type: Option<&str>) -> Option<&'a syn::Type> {
    if let syn::Type::Path(syn::TypePath {
        qself: None,
        ref path,
    }) = ty
    {
        if path.segments.len() != 1 {
            return None;
        }

        if let Some(outer_ty) = outer_type {
            if path.segments[0].ident != outer_ty {
                return None;
            }
        }

        // inner_type would be e.g <String> in Option<String> or <K, V> in HashMap<K, V>
        if let syn::PathArguments::AngleBracketed(ref inner_type) = path.segments[0].arguments {
            // Extract only if a single type is present
            if inner_type.args.len() != 1 {
                return None;
            }

            // Extract the type from the <[..args]>
            if let syn::GenericArgument::Type(ref ty) = inner_type.args[0] {
                return Some(ty);
            }
        }
    }
    None
}

fn get_associated_types<'a>(
    ty: &'a syn::Type,
    generic_types: &Vec<&syn::Ident>,
) -> Option<&'a syn::TypePath> {
    if let Some(inner_ty) = inner_type(ty, None) {
        return get_associated_types(inner_ty, generic_types);
    }

    if let syn::Type::Path(type_path) = ty {
        if type_path.path.segments.len() < 2 {
            return None;
        }

        let type_ident = &type_path.path.segments[0].ident;
        if generic_types.contains(&type_ident) {
            return Some(type_path);
        }
    }
    None
}
