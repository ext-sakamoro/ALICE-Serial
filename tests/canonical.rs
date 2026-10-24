//! Oracle: one byte string per value
//!
//! - the header (magic, version) and trailing bytes are checked by `decode`
//! - integers in a longer form than they need, NaNs other than the canonical
//!   one, map keys out of order or repeated, and varints with extra bytes are
//!   refused with the error that names them
//! - nesting deeper than `MAX_DEPTH` is refused by both sides, and a deeply
//!   nested forged input is refused without exhausting the stack
//! - every input of up to 2 bytes, and a fixed sample of longer ones, either
//!   fails or re-encodes to exactly itself
//! - `SERIAL_SEMANTICS_ID` is the SHA-256 of `FORMAT_SPEC`

use alice_serial::{
    decode, decode_value, decode_varint, encode, encode_value, encode_varint, SerialError, Value,
    FORMAT_SPEC, FORMAT_VERSION, MAGIC, MAX_DEPTH, SERIAL_SEMANTICS_ID,
};
use sha2::{Digest, Sha256};

fn s(x: &str) -> Value {
    Value::Str(x.into())
}

fn framed(body: &[u8]) -> Vec<u8> {
    let mut out = MAGIC.to_vec();
    out.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    out.extend_from_slice(body);
    out
}

#[test]
fn the_header_is_magic_then_version() {
    let bytes = encode(&Value::Null).unwrap();
    assert_eq!(bytes, [b'A', b'L', b'S', b'R', 1, 0, 0xC0]);
    assert_eq!(decode(&bytes), Ok(Value::Null));

    let mut magic = bytes.clone();
    magic[0] = b'B';
    assert_eq!(decode(&magic), Err(SerialError::BadMagic));
    // a header with no value after it still has its magic checked first
    assert_eq!(decode(b"BLSR\x01\x00"), Err(SerialError::BadMagic));
    assert_eq!(decode(b"ALSR\x01\x00"), Err(SerialError::BufferTooShort));

    let mut version = bytes.clone();
    version[4] = 2;
    assert_eq!(decode(&version), Err(SerialError::UnsupportedVersion(2)));

    for cut in 0..bytes.len() {
        assert_eq!(
            decode(&bytes[..cut]),
            Err(SerialError::BufferTooShort),
            "prefix of {cut} bytes"
        );
    }

    let mut extra = bytes;
    extra.push(0xC0);
    assert_eq!(decode(&extra), Err(SerialError::TrailingBytes));
}

#[test]
fn integers_have_one_form() {
    // 0..=127 is the one-byte fixint; the 9-byte form of the same value is refused
    for n in [0u8, 5, 127] {
        let mut long = vec![0xCF];
        long.extend_from_slice(&u64::from(n).to_be_bytes());
        assert_eq!(decode_value(&long), Err(SerialError::NonCanonical), "{n}");
        assert_eq!(encode_value(&Value::U64(u64::from(n))).unwrap(), [n]);
    }
    let mut first_long = vec![0xCF];
    first_long.extend_from_slice(&128u64.to_be_bytes());
    assert_eq!(decode_value(&first_long), Ok((Value::U64(128), 9)));
    assert_eq!(encode_value(&Value::U64(128)).unwrap(), first_long);
}

#[test]
fn floats_are_carried_bit_exact() {
    // every bit pattern is its own value: NaN payloads, the sign of a NaN and
    // -0.0 come back as written, and each has exactly one encoding
    let f64_bits = [
        0x7FF8_0000_0000_0000u64, // quiet NaN
        0x7FF8_0000_0000_0001,    // NaN with a payload
        0xFFF8_0000_0000_0000,    // negative NaN
        0x7FF0_0000_0000_0001,    // signalling NaN
        0x8000_0000_0000_0000,    // -0.0
        0,
        0x7FF0_0000_0000_0000, // +inf
        1,                     // smallest subnormal
    ];
    for bits in f64_bits {
        let b = encode_value(&Value::F64(f64::from_bits(bits))).unwrap();
        let mut want = vec![0xCB];
        want.extend_from_slice(&bits.to_be_bytes());
        assert_eq!(b, want, "{bits:#018x}");
        let (back, _) = decode_value(&b).unwrap();
        assert!(
            matches!(back, Value::F64(g) if g.to_bits() == bits),
            "{bits:#018x}"
        );
        let packed = encode_value(&Value::F64Array(vec![f64::from_bits(bits)])).unwrap();
        let (back, _) = decode_value(&packed).unwrap();
        assert!(
            matches!(&back, Value::F64Array(a) if a.len() == 1 && a[0].to_bits() == bits),
            "{bits:#018x}"
        );
    }
    let f32_bits = [
        0x7FC0_0000u32,
        0x7FC0_0001,
        0xFFC0_0000,
        0x7F80_0001,
        0x8000_0000,
        1,
    ];
    for bits in f32_bits {
        let b = encode_value(&Value::F32(f32::from_bits(bits))).unwrap();
        let mut want = vec![0xCA];
        want.extend_from_slice(&bits.to_be_bytes());
        assert_eq!(b, want, "{bits:#010x}");
        let (back, _) = decode_value(&b).unwrap();
        assert!(
            matches!(back, Value::F32(g) if g.to_bits() == bits),
            "{bits:#010x}"
        );
        let packed = encode_value(&Value::F32Array(vec![f32::from_bits(bits)])).unwrap();
        let (back, _) = decode_value(&packed).unwrap();
        assert!(
            matches!(&back, Value::F32Array(a) if a.len() == 1 && a[0].to_bits() == bits),
            "{bits:#010x}"
        );
    }
}

