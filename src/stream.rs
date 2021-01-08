use std::io::{Read, Result, Write};

use crate::convert;
use crate::rom::Endianness;

/// Default buffer size to use for `Reader` and `Writer`.
const BUFFER_SIZE: usize = 1024 * 16;

// Assert default buffer size is divisible by 4.
const_assert_eq!(BUFFER_SIZE % 4, 0);

/// Reader for translating data from a base `Endianness` into `Endianness::Big` format.
pub struct Reader<'r, T: Read> {
    buffer: Vec<u8>,
    endianness: Endianness,
    idx: usize,
    length: usize,
    reader: &'r mut T,
}

impl<'r, T: Read> Reader<'r, T> {
    pub fn from(reader: &'r mut T, endianness: Endianness) -> Self {
        Self::with_buffer_size(reader, endianness, BUFFER_SIZE)
    }

    pub fn with_buffer_size(reader: &'r mut T, endianness: Endianness, capacity: usize) -> Self {
        Self {
            buffer: vec![0; capacity],
            endianness,
            idx: 0,
            length: 0,
            reader,
        }
    }

    /// Read bytes from the buffer.
    fn buf_read(&mut self, length: usize) -> &[u8] {
        let buf = &self.buffer[self.idx..self.idx + length];
        self.idx += length;
        buf
    }

    /// Refill the contents of the buffer and reset the index to 0.
    fn refill(&mut self) -> Result<usize> {
        let length = self.reader.read(&mut *self.buffer)?;
        convert::convert(&mut self.buffer[..length], self.endianness, Endianness::Big).unwrap();
        self.idx = 0;
        self.length = length;
        Ok(length)
    }

    /// Get the amount of data remaining in the buffer.
    fn remaining(&self) -> usize {
        self.length - self.idx
    }
}

impl<'r, T: Read> Read for Reader<'r, T> {
    fn read(&mut self, mut buf: &mut [u8]) -> Result<usize> {
        let length = buf.len();
        let mut written = 0;

        if self.remaining() == 0 {
            self.refill()?;
        }

        loop {
            let remaining = self.remaining();
            if (length - written) <= remaining {
                // Final write from buffer
                let data = self.buf_read(length - written);
                let wrote = buf.write(data)?;
                written += wrote;
                break;
            } else {
                // Write the remaining data, and refresh buffer
                let data = self.buf_read(remaining);
                let wrote = buf.write(data)?;
                written += wrote;

                if self.refill()? == 0 {
                    break;
                }
            }
        }

        Ok(written)
    }
}

/// Writer for translating data from `Endianness::Big` into a base `Endianness` format.
pub struct Writer<'w, T: Write> {
    buffer: Vec<u8>,
    endianness: Endianness,
    length: usize,
    writer: &'w mut T,
}

impl<'w, T: Write> Writer<'w, T> {
    pub fn from(writer: &'w mut T, endianness: Endianness) -> Self {
        Self::with_buffer_size(writer, endianness, BUFFER_SIZE)
    }

    pub fn with_buffer_size(writer: &'w mut T, endianness: Endianness, capacity: usize) -> Self {
        Self {
            buffer: vec![0; capacity],
            endianness,
            length: 0,
            writer,
        }
    }

    fn buf_write(&mut self, bytes: &[u8]) -> usize {
        let slice = &mut self.buffer[self.length..self.length + bytes.len()];
        slice.copy_from_slice(bytes);
        self.length += bytes.len();
        bytes.len()
    }

    /// Flush buffer without flushing the underlying writer.
    fn buf_flush(&mut self) -> Result<()> {
        convert::convert(&mut self.buffer[..self.length], Endianness::Big, self.endianness).unwrap();
        let data = &self.buffer[..self.length];
        self.writer.write_all(data)?;
        self.length = 0;
        Ok(())
    }

    fn remaining(&self) -> usize {
        self.buffer.len() - self.length
    }
}

impl<'w, T: Write> Write for Writer<'w, T> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let mut idx = 0;
        let mut written = 0;

        loop {
            let remaining = self.remaining();
            if remaining >= (buf.len() - idx) {
                // If buffer has enough space, write all to it
                let wrote = self.buf_write(&buf[idx..]);
                written += wrote;
                break;
            } else {
                // let slice = &buf[idx..idx + remaining];
                let wrote = self.buf_write(&buf[idx..idx + remaining]);
                self.buf_flush()?;
                written += wrote;
                idx += remaining;
            }
        }

        Ok(written)
    }

    fn flush(&mut self) -> Result<()> {
        self.buf_flush()?;
        self.writer.flush()
    }
}
