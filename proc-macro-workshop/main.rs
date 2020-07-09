#![cfg_attr(feature = "nightly", feature(const_panic))]

use bitfield::*;

// #[bitfield]
// pub struct RedirectionTableEntry {
//     acknowledged: bool,
//     trigger_mode: TriggerMode,
//     delivery_mode: DeliveryMode,
//     reserved: B3,
// }

#[derive(BitfieldSpecifier)]
pub enum TriggerMode {
    Edge = 0,
    Level = 1,
}

#[bitfield]
pub struct RedirectionTableEntry {
    delivery_mode: DeliveryMode,
    reserved: B5,
}

const F: isize = 3;
const G: isize = 0;

#[derive(BitfieldSpecifier)]
pub enum DeliveryMode {
    Fixed = F,
    Lowest,
    SMI,
    RemoteRead,
    NMI,
    Init = G,
    Startup,
    External,
}
fn main() {}
