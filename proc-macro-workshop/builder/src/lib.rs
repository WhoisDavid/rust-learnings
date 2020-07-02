use proc_macro::TokenStream;
use proc_macro2;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields};

// Extract the simple inner type of an outer type from a field
// e.g x: Option<String> => Some(String)
// e.g x: String => None
fn inner_type<'a>(outer_type: &str, ty: &'a syn::Type) -> Option<&'a syn::Type> {
    if let syn::Type::Path(syn::TypePath {
        qself: None,
        ref path,
    }) = ty
    {
        if path.segments.len() != 1 || path.segments[0].ident != outer_type {
            return None;
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

// Retrieve the field attribute with given ident e.g:
// ```
// #[attr_ident(..)]
// field: Vec<T>,
// ```
fn get_attr<'a>(attr_ident: &str, field: &'a syn::Field) -> Option<&'a syn::Attribute> {
    let attrs = &field.attrs;
    for attr in attrs {
        if attr.path.segments.len() == 1 && attr.path.segments[0].ident == attr_ident {
            return Some(attr);
        }
    }
    None
}

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    // Pretty print the ast
    // eprintln!("{:#?}", ast);

    // Name of the object - here "Command"
    let derived_obj_ident = ast.ident;
    // Build a new ident for the builder
    // let ident = Ident::new(&format!("{}Builder", name), name.span());
    let builder_ident = format_ident!("{}Builder", derived_obj_ident);

    // Named fields of the derived struct
    // e.g `executable: String`
    let fields = match ast.data {
        Data::Struct(s) => {
            if let Fields::Named(named_fields) = s.fields {
                named_fields.named
            } else {
                unimplemented!("derive(Builder) only supports named fields")
            }
        }
        other => unimplemented!(
            "derive(Builder) only supports Struct and is not implemented for {:?}",
            other
        ),
    };

    // Helper to retrieve the `builder` attribute if it is present
    let get_builder_attr = |field| get_attr("builder", field);

    // Default fields for the builder
    // If the field has attribute `builder`, it is assumed to be of type Vec and default to Vec::new().
    // Otherwise, assume to be Option<T> and default to None.
    let builder_default_fields = fields.iter().map(|field| {
        let name = &field.ident;
        if get_builder_attr(field).is_some() {
            quote!( #name: ::std::vec::Vec::new() )
        } else {
            quote!( #name: ::std::option::Option::None )
        }
    });

    // Fields of the builder struct
    // Takes the fields of the derived object and optionized their type
    // e.g field: T => field: Option<T>
    // Exception are for Options and fields with the `builder` attribute where the type is kept as is.
    let builder_fields = fields.iter().map(|field| {
        let name = &field.ident;
        let ty = &field.ty;
        if inner_type("Option", ty).is_some() || get_builder_attr(field).is_some() {
            quote!( #name: #ty )
        } else {
            quote!( #name: ::std::option::Option<#ty> )
        }
    });

    // Builder setters for fields without `builder` attribute
    // Extracts the inner option type if the field is an option
    let setters = fields.iter().filter_map(|field| {
        let name = &field.ident;
        let ty = &field.ty;

        // Skip if field has attribute `builder`
        if get_builder_attr(field).is_some() {
            return None;
        }

        // Inner type if option
        // i.e. inner_ty = T if ty = Option<T> or ty
        let inner_ty = inner_type("Option", ty).unwrap_or(ty);
        Some(quote!( fn #name(&mut self, #name: #inner_ty) -> &mut Self {
            self.#name = ::std::option::Option::Some(#name);
            self
        }))
    });

    // Compile error for the builder attribute
    fn builder_attr_error<T: quote::ToTokens>(tokens: T) -> Option<proc_macro2::TokenStream> {
        Some(
            syn::Error::new_spanned(tokens, "expected `builder(each = \"...\")`")
                .to_compile_error(),
        )
    };

    // Builder setters for fields with `builder` attribute
    // Field are assumed to be of type Vec<T>
    let vec_setters = fields.iter().filter_map(|field| {
        let name = field.ident.as_ref().unwrap();
        let ty = &field.ty;
        let attr = get_builder_attr(field)?;
        match attr.parse_meta() {
            // Arguments to the builder attribute: `#[builder(..)]`
            // Here meta_list.path.is_ident("builder") == true
            Ok(syn::Meta::List(meta_list)) => {
                // We expect only one expression
                if meta_list.nested.len() != 1 {
                    return builder_attr_error(meta_list.nested);
                }

                // Expecting `each = ..`
                match &meta_list.nested[0] {
                    syn::NestedMeta::Meta(syn::Meta::NameValue(name_value)) => {
                        if !name_value.path.is_ident("each") {
                            // `builder` argument name is not "each"
                            return builder_attr_error(&name_value.path);
                        }
                        // Argument value: `..` of `each = ..`
                        match &name_value.lit {
                            syn::Lit::Str(val) => {
                                let arg_name = format_ident!("{}", val.value(), span = val.span());
                                let inner_ty = inner_type("Vec", ty).unwrap();
                                Some(quote!(
                                    fn #arg_name(&mut self, #arg_name: #inner_ty) -> &mut Self {
                                        self.#name.push( #arg_name );
                                        self
                                    }
                                ))
                            }
                            _ => builder_attr_error(&name_value.lit),
                        }
                    }
                    other => builder_attr_error(other),
                }
            }
            // Only supports builder for now
            _ => builder_attr_error(attr),
        }
    });

    // Fields used in the build method of the Builder
    // If the field is an Option or a `builder` (type Vec), it is assumed optional.
    // Otherwise, it is required and will fail if not set/
    let builder_build_fields = fields.iter().map(|field| {
        let name = &field.ident;
        let ty = &field.ty;
        if inner_type("Option", ty).is_some() || get_builder_attr(field).is_some() {
            quote!( #name: self.#name.clone() )
        } else {
            quote!( #name: self.#name.clone().ok_or(concat!(stringify!(#name), " is not set"))? )
        }
    });

    // Actual macro expansion
    let output = quote!(
        pub struct #builder_ident {
            #(#builder_fields),*
        }

        impl #builder_ident {
            pub fn build(&mut self) -> ::std::result::Result<#derived_obj_ident, ::std::boxed::Box<dyn ::std::error::Error>> {
                ::std::result::Result::Ok(#derived_obj_ident {
                    #(#builder_build_fields),*
                })
            }

            #(#setters)*
            #(#vec_setters)*
        }

        impl #derived_obj_ident {
            pub fn builder() -> #builder_ident {
                #builder_ident {
                    #(#builder_default_fields),*
                }
            }
        }
    );

    TokenStream::from(output)
}
