#![cfg_attr(feature = "nightly", feature(const_panic))]

extern crate proc_macro;

// use proc_macro::TokenStream;
use proc_macro2::TokenStream;
use quote::quote;
use syn::parse_macro_input;
use syn::Item;

// Generate B1, B2, .., B64 with implementation of the Specifier trait
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
                    type TYPE = #size_type;

                    fn get(data: &[u8], mut offset: usize) -> Self::TYPE {
                        let mut byte_idx = (offset + 1) / 8;
                        offset %= 8;
                        let mut remaining_bits = Self::BITS;
                        let mut out: Self::TYPE = 0;
                        while remaining_bits > 0 {
                            let bits_in_current_byte = std::cmp::min(remaining_bits, 8 - offset);
                            let new_byte: u8 = if bits_in_current_byte == 8 {
                                data[byte_idx]
                            } else {
                                let mask = (1 << bits_in_current_byte) - 1 << offset;
                                data[byte_idx].mid(offset, bits_in_current_byte) >> offset
                            };
                            out += (new_byte as Self::TYPE) << (Self::BITS - remaining_bits);

                            remaining_bits -= bits_in_current_byte;
                            byte_idx += 1;
                            offset = 0;
                        }
                        return out;
                    }

                    fn set(data: &mut [u8], mut offset: usize, mut val: Self::TYPE) {
                        let mut byte_idx = (offset + 1) / 8;
                        offset %= 8;
                        let bits = Self::BITS;
                        let mut remaining_bits = bits;
                        while remaining_bits > 0 {
                            let bits_in_current_byte = std::cmp::min(remaining_bits, 8 - offset);
                            let new_byte: u8 = if bits_in_current_byte == 8 {
                                // Truncates the u8 values
                                (val as u8)
                            } else {
                                let previous_bits = data[byte_idx].first(offset);
                                let next_bits = data[byte_idx].last(8 - bits_in_current_byte);
                                previous_bits + ((val as u8) << offset) + next_bits
                            };
                            data[byte_idx] = new_byte;
                            val -= (new_byte as Self::TYPE) >> bits_in_current_byte;
                            remaining_bits -= bits_in_current_byte;
                            byte_idx += 1;
                            offset = 0;
                        }
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
            pub fn #get_ident(&self) -> <#ty as Specifier>::TYPE {
                // self.get::<#ty>(#offset)
                #ty::get(&self.data, #offset)
            }

            pub fn #set_ident(&mut self, val: <#ty as Specifier>::TYPE) {
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

// fn _expand_generic_get() -> TokenStream {
//     quote!(
//         fn get<T: Specifier>(&self, mut offset: usize) -> <T as Specifier>::TYPE {
//             let mut byte_idx = (offset + 1) / 8;
//             offset %= 8;
//             let bits = <T as Specifier>::BITS;
//             let mut remaining_bits = bits;
//             let mut out = <T as Specifier>::TYPE::from(0);

//             while remaining_bits > 0 {
//                 let bits_in_current_byte = std::cmp::min(remaining_bits, 8) - offset;
//                 let new_byte = if bits_in_current_byte == 8 {
//                     self.data[byte_idx]
//                 } else {
//                     let mask = (1 << bits_in_current_byte) - 1 << offset;
//                     (self.data[byte_idx] & mask) >> offset
//                 };
//                 out += <T as Specifier>::TYPE::from(new_byte) << (bits - remaining_bits);

//                 remaining_bits -= bits_in_current_byte;
//                 byte_idx += 1;
//                 offset = 0;
//             }
//             return out;
//         }
//     )
// }

// fn expand_generic_set() -> TokenStream {
//     quote!(
//         fn set<T: Specifier>(&mut self, offset: usize, val: <T as Specifier>::TYPE) {
//             let mut byte_idx = offset / 8;
//             offset %= 8;
//             let bits = <T as Specifier>::BITS;
//             let mut remaining_bits = bits;
//             let mut out = <T as Specifier>::TYPE::from(0);

//             while remaining_bits > 0 {
//                 let bits_in_current_byte = std::cmp::min(remaining_bits, 8) - offset;
//                 let mut new_byte = if bits_in_current_byte == 8 {
//                     self.data[byte_idx]
//                 } else {
//                     let mask = (1 << bits_in_current_byte) - 1 << offset;
//                     (self.data[byte_idx] & mask) >> offset
//                 };
//                 out += <T as Specifier>::TYPE::from(new_byte) << (bits - remaining_bits);

//                 remaining_bits -= bits_in_current_byte;
//                 byte_idx += 1;
//                 offset = 0;
//             }
//             return out;
//         }
//     )
// }
