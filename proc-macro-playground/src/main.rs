use proc_macro_tests::Builder;
use std::collections::HashMap;

#[derive(Builder)]
struct Test {
    name: HashMap<u32, u64>,
}

fn main() {
    // println!("Hello, world!");
}
