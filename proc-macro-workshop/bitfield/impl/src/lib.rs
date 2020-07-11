#![cfg_attr(feature = "nightly", feature(const_panic))]

extern crate proc_macro;

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::parse_macro_input;
use syn::Item;

/// Generate B1, B2, .., B64 with implementation of the Specifier trait
#[proc_macro]
pub fn generate_bit_specifiers(_input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut output = TokenStream::new();

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

    // Implement LastByte trait for integer primitives with `as u8`
    output.extend(impl_last_byte());
    // Implement BitOps
    output.extend(bit_ops_impl());
    output.extend(bit_specifiers);
    output.into()
}

/// Implement LastByte trait for integer primitives `u8`, `u16`, .., `u128` using `as u8`
fn impl_last_byte() -> TokenStream {
    let int_types = [
        quote!(u8),
        quote!(u16),
        quote!(u32),
        quote!(u64),
        quote!(u128),
    ];

    // Implement LastByte trait for primitives
    quote!(
        #[doc = "Implement last byte for integer primitives using `as u8`"]
        #(impl LastByte for #int_types {
            fn last_byte(self) -> u8 {
                self as u8
            }
        })*
    )
}

/// Match a given number of bits to the narrowest unsigned integer type that can hold it
fn size_to_type(bits: usize) -> TokenStream {
    match bits {
        1..=8 => quote!(u8),
        9..=16 => quote!(u16),
        17..=32 => quote!(u32),
        33..=64 => quote!(u64),
        65..=128 => quote!(u128),
        _ => unreachable!(),
    }
}

/// Defines BitOps trait and implement it for `u8`
fn bit_ops_impl() -> TokenStream {
    quote!(
        #[doc = "Simple trait to extract bits from primitive integer type"]
        trait BitOps {
            fn first(self, n: usize) -> u8;
            fn last(self, n: usize) -> u8;
            fn mid(self, start: usize, len: usize) -> u8;
        }

        #[doc = "Ops to extract bits from `u8` byte"]
        impl BitOps for u8 {
            fn first(self, n: usize) -> u8 {
                match n {
                    0 => 0,
                    1..=7 => self & ((1 << n) - 1),
                    _ => self,
                }
            }

            fn last(self, n: usize) -> u8 {
                match n {
                    0 => 0,
                    1..=7 => self & !((1 << (8 - n)) - 1),
                    _ => self,
                }
            }

            fn mid(self, start: usize, len: usize) -> u8 {
                match (start, start + len) {
                    (0, _) => self.first(len),
                    (_, l) if l >= 8 => self.last(8 - start),
                    _ => self & (((1 << len) - 1) << start),
                }
            }
        }
    )
}

/// syn helper struct to parse bits attributes
struct BitAttribute {
    bits: syn::LitInt,
}

/// Parses the following attribute:
/// ```
/// #[bits=8]
///       ^^
/// ```
impl syn::parse::Parse for BitAttribute {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let _: syn::Token![=] = input.parse()?;
        let bits: syn::LitInt = input.parse()?;
        Ok(Self { bits })
    }
}