/// One golden encoding per type, written out by hand from `FORMAT_SPEC`.
#[test]
fn golden_vectors() {
    let cases: Vec<(Value, Vec<u8>)> = vec![
        (Value::Null, vec![0xC0]),
        (Value::Bool(false), vec![0xC2]),
        (Value::Bool(true), vec![0xC3]),
        (Value::U64(0), vec![0x00]),
        (Value::U64(127), vec![0x7F]),
        (Value::U64(128), vec![0xCF, 0, 0, 0, 0, 0, 0, 0, 0x80]),
        (
            Value::I64(-2),
            vec![0xD3, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFE],
        ),
        (Value::F32(1.5), vec![0xCA, 0x3F, 0xC0, 0, 0]),
        (Value::F64(1.5), vec![0xCB, 0x3F, 0xF8, 0, 0, 0, 0, 0, 0]),
        (s("hé"), vec![0xDB, 0, 0, 0, 3, b'h', 0xC3, 0xA9]),
        (Value::Bytes(vec![1, 2]), vec![0xC6, 0, 0, 0, 2, 1, 2]),
        (
            Value::F32Array(vec![1.5, -2.0]),
            vec![0xD4, 0, 0, 0, 2, 0, 0, 0xC0, 0x3F, 0, 0, 0, 0xC0],
        ),
        (
            Value::F64Array(vec![1.5]),
            vec![0xD5, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0xF8, 0x3F],
        ),
        (
            Value::U32Array(vec![1, 0x0102_0304]),
            vec![0xD6, 0, 0, 0, 2, 1, 0, 0, 0, 4, 3, 2, 1],
        ),
        (
            Value::U64Array(vec![0x0102]),
            vec![0xD7, 0, 0, 0, 1, 2, 1, 0, 0, 0, 0, 0, 0],
        ),
        (
            Value::I64Array(vec![-1]),
            vec![
                0xD8, 0, 0, 0, 1, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            ],
        ),
        (Value::F32Array(vec![]), vec![0xD4, 0, 0, 0, 0]),
        (
            Value::Array(vec![Value::Null, Value::U64(1)]),
            vec![0xDD, 0, 0, 0, 2, 0xC0, 0x01],
        ),
        (
            Value::Map(vec![
                (Value::U64(2), Value::Null),
                (Value::U64(1), Value::Bool(true)),
            ]),
            vec![0xDF, 0, 0, 0, 2, 0x01, 0xC3, 0x02, 0xC0],
        ),
    ];
    for (v, want) in &cases {
        assert_eq!(&encode_value(v).unwrap(), want, "{v:?}");
        let (back, used) = decode_value(want).unwrap();
        assert_eq!(used, want.len(), "{v:?}");
        assert_eq!(&encode_value(&back).unwrap(), want, "{v:?}");
    }
    assert_eq!(
        encode(&Value::Null).unwrap(),
        [b'A', b'L', b'S', b'R', 1, 0, 0xC0]
    );
}

#[test]
fn packed_arrays_check_their_byte_length() {
    for (tag, width) in [(0xD4u8, 4usize), (0xD5, 8), (0xD6, 4), (0xD7, 8), (0xD8, 8)] {
        // two elements, one byte short
        let mut b = vec![tag, 0, 0, 0, 2];
        b.extend(core::iter::repeat_n(0u8, 2 * width - 1));
        assert_eq!(
            decode_value(&b),
            Err(SerialError::BufferTooShort),
            "{tag:#04x}"
        );
        b.push(0);
        assert!(decode_value(&b).is_ok(), "{tag:#04x}");
        // a forged count is refused before it sizes anything
        assert_eq!(
            decode_value(&[tag, 0xFF, 0xFF, 0xFF, 0xFF, 0]),
            Err(SerialError::BufferTooShort),
            "{tag:#04x}"
        );
    }
}

