#![cfg_attr(not(feature = "unsafe"), forbid(unsafe_code))]

use bytes::{Bytes, BytesMut, BufMut};
use std::io;

pub mod huffman;
pub mod lz77;

// TODO: Rename this to DirectBitBuf? I should have another
// implementation that outputs to an infallible sink so that
// errors wont have to be accounted for.
pub struct BitBuf<W> {
    cur: u8,
    out: W,
}

impl<W: io::Write> BitBuf<W> {
    pub fn new(out: W) -> Self {
	Self { cur: 0x1, out }
    }

    #[inline]
    pub fn put(&mut self, bit: bool) -> io::Result<()> {
	let bit = bit as u8;
	let flush = (self.cur & 0x80) != 0;

	self.cur = (self.cur << 1) | bit;

	// TODO: use (flush as u8) as a length for a slice to flush?
        // let v = self.cur & (flush as u8 * u8::MAX);

	if flush {
	    self.out.write(&[self.cur])?;
	    // self.out.put_u8(self.cur);
	    self.cur = 1;
	}
	Ok(())

	//let mask = flush as u8 * 255;
	//self.cur = self.cur ^ ((self.cur ^ 0x1) & mask);
    }

    pub fn putnc<const N: usize>(&mut self, b: u8) -> io::Result<()> {
	// TODO: Find some faster way?
	for n in (0..N).rev() {
	    let bit = b & (1 << n);
	    self.put(bit != 0)?;
	}
	Ok(())
    }

    /// Use with caution. Forcefully flushing is expensive and will pad with zeros.
    pub fn write_flush(&mut self) -> io::Result<()> {
	while self.cur & 0x80 == 0 {
	    self.cur <<= 1;
	}
	// Remove leftmost set bit (sentinel value)
	self.cur <<= 1;

	self.out.write(&[self.cur])?;
	self.cur = 1;
	Ok(())
    }

    pub fn reset(&mut self) {
	self.cur = 1;
    }
}

pub trait Driver {
    fn feed(bytes: Bytes, out: BytesMut);
}

pub trait WriteHeader {
    fn hwrite<W: io::Write>(&self, out: &mut BitBuf<W>) -> io::Result<()>;
}

pub struct DeflateHeader {
    last: bool,
    kind: DeflateHeaderKind
}

impl WriteHeader for DeflateHeader {
    fn hwrite<W: io::Write>(&self, out: &mut BitBuf<W>) -> io::Result<()> {
	out.put(self.last)?;
	self.kind.hwrite(out)
    }
}

pub enum DeflateHeaderKind {
    Stored,
    StaticHf,
    DynamicHf,
}

impl WriteHeader for DeflateHeaderKind {
    fn hwrite<W: io::Write>(&self, out: &mut BitBuf<W>) -> io::Result<()> {
	match self {
	    DeflateHeaderKind::Stored => out.putnc::<2>(0b00),
	    DeflateHeaderKind::StaticHf => out.putnc::<2>(0b01),
	    DeflateHeaderKind::DynamicHf => out.putnc::<2>(0b10),
	}
    }
}

#[cfg(test)]
mod tests {
    use crate::{WriteHeader, DeflateHeader, DeflateHeaderKind, BitBuf};

    #[test]
    fn bitbuf_putnc() {
	let mut out = Vec::new();
	let mut bitbuf = BitBuf::new(&mut out);

	bitbuf.putnc::<3>(0b110).unwrap();
	bitbuf.putnc::<2>(0b11).unwrap();
	bitbuf.write_flush().unwrap();

	assert_eq!(out, vec![0b11011000]);

	out.clear();
	let mut bitbuf = BitBuf::new(&mut out);
	bitbuf.write_flush().unwrap();
	assert!(out.is_empty());
    }

    #[test]
    fn deflate_header() {
	let mut out = Vec::new();
	let mut bitbuf = BitBuf::new(&mut out);

	let header = DeflateHeader {
	    last: true,
	    kind: DeflateHeaderKind::Stored,
	};

	header.hwrite(&mut bitbuf).unwrap();
	bitbuf.write_flush().unwrap();

	assert_eq!(out, vec![0b10000000]);
    }
}
