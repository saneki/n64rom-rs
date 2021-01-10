use byteorder::{BigEndian, ByteOrder, ReadBytesExt, WriteBytesExt};
use std::fmt;
use std::io::{self, Cursor};
use std::io::prelude::*;
use std::str::{self, Utf8Error};
use thiserror::Error;

use crate::convert;
use crate::ipl3::IPL3;
use crate::rom::Endianness;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    IOError(#[from] io::Error),
    #[error("Unknown byte order from magic ({0:#08X})")]
    UnknownByteOrder(u32),
}

#[derive(Clone, Copy, Default)]
/// Represents the initial four bytes of the rom header.
///
/// This value is often used to infer the byte order of the rom data.
pub struct Magic([u8; 4]);

impl fmt::Display for Magic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let order = self.byte_order();
        match order {
            Ok(Endianness::Big) => write!(f, "Big Endian"),
            Ok(Endianness::Little) => write!(f, "Little Endian"),
            Ok(Endianness::Mixed) => write!(f, "Mixed Endian"),
            Err(Error::UnknownByteOrder(val)) => {
                write!(f, "Unknown (0x{:08X})", val)
            }
            _ => write!(f, "Unknown"),
        }
    }
}

impl Magic {
    pub const SIZE: usize = 4;

    /// Infer the byte order (endianness) of the following data.
    pub fn byte_order(&self) -> Result<Endianness, Error> {
        Magic::infer_byte_order(&self.0)
    }

    // Register: `PI_BSD_DOM1_LAT_REG`.
    pub fn device_latency(&self) -> u8 {
        self.0[0]
    }

    // Register: `PI_BSD_DOM1_PGS_REG`.
    pub fn device_page_size(&self) -> u8 {
        self.0[2]
    }

    // Register: `PI_BSD_DOM1_PWD_REG`.
    pub fn device_rw_pulse_width(&self) -> u8 {
        self.0[1]
    }

    // Register: `PI_BSD_DOM1_RLS_REG`.
    pub fn device_rw_release_duration(&self) -> u8 {
        self.0[3]
    }

    /// Construct using at least 4 bytes.
    pub fn from(bytes: &[u8]) -> Self {
        // Cleaner way to do this?
        let mut magic = Magic::new();
        magic.0.copy_from_slice(&bytes[..4]);
        magic
    }

    pub fn infer_byte_order(data: &[u8]) -> Result<Endianness, Error> {
        let value = BigEndian::read_u32(data);
        match value {
            0x8037_1240 => Ok(Endianness::Big),
            0x4012_3780 => Ok(Endianness::Little),
            0x3780_4012 => Ok(Endianness::Mixed),
            _ => Err(Error::UnknownByteOrder(value)),
        }
    }

    pub fn new() -> Self {
        Self([128, 55, 18, 64])
    }

    /// Convert to a `u32` value.
    pub fn to_u32(&self) -> u32 {
        BigEndian::read_u32(self.as_ref())
    }
}

impl AsMut<[u8; 4]> for Magic {
    fn as_mut(&mut self) -> &mut [u8; 4] {
        &mut self.0
    }
}

impl AsRef<[u8; 4]> for Magic {
    fn as_ref(&self) -> &[u8; 4] {
        &self.0
    }
}

/// Media format of rom.
#[derive(Clone, Copy, Default)]
pub struct Media([u8; 4]);

impl Media {
    /// Get slice as string.
    pub fn as_str(&self) -> Result<&str, Utf8Error> {
        str::from_utf8(&self.0)
    }

    /// Get all values as `char` tuple.
    pub fn chars(&self) -> (char, char, char, char) {
        (self.0[0] as char, self.0[1] as char, self.0[2] as char, self.0[3] as char)
    }

    /// Get all values as `u8` tuple.
    pub fn values(&self) -> (u8, u8, u8, u8) {
        (self.0[0], self.0[1], self.0[2], self.0[3])
    }
}

impl AsMut<[u8; 4]> for Media {
    fn as_mut(&mut self) -> &mut [u8; 4] {
        &mut self.0
    }
}

impl AsRef<[u8; 4]> for Media {
    fn as_ref(&self) -> &[u8; 4] {
        &self.0
    }
}

