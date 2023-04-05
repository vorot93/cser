use super::error::Error;

#[derive(Clone, Debug, PartialEq)]
pub struct Reader<'a> {
    pub buf: &'a [u8],
    offset: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Writer {
    pub buf: Vec<u8>,
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, offset: 0 }
    }

    /// Read n bytes.
    pub fn read(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let res = &self
            .buf
            .get(self.offset..self.offset + n)
            .ok_or(Error::MalformedEncoding)?;
        self.offset += n;

        Ok(res)
    }

    /// Reads 1 byte.
    pub fn read_byte(&mut self) -> Result<u8, Error> {
        let res = self
            .buf
            .get(self.offset)
            .copied()
            .ok_or(Error::MalformedEncoding)?;
        self.offset += 1;

        Ok(res)
    }

    // Position of internal cursor.
    pub fn position(&self) -> usize {
        self.offset
    }

    // Empty returns true if the whole buffer is consumed
    pub fn empty(&self) -> bool {
        self.buf.len() == self.offset
    }
}

impl Writer {
    pub fn new(buf: Vec<u8>) -> Self {
        Self { buf }
    }

    // Write the byte to the buffer.
    pub fn write(&mut self, v: &[u8]) {
        self.buf.extend_from_slice(v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    const N: usize = 100;
    const BB: &[u8] = &hex!("0000FF0900");

    #[test]
    fn writer_reader() {
        let mut w = Writer::new(Vec::with_capacity(N / 2));
        for i in 0..N {
            w.write(&[i as u8]);
        }
        assert_eq!(N, w.buf.len());
        w.write(BB.as_ref());
        assert_eq!(N + BB.len(), w.buf.len());

        let mut r = Reader::new(&w.buf);
        assert_eq!(N + BB.len(), r.buf.len());
        assert!(!r.empty());
        for exp in 0..N {
            let got = r.read_byte().unwrap();
            assert_eq!(exp as u8, got);
        }
        assert_eq!(N, r.position());
        let got = r.read(BB.len()).unwrap();
        assert_eq!(BB, got);
        assert!(r.empty());
    }
}
