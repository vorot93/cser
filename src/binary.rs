use super::{
    bits,
    error::Error,
    fast,
    read_writer::{read_uint64_compact, write_uint64_compact, Reader},
    Decodable,
};

/// Packs body bytes and bits into raw
pub(crate) fn binary_from_cser(bbits: &[u8], bbytes: Vec<u8>) -> Vec<u8> {
    let mut body_bytes = fast::Writer::new(bbytes);
    body_bytes.write(bbits);
    // write bits size
    let mut size_writer = fast::Writer::new(Vec::with_capacity(4));
    write_uint64_compact(&mut size_writer, bbits.len().try_into().unwrap());

    let mut size_buf = size_writer.buf;
    size_buf.reverse();
    body_bytes.write(&size_buf);
    body_bytes.buf
}

/// Unpacks raw on body bytes and bits
pub(crate) fn binary_to_cser(mut raw: &[u8]) -> Result<(&[u8], &[u8]), Error> {
    // read bitsArray size
    let mut bits_size_buf = tail(raw, 9).to_vec();
    bits_size_buf.reverse();
    let mut bits_size_reader = fast::Reader::new(&bits_size_buf);
    let bits_size = usize::try_from(read_uint64_compact(&mut bits_size_reader)?).unwrap();
    raw = &raw[..raw.len() - bits_size_reader.position()];

    if raw.len() < bits_size {
        return Err(Error::MalformedEncoding);
    }

    let (bbytes, bbits) = raw.split_at(raw.len() - bits_size);

    Ok((bbits, bbytes))
}

pub fn deserialize<T>(input: &[u8]) -> Result<T, T::Error>
where
    T: Decodable,
    T::Error: From<Error>,
{
    deserialize_cb::<T, T::Error>(input, |handler| T::decode(handler))
}

fn deserialize_cb<T, E>(
    input: &[u8],
    handler: impl FnOnce(&mut Reader) -> Result<T, E>,
) -> Result<T, E>
where
    E: From<Error>,
{
    let (bbits, bbytes) = binary_to_cser(input)?;

    let mut body_reader = Reader {
        bits_r: bits::Reader::new(bbits),
        bytes_r: fast::Reader::new(bbytes),
    };
    let out = (handler)(&mut body_reader)?;

    // check that everything is read
    if body_reader.bits_r.non_read_bytes() > 1 {
        return Err(Error::NonCanonicalEncoding.into());
    }
    let tail = body_reader.bits_r.read(body_reader.bits_r.non_read_bits());
    if tail != 0 {
        return Err(Error::NonCanonicalEncoding.into());
    }
    if !body_reader.bytes_r.empty() {
        return Err(Error::NonCanonicalEncoding.into());
    }

    Ok(out)
}

