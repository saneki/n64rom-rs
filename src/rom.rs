use std::fmt;
use std::io::{self, Read, Write};

use crate::bytes::Endianness;
use crate::header::{HeaderError, N64Header, HEADER_SIZE};
use crate::io::{Reader, Writer};
use crate::ipl3::{IPL3, IPL_SIZE};
use crate::util::{FileSize, MEBIBYTE};

pub struct Rom {
    pub header: N64Header,
    pub ipl3: IPL3,
    pub data: Vec<u8>,

    /// Byte order (endianness) of rom file.
    order: Endianness,
}

impl fmt::Display for Rom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let filesize = FileSize::from(self.len(), MEBIBYTE);
        let mut builder = Vec::<String>::new();
        builder.push(format!("{}", self.header));
        builder.push(format!("  IPL3: {}", self.ipl3));
        builder.push(format!("  Byte Order: {}", self.order));
        match filesize {
            FileSize::Float(value) => {
                builder.push(format!("  Rom Size: {:.*} MiB", 1, value));
            }
            FileSize::Int(value) => {
                builder.push(format!("  Rom Size: {} MiB", value));
            }
        }
        write!(f, "{}", builder.join("\n"))
    }
}

impl Rom {
    pub fn check_crc(&self) -> (bool, (u32, u32)) {
        let crcs = self.header.crcs();
        let calc = self.ipl3.compute_crcs(&self.data, &[]);
        let result = crcs == calc;
        (result, calc)
    }

    /// Correct the CRC values in the header.
    pub fn correct_crc(&mut self) -> bool {
        let (result, (calc1, calc2)) = self.check_crc();
        match result {
            true => result,
            false => {
                // Update the header CRC fields
                self.header.crc1 = calc1;
                self.header.crc2 = calc2;

                result
            }
        }
    }

    pub fn order(&self) -> &Endianness {
        &self.order
    }

    pub fn read<T>(mut reader: &mut T) -> Result<Self, HeaderError>
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

        let rom = Self {
            header,
            ipl3,
            data,
            order,
        };

        Ok(rom)
    }

    pub fn write<'a, T>(&self, writer: &'a mut T, endianness: Option<&Endianness>) -> io::Result<usize>
    where
        T: Write,
    {
        let order = match endianness {
            // Use endianness if specified
            Some(e) => e,
            // Otherwise default to original order
            _ => &self.order,
        };

        // Wrap in writer that respects chosen byte order
        let mut writer = Writer::from(writer, order);

        // Write header, IPL3 and data
        let mut written = self.header.write(&mut writer)?;
        written += self.ipl3.write(&mut writer)?;
        written += writer.write(&self.data)?;

        // Todo: Compare total amount written to expected length

        Ok(written)
    }

    pub fn len(&self) -> usize {
        HEADER_SIZE + IPL_SIZE + self.data.len()
    }
}