#[test]
fn map_keys_are_written_sorted_and_read_only_sorted() {
    let unsorted = Value::Map(vec![(s("bb"), Value::U64(2)), (s("a"), Value::U64(1))]);
    let sorted = Value::Map(vec![(s("a"), Value::U64(1)), (s("bb"), Value::U64(2))]);
    let bytes = encode_value(&unsorted).unwrap();
    assert_eq!(bytes, encode_value(&sorted).unwrap());
    assert_eq!(decode_value(&bytes), Ok((sorted, bytes.len())));

    // the same entries in the written order are refused
    let mut swapped = vec![0xDF, 0, 0, 0, 2];
    swapped.extend_from_slice(&encode_value(&s("bb")).unwrap());
    swapped.push(2);
    swapped.extend_from_slice(&encode_value(&s("a")).unwrap());
    swapped.push(1);
    assert_eq!(decode_value(&swapped), Err(SerialError::UnsortedKeys));

    let mut twice = vec![0xDF, 0, 0, 0, 2];
    for v in [1u8, 2] {
        twice.extend_from_slice(&encode_value(&s("a")).unwrap());
        twice.push(v);
    }
    assert_eq!(decode_value(&twice), Err(SerialError::DuplicateKey));

    let dup = Value::Map(vec![(s("a"), Value::U64(1)), (s("a"), Value::U64(2))]);
    assert_eq!(encode_value(&dup), Err(SerialError::DuplicateKey));

    // keys of different types are ordered by their encodings too
    let mixed = Value::Map(vec![(s("x"), Value::Null), (Value::U64(7), Value::Null)]);
    let b = encode_value(&mixed).unwrap();
    let (back, _) = decode_value(&b).unwrap();
    assert_eq!(
        back,
        Value::Map(vec![(Value::U64(7), Value::Null), (s("x"), Value::Null)])
    );
}

fn nested(depth: usize) -> Value {
    let mut v = Value::Null;
    for i in 0..depth {
        v = if i % 2 == 0 {
            Value::Array(vec![v])
        } else {
            Value::Map(vec![(Value::Null, v)])
        };
    }
    v
}

#[test]
fn nesting_stops_at_max_depth_on_both_sides() {
    let deepest = nested(MAX_DEPTH);
    let bytes = encode(&deepest).unwrap();
    assert_eq!(decode(&bytes), Ok(deepest));

    let too_deep = nested(MAX_DEPTH + 1);
    assert_eq!(encode(&too_deep), Err(SerialError::TooDeep));

    // the same shape written by hand, one level deeper than the encoder allows
    let mut body = Vec::new();
    for _ in 0..=MAX_DEPTH {
        body.extend_from_slice(&[0xDD, 0, 0, 0, 1]);
    }
    body.push(0xC0);
    assert_eq!(decode(&framed(&body)), Err(SerialError::TooDeep));

    // a forged input far deeper than any stack is refused at the bound
    let mut huge = Vec::new();
    for _ in 0..200_000 {
        huge.extend_from_slice(&[0xDD, 0, 0, 0, 1]);
    }
    assert_eq!(decode_value(&huge), Err(SerialError::TooDeep));
}

/// `depth` maps, each holding the next one as the value (or the key) of its one entry.
fn nested_maps(depth: usize, in_key: bool) -> Value {
    let mut v = Value::Null;
    for _ in 0..depth {
        v = if in_key {
            Value::Map(vec![(v, Value::Null)])
        } else {
            Value::Map(vec![(Value::Null, v)])
        };
    }
    v
}

fn map_bytes(depth: usize, in_key: bool) -> Vec<u8> {
    // one entry per level: `df 00 00 00 01`, then the key and the value
    let mut out = Vec::new();
    for _ in 0..depth {
        out.extend_from_slice(&[0xDF, 0, 0, 0, 1]);
        if !in_key {
            out.extend_from_slice(&[0xC0]);
        }
    }
    out.extend_from_slice(&[0xC0]);
    if in_key {
        out.extend(core::iter::repeat_n(0xC0u8, depth));
    }
    out
}

