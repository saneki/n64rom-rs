use std::fmt;
use std::io::{self, Read, Write};

use crate::bytes::Endianness;
use crate::header::{Header, HeaderError, HEADER_SIZE};
use crate::io::{Reader, Writer};
use crate::ipl3::{IPL3, IPL_SIZE};
use crate::util::{FileSize, MEBIBYTE};

/// Total size of rom header and IPL3. This will be the file offset where data begins.
pub const HEAD_SIZE: usize = HEADER_SIZE + IPL_SIZE;

#[derive(Clone)]
pub struct Rom {
    pub header: Header,
    pub ipl3: IPL3,
    pub data: Vec<u8>,

    /// Byte order (endianness) of rom file.
    order: Endianness,
}

impl fmt::Display for Rom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut builder = Vec::<String>::new();
        builder.push(format!("{}", self.header));
        builder.push(format!("  IPL3: {}", self.ipl3));
        builder.push(format!("  Byte Order: {}", self.order));
        // Only show rom size if we have data.
        if self.data.len() > 0 {
            let filesize = FileSize::from(self.len() as u64, MEBIBYTE);
            match filesize {
                FileSize::Float(value) => {
                    builder.push(format!("  Rom Size: {:.*} MiB", 1, value));
                }
                FileSize::Int(value) => {
                    builder.push(format!("  Rom Size: {} MiB", value));
                }
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

    pub fn from(header: Header, ipl3: IPL3, data: Vec<u8>, order: Endianness) -> Self {
        Self {
            header,
            ipl3,
            data,
            order,
        }
    }

    pub fn order(&self) -> &Endianness {
        &self.order
    }

    /// Read Rom with all data.
    pub fn read<T: Read>(mut reader: &mut T) -> Result<Self, HeaderError> {
        Self::read_with_body(&mut reader, true)
    }

    /// Read Rom.
    pub fn read_with_body<T: Read>(mut reader: &mut T, read_body: bool) -> Result<Self, HeaderError> {
        // Read header & infer endianness
        let (header, order) = Header::read(&mut reader)?;

        // Create new reader based on endianness, read remaining with it
        let mut reader = Reader::from(&mut reader, &order);
        let ipl3 = IPL3::read(&mut reader)?;

        // Read data if specified
        let mut data = Vec::new();
        if read_body {
            reader.read_to_end(&mut data)?;
        }
        let data = data;

        let rom = Self {
            header,
            ipl3,
            data,
            order,
        };

        Ok(rom)
    }

    pub fn write<'a, T: Write>(&self, writer: &'a mut T, endianness: Option<&Endianness>) -> io::Result<usize> {
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
        writer.flush()?;

        // Todo: Compare total amount written to expected length

        Ok(written)
    }

    pub fn len(&self) -> usize {
        HEAD_SIZE + self.data.len()
    }
}
