use derive_debug::CustomDebug;
use std::fmt::Debug;

pub trait Trait {
    type Value;
}

#[derive(CustomDebug)]
#[debug(bound = "T::Value: Debug")]
pub struct Wrapper<T: Trait> {
    field: Field<T>,
}

#[derive(CustomDebug)]
struct Field<T: Trait> {
    values: Vec<T::Value>,
}

fn main() {
    struct Id;

    impl Trait for Id {
        type Value = u8;
    }
}