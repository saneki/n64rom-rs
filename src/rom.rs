use std::fmt;
use std::io::Read;

use crate::bytes::Endianness;
use crate::header::{HeaderError, N64Header, HEADER_SIZE};
use crate::io::Reader;
use crate::ipl3::{IPL3, IPL_SIZE};

pub struct Rom {
    header: N64Header,
    ipl3: IPL3,
    data: Vec<u8>,

    /// Byte order (endianness) of rom file.
    order: Endianness,
}

impl fmt::Display for Rom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut builder = Vec::<String>::new();
        builder.push(format!("{}", self.header));
        builder.push(format!("IPL3: {}", self.ipl3));
        builder.push(format!("Byte Order: {}", self.order));
        builder.push(format!("Full Length: 0x{:08X}", self.len()));
        write!(f, "{}", builder.join("\n"))
    }
}

impl Rom {
    pub fn read<T>(mut reader: &mut T) -> Result<Rom, HeaderError>
    where
        T: Read,
    {
        // Read header & infer endianness
        let (header, order) = N64Header::read(&mut reader)?;

        // Create new reader based on endianness, read remaining with it
        let mut reader = Reader::from(&mut reader, &order);
        let ipl3 = IPL3::read(&mut reader)?;

        let mut data: Vec<u8> = Vec::new();
        reader.read_to_end(&mut data)?;

        let rom = Rom {
            header,
            ipl3,
            data,
            order,
        };

        Ok(rom)
    }

    pub fn len(&self) -> usize {
        HEADER_SIZE + IPL_SIZE + self.data.len()
    }
}
