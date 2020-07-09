#![cfg_attr(feature = "nightly", feature(const_panic))]

extern crate proc_macro;

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse_macro_input;
use syn::Item;

// Generate B1, B2, .., B64 with implementation of the Specifier trait
#[proc_macro]
pub fn generate_bit_specifiers(_input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut output = TokenStream::new();

    let int_types = [
        quote!(u8),
        quote!(u16),
        quote!(u32),
        quote!(u64),
        quote!(u128),
    ];

    let last_byte = quote!(
        #(impl LastByte for #int_types {
            fn last_byte(self) -> u8 {
                self as u8
            }
        })*
    );
    output.extend(last_byte);

    let bit_specifiers = (1usize..=64).map(|idx| {
        let ident = syn::Ident::new(&format!("B{}", idx), proc_macro2::Span::call_site());
        let size_type = size_to_type(idx);
        quote! (
            pub enum #ident {}
            impl Specifier for #ident {
                const BITS: usize = #idx;
                type IntType = #size_type;
                type Interface = #size_type;

                fn to_interface(int_val: Self::IntType) -> Self::Interface {
                    int_val as Self::Interface
                }
            }
        )
    });

    output.extend(bit_ops_impl());
    output.extend(bit_specifiers);
    output.into()
}

fn size_to_type(idx: usize) -> TokenStream {
    match idx {
        1..=8 => quote!(u8),
        9..=16 => quote!(u16),
        17..=32 => quote!(u32),
        33..=64 => quote!(u64),
        65..=128 => quote!(u128),
        _ => unreachable!(),
    }
}

fn bit_ops_impl() -> TokenStream {
    quote!(
        trait BitOps {
            fn first(self, n: usize) -> u8;
            fn last(self, n: usize) -> u8;
            fn mid(self, start: usize, len: usize) -> u8;
        }

        impl BitOps for u8 {
            fn first(self, n: usize) -> u8 {
                if n >= 8 {
                    self
                } else {
                    self & ((1 << n) - 1)
                }
            }

            fn last(self, n: usize) -> u8 {
                if n >= 8 {
                    self
                } else {
                    // u8::MAX - (1 << (8-n)) + 1
                    self & !((1 << (8-n)) - 1)
                }
            }

            fn mid(self, start: usize, len: usize) -> u8 {
                if start == 0 {
                    self.first(len)
                } else if start + len >= 8 {
                    self.last(8-start)
                } else {
                    self & (((1 << len) - 1) << start)
                }
            }
        }
    )
}

#[proc_macro_attribute]
pub fn bitfield(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let _ = args;
    let item = parse_macro_input!(input as syn::Item);

    match item {
        Item::Struct(s) => {
            let ident = &s.ident;
            let fields_ty = s.fields.iter().map(|field| &field.ty);
            let get_sets = getters_setters(&s.fields);
            let size = quote!(0 #(+ <#fields_ty as Specifier>::BITS)*);
            let error = format!(
                "#[bitfield] on `{}` requires the total bit size to be a multiple of 8 bits.",
                ident.to_string()
            );
            quote!(
                pub struct #ident {
                    data: [u8; ( #size ) / 8],
                }

                // Conditional consts and panic in const requires nightly
                #[cfg(feature="nightly")]
                const _: usize = if ( ( #size ) % 8 == 0 ) {
                    0
                }else{
                    panic!(#error)
                };

                impl #ident {

                    pub fn new() -> Self {
                        Self { data: [0u8; ( #size ) / 8] }
                    }

                    #get_sets
                }
            )
            .into()
        }
        _ => unimplemented!("Only struct"),
    }
}

fn getters_setters(fields: &syn::Fields) -> TokenStream {
    let getters = fields.iter().scan(quote!(0), |offset, field| {
        let ident = field.ident.as_ref().expect("Namef field");
        let get_ident = quote::format_ident!("get_{}", ident);
        let set_ident = quote::format_ident!("set_{}", ident);

        let ty = &field.ty;
        let to = quote!((#offset + <#ty as Specifier>::BITS));

        let getter = quote!(
            pub fn #get_ident(&self) -> <#ty as Specifier>::Interface {
                // self.get::<#ty>(#offset)
                #ty::get(&self.data, #offset)
            }

            pub fn #set_ident(&mut self, val: <#ty as Specifier>::Interface) {
                //self.data[#offset..#to] = val;
                #ty::set(&mut self.data, #offset, val)
            }
        );
        *offset = to;
        Some(getter)
    });
    let mut output = TokenStream::new();
    output.extend(getters);
    output
}

#[proc_macro_derive(BitfieldSpecifier)]
pub fn derive_bitfield_specifier(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);

    let enum_ident = ast.ident;

    // No attributes
    let attrs = ast.attrs;
    assert!(attrs.is_empty());

    let variants = match ast.data {
        syn::Data::Enum(e) => e.variants,
        // Struct / Union
        _ => {
            return syn::Error::new_spanned(enum_ident, "BitfieldSpecifier only supports enum")
                .to_compile_error()
                .into()
        }
    };

    // Check that the number of variants is a power of two
    let variant_count = variants.len();
    if !variant_count.is_power_of_two() {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "BitfieldSpecifier expected a number of variants which is a power of 2",
        )
        .to_compile_error()
        .into();
    }

    // Number of bits (i.e. which power of two) is the number of trailing zeros
    let bits = variant_count.trailing_zeros() as usize;
    let size_type = size_to_type(bits);

    // Build match patterns for variants
    let match_variants = variants.iter().map(|variant| {
        let ident = &variant.ident;

        // Create a new ident `enum_variant` to be used in the match patterns
        let other_ident = syn::Ident::new(
            &format!(
                "{}_{}",
                enum_ident.to_string().to_lowercase(),
                ident.to_string().to_lowercase()
            ),
            ident.span(),
        );
        // Rely on x if x == Enum::Variant
        quote!( #other_ident if #other_ident == Self::#ident as Self::IntType => Self::#ident )
    });

    let enum_variants = variants.iter().map(|variant| {
        let ident = &variant.ident;
        quote!( #enum_ident::#ident  )
    });

    let error = format!(
        "\nError in BitfieldSpecifier for {}.\nBitfieldSpecifier expects discriminants in the range 0..2^BITS.\nOutside of range:",
        enum_ident.to_string()
    );

    quote!(
        #(
            // Conditional consts and panic in const requires nightly
            // #[cfg(feature="nightly")]
            const _: usize = if ( (#enum_variants as usize) < #variant_count) {
                0
            }else{
                panic!(concat!(#error, stringify!(#enum_variants), " >= ", #variant_count, "\n"))
            };
        )*

        impl From<#enum_ident> for #size_type {
            fn from(x: #enum_ident) -> #size_type {
                x as #size_type
            }
        }

        impl Specifier for #enum_ident {
            const BITS: usize = #bits;
            type IntType = #size_type;
            type Interface = Self;

            fn to_interface(int_val: Self::IntType) -> Self::Interface {
                // #(#unknown_discriminant)*
                match int_val {
                    #(#match_variants),*,
                    _ => panic!("Not supported"),
                }
            }
        }
    )
    .into()
}
