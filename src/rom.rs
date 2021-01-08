use std::fmt;
use std::io::{self, Read, Write};
use thiserror::Error;

use crate::header::Header;
use crate::ipl3::{IPL3, IPL_SIZE};
use crate::stream::{Reader, Writer};
use crate::util::{FileSize, MEBIBYTE};

/// Total size of rom header and IPL3. This will be the file offset where data begins.
pub const HEAD_SIZE: usize = Header::SIZE + IPL_SIZE;

/// Maximum expected rom size (64 MiB).
pub const MAX_SIZE: usize = 1024 * 1024 * 64;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    IOError(#[from] io::Error),
    #[error("{0}")]
    HeaderError(#[from] crate::header::Error),
    #[error("Unsupported endianness for this operation: {0}")]
    UnsupportedEndianness(Endianness),
}

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

#[derive(Clone)]
pub struct Rom {
    pub header: Header,
    pub ipl3: IPL3,
    /// Full Rom image data.
    pub image: Vec<u8>,
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
        if self.image.len() > HEAD_SIZE {
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
        let calc = self.ipl3.compute_crcs(&self.image[HEAD_SIZE..], &[]);
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

    /// Get slice of Rom image data, not including header or IPL3.
    pub fn data(&self) -> &[u8] {
        &self.image[HEAD_SIZE..]
    }

    /// Get slice of Rom image data as mutable, not including header or IPL3.
    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.image[HEAD_SIZE..]
    }

    /// Construct from a raw image without copying. Requires image data to be in big-endian format.
    pub fn from_image(image: Vec<u8>) -> Result<Self, Error> {
        let mut head = &image[..HEAD_SIZE];
        // Read header & infer endianness.
        let (header, order) = Header::read_ordered(&mut head)?;
        if order == Endianness::Big {
            let ipl3 = IPL3::read(&mut head)?;
            Ok(Rom::from(header, ipl3, image, order))
        } else {
            Err(Error::UnsupportedEndianness(order))
        }
    }

    pub fn from(header: Header, ipl3: IPL3, image: Vec<u8>, order: Endianness) -> Self {
        Self {
            header,
            ipl3,
            image,
            order,
        }
    }

    /// Get slice of full Rom image data.
    pub fn full(&self) -> &[u8] {
        &self.image[..]
    }

    /// Get slice of full Rom image data as mutable.
    pub fn full_mut(&mut self) -> &mut [u8] {
        &mut self.image[..]
    }

    pub fn order(&self) -> Endianness {
        self.order
    }

    /// Read Rom with all data.
    pub fn read<T: Read>(mut reader: &mut T) -> Result<Self, crate::header::Error> {
        Self::read_with_body(&mut reader, true)
    }

    /// Read Rom.
    pub fn read_with_body<T: Read>(mut reader: &mut T, read_body: bool) -> Result<Self, crate::header::Error> {
        // Read header & infer endianness
        let (header, order) = Header::read_ordered(&mut reader)?;

        // Create new reader based on endianness, read remaining with it
        let mut reader = Reader::from(&mut reader, order);
        let ipl3 = IPL3::read(&mut reader)?;

        // Read rom data into buffer.
        let mut image = Vec::new();
        header.write(&mut image)?;
        image.extend(ipl3.get_ipl());
        // Read remaining data if specified.
        if read_body {
            reader.read_to_end(&mut image)?;
        }
        let image = image;

        let rom = Self {
            header,
            ipl3,
            image,
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
        let mut writer = Writer::from(writer, *order);

        // Write header, IPL3 and data
        let mut written = self.header.write(&mut writer)?;
        written += self.ipl3.write(&mut writer)?;
        written += writer.write(&self.image[HEAD_SIZE..])?;
        writer.flush()?;

        // Todo: Compare total amount written to expected length

        Ok(written)
    }

    pub fn len(&self) -> usize {
        self.image.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