/// Main macro `bitfield`
/// Parses a Struct, validates field sizes,
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

            // Check that fields with #[bits=X] attribute have a type of size `X`
            // We use an array size check to validate the size is correct
            let bits_attrs_check = s
                .fields
                .iter()
                .filter_map(|field| {
                    let ty = &field.ty;
                    let attrs = &field.attrs;
                    for attr in attrs {
                        // #[bits=..]
                        //   ^^^^
                        if attr.path.is_ident("bits") {
                            // At this point `attr.tokens` is the following part of the attribute:
                            // #[bits=..]
                            //       ^^^
                            let bits = syn::parse2::<BitAttribute>(attr.tokens.clone()).ok()?.bits;
                            return Some(
                                quote_spanned!(bits.span() => const _: [(); #bits] = [(); <#ty as Specifier>::BITS];),
                            );
                        }
                    }
                    None
                });

            let getters_setters = define_getters_setters(&s.fields);
            // Total size calculated as the sum of the inner `<T as Specifier>::BITS` associated consts
            let total_bit_size = quote!(0 #(+ <#fields_ty as Specifier>::BITS)*);

            // Formatted error message for the size check
            let error = format!(
                "#[bitfield] on `{}` requires the total bit size to be a multiple of 8 bits.",
                ident.to_string()
            );

            quote!(
                #[doc = "Converted bitfield struct"]
                pub struct #ident {
                    data: [u8; ( #total_bit_size ) / 8],
                }

                #(#bits_attrs_check)*

                // Conditional consts and panic in consts requires nightly
                #[cfg(feature="nightly")]
                const _: usize = if ( ( #total_bit_size ) % 8 == 0 ) {
                    0
                }else{
                    panic!(#error)
                };

                impl #ident {

                    pub fn new() -> Self {
                        Self { data: [0u8; ( #total_bit_size ) / 8] }
                    }

                    #getters_setters
                }
            )
            .into()
        }
        _ => unimplemented!("Only struct"),
    }
}

fn define_getters_setters(fields: &syn::Fields) -> TokenStream {
    let getters = fields.iter().scan(quote!(0), |offset, field| {
        let ident = field.ident.as_ref().expect("Namef field");

        // get_[field name] and set_[field name] idents
        let get_ident = quote::format_ident!("get_{}", ident);
        let set_ident = quote::format_ident!("set_{}", ident);

        let ty = &field.ty;

        let output = quote!(
            pub fn #get_ident(&self) -> <#ty as Specifier>::Interface {
                #ty::get(&self.data, #offset)
            }

            pub fn #set_ident(&mut self, val: <#ty as Specifier>::Interface) {
                #ty::set(&mut self.data, #offset, val)
            }
        );
        // Move the offset by the number of bits in the current type
        *offset = quote!((#offset + <#ty as Specifier>::BITS));
        Some(output)
    });

    quote!(#(#getters)*)
}

/// Derive BitfieldSpecifier macro for Enums
/// Parses enums and implements Specifier for it.
// In particular, constructs `to_interface` to match a discriminant to its variant.
// Compile time checks: number of variants is a power of two and discriminant size within bit range
#[proc_macro_derive(BitfieldSpecifier)]
pub fn derive_bitfield_specifier(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    let enum_ident = ast.ident;

    // No attributes
    let attrs = ast.attrs;
    assert!(attrs.is_empty());

    // Variants of the enum
    // returns an error if underlying `syn::Data` is not an Enum
    let variants = match ast.data {
        syn::Data::Enum(e) => e.variants,
        // Struct / Union
        _ => {
            return syn::Error::new_spanned(enum_ident, "BitfieldSpecifier only supports enum")
                .to_compile_error()
                .into()
        }
    };

    // Check that the number of variants is a power of two.
    // If not, return an error.
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

        // Create a new ident `[enum]_[variant]` to be used in the match patterns
        // Not really needed but clearer in expansion
        let unique_ident = syn::Ident::new(
            &format!(
                "{}_{}",
                enum_ident.to_string().to_lowercase(),
                ident.to_string().to_lowercase()
            ),
            ident.span(),
        );
        // Rely on the pattern:
        // x if x == Enum::Variant as Enum::IntType
        quote!( #unique_ident if #unique_ident == Self::#ident as Self::IntType => Self::#ident )
    });

    // Iterator of full variant name: `Enum::Variant`
    let enum_variants = variants.iter().map(|variant| {
        let ident = &variant.ident;
        quote!( #enum_ident::#ident  )
    });

    // Formatted error message in case the discriminant of a variant is outside of the allowed bit range.
    let error = format!(
        "\nError in BitfieldSpecifier for {}.\nBitfieldSpecifier expects discriminants in the range 0..2^BITS.\nOutside of range:",
        enum_ident.to_string()
    );

    quote!(
        // Compile time checks on the size of the discriminant
        #(
            // Conditional consts and panic in const requires nightly
            #[cfg(feature="nightly")]
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
                match int_val {
                    #(#match_variants),*,
                    _ => panic!("Not supported"),
                }
            }
        }
    )
    .into()
}
