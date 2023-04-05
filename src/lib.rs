mod binary;
mod bits;
mod error;
mod fast;
mod imp;
mod read_writer;

pub use self::{
    binary::deserialize,
    bits::{Reader as BitsReader, Writer as BitsWriter},
    error::Error,
    read_writer::{Reader, Writer},
};
use auto_impl::auto_impl;
#[cfg(feature = "derive")]
pub use cser_derive::*;
use derive_more::Deref;

#[auto_impl(&, Box, Arc)]
pub trait Encodable {
    fn encode(&self, out: &mut Writer);
}

pub trait Decodable: Sized {
    type Error;

    fn decode(buf: &mut Reader<'_>) -> Result<Self, Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deref)]
pub struct U56(u64);

impl TryFrom<u64> for U56 {
    type Error = ();

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        if value > 0x00ff_ffff_ffff_ffff {
            Err(())
        } else {
            Ok(Self(value))
        }
    }
}
