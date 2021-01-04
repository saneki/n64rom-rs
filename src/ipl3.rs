use byteorder::{BigEndian, ByteOrder};
use crc32fast::Hasher;
use itertools::Itertools;
use std::fmt;
use std::fs::File;
use std::io::{self, Read, Write};
use std::num::Wrapping;
use std::path::Path;
use thiserror::Error;

pub const IPL_SIZE: usize = 0x0fc0;
pub const PROGRAM_SIZE: usize = 1024 * 1024;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    IOError(#[from] io::Error),
    #[error("Unable to read IPL3: Expected file size {}, found {0}", IPL_SIZE)]
    FileSizeError(u64),
}

#[derive(Clone, Copy)]
/// IPL3 definitions.
pub enum IPL3 {
    Cic6101([u8; IPL_SIZE]),
    Cic6102([u8; IPL_SIZE]),
    Cic6103([u8; IPL_SIZE]),
    Cic6105([u8; IPL_SIZE]),
    Cic6106([u8; IPL_SIZE]),
    Cic7102([u8; IPL_SIZE]),
    Unknown([u8; IPL_SIZE]),
}

impl fmt::Display for IPL3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Cic6101(_) => "CIC-NUS-6101",
            Self::Cic6102(_) => "CIC-NUS-6102",
            Self::Cic6103(_) => "CIC-NUS-6103",
            Self::Cic6105(_) => "CIC-NUS-6105",
            Self::Cic6106(_) => "CIC-NUS-6106",
            Self::Cic7102(_) => "CIC-NUS-7102",
            Self::Unknown(_) => "Unknown",
        };
        write!(f, "{}", s)
    }
}

impl fmt::Debug for IPL3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Self as fmt::Display>::fmt(self, f)
    }
}

impl IPL3 {
    pub fn read<T: Read>(reader: &mut T) -> io::Result<Self> {
        // Read file contents
        let mut ipl = [0; IPL_SIZE];
        reader.read_exact(&mut ipl)?;

        // Check for known IPLs
        let mut hasher = Hasher::new();
        hasher.update(&ipl);
        let ipl3 = match hasher.finalize() {
            0x6170_a4a1 => Self::Cic6101(ipl),
            0x90bb_6cb5 => Self::Cic6102(ipl),
            0x0b05_0ee0 => Self::Cic6103(ipl),
            0x98bc_2c86 => Self::Cic6105(ipl),
            0xacc8_580a => Self::Cic6106(ipl),
            0x009e_9ea3 => Self::Cic7102(ipl),
            _ => Self::Unknown(ipl),
        };

        Ok(ipl3)
    }

    pub fn read_path(path: impl AsRef<Path>) -> Result<Self, Error> {
        // TODO
        let mut f = File::open(path)?;

        // Check the file size
        let metadata = f.metadata()?;
        let len = metadata.len();
        if len as usize != IPL_SIZE {
            return Err(Error::FileSizeError(len))
        }

        let ipl3 = Self::read(&mut f)?;

        Ok(ipl3)
    }

    pub fn get_ipl(&self) -> &[u8; IPL_SIZE] {
        match self {
            Self::Cic6101(bin) => bin,
            Self::Cic6102(bin) => bin,
            Self::Cic6103(bin) => bin,
            Self::Cic6105(bin) => bin,
            Self::Cic6106(bin) => bin,
            Self::Cic7102(bin) => bin,
            Self::Unknown(bin) => bin,
        }
    }

    pub fn compute_crcs(&self, program: &[u8], fs: &[u8]) -> (u32, u32) {
        let padding_length = (2 - (program.len() & 1)) & 1;
        let padding = [0; 1];
        let program = program
            .iter()
            .chain(&padding[0..padding_length])
            .chain(fs.iter())
            .chain(std::iter::repeat(&0))
            .take(PROGRAM_SIZE)
            .cloned()
            .chunks(4);

        // Initial checksum value
        let checksum = match self {
            Self::Cic6103(_) => 0xa388_6759,
            Self::Cic6105(_) => 0xdf26_f436,
            Self::Cic6106(_) => 0x1fea_617a,
            _ => 0xf8ca_4ddc,
        };

        // NUS-IPL3-6105 has a special 64-word table hidden in the IPL
        let mut ipl = self.get_ipl().chunks(4).skip(452).take(64).cycle();

        // Six accumulators
        let mut acc1 = Wrapping(checksum);
        let mut acc2 = Wrapping(checksum);
        let mut acc3 = Wrapping(checksum);
        let mut acc4 = Wrapping(checksum);
        let mut acc5 = Wrapping(checksum);
        let mut acc6 = Wrapping(checksum);

        // Some temporary state
        let mut current;
        let mut rotated;

        // Iterate 1-word at a time
        for chunk in &program {
            // Fetch the current word and rotate it by itself
            current = Wrapping(BigEndian::read_u32(&chunk.collect::<Vec<_>>()));
            rotated = current.rotate_left((current & Wrapping(0x1f)).0);

            // Advance accumulator 1
            acc1 += current;

            // Advance accumulator 2
            if acc1 < current {
                acc2 += Wrapping(1);
            }

            // Advance accumulator 3
            acc3 ^= current;

            // Advance accumulator 4
            acc4 += rotated;

            // Advance accumulator 5
            if acc5 > current {
                acc5 ^= rotated;
            } else {
                acc5 ^= acc1 ^ current;
            }

            // Advance accumulator 6
            match self {
                Self::Cic6105(_) => {
                    let current_ipl = ipl.next().unwrap();
                    let current_ipl = Wrapping(BigEndian::read_u32(&current_ipl));
                    acc6 += current ^ current_ipl;
                }
                _ => {
                    acc6 += current ^ acc4;
                }
            }
        }

        let (crc1, crc2) = match self {
            Self::Cic6103(_) => ((acc1 ^ acc2) + acc3, (acc4 ^ acc5) + acc6),
            Self::Cic6106(_) => (acc1 * acc2 + acc3, acc4 * acc5 + acc6),
            _ => (acc1 ^ acc2 ^ acc3, acc4 ^ acc5 ^ acc6),
        };

        (crc1.0, crc2.0)
    }

