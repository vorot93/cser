use cser::{deserialize, *};
use hex_literal::hex;

#[derive(Clone, Debug, PartialEq, Encodable, Decodable)]
struct DeriveTest {
    field_a: u32,
    field_b: u64,
}

#[derive(Clone, Debug, PartialEq, EncodableWrapper, DecodableWrapper)]
struct DeriveWrapperTest(DeriveTest);

#[test]
fn encodable_derive_equivalence() {
    let field_a = 0x1234_5678;
    let field_b = 0x1234_5678_9abc_def0;

    let mut writer_derive = Writer::new();
    let value = DeriveTest { field_a, field_b };
    value.encode(&mut writer_derive);

    let mut writer_manual = Writer::new();
    field_a.encode(&mut writer_manual);
    field_b.encode(&mut writer_manual);

    assert_eq!(writer_derive, writer_manual);
}

#[test]
fn vec_specialization() {
    {
        let bytestring = vec![0x42_u8, 0x43_u8];
        let mut writer1 = Writer::new();
        bytestring.as_slice().encode(&mut writer1);

        let mut writer2 = Writer::new();
        bytestring.encode(&mut writer2);
        assert_eq!(writer1, writer2);
        let out = writer1.output();
        assert_eq!(out, hex!("0242430181"));
        assert_eq!(deserialize::<Vec<u8>>(&out).unwrap(), bytestring);
    }

    {
        let numbers = vec![0xAABB_u64, 0xCCDD_u64];
        let mut writer3 = Writer::new();
        numbers.encode(&mut writer3);
        let out = writer3.output();
        assert_eq!(out, hex!("02BBAADDCC2481"));
        assert_eq!(deserialize::<Vec<u64>>(&out).unwrap(), numbers);
    }
}
