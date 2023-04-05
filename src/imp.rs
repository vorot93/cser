use super::{
    error::Error,
    read_writer::{Reader, Writer},
    Decodable, Encodable, U56,
};
use bytes::Bytes;
use std::any::Any;

impl Encodable for u8 {
    fn encode(&self, out: &mut Writer) {
        out.bytes_w.write(&[*self])
    }
}

impl Decodable for u8 {
    type Error = Error;

    fn decode(buf: &mut Reader<'_>) -> anyhow::Result<Self, Self::Error> {
        buf.bytes_r.read_byte()
    }
}

impl Encodable for bool {
    fn encode(&self, out: &mut Writer) {
        out.bits_w.write(1, usize::from(*self))
    }
}

impl Decodable for bool {
    type Error = Error;

    fn decode(buf: &mut Reader<'_>) -> Result<Self, Self::Error> {
        Ok(u8::try_from(buf.bits_r.read(1)).map_err(|_| Error::OverFlowError)? != 0)
    }
}

impl Encodable for u16 {
    fn encode(&self, out: &mut Writer) {
        out.write_u64_bits(1, 1, (*self).into())
    }
}

impl Decodable for u16 {
    type Error = Error;

    fn decode(buf: &mut Reader<'_>) -> Result<Self, Self::Error> {
        buf.read_u64_bits(1, 1)?
            .try_into()
            .map_err(|_| Error::OverFlowError)
    }
}

impl Encodable for u32 {
    fn encode(&self, out: &mut Writer) {
        out.write_u64_bits(1, 2, (*self).into())
    }
}

impl Decodable for u32 {
    type Error = Error;

    fn decode(buf: &mut Reader<'_>) -> Result<Self, Self::Error> {
        buf.read_u64_bits(1, 2)?
            .try_into()
            .map_err(|_| Error::OverFlowError)
    }
}

impl Encodable for u64 {
    fn encode(&self, out: &mut Writer) {
        out.write_u64_bits(1, 3, *self)
    }
}

impl Decodable for u64 {
    type Error = Error;

    fn decode(buf: &mut Reader<'_>) -> Result<Self, Self::Error> {
        buf.read_u64_bits(1, 3)
    }
}

impl Encodable for i64 {
    fn encode(&self, out: &mut Writer) {
        (*self < 0).encode(out);
        self.unsigned_abs().encode(out);
    }
}

impl Decodable for i64 {
    type Error = Error;

    fn decode(buf: &mut Reader<'_>) -> Result<Self, Self::Error> {
        let neg = bool::decode(buf)?;
        let abs = u64::decode(buf)?;
        if neg && abs == 0 {
            return Err(Error::NonCanonicalEncoding);
        }
        let mut v = 0_i64.overflowing_add_unsigned(abs).0;
        if neg {
            v = v.overflowing_neg().0;
        }
        Ok(v)
    }
}

impl Encodable for U56 {
    fn encode(&self, out: &mut Writer) {
        out.write_u64_bits(0, 3, **self)
    }
}

impl Decodable for U56 {
    type Error = Error;

    fn decode(buf: &mut Reader<'_>) -> Result<Self, Self::Error> {
        U56::try_from(buf.read_u64_bits(0, 3)?).map_err(|_| Error::OverFlowError)
    }
}

impl<T> Encodable for Option<T>
where
    T: Encodable,
{
    fn encode(&self, out: &mut Writer) {
        if let Some(v) = self {
            Encodable::encode(&true, out);
            Encodable::encode(&v, out);
        } else {
            Encodable::encode(&false, out);
        }
    }
}

impl<T> Decodable for Option<T>
where
    T: Decodable,
    T::Error: From<Error>,
{
    type Error = T::Error;

    fn decode(buf: &mut Reader<'_>) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        let is_some = Decodable::decode(buf)?;

        Ok(if is_some {
            Some(<T as Decodable>::decode(buf)?)
        } else {
            None
        })
    }
}

impl Encodable for &[u8] {
    fn encode(&self, out: &mut Writer) {
        U56::try_from(u64::try_from(self.len()).unwrap())
            .unwrap()
            .encode(out);
        out.bytes_w.write(self)
    }
}

impl Encodable for Bytes {
    fn encode(&self, out: &mut Writer) {
        (&**self).encode(out)
    }
}

impl Decodable for Bytes {
    type Error = Error;

    fn decode(buf: &mut Reader<'_>) -> Result<Self, Self::Error> {
        let len = U56::decode(buf)?
            .0
            .try_into()
            .map_err(|_| Error::OverFlowError)?;

        Ok(Bytes::copy_from_slice(buf.bytes_r.read(len)?))
    }
}

impl<T> Encodable for Vec<T>
where
    T: Encodable + 'static,
{
    fn encode(&self, out: &mut Writer) {
        if let Some(s) = <dyn Any>::downcast_ref::<Vec<u8>>(self) {
            s.as_slice().encode(out)
        } else {
            u32::try_from(self.len()).unwrap().encode(out);
            for item in self {
                item.encode(out)
            }
        }
    }
}

