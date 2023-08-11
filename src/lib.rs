#![cfg_attr(not(feature = "unsafe"), forbid(unsafe_code))]

use bytes::{Bytes, BytesMut};

pub mod huffman;
pub mod lz77;

pub trait Driver {
    fn feed(bytes: Bytes, out: BytesMut);
}

pub struct DeflateHeader {
    last: bool,
    kind: DeflateHeaderKind
}

pub enum DeflateHeaderKind {
    Stored,
    StaticHf,
    DynamicHf,
}
