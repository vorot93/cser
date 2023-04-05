#[derive(Clone, Debug, PartialEq)]
pub struct Writer {
    bytes: Vec<u8>,
    bit_offset: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Reader<'a> {
    bytes: &'a [u8],
    byte_offset: usize,
    bit_offset: usize,
}

fn zero_top_byte_bits(v: usize, bits: usize) -> usize {
    let mask = 0xff_usize >> bits;
    v & mask
}

impl Writer {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            bit_offset: 0,
        }
    }

    #[must_use]
    fn byte_bits_free(&self) -> usize {
        8 - self.bit_offset
    }

    fn write_into_last_byte(&mut self, v: usize) {
        let i = self.bytes.len() - 1;
        self.bytes[i] |= (v << self.bit_offset) as u8;
    }

    pub fn write(&mut self, bits: usize, v: usize) {
        if self.bit_offset == 0 {
            self.bytes.push(0);
        }
        let free = self.byte_bits_free();
        if bits <= free {
            let to_write = bits;
            // appending v to the bit array
            self.write_into_last_byte(v);
            // increment offsets
            if to_write == free {
                self.bit_offset = 0;
            } else {
                self.bit_offset += to_write;
            }
        } else {
            let to_write = free;
            let clear = self.bit_offset; // 8 - free

            // zeroing top `clear` bits and appending result to the bit array
            self.write_into_last_byte(zero_top_byte_bits(v, clear));
            // increment offsets
            self.bit_offset = 0;
            self.write(bits - to_write, v >> to_write);
        }
    }

    pub fn view_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            byte_offset: 0,
            bit_offset: 0,
        }
    }

    pub fn byte_bits_free(&self) -> usize {
        8 - self.bit_offset
    }

    pub fn read(&mut self, bits: usize) -> usize {
        // perform all the checks in the same function to make CPU branch predictor work better
        if bits == 0 {
            return 0;
        }

        let mut v;

        let free = self.byte_bits_free();
        if bits <= free {
            let to_read = bits;
            let clear = 8 - (self.bit_offset + to_read);
            v = zero_top_byte_bits(usize::from(self.bytes[self.byte_offset]), clear)
                >> self.bit_offset;
            // increment offsets
            if to_read == free {
                self.bit_offset = 0;
                self.byte_offset += 1;
            } else {
                self.bit_offset += to_read;
            }
        } else {
            let to_read = free;
            v = usize::from(self.bytes[self.byte_offset]) >> self.bit_offset;
            // increment offsets
            self.bit_offset = 0;
            self.byte_offset += 1;
            // read rest
            let rest = self.read(bits - to_read);
            v |= rest << to_read;
        }
        v
    }

    pub fn view(&self, bits: usize) -> usize {
        self.clone().read(bits)
    }

    // Returns a number of non-consumed bytes
    pub fn non_read_bytes(&self) -> usize {
        self.bytes.len() - self.byte_offset
    }

    // Returns a number of non-consumed bits
    pub fn non_read_bits(&self) -> usize {
        self.non_read_bytes() * 8 - self.bit_offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{thread_rng, Rng};
    use std::fmt::Display;

    #[test]
    fn bit_array_empty() {
        test_bit_array(&[], "empty")
    }

    #[test]
    fn bit_array_b0() {
        test_bit_array(&[TestWord { bits: 1, v: 0b0 }], "b0")
    }

    #[test]
    fn bit_array_b1() {
        test_bit_array(&[TestWord { bits: 1, v: 0b1 }], "b1")
    }

    #[test]
    fn bit_array_b010101010() {
        test_bit_array(
            &[TestWord {
                bits: 9,
                v: 0b010101010,
            }],
            "b010101010",
        )
    }

    #[test]
    fn bit_array_b01010101010101010() {
        test_bit_array(
            &[TestWord {
                bits: 17,
                v: 0b01010101010101010,
            }],
            "b01010101010101010",
        )
    }

    #[test]
    fn bit_array_rand1() {
        for i in 0..50 {
            test_bit_array(&gen_test_words(24, 1), format!("1 bit, case#{i}"))
        }
    }

    #[test]
    fn bit_array_rand8() {
        for i in 0..50 {
            test_bit_array(&gen_test_words(100, 8), format!("8 bits, case#{i}"))
        }
    }

    #[test]
    fn bit_array_rand17() {
        for i in 0..50 {
            test_bit_array(&gen_test_words(50, 17), format!("17 bits, case#{i}"))
        }
    }

    fn gen_test_words(max_count: usize, max_bits: usize) -> Vec<TestWord> {
        (0..thread_rng().gen_range(0..max_count))
            .map(|_| {
                let bits = if max_bits == 1 {
                    1
                } else {
                    1 + thread_rng().gen_range(0..(max_bits - 1))
                };
                let v = thread_rng().gen_range(0..(1 << bits));
                TestWord { bits, v }
            })
            .collect()
    }

    fn bytes_to_fit(bits: usize) -> usize {
        if bits % 8 == 0 {
            bits / 8
        } else {
            bits / 8 + 1
        }
    }

    #[derive(Clone, Copy)]
    struct TestWord {
        bits: usize,
        v: usize,
    }

    fn test_bit_array(words: &[TestWord], name: impl Display) {
        let mut writer = Writer::new(Vec::with_capacity(100));

        let mut total_bits_written = 0;
        for w in words {
            writer.write(w.bits, w.v);
            total_bits_written += w.bits
        }
        assert_eq!(
            bytes_to_fit(total_bits_written),
            writer.view_bytes().len(),
            "{name}"
        );

        let mut reader = Reader::new(writer.view_bytes());
        let mut total_bits_read = 0;
        for w in words {
            assert_eq!(
                bytes_to_fit(total_bits_written) * 8 - total_bits_read,
                reader.non_read_bits(),
                "{name}"
            );
            assert_eq!(
                bytes_to_fit(reader.non_read_bits()),
                reader.non_read_bytes(),
                "{name}"
            );

            let v = reader.read(w.bits);
            assert_eq!(w.v, v, "{name}");
            total_bits_read += w.bits;

            assert_eq!(
                bytes_to_fit(total_bits_written) * 8 - total_bits_read,
                reader.non_read_bits(),
                "{name}"
            );
            assert_eq!(
                bytes_to_fit(reader.non_read_bits()),
                reader.non_read_bytes(),
                "{name}"
            );
        }

        // read the tail
        let zero = reader.read(reader.non_read_bits());
        assert_eq!(0, zero, "{name}");
        assert_eq!(0, reader.non_read_bits(), "{name}");
        assert_eq!(0, reader.non_read_bytes(), "{name}");
    }
}
