use std::io::{Read, Result, Write};
use crate::bytes::Endianness;

// Todo: Assert compile-time check of divisible-by-4
const BUFFER_SIZE: usize = 1024 * 16;

pub struct Reader<'r, T>
where
    T: Read,
{
    buffer: Box<[u8; BUFFER_SIZE]>,
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
            buffer: box[0; BUFFER_SIZE],
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
        let length = self.reader.read(&mut *self.buffer)?;
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
                written = written + wrote;
                break;
            } else {
                // Write the remaining data, and refresh buffer
                let data = self.buf_read(remaining);
                let wrote = buf.write(data)?;
                written = written + wrote;

                if self.refill()? == 0 {
                    break;
                }
            }
        }

        Ok(written)
    }
}

/// Writer.
pub struct Writer<'w, T>
where
    T: Write,
{
    buffer: Box<[u8; BUFFER_SIZE]>,
    endianness: Endianness,
    length: usize,
    writer: &'w mut T,
}

impl<'w, T> Writer<'w, T>
where
    T: Write,
{
    pub fn from(writer: &'w mut T, endianness: &Endianness) -> Self {
        Self {
            buffer: box[0; BUFFER_SIZE],
            endianness: *endianness,
            length: 0,
            writer,
        }
    }

    fn buf_write(&mut self, bytes: &[u8]) -> usize {
        let slice = &mut self.buffer[self.length..self.length + bytes.len()];
        slice.copy_from_slice(bytes);
        self.length = self.length + bytes.len();
        bytes.len()
    }

    /// Flush buffer without flushing the underlying writer.
    fn buf_flush(&mut self) -> Result<()> {
        self.endianness.swap(&mut self.buffer[..self.length]);
        let data = &self.buffer[..self.length];
        self.writer.write(data)?;
        self.length = 0;
        Ok(())
    }

    fn remaining(&self) -> usize {
        self.buffer.len() - self.length
    }
}

impl<'w, T> Write for Writer<'w, T>
where
    T: Write,
{
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let mut idx = 0;
        let mut written = 0;

        loop {
            let remaining = self.remaining();
            if remaining >= (buf.len() - idx) {
                // If buffer has enough space, write all to it
                let wrote = self.buf_write(&buf[idx..]);
                written = written + wrote;
                break;
            } else {
                // let slice = &buf[idx..idx + remaining];
                let wrote = self.buf_write(&buf[idx..idx + remaining]);
                self.buf_flush()?;
                written = written + wrote;
                idx = idx + remaining;
            }
        }

        Ok(written)
    }

    fn flush(&mut self) -> Result<()> {
        self.buf_flush()?;
        self.writer.flush()
    }
}
