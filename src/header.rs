use std::fmt::{self, Display, Formatter};
use std::io::{self, Cursor};
use std::io::prelude::*;
use std::str::{self, Utf8Error};
use std::string::FromUtf8Error;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use crate::ipl3::IPL3;

pub const HEADER_SIZE: usize = 0x40;

#[derive(Debug, Clone, Copy)]
pub struct N64Header {
    // 0x00
    device_latency: u8,             // PI_BSD_DOM1_LAT_REG
    device_rw_pulse_width: u8,      // PI_BSD_DOM1_PWD_REG
    device_page_size: u8,           // PI_BSD_DOM1_PGS_REG
    device_rw_release_duration: u8, // PI_BSD_DOM1_RLS_REG
    clock_rate: u32,                // Unused by IPL and OS
    entry_point: u32,               // Executable start address/entry point
    release: u32,                   // Unused by IPL and OS

    // 0x10
    crc1: u32,
    crc2: u32,
    _reserved_1: [u8; 8],

    // 0x20
    name: [u8; 20],
    _reserved_2: [u8; 7],
    manufacturer: u8,
    cart_id: [u8; 2],
    region_code: u8,
    _reserved_3: u8,
}

impl fmt::Display for N64Header {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut builder = Vec::<String>::new();
        let name = self.name().unwrap_or("<???>");
        let region_id = self.region_id_as_str().unwrap_or(String::from("????"));
        builder.push(format!("N64 ROM Header: {}", name));
        builder.push(String::from("  Device:"));
        builder.push(format!("    Latency:             0x{:02X}", self.device_latency));
        builder.push(format!("    RW Pulse Width:      0x{:02X}", self.device_rw_pulse_width));
        builder.push(format!("    Page Size:           0x{:02X}", self.device_page_size));
        builder.push(format!("    RW Release Duration: 0x{:02X}", self.device_rw_release_duration));
        builder.push(format!("  Checksums: 0x{:08X} 0x{:08X}", self.crc1, self.crc2));
        builder.push(format!("  Region: {}", region_id));
        builder.push(format!("    Manufacturer: {}", self.manufacturer as char));
        builder.push(format!("    Cart ID:      {}{}", self.cart_id[0] as char, self.cart_id[1] as char));
        builder.push(format!("    Region Code:  {}", self.region_code as char));
        write!(f, "{}", builder.join("\n"))
    }
}

impl N64Header {
    pub fn from_bytes(bytes: &[u8]) -> io::Result<N64Header> {
        let mut cursor = Cursor::new(&bytes);
        Self::read(&mut cursor)
    }

    pub fn read<T>(reader: &mut T) -> io::Result<N64Header>
    where
        T: Read,
    {
        let device_latency = reader.read_u8()?;
        let device_rw_pulse_width = reader.read_u8()?;
        let device_page_size = reader.read_u8()?;
        let device_rw_release_duration = reader.read_u8()?;
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

        let header = N64Header {
            // 0x00
            device_latency,
            device_rw_pulse_width,
            device_page_size,
            device_rw_release_duration,
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

    pub fn region_id_as_str(&self) -> Result<String, FromUtf8Error> {
        String::from_utf8(self.region_id())
    }

    pub fn new(
        entry_point: u32,
        name_str: &str,
        program: &[u8],
        fs: &[u8],
        ipl3: &IPL3,
    ) -> N64Header {
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

        N64Header {
            // 0x00
            device_latency: 128,
            device_rw_pulse_width: 55,
            device_page_size: 18,
            device_rw_release_duration: 64,
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
        buffer.push(self.device_latency);
        buffer.push(self.device_rw_pulse_width);
        buffer.push(self.device_page_size);
        buffer.push(self.device_rw_release_duration);
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
}
