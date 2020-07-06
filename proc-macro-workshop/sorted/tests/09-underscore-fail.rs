// There is one other common type of pattern that would be nice to support --
// the wildcard or underscore pattern. The #[sorted] macro should check that if
// a wildcard pattern is present then it is the last one.

use sorted::sorted;

#[sorted]
pub enum Conference {
    RustBeltRust,
    RustConf,
    RustFest,
    RustLatam,
    RustRush,
}

#[sorted]
pub enum Location {
    Europe,
    Latam,
    US,
    Elsewhere,
}

impl Conference {
    #[sorted::check]
    pub fn region(&self) -> Location {
        use self::Conference::*;
        use self::Location::*;

        #[sorted]
        match self {
            RustFest => Europe,
            _ => Elsewhere,
            RustLatam =>
            {
                #[sorted]
                match self {
                    RustLatam => Latam,
                    RustFest => Europe,
                    _ => Elsewhere,
                }
            }
        };

        #[sorted]
        match self {
            RustFest => Europe,
            RustConf => US,
            _ => Elsewhere,
        }
    }
}

fn main() {}
