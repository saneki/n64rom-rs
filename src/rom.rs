use failure::Fail;
use std::fmt;
use std::io::{self, Read, Write};

use crate::bytes::Endianness;
use crate::header::{Header, HeaderError, HEADER_SIZE};
use crate::io::{Reader, Writer};
use crate::ipl3::{IPL3, IPL_SIZE};
use crate::util::{FileSize, MEBIBYTE};

/// Total size of rom header and IPL3. This will be the file offset where data begins.
pub const HEAD_SIZE: usize = HEADER_SIZE + IPL_SIZE;

#[derive(Debug, Fail)]
pub enum RomError {
    #[fail(display = "{}", _0)]
    IOError(#[cause] io::Error),

    #[fail(display = "{}", _0)]
    HeaderError(#[cause] HeaderError),

    #[fail(display = "Unsupported endianness for this operation: {}", _0)]
    UnsupportedEndianness(Endianness),
}

impl From<io::Error> for RomError {
    fn from(e: io::Error) -> Self {
        RomError::IOError(e)
    }
}

impl From<HeaderError> for RomError {
    fn from(e: HeaderError) -> Self {
        RomError::HeaderError(e)
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

    /// Construct from a raw image without copying. Requires image data to be in big-endian format.
    pub fn from_image(image: Vec<u8>) -> Result<Self, RomError> {
        let mut head = &image[..HEAD_SIZE];
        // Read header & infer endianness.
        let (header, order) = Header::read(&mut head)?;
        if order == Endianness::Big {
            let ipl3 = IPL3::read(&mut head)?;
            Ok(Rom::from(header, ipl3, image, order))
        } else {
            Err(RomError::UnsupportedEndianness(order))
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

        // Read rom data into buffer.
        let mut image = Vec::new();
        image.extend(header.to_vec());
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
        let mut writer = Writer::from(writer, order);

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
}
