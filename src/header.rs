use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::fmt;
use std::io::{self, Cursor};
use std::io::prelude::*;
use std::iter::FromIterator;
use std::str::{self, Utf8Error};
use thiserror::Error;

use crate::ipl3::IPL3;
use crate::rom::Endianness;
use crate::stream::Reader;

pub const HEADER_SIZE: usize = 0x40;
pub const MAGIC_SIZE: usize = 4;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    IOError(#[from] io::Error),
    #[error("Unknown byte order from magic ({0:#08X})")]
    UnknownByteOrder(u32),
}

#[derive(Clone, Copy)]
/// Represents the initial four bytes of the rom header.
///
/// This value is often used to infer the byte order of the rom data.
pub struct Magic {
    device_latency: u8,             // PI_BSD_DOM1_LAT_REG
    device_rw_pulse_width: u8,      // PI_BSD_DOM1_PWD_REG
    device_page_size: u8,           // PI_BSD_DOM1_PGS_REG
    device_rw_release_duration: u8, // PI_BSD_DOM1_RLS_REG
}

impl fmt::Display for Magic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let order = self.infer_endianness();
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
    /// Infer the byte order (endianness) of the following data.
    ///
    /// If Little or Mixed endianness, will need to read properly.
    pub fn infer_endianness(&self) -> Result<Endianness, Error>
    {
        let value = self.to_u32();
        match value {
            0x8037_1240 => Ok(Endianness::Big),
            0x4012_3780 => Ok(Endianness::Little),
            0x3780_4012 => Ok(Endianness::Mixed),
            _ => Err(Error::UnknownByteOrder(value)),
        }
    }

    /// Whether or not this value matches what is "expected".
    ///
    /// If we are reading the file correctly, it should match the BigEndian value.
    pub fn is_expected(&self) -> bool {
        let endianness = self.infer_endianness();
        matches!(endianness, Ok(Endianness::Big))
    }

    /// Construct using at least 4 bytes.
    pub fn from(bytes: &[u8]) -> Self {
        Self {
            device_latency: bytes[0],
            device_rw_pulse_width: bytes[1],
            device_page_size: bytes[2],
            device_rw_release_duration: bytes[3],
        }
    }

    /// Convert to 4 bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        vec![
            self.device_latency,
            self.device_rw_pulse_width,
            self.device_page_size,
            self.device_rw_release_duration,
        ]
    }

    /// Convert to a `u32` value.
    pub fn to_u32(&self) -> u32 {
        let bytes = self.to_bytes();
        let mut cursor = Cursor::new(bytes);
        cursor.read_u32::<BigEndian>().unwrap()
    }
}

#[derive(Clone, Copy)]
pub struct Header {
    // 0x00
    magic: Magic,
    clock_rate: u32,                // Unused by IPL and OS
    entry_point: u32,               // Executable start address/entry point
    release: u32,                   // Unused by IPL and OS

    // 0x10
    crate crc1: u32,
    crate crc2: u32,
    _reserved_1: [u8; 8],

    // 0x20
    name: [u8; 20],
    _reserved_2: [u8; 7],
    manufacturer: u8,
    cart_id: [u8; 2],
    region_code: u8,
    _reserved_3: u8,
}

impl fmt::Display for Header {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut builder = Vec::<String>::new();
        let name = self.name().unwrap_or("<???>");
        let region_id = self.region_id_as_str();
        builder.push(format!("N64 ROM Header: {}", name));
        builder.push(format!("  Checksums: (0x{:08X}, 0x{:08X})", self.crc1, self.crc2));
        builder.push(format!("  Region: {}", region_id));
        builder.push(format!("    Manufacturer: {}", self.manufacturer as char));
        builder.push(format!("    Cart ID:      {}{}", self.cart_id[0] as char, self.cart_id[1] as char));
        builder.push(format!("    Region Code:  {}", self.region_code as char));
        write!(f, "{}", builder.join("\n"))
    }
}

