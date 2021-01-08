use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;
use thiserror::Error;

use crate::header::Magic;
use crate::rom::{Endianness, Rom};

#[derive(Debug, Error)]
pub enum Error {
    #[error("Buffer length must be 4-byte aligned to perform conversion, instead found length: {0}")]
    AlignmentError(usize),
    #[error("During conversion, read {0} bytes but only wrote {1} bytes")]
    FileWriteError(usize, usize),
    #[error("{0}")]
    HeaderError(#[from] crate::header::Error),
    #[error("{0}")]
    IOError(#[from] io::Error),
}

/// Perform 4-byte swap between Big Endian and Little Endian.
fn swap_big_little(buf: &mut [u8]) {
    buf.swap(0, 3);
    buf.swap(1, 2);
}

/// Perform 4-byte swap between Big Endian and Mixed Endian.
fn swap_big_mixed(buf: &mut [u8]) {
    buf.swap(0, 1);
    buf.swap(2, 3);
}

/// Perform 4-byte swap between Little Endian and Mixed Endian.
fn swap_little_mixed(buf: &mut [u8]) {
    buf.swap(0, 2);
    buf.swap(1, 3);
}

pub enum ConvertStatus {
    AlreadyConverted,
    Complete,
}

trait RomConvert {
    fn convert_to_big(buf: &mut [u8]) -> ConvertStatus;
    fn convert_to_little(buf: &mut [u8]) -> ConvertStatus;
    fn convert_to_mixed(buf: &mut [u8]) -> ConvertStatus;
}

struct BigEndianConverter;

impl RomConvert for BigEndianConverter {
    fn convert_to_big(_: &mut [u8]) -> ConvertStatus {
        ConvertStatus::AlreadyConverted
    }

    fn convert_to_little(buf: &mut [u8]) -> ConvertStatus {
        for chunk in buf.chunks_exact_mut(4) {
            swap_big_little(chunk);
        }
        ConvertStatus::Complete
    }

    fn convert_to_mixed(buf: &mut [u8]) -> ConvertStatus {
        for chunk in buf.chunks_exact_mut(4) {
            swap_big_mixed(chunk);
        }
        ConvertStatus::Complete
    }
}

struct LittleEndianConverter;

impl RomConvert for LittleEndianConverter {
    fn convert_to_big(buf: &mut [u8]) -> ConvertStatus {
        for chunk in buf.chunks_exact_mut(4) {
            swap_big_little(chunk);
        }
        ConvertStatus::Complete
    }

    fn convert_to_little(_: &mut [u8]) -> ConvertStatus {
        ConvertStatus::AlreadyConverted
    }

    fn convert_to_mixed(buf: &mut [u8]) -> ConvertStatus {
        for chunk in buf.chunks_exact_mut(4) {
            swap_little_mixed(chunk);
        }
        ConvertStatus::Complete
    }
}

struct MixedEndianConverter;

impl RomConvert for MixedEndianConverter {
    fn convert_to_big(buf: &mut [u8]) -> ConvertStatus {
        for chunk in buf.chunks_exact_mut(4) {
            swap_big_mixed(chunk);
        }
        ConvertStatus::Complete
    }

    fn convert_to_little(buf: &mut [u8]) -> ConvertStatus {
        for chunk in buf.chunks_exact_mut(4) {
            swap_little_mixed(chunk);
        }
        ConvertStatus::Complete
    }

    fn convert_to_mixed(_: &mut [u8]) -> ConvertStatus {
        ConvertStatus::AlreadyConverted
    }
}

fn convert_with<T: RomConvert>(buf: &mut [u8], target: Endianness) -> ConvertStatus {
    match target {
        Endianness::Big => T::convert_to_big(buf),
        Endianness::Little => T::convert_to_little(buf),
        Endianness::Mixed => T::convert_to_mixed(buf),
    }
}

/// Convert data from the current `Endianness` to a target `Endianness`.
pub fn convert(buf: &mut [u8], current: Endianness, target: Endianness) -> Result<ConvertStatus, Error> {
    if buf.len() % 4 == 0 {
        let result = match current {
            Endianness::Big =>
                convert_with::<BigEndianConverter>(buf, target),
            Endianness::Little =>
                convert_with::<LittleEndianConverter>(buf, target),
            Endianness::Mixed =>
                convert_with::<MixedEndianConverter>(buf, target),
        };
        Ok(result)
    } else {
        Err(Error::AlignmentError(buf.len()))
    }
}

/// Convert `Rom` data to a target `Endianness`.
pub fn convert_rom(rom: &mut Rom, target: Endianness) -> Result<ConvertStatus, Error> {
    let order = rom.order();
    convert(&mut rom.image, order, target)
}

/// Convenience function to convert a given rom `File` to the specified `Endianness`.
pub fn convert_rom_file(in_file: &mut File, out_file: &mut File, target: Endianness) -> Result<(ConvertStatus, usize), Error> {
    // Read first 4 bytes (magic value) to infer endianness.
    in_file.seek(SeekFrom::Start(0))?;
    let mut magic_bytes: [u8; 4] = [0; 4];
    in_file.read_exact(&mut magic_bytes)?;
    let order = Magic::infer_byte_order(&magic_bytes)?;

    // Determine filesize in attempt to prevent buffer from re-allocating.
    let filesize = std::cmp::min(in_file.metadata()?.len(), crate::rom::MAX_SIZE as u64);
    let mut contents = Vec::with_capacity(filesize as usize);

    // Read file into memory and perform conversion.
    in_file.seek(SeekFrom::Start(0))?;
    let mut handle = in_file.take(crate::rom::MAX_SIZE as u64);
    let read_amount = handle.read_to_end(&mut contents)?;

    // Perform endianness conversion.
    let result = convert(&mut contents, order, target)?;

    // Write resulting contents to same file.
    let write_amount = out_file.write(&contents)?;

    if read_amount == write_amount {
        Ok((result, read_amount))
    } else {
        Err(Error::FileWriteError(read_amount, write_amount))
    }
}

/// Convenience function to convert a rom file at a given `Path` to the specified `Endianness`.
pub fn convert_rom_path(in_path: impl AsRef<Path>, out_path: impl AsRef<Path>, target: Endianness) -> Result<(ConvertStatus, usize), Error> {
    let mut in_file = OpenOptions::new().read(true).open(in_path)?;
    let mut out_file = OpenOptions::new().write(true).create(true).truncate(true).open(out_path)?;
    convert_rom_file(&mut in_file, &mut out_file, target)
}
