use std::io::{Read, Result, Write};
use std::string::ToString;
use crate::bytes::{self, BigEndian, LittleEndian, Mixed};

// Todo: Assert compile-time check of divisible-by-4
const BUFFER_SIZE: usize = 4096;

#[derive(Clone, Copy)]
/// Convenience wrapper enum around the separate Swap endianness enums.
pub enum Endianness {
    Big,
    Little,
    Mixed,
}

impl ToString for Endianness {
    fn to_string(&self) -> String {
        match self {
            Endianness::Big => String::from("Big Endian"),
            Endianness::Little => String::from("Little Endian"),
            Endianness::Mixed => String::from("Mixed"),
        }
    }
}

/// Wrapper implementation around the separate Swap endianness enums.
impl Endianness {
    fn swap(&self, buf: &mut [u8]) {
        match self {
            Endianness::Big => bytes::swap_bytes::<BigEndian>(buf),
            Endianness::Little => bytes::swap_bytes::<LittleEndian>(buf),
            Endianness::Mixed => bytes::swap_bytes::<Mixed>(buf),
        }
    }
}

pub struct Reader<'r, T>
where
    T: Read,
{
    buffer: [u8; BUFFER_SIZE],
    endianness: Endianness,
    idx: usize,
    length: usize,
    reader: &'r mut T,
}

impl<'r, T> Reader<'r, T>
where
    T: Read,
{
    pub fn from(reader: &'r mut T, endianness: &Endianness) -> Self {
        Self {
            buffer: [0; BUFFER_SIZE],
            endianness: *endianness,
            idx: 0,
            length: 0,
            reader,
        }
    }

    /// Read bytes from the buffer.
    fn buf_read(&mut self, length: usize) -> &[u8] {
        let buf = &self.buffer[self.idx..self.idx + length];
        self.idx = self.idx + length;
        buf
    }

    /// Refill the contents of the buffer and reset the index to 0.
    fn refill(&mut self) -> Result<usize> {
        let length = self.reader.read(&mut self.buffer)?;
        self.endianness.swap(&mut self.buffer[..length]);
        self.idx = 0;
        self.length = length;
        Ok(length)
    }

    /// Get the amount of data remaining in the buffer.
    fn remaining(&self) -> usize {
        self.length - self.idx
    }
}

impl<'r, T> Read for Reader<'r, T>
where
    T: Read,
{
    fn read(&mut self, mut buf: &mut [u8]) -> Result<usize> {
        let mut length = buf.len();
        let mut wlength = 0;

        if self.remaining() == 0 {
            self.refill()?;
        }

        loop {
            let remaining = self.remaining();
            if length <= remaining {
                // Final write from buffer
                let data = self.buf_read(length);
                let written = buf.write(data)?;

                // Update remaining length and write length
                // length = length - written;
                wlength = wlength + written;

                break;
            } else {
                // Write the remaining data, and refresh buffer
                let data = self.buf_read(remaining);
                let written = buf.write(data)?;

                // Update remaining length and write length
                length = length - written;
                wlength = wlength + written;

                let amt = self.refill()?;
                if amt == 0 {
                    break;
                }
            }
        }

        Ok(wlength)
    }
}