#[derive(Clone, Copy, Default)]
pub struct Header {
    // Magic number and PI registers.
    magic: Magic,
    /// Unused by IPL and OS.
    clock_rate: u32,
    /// Executable start address/entry point.
    entry_point: u32,
    /// Unused by IPL and OS.
    release: u32,
    /// First CRC value.
    crate crc1: u32,
    /// Second CRC value.
    crate crc2: u32,
    _reserved_1: [u8; 8],
    /// Rom name.
    name: [u8; 20],
    _reserved_2: [u8; 7],
    /// Region identifier.
    media: Media,
    _reserved_3: u8,
}

impl fmt::Display for Header {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self.name_str().unwrap_or("<???>").trim();
        let media_str = self.media.as_str().unwrap_or("????");
        write!(formatter, "N64 ROM Header: {}\n", name)?;
        write!(formatter, "  Checksums: (0x{:08X}, 0x{:08X})\n", self.crc1, self.crc2)?;
        write!(formatter, "  Media Format: {}", media_str)
    }
}

impl Header {
    pub const SIZE: usize = 0x40;

    /// Get CRC values.
    pub fn crcs(&self) -> (u32, u32) {
        (self.crc1, self.crc2)
    }

    /// Get magic number field.
    pub fn magic(&self) -> &Magic {
        &self.magic
    }

    /// Get media format field.
    pub fn media(&self) -> &Media {
        &self.media
    }

    /// Get rom name as bytes.
    pub fn name(&self) -> &[u8; 20] {
        &self.name
    }

    /// Get rom name decoded as UTF-8.
    pub fn name_str(&self) -> Result<&str, Utf8Error> {
        str::from_utf8(&self.name)
    }

    /// Create a new `Header`.
    pub fn new(entry_point: u32, name: &str, media: &[u8], program: &[u8], fs: &[u8], ipl3: &IPL3) -> Self {
        let mut header = Self::default();
        let (crc1, crc2) = ipl3.compute_crcs(program, fs);
        let name_bytes = &name.as_bytes()[..20];
        header.magic = Magic::new();
        header.clock_rate = 15;
        header.entry_point = ipl3.offset(entry_point);
        header.release = 0;
        header.crc1 = crc1;
        header.crc2 = crc2;
        header.name.copy_from_slice(name_bytes);
        header.media.as_mut().copy_from_slice(&media[..4]);
        header
    }

    /// Read ordered by converting to big endian.
    pub fn read_ordered<T: Read>(reader: &'_ mut T) -> Result<(Self, Endianness), Error> {
        let mut buf = [0; Header::SIZE];
        reader.read_exact(&mut buf)?;
        // Infer byte order and convert buffer to big endian.
        let order = Magic::infer_byte_order(&buf)?;
        convert::convert(&mut buf, order, Endianness::Big).unwrap();
        let buf = buf;
        // Read Header from buffer.
        let mut cursor = Cursor::new(&buf);
        let header = Self::read(&mut cursor)?;
        Ok((header, order))
    }

    /// Read without checking for endianness.
    pub fn read<T: Read>(reader: &mut T) -> io::Result<Self> {
        let mut header = Header::default();
        reader.read_exact(header.magic.as_mut())?;
        header.clock_rate = reader.read_u32::<BigEndian>()?;
        header.entry_point = reader.read_u32::<BigEndian>()?;
        header.release = reader.read_u32::<BigEndian>()?;
        header.crc1 = reader.read_u32::<BigEndian>()?;
        header.crc2 = reader.read_u32::<BigEndian>()?;
        reader.read_exact(&mut header._reserved_1)?;
        reader.read_exact(&mut header.name)?;
        reader.read_exact(&mut header._reserved_2)?;
        reader.read_exact(header.media.as_mut())?;
        header._reserved_3 = reader.read_u8()?;
        Ok(header)
    }

    pub fn write<T: Write>(&self, writer: &'_ mut T) -> io::Result<usize> {
        writer.write_all(self.magic.as_ref())?;
        writer.write_u32::<BigEndian>(self.clock_rate)?;
        writer.write_u32::<BigEndian>(self.entry_point)?;
        writer.write_u32::<BigEndian>(self.release)?;
        writer.write_u32::<BigEndian>(self.crc1)?;
        writer.write_u32::<BigEndian>(self.crc2)?;
        writer.write_all(&self._reserved_1)?;
        writer.write_all(&self.name)?;
        writer.write_all(&self._reserved_2)?;
        writer.write_all(self.media.as_ref())?;
        writer.write_u8(self._reserved_3)?;
        Ok(Header::SIZE)
    }
}
