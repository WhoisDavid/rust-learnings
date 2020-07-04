use seq::seq;

seq!(N in 16..=20 {
    enum E {
        #(
            Variant#N,
        )*
    }
});

fn main() {
    let e = E::Variant16;

    let desc = match e {
        E::Variant16 => "min",
        E::Variant17 | E::Variant18 | E::Variant19 => "in between",
        E::Variant20 => "max",
    };
}