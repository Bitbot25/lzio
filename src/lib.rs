#![cfg_attr(feature = "unstable", feature(allocator_api))]
#![cfg_attr(not(feature = "unsafe"), forbid(unsafe_code))]

use bytes::{Bytes, BytesMut};

pub mod huffman;

pub trait Driver {
    fn feed(bytes: Bytes, out: BytesMut);
}
