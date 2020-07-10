#![cfg_attr(feature = "nightly", feature(const_panic))]

use bitfield::*;

// #[bitfield]
// pub struct RedirectionTableEntry {
//     acknowledged: bool,
//     trigger_mode: TriggerMode,
//     delivery_mode: DeliveryMode,
//     reserved: B3,
// }

#[bitfield]
pub struct RedirectionTableEntry {
    #[bits = 1]
    trigger_mode: TriggerMode,
    #[bits = 3]
    delivery_mode: DeliveryMode,
    reserved: B4,
}

#[derive(BitfieldSpecifier, Debug)]
pub enum TriggerMode {
    Edge = 0,
    Level = 1,
}

#[derive(BitfieldSpecifier, Debug)]
pub enum DeliveryMode {
    Fixed = 0b000,
    Lowest = 0b001,
    SMI = 0b010,
    RemoteRead = 0b011,
    NMI = 0b100,
    Init = 0b101,
    Startup = 0b110,
    External = 0b111,
}

fn main() {}
