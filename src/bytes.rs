use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq)]
/// Convenience wrapper enum around the separate Swap endianness enums.
pub enum Endianness {
    Big,
    Little,
    Mixed,
}

impl fmt::Display for Endianness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Big => write!(f, "Big Endian"),
            Self::Little => write!(f, "Little Endian"),
            Self::Mixed => write!(f, "Mixed"),
        }
    }
}
