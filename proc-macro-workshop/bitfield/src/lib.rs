#![cfg_attr(feature = "nightly", feature(const_panic))]

// Crates that have the "proc-macro" crate type are only allowed to export
// procedural macros. So we cannot have one crate that defines procedural macros
// alongside other types of public APIs like traits and structs.
//
// For this project we are going to need a #[bitfield] macro but also a trait
// and some structs. We solve this by defining the trait and structs in this
// crate, defining the attribute macro in a separate bitfield-impl crate, and
// then re-exporting the macro from this crate so that users only have one crate
// that they need to import.
//
// From the perspective of a user of this crate, they get all the necessary APIs
// (macro, trait, struct) through the one bitfield crate.

pub use bitfield_impl::bitfield;
pub use bitfield_impl::generate_bit_specifiers;

pub trait Specifier {
    const BITS: usize;
    type TYPE: From<u8> + std::ops::Shl<usize, Output = Self::TYPE> + std::ops::AddAssign;

    fn get(data: &[u8], offset: usize) -> Self::TYPE;
    fn set(data: &mut [u8], offset: usize, val: Self::TYPE);
}

generate_bit_specifiers!();
