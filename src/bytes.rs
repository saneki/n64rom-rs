pub fn swap_bytes<T>(buf: &mut [u8]) where T: Swap {
    T::swap(buf)
}

pub trait Swap {
    fn swap(buf: &mut [u8]);
}

pub enum BigEndian {}
pub type BE = BigEndian;

impl Swap for BigEndian {
    fn swap(_: &mut [u8]) {
        // Empty
    }
}

pub enum LittleEndian {}
pub type LE = LittleEndian;

impl Swap for LittleEndian {
    fn swap(buf: &mut [u8]) {
        assert_eq!(buf.len() % 4, 0, "Byte swapping requires a multiple of two");
        let swaps = buf.len() / 4;
        for i in 0..swaps {
            let idx = i*4;
            let temp = buf[idx];
            buf[idx] = buf[idx+1];
            buf[idx+1] = buf[idx+2];
            buf[idx+2] = buf[idx+3];
            buf[idx+3] = temp;
        }
    }
}

pub enum Mixed {}
pub type MX = Mixed;

impl Swap for Mixed {
    fn swap(buf: &mut [u8]) {
        assert_eq!(buf.len() % 2, 0, "Byte swapping requires a multiple of two");
        let swaps = buf.len() / 2;
        for i in 0..swaps {
            let idx = i*2;
            let temp = buf[idx];
            buf[idx] = buf[idx+1];
            buf[idx+1] = temp;
        }
    }
}

#[derive(Clone, Copy)]
/// Convenience wrapper enum around the separate Swap endianness enums.
pub enum Endianness {
    Big,
    Little,
    Mixed,
}

impl ToString for Endianness {
    fn to_string(&self) -> String {
        match self {
            Endianness::Big => String::from("Big Endian"),
            Endianness::Little => String::from("Little Endian"),
            Endianness::Mixed => String::from("Mixed"),
        }
    }
}

/// Wrapper implementation around the separate Swap endianness enums.
impl Endianness {
    pub fn swap(&self, buf: &mut [u8]) {
        match self {
            Endianness::Big => swap_bytes::<BigEndian>(buf),
            Endianness::Little => swap_bytes::<LittleEndian>(buf),
            Endianness::Mixed => swap_bytes::<Mixed>(buf),
        }
    }
}
