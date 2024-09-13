use super::{bits, error::*, fast, Decodable, U56};

#[derive(Clone, Debug, PartialEq)]
pub struct Writer {
    pub bits_w: bits::Writer,
    pub bytes_w: fast::Writer,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Reader<'a> {
    pub bits_r: bits::Reader<'a>,
    pub bytes_r: fast::Reader<'a>,
}

impl Writer {
    pub fn new() -> Self {
        let bbits = Vec::with_capacity(32);
        let bbytes = Vec::with_capacity(200);
        Self {
            bits_w: bits::Writer::new(bbits),
            bytes_w: fast::Writer::new(bbytes),
        }
    }

    pub fn output(self) -> Vec<u8> {
        crate::binary::binary_from_cser(self.bits_w.view_bytes(), self.bytes_w.buf)
    }
}

impl Default for Writer {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn write_uint64_compact(bytes_w: &mut fast::Writer, mut v: u64) {
    loop {
        let mut chunk = v & 0b01111111;
        v >>= 7;
        if v == 0 {
            // stop flag
            chunk |= 0b10000000;
        }
        bytes_w.write(&[u8::try_from(chunk).unwrap()]);
        if v == 0 {
            break;
        }
    }
}

pub(crate) fn read_uint64_compact(bytes_r: &mut fast::Reader) -> Result<u64, Error> {
    let mut v = 0_u64;
    let mut stop = false;
    let mut i = 0;
    while !stop {
        let chunk = u64::from(bytes_r.read_byte()?);
        stop = (chunk & 0b10000000) != 0;
        let word = chunk & 0b01111111;
        v |= word << (i * 7);
        // last byte cannot be zero
        if i > 0 && stop && word == 0 {
            return Err(Error::NonCanonicalEncoding);
        }

        i += 1;
    }

    Ok(v)
}

fn write_uint64_bit_compact(bytes_w: &mut fast::Writer, mut v: u64, min_size: usize) -> usize {
    let mut size = 0;
    while size < min_size || v != 0 {
        bytes_w.write(&[v as u8]);
        size += 1;
        v >>= 8;
    }

    size
}

fn read_uint64_bit_compact(bytes_r: &mut fast::Reader, size: usize) -> Result<u64, Error> {
    let mut v = 0_u64;
    let mut last = 0_u8;

    let buf = bytes_r.read(size)?;
    for (i, &b) in buf.iter().enumerate() {
        v |= u64::from(b) << u64::try_from(8 * i).map_err(|_| Error::OverFlowError)?;
        last = b;
    }

    if size > 1 && last == 0 {
        return Err(Error::NonCanonicalEncoding);
    }

    Ok(v)
}

impl<'a> Reader<'a> {
    pub fn read_u64_bits(&mut self, min_size: usize, bits_for_size: usize) -> Result<u64, Error> {
        let size = self.bits_r.read(bits_for_size) + min_size;
        read_uint64_bit_compact(&mut self.bytes_r, size)
    }

    pub fn slice_bytes(&mut self, max_len: usize) -> Result<&[u8], Error> {
        // read slice size
        let size = usize::try_from(*U56::decode(self)?).map_err(|_| Error::OverFlowError)?;
        if size > max_len {
            return Err(Error::TooLargeAlloc);
        }
        self.bytes_r.read(size)
    }
}

impl Writer {
    pub(crate) fn write_u64_bits(&mut self, min_size: usize, bits_for_size: usize, v: u64) {
        let size = write_uint64_bit_compact(&mut self.bytes_w, v, min_size);
        self.bits_w.write(bits_for_size, size - min_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Encodable;

    #[test]
    fn test_uint64_compact() {
        for (fixture, expected) in [
            (&[0b01111111_u8, 0b11111111_u8] as &[u8], Ok(0x3fff_u64)),
            (
                &[0b01111111_u8, 0b01111111_u8, 0b10000000_u8] as &[u8],
                Err(Error::NonCanonicalEncoding),
            ),
        ] {
            let mut r = fast::Reader::new(fixture);
            assert_eq!(expected, read_uint64_compact(&mut r));
        }
    }

    #[test]
    fn test_uint64_bit_compact() {
        for (fixture, expected) in [
            (&[0b11111111_u8, 0b00111111_u8] as &[u8], Ok(0x3fff_u64)),
            (
                &[0b01111111_u8, 0b01111111_u8, 0b00000000_u8] as &[u8],
                Err(Error::NonCanonicalEncoding),
            ),
        ] {
            let mut r = fast::Reader::new(fixture);
            assert_eq!(expected, read_uint64_bit_compact(&mut r, fixture.len()));
        }
    }

    #[test]
    fn i64() {
        let mut w = Writer::new();

        // canonical
        0_i64.encode(&mut w);

        // non-canonical
        true.encode(&mut w);
        0_u64.encode(&mut w);

        let mut r = Reader {
            bits_r: bits::Reader::new(w.bits_w.view_bytes()),
            bytes_r: fast::Reader::new(&w.bytes_w.buf),
        };

        assert_eq!(i64::decode(&mut r), Ok(0));
        assert_eq!(i64::decode(&mut r), Err(Error::NonCanonicalEncoding));
    }
}