impl Header {
    pub fn crcs(&self) -> (u32, u32) {
        (self.crc1, self.crc2)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<(Self, Endianness), Error> {
        let mut cursor = Cursor::new(&bytes);
        Self::read(&mut cursor)
    }

    pub fn read<T: Read>(reader: &'_ mut T) -> Result<(Self, Endianness), Error> {
        let mut buf = [0u8; HEADER_SIZE];
        reader.read_exact(&mut buf)?;
        let buf = buf;

        let magic = Magic::from(&buf[..MAGIC_SIZE]);
        let order = magic.infer_endianness()?;

        let mut cursor = Cursor::new(buf.to_vec());
        let mut reader = Reader::from(&mut cursor, &order);
        let header = Self::read_raw(&mut reader)?;
        Ok((header, order))
    }

    /// Read without checking for endianness.
    pub fn read_raw<T: Read>(reader: &mut T) -> io::Result<Self> {
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        let magic = Magic::from(&magic);

        let clock_rate = reader.read_u32::<BigEndian>()?;
        let entry_point = reader.read_u32::<BigEndian>()?;
        let release = reader.read_u32::<BigEndian>()?;

        let crc1 = reader.read_u32::<BigEndian>()?;
        let crc2 = reader.read_u32::<BigEndian>()?;

        let mut _reserved_1 = [0u8; 8];
        reader.read_exact(&mut _reserved_1)?;
        let _reserved_1 = _reserved_1;

        let mut name = [0u8; 20];
        reader.read_exact(&mut name)?;
        let name = name;

        let mut _reserved_2 = [0u8; 7];
        reader.read_exact(&mut _reserved_2)?;
        let _reserved_2 = _reserved_2;

        let manufacturer = reader.read_u8()?;

        let mut cart_id = [0u8; 2];
        reader.read_exact(&mut cart_id)?;
        let cart_id = cart_id;

        let region_code = reader.read_u8()?;
        let _reserved_3 = reader.read_u8()?;

        let header = Self {
            // 0x00
            magic,
            clock_rate,
            entry_point,
            release,

            // 0x10
            crc1,
            crc2,
            _reserved_1,

            // 0x20
            name,
            _reserved_2,
            manufacturer,
            cart_id,
            region_code,
            _reserved_3,
        };

        Ok(header)
    }

    pub fn name(&self) -> Result<&str, Utf8Error> {
        str::from_utf8(&self.name)
    }

    pub fn fill_region_id(&self, region_id: &mut [u8; 4]) {
        region_id[0] = self.manufacturer;
        region_id[1] = self.cart_id[0];
        region_id[2] = self.cart_id[1];
        region_id[3] = self.region_code;
    }

    pub fn region_id(&self) -> Vec<u8> {
        let mut region_id = [0u8; 4];
        self.fill_region_id(&mut region_id);
        region_id.to_vec()
    }

    pub fn region_id_as_chars(&self) -> Vec<char> {
        let bytes = self.region_id();
        bytes.into_iter().map(|x| x as char).collect()
    }

    pub fn region_id_as_str(&self) -> String {
        let chars = self.region_id_as_chars();
        String::from_iter(&chars)
    }

    pub fn new(
        entry_point: u32,
        name_str: &str,
        program: &[u8],
        fs: &[u8],
        ipl3: &IPL3,
    ) -> Self {
        let (crc1, crc2) = ipl3.compute_crcs(program, fs);
        let entry_point = ipl3.offset(entry_point);

        let name_str = format!("{:20}", name_str);
        let mut name = [0; 20];
        name.copy_from_slice(name_str.as_bytes());
        let name = name;

        let cart_id_str = b"KW"; // KodeWerx!
        let mut cart_id = [0; 2];
        cart_id.copy_from_slice(cart_id_str);
        let cart_id = cart_id;

        let magic = Magic {
            device_latency: 128,
            device_rw_pulse_width: 55,
            device_page_size: 18,
            device_rw_release_duration: 64,
        };

        Self {
            // 0x00
            magic,
            clock_rate: 15,
            entry_point,
            release: 0,

            // 0x10
            crc1,
            crc2,
            _reserved_1: [0; 8],

            // 0x20
            name,
            _reserved_2: [0; 7],
            manufacturer: b'N', // Nintendo
            cart_id,
            region_code: b'E', // USA/English
            _reserved_3: 0,
        }
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buffer = Vec::new();

        // 0x00
        buffer.extend_from_slice(&self.magic.to_bytes());
        buffer.write_u32::<BigEndian>(self.clock_rate).unwrap();
        buffer.write_u32::<BigEndian>(self.entry_point).unwrap();
        buffer.write_u32::<BigEndian>(self.release).unwrap();

        // 0x10
        buffer.write_u32::<BigEndian>(self.crc1).unwrap();
        buffer.write_u32::<BigEndian>(self.crc2).unwrap();
        buffer.extend_from_slice(&self._reserved_1);

        // 0x20
        buffer.extend_from_slice(&self.name);
        buffer.extend_from_slice(&self._reserved_2);
        buffer.push(self.manufacturer);
        buffer.extend_from_slice(&self.cart_id);
        buffer.push(self.region_code);
        buffer.push(self._reserved_3);

        buffer
    }

    pub fn write<T: Write>(&self, writer: &'_ mut T) -> io::Result<usize> {
        let data = self.to_vec();
        writer.write(&data)
    }
}
