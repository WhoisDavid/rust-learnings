use proc_macro::TokenStream;
// use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields};

fn inner_type<'a>(ty: &'a syn::Type) -> Option<&'a syn::Type> {
    if let syn::Type::Path(syn::TypePath {
        qself: None,
        ref path,
    }) = ty
    {
        if path.segments.len() != 1 {
            return None;
        }

        if let syn::PathArguments::AngleBracketed(ref inner_type) = path.segments[0].arguments {
            // Extract only if a single type is present
            // eprintln!("INNER: {:#?}", inner_type.args);
            inner_type.args.iter().for_each(|f| {
                if let syn::GenericArgument::Type(ref ty) = f {
                    eprintln!("INNER: {:#?}", ty);
                }
            });
        }
        // inner_type would be e.g <String> in Option<String> or <K, V> in HashMap<K, V>
    }
    None
}

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    // Pretty print the ast
    // eprintln!("{:#?}", ast);
    // Name of the object - here "Command"
    let name = ast.ident;

    let fields = match ast.data {
        Data::Struct(s) => {
            if let Fields::Named(named_fields) = s.fields {
                named_fields.named
            } else {
                panic!("derive(Builder) only supports named fields")
            }
        }
        other => panic!(
            "derive(Builder) only supports Struct and is not implemented for {:?}",
            other
        ),
    };

    fields.iter().for_each(|field| {
        let name = &field.ident;
        let ty = &field.ty;
        inner_type(ty);
    });

    TokenStream::new()
}