fn tail(b: &[u8], cap: usize) -> &[u8] {
    if b.len() > cap {
        &b[b.len() - cap..]
    } else {
        b
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Decodable, Encodable, Writer, U56};
    use ethnum::{AsU256, U256};

    #[cfg(test)]
    fn marshal_binary_adapter(
        marshal_cser: impl FnOnce(&mut Writer) -> anyhow::Result<()>,
    ) -> anyhow::Result<Vec<u8>> {
        let mut w = Writer::new();
        (marshal_cser)(&mut w)?;

        Ok(w.output())
    }

    #[test]
    fn empty() {
        let buf = marshal_binary_adapter(|_| Ok(())).unwrap();

        deserialize_cb(&buf, |_| Ok::<_, Error>(())).unwrap();
    }

    #[test]
    fn err() {
        let mut buf = vec![];

        // Write
        buf.append(
            &mut marshal_binary_adapter(|w| {
                u64::MAX.encode(w);
                Ok(())
            })
            .unwrap(),
        );

        // Read None
        // nothing unmarshal
        assert_eq!(
            deserialize_cb(&[], |_| Ok(())),
            Err(Error::MalformedEncoding)
        );

        // Read Err
        let e = Error::Custom("custom");
        // unmarshal
        assert_eq!(
            deserialize_cb(&buf, |r| {
                assert_eq!(u64::decode(r).unwrap(), u64::MAX);
                Err::<(), _>(e)
            }),
            Err(e)
        );

        // "Read 0"

        // unpack
        let (_, bbytes) = binary_to_cser(&buf).unwrap();
        let l = bbytes.len();
        // pack with wrong bits size
        let mut corrupted = fast::Writer::new(bbytes.to_vec());
        let mut size_writer = fast::Writer::new(Vec::with_capacity(4));
        write_uint64_compact(&mut size_writer, u64::try_from(l).unwrap() + 1);
        let mut size_buf = size_writer.buf;
        size_buf.reverse();
        corrupted.write(&size_buf);
        // corrupted unpack
        assert_eq!(
            binary_to_cser(&corrupted.buf),
            Err(Error::MalformedEncoding)
        );
        // corrupted unmarshal
        assert_eq!(
            deserialize_cb(&corrupted.buf, |r| {
                assert_eq!(u64::decode(r).unwrap(), u64::MAX);
                Ok(())
            }),
            Err(Error::MalformedEncoding)
        );

        #[allow(clippy::type_complexity)]
        let repack_with_defect =
            |defect: Box<dyn FnOnce(&mut Vec<u8>, &mut Vec<u8>) -> Result<(), Error>>| {
                // unpack
                let (bbits, bbytes) = binary_to_cser(&buf).unwrap();
                let mut bbits = bbits.to_vec();
                let mut bbytes = bbytes.to_vec();
                // pack with defect
                let err_exp = (defect)(&mut bbits, &mut bbytes);
                let corrupted = binary_from_cser(&bbits, bbytes);
                // corrupted unmarshal
                assert_eq!(
                    deserialize_cb(&corrupted, |r| {
                        let _ = u64::decode(r);
                        Ok(())
                    }),
                    err_exp
                );
            };

        (repack_with_defect)(Box::new(|_, _| {
            // no defect
            Ok(())
        }));

        (repack_with_defect)(Box::new(|_, bbytes| {
            bbytes.push(0xFF);
            Err(Error::NonCanonicalEncoding)
        }));

        (repack_with_defect)(Box::new(|bbits, _| {
            bbits.push(0x0F);
            Err(Error::NonCanonicalEncoding)
        }));

        (repack_with_defect)(Box::new(|_, bbytes| {
            bbytes.truncate(bbytes.len() - 1);
            Err(Error::NonCanonicalEncoding)
        }));
    }

    #[test]
    fn vals() {
        let exp_u256 = [0.as_u256(), 1.as_u256(), 0xF_FF_FF.as_u256(), U256::MAX];
        let exp_bool = vec![true, false];
        let exp_fixed_bytes_empty = vec![[]];
        let exp_fixed_bytes = vec![[rand::random::<u8>(); 0xFF]];
        let exp_slice_bytes = vec![vec![rand::random::<u8>(); 0xFF]];
        let exp_u8 = vec![0, 1, u8::MAX];
        let exp_u16 = vec![0, 1, u16::MAX];
        let exp_u32 = vec![0, 1, u32::MAX];
        let exp_u64 = vec![0, 1, u64::MAX];
        let exp_i64 = vec![0, 1, i64::MIN, i64::MAX];
        let exp_u56 = [0_u64, 1, 1 << ((8 * 7) - 1)]
            .into_iter()
            .map(|v| U56::try_from(v).unwrap())
            .collect::<Vec<_>>();

        let buf = marshal_binary_adapter(|w| {
            for v in &exp_u256 {
                v.encode(w);
            }
            for v in &exp_bool {
                v.encode(w);
            }
            for v in &exp_fixed_bytes_empty {
                v.encode(w);
            }
            for v in &exp_fixed_bytes {
                v.encode(w);
            }
            for v in &exp_slice_bytes {
                v.as_slice().encode(w);
            }
            for v in &exp_u8 {
                v.encode(w);
            }
            for v in &exp_u16 {
                v.encode(w);
            }
            for v in &exp_u32 {
                v.encode(w);
            }
            for v in &exp_u64 {
                v.encode(w);
            }
            for v in &exp_i64 {
                v.encode(w);
            }
            for v in &exp_u56 {
                v.encode(w);
            }
            Ok(())
        })
        .unwrap();

        deserialize_cb(&buf, |r| {
            for v in &exp_u256 {
                assert_eq!(U256::decode(r).unwrap(), *v);
            }
            for v in &exp_bool {
                assert_eq!(bool::decode(r).unwrap(), *v);
            }
            for &v in &exp_fixed_bytes_empty {
                let got = <[u8; 0] as Decodable>::decode(r).unwrap();
                assert_eq!(got, v);
            }
            for &v in &exp_fixed_bytes {
                let got = <[u8; 255] as Decodable>::decode(r).unwrap();
                assert_eq!(got, v);
            }
            for v in &exp_slice_bytes {
                assert_eq!(r.slice_bytes(v.len()).unwrap(), *v);
            }
            for v in &exp_u8 {
                assert_eq!(u8::decode(r).unwrap(), *v);
            }
            for v in &exp_u16 {
                assert_eq!(u16::decode(r).unwrap(), *v);
            }
            for v in &exp_u32 {
                assert_eq!(u32::decode(r).unwrap(), *v);
            }
            for v in &exp_u64 {
                assert_eq!(u64::decode(r).unwrap(), *v);
            }
            for v in &exp_i64 {
                assert_eq!(i64::decode(r).unwrap(), *v);
            }
            for v in &exp_u56 {
                assert_eq!(U56::decode(r).unwrap(), *v);
            }
            Ok::<_, Error>(())
        })
        .unwrap();
    }
}
