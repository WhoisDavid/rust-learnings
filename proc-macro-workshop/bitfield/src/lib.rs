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
pub use bitfield_impl::BitfieldSpecifier;

/// Single method trait to extract the least significant byte from Self
pub trait LastByte {
    /// Returns the last byte (least significant byte) of `Self`
    fn last_byte(self) -> u8;
}

pub trait Specifier {
    const BITS: usize;
    type IntType: From<u8>
        + From<Self::Interface>
        + Copy
        + LastByte
        + std::ops::Shl<usize, Output = Self::IntType>
        + std::ops::Shr<usize, Output = Self::IntType>
        + std::ops::AddAssign
        + std::ops::ShrAssign<usize>;
    type Interface;

    fn to_interface(int_val: Self::IntType) -> Self::Interface;

    /// Get a value on given on a slice of bytes given a certain offset and a number of bits (Self::BITS)
    ///   offset
    ///   ^^^^^
    ///  |ABCDEFGH|IJKLMNOP| ==> FGHIJK
    /// lsb    ^^^ ^^^         lsb
    ///        BITS=6
    fn get(data: &[u8], mut offset: usize) -> Self::Interface {
        let mut byte_idx = offset / 8;
        offset %= 8;
        let mut remaining_bits = Self::BITS;
        let mut out: Self::IntType = Self::IntType::from(0);
        while remaining_bits > 0 {
            let bits_in_current_byte = std::cmp::min(remaining_bits, 8 - offset);
            let new_byte: u8 = if bits_in_current_byte == 8 {
                data[byte_idx]
            } else {
                // Get the bits at given offset and shift right to make first bit after the offset the lsb.
                //  offset
                //   ^^^^
                //  |####XYZ#| ==> |XYZ00000|
                // lsb   ^^^      lsb
                //
                data[byte_idx].mid(offset, bits_in_current_byte) >> offset
            };
            out += Self::IntType::from(new_byte) << (Self::BITS - remaining_bits);
            remaining_bits -= bits_in_current_byte;
            byte_idx += 1;
            offset = 0;
        }
        Self::to_interface(out)
    }

    /// Set a value on a slice of bytes given a certain offset maintaining all other bits
    ///   offset               val     offset
    ///   ^^^^^               FOOBAR   ^^^^^
    ///  |ABCDEFGH|IJKLMNOP|   ==>    |ABCDEFOO|BARLMNOP|
    /// lsb                          lsb    ^^^ ^^^
    ///                                       val
    fn set(data: &mut [u8], mut offset: usize, val: Self::Interface) {
        let mut byte_idx = offset / 8;
        offset %= 8;
        let bits = Self::BITS;
        let mut remaining_bits = bits;
        let mut val_int = Self::IntType::from(val);
        while remaining_bits > 0 {
            let bits_in_current_byte = std::cmp::min(remaining_bits, 8 - offset);
            let new_byte: u8 = if bits_in_current_byte == 8 {
                // Truncates the u8 values
                val_int.last_byte()
            } else {
                // Get the bits at given offset and shift right to
                //   prev   next    prev   next
                //   ^^^^   ^       ^^^^   ^
                //  |ABCDEFGH| ==> |ABCDXYZH|
                // lsb   ^^^      lsb   ^^^
                //       slot           val
                let prev_bits = data[byte_idx].first(offset);
                let next_bits = data[byte_idx].last(8 - bits_in_current_byte - offset);
                prev_bits + (val_int.last_byte() << offset) + next_bits
            };
            data[byte_idx] = new_byte;
            val_int >>= bits_in_current_byte;
            remaining_bits -= bits_in_current_byte;
            byte_idx += 1;
            offset = 0;
        }
    }
}

// Implement Specifier for bool type here since it is static
impl Specifier for bool {
    const BITS: usize = 1;
    type IntType = u8;
    type Interface = Self;

    fn to_interface(int_val: Self::IntType) -> Self::Interface {
        match int_val {
            0 => false,
            1 => true,
            _ => panic!("Bool can only be converted from 0 or 1"),
        }
    }
}

generate_bit_specifiers!();