impl<T> Decodable for Vec<T>
where
    T: Decodable + 'static,
    T::Error: From<Error>,
{
    type Error = T::Error;

    fn decode(buf: &mut Reader<'_>) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        let mut v = Vec::<T>::new();
        if let Some(out) = <dyn Any>::downcast_mut::<Vec<u8>>(&mut v) {
            let len = U56::decode(buf)?.0.try_into().unwrap();

            out.extend_from_slice(buf.bytes_r.read(len)?);
        } else {
            let len = usize::try_from(u32::decode(buf)?).unwrap();

            v.reserve_exact(len);
            for _ in 0..len {
                v.push(T::decode(buf)?);
            }
        }

        Ok(v)
    }
}

impl<const LEN: usize> Encodable for [u8; LEN] {
    fn encode(&self, out: &mut Writer) {
        out.bytes_w.write(self)
    }
}

impl<const LEN: usize> Decodable for [u8; LEN] {
    type Error = Error;

    fn decode(buf: &mut Reader<'_>) -> Result<Self, Self::Error> {
        let mut v = [0; LEN];
        v.copy_from_slice(buf.bytes_r.read(LEN).map_err(|_| Error::OverFlowError)?);
        Ok(v)
    }
}

impl<T, const LEN: usize> Encodable for arrayvec::ArrayVec<T, LEN>
where
    T: Encodable + 'static,
{
    fn encode(&self, out: &mut Writer) {
        if let Some(s) = <dyn Any>::downcast_ref::<arrayvec::ArrayVec<u8, LEN>>(self) {
            s.as_slice().encode(out)
        } else {
            u32::try_from(self.len()).unwrap().encode(out);
            for item in self {
                item.encode(out)
            }
        }
    }
}

impl<T, const LEN: usize> Decodable for arrayvec::ArrayVec<T, LEN>
where
    T: Decodable + 'static,
    T::Error: From<Error>,
{
    type Error = T::Error;

    fn decode(buf: &mut Reader<'_>) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        let mut v = Self::new();
        if let Some(out) = <dyn Any>::downcast_mut::<arrayvec::ArrayVec<u8, LEN>>(&mut v) {
            let len = U56::decode(buf)?.0.try_into().unwrap();

            out.try_extend_from_slice(buf.bytes_r.read(len)?)
                .map_err(|_| Error::OverFlowError)?;
        } else {
            let len = usize::try_from(u32::decode(buf)?).unwrap();

            for _ in 0..len {
                v.try_push(T::decode(buf)?)
                    .map_err(|_| Error::OverFlowError)?;
            }
        }

        Ok(v)
    }
}

impl Encodable for String {
    fn encode(&self, out: &mut Writer) {
        self.as_bytes().encode(out)
    }
}

impl Decodable for String {
    type Error = anyhow::Error;

    fn decode(buf: &mut Reader<'_>) -> Result<Self, Self::Error> {
        Ok(String::from_utf8(Decodable::decode(buf)?)?)
    }
}

#[macro_export]
macro_rules! impl_encodable_for_wrapper {
    ($wrapper:ty, $base:ty) => {
        impl $crate::Encodable for $wrapper {
            fn encode(&self, out: &mut $crate::Writer) {
                self.0.encode(out)
            }
        }

        impl $crate::Decodable for $wrapper {
            type Error = <$base as $crate::Decodable>::Error;

            fn decode(buf: &mut $crate::Reader<'_>) -> Result<Self, Self::Error> {
                <$base>::decode(buf).map(<$wrapper>::from)
            }
        }
    };
}

#[cfg(feature = "ethereum-types")]
mod ethereum_types_impl {
    impl_encodable_for_wrapper!(ethereum_types::Address, [u8; 20]);
    impl_encodable_for_wrapper!(ethereum_types::H256, [u8; 32]);
    impl_encodable_for_wrapper!(ethereum_types::H512, [u8; 64]);
}

#[cfg(feature = "ethnum")]
mod ethnum_impl {
    use super::*;

    impl Encodable for ethnum::U256 {
        fn encode(&self, out: &mut Writer) {
            let v = self.to_be_bytes();
            let v = v.as_ref();
            let v = &v[v.iter().take_while(|&&b| b == 0).count()..];

            Encodable::encode(&v, out)
        }
    }

    impl Decodable for ethnum::U256 {
        type Error = Error;

        fn decode(buf: &mut Reader<'_>) -> Result<Self, Self::Error>
        where
            Self: Sized,
        {
            let data = arrayvec::ArrayVec::<u8, { (Self::BITS / 8) as usize }>::decode(buf)?;

            let mut v = [0; (Self::BITS / 8) as usize];

            v[(Self::BITS / 8) as usize - data.len()..].copy_from_slice(&data);
            Ok(Self::from_be_bytes(v))
        }
    }
}