    /// Offset the entry point for the current IPL3
    pub fn offset(&self, entry_point: u32) -> u32 {
        entry_point
            + match self {
                Self::Cic6103(_) => 0x0010_0000,
                Self::Cic6106(_) => 0x0020_0000,
                _ => 0,
            }
    }

    pub fn write<T: Write>(&self, writer: &'_ mut T) -> io::Result<usize> {
        let ipl = self.get_ipl();
        writer.write(ipl)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_ipl3_6101() {
        let ipl3 = IPL3::Cic6101([0; IPL_SIZE]);
        let program: Vec<u8> = (0..PROGRAM_SIZE).map(|i| i as u8).collect();

        let (crc1, crc2) = ipl3.compute_crcs(&program, &[]);

        assert_eq!(crc1, 0xfac8_47da);
        assert_eq!(crc2, 0xb2de_a121);
    }

    #[test]
    fn crc_ipl3_6102() {
        let ipl3 = IPL3::Cic6102([0; IPL_SIZE]);
        let program: Vec<u8> = (0..PROGRAM_SIZE).map(|i| i as u8).collect();

        let (crc1, crc2) = ipl3.compute_crcs(&program, &[]);

        assert_eq!(crc1, 0xfac8_47da);
        assert_eq!(crc2, 0xb2de_a121);
    }

    #[test]
    fn crc_ipl3_6103() {
        let ipl3 = IPL3::Cic6103([0; IPL_SIZE]);
        let program: Vec<u8> = (0..PROGRAM_SIZE).map(|i| i as u8).collect();

        let (crc1, crc2) = ipl3.compute_crcs(&program, &[]);

        assert_eq!(crc1, 0xa98e_6d67);
        assert_eq!(crc2, 0x3bee_c487);
    }

    #[test]
    fn crc_ipl3_6105() {
        let ipl3 = IPL3::Cic6105([0; IPL_SIZE]);
        let program: Vec<u8> = (0..PROGRAM_SIZE).map(|i| i as u8).collect();

        let (crc1, crc2) = ipl3.compute_crcs(&program, &[]);

        assert_eq!(crc1, 0xe124_ee34);
        assert_eq!(crc2, 0x8ceb_5e63);
    }

    #[test]
    fn crc_ipl3_6106() {
        let ipl3 = IPL3::Cic6106([0; IPL_SIZE]);
        let program: Vec<u8> = (0..PROGRAM_SIZE).map(|i| i as u8).collect();

        let (crc1, crc2) = ipl3.compute_crcs(&program, &[]);

        assert_eq!(crc1, 0x66c6_70aa);
        assert_eq!(crc2, 0x3874_9798);
    }

    #[test]
    fn crc_ipl3_7102() {
        let ipl3 = IPL3::Cic7102([0; IPL_SIZE]);
        let program: Vec<u8> = (0..PROGRAM_SIZE).map(|i| i as u8).collect();

        let (crc1, crc2) = ipl3.compute_crcs(&program, &[]);

        assert_eq!(crc1, 0xfac8_47da);
        assert_eq!(crc2, 0xb2de_a121);
    }

    #[test]
    fn offset_ipl3_6101() {
        let ipl3 = IPL3::Cic6101([0; IPL_SIZE]);
        assert_eq!(ipl3.offset(0x8000_0400), 0x8000_0400);
    }

    #[test]
    fn offset_ipl3_6102() {
        let ipl3 = IPL3::Cic6102([0; IPL_SIZE]);
        assert_eq!(ipl3.offset(0x8000_0400), 0x8000_0400);
    }

    #[test]
    fn offset_ipl3_6103() {
        let ipl3 = IPL3::Cic6103([0; IPL_SIZE]);
        assert_eq!(ipl3.offset(0x8000_0400), 0x8010_0400);
    }

    #[test]
    fn offset_ipl3_6105() {
        let ipl3 = IPL3::Cic6105([0; IPL_SIZE]);
        assert_eq!(ipl3.offset(0x8000_0400), 0x8000_0400);
    }

    #[test]
    fn offset_ipl3_6106() {
        let ipl3 = IPL3::Cic6106([0; IPL_SIZE]);
        assert_eq!(ipl3.offset(0x8000_0400), 0x8020_0400);
    }

    #[test]
    fn offset_ipl3_7102() {
        let ipl3 = IPL3::Cic7102([0; IPL_SIZE]);
        assert_eq!(ipl3.offset(0x8000_0400), 0x8000_0400);
    }
}