#[test]
fn map_keys_and_values_count_toward_the_depth() {
    for in_key in [false, true] {
        let deepest = nested_maps(MAX_DEPTH, in_key);
        let bytes = encode_value(&deepest).unwrap();
        assert_eq!(bytes, map_bytes(MAX_DEPTH, in_key), "in_key {in_key}");
        assert_eq!(
            decode_value(&bytes),
            Ok((deepest, bytes.len())),
            "in_key {in_key}"
        );

        assert_eq!(
            encode_value(&nested_maps(MAX_DEPTH + 1, in_key)),
            Err(SerialError::TooDeep),
            "in_key {in_key}"
        );
        assert_eq!(
            decode_value(&map_bytes(MAX_DEPTH + 1, in_key)),
            Err(SerialError::TooDeep),
            "in_key {in_key}"
        );
    }
}

#[test]
fn a_forged_count_is_refused_before_it_sizes_anything() {
    for tag in [0xDD, 0xDF] {
        assert_eq!(
            decode_value(&[tag, 0xFF, 0xFF, 0xFF, 0xFF, 0xC0]),
            Err(SerialError::BufferTooShort)
        );
    }
    assert_eq!(
        decode_value(&[0xDB, 0xFF, 0xFF, 0xFF, 0xFF, b'a']),
        Err(SerialError::BufferTooShort)
    );
}

#[test]
fn varints_have_one_form() {
    assert_eq!(decode_varint(&[0x80, 0x00]), Err(SerialError::NonCanonical));
    assert_eq!(decode_varint(&[0x81, 0x00]), Err(SerialError::NonCanonical));
    assert_eq!(decode_varint(&[0x00]), Ok((0, 1)));
    assert_eq!(decode_varint(&[0x80]), Err(SerialError::BufferTooShort));

    let max = encode_varint(u64::MAX);
    assert_eq!(max.len(), 10);
    assert_eq!(decode_varint(&max), Ok((u64::MAX, 10)));
    let mut over = max.clone();
    over[9] = 0x02;
    assert_eq!(decode_varint(&over), Err(SerialError::TooLong));
    let mut eleven = max;
    eleven[9] = 0x81;
    eleven.push(0x00);
    assert_eq!(decode_varint(&eleven), Err(SerialError::TooLong));

    for n in [0u64, 1, 127, 128, 300, 1 << 35, u64::MAX - 1] {
        let b = encode_varint(n);
        assert_eq!(decode_varint(&b), Ok((n, b.len())), "{n}");
    }
}

fn reencodes_to_itself(bytes: &[u8]) -> bool {
    match decode_value(bytes) {
        Ok((v, used)) => encode_value(&v).unwrap() == bytes[..used],
        Err(_) => true,
    }
}

#[test]
fn every_short_input_fails_or_reencodes_to_itself() {
    let mut accepted = 0usize;
    for a in 0..=255u8 {
        assert!(reencodes_to_itself(&[a]), "{a:#04x}");
        accepted += usize::from(decode_value(&[a]).is_ok());
        for b in 0..=255u8 {
            assert!(reencodes_to_itself(&[a, b]), "{a:#04x} {b:#04x}");
        }
    }
    // 128 fixints + null + false + true
    assert_eq!(accepted, 131, "the one-byte values this format defines");
}

#[test]
fn sampled_longer_inputs_fail_or_reencode_to_themselves() {
    // fixed-seed xorshift; each sample starts with a defined tag so the
    // decoder gets past the first byte
    let tags = [
        0xCF, 0xD3, 0xCA, 0xCB, 0xDB, 0xC6, 0xD4, 0xD5, 0xD6, 0xD7, 0xD8, 0xDD, 0xDF,
    ];
    let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    let mut accepted = 0usize;
    for _ in 0..20_000 {
        let len = 1 + usize::try_from(next() % 24).unwrap();
        let mut buf: Vec<u8> = (0..len).map(|_| next().to_le_bytes()[4]).collect();
        buf[0] = tags[usize::try_from(next() % 13).unwrap()];
        if buf.len() >= 5 && matches!(buf[0], 0xDB | 0xC6 | 0xD4..=0xD8 | 0xDD | 0xDF) {
            // short lengths and counts, so some samples decode
            buf[1..4].fill(0);
            buf[4] %= 4;
        }
        assert!(reencodes_to_itself(&buf), "{buf:02x?}");
        accepted += usize::from(decode_value(&buf).is_ok());
    }
    assert!(
        accepted > 1000,
        "only {accepted} samples decoded: the sample measured little"
    );
}

#[test]
fn the_semantics_id_is_the_hash_of_the_format_spec() {
    let digest: [u8; 32] = Sha256::digest(FORMAT_SPEC.as_bytes()).into();
    assert_eq!(digest, SERIAL_SEMANTICS_ID);
    assert!(FORMAT_SPEC.starts_with(&format!("alice-serial format {FORMAT_VERSION}\n")));
    assert!(FORMAT_SPEC.contains(&format!("at most {MAX_DEPTH} arrays and maps")));
}
