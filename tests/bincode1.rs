//! Oracle: the bincode 1 default layout, byte for byte
//!
//! Every expected byte string below is written out by hand from the layout
//! (`src/bincode1-layout.txt`), not produced by the encoder under test:
//! - each serde data-model kind encodes to the bytes the layout names, and
//!   decodes back to the same value
//! - malformed input (bool, Option tag, char, UTF-8, variant index, short
//!   input, a length that does not fit the remaining bytes) is refused with the
//!   error that names it, without panicking
//! - `from_slice` refuses trailing bytes, `from_slice_prefix` returns how many
//!   bytes the value used, and neither side reads or writes past the limit
//! - `BINCODE1_LAYOUT_ID` is the SHA-256 of `BINCODE1_LAYOUT`
#![cfg(feature = "serde")]

use std::collections::BTreeMap;

use alice_serial::bincode1::{
    encode_into, from_slice, from_slice_prefix, from_slice_prefix_with_depth,
    from_slice_with_depth, to_vec, Bincode1Error, BINCODE1_LAYOUT, BINCODE1_LAYOUT_ID,
    DEFAULT_MAX_DEPTH,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const BIG: usize = 1 << 20;

/// encode, compare with the hand-written bytes, decode back, compare with the value
fn pins<T>(value: &T, expected: &[u8])
where
    T: Serialize + for<'de> Deserialize<'de> + PartialEq + std::fmt::Debug,
{
    let bytes = to_vec(value, BIG).unwrap();
    assert_eq!(bytes, expected, "encode of {value:?}");
    let back: T = from_slice(expected, BIG).unwrap();
    assert_eq!(&back, value, "decode of {expected:02x?}");
    assert_eq!(
        from_slice_prefix::<T>(expected, BIG).unwrap(),
        (back, expected.len())
    );
}

#[test]
fn integers_are_fixed_width_little_endian() {
    pins(&0x12u8, &[0x12]);
    pins(&-2i8, &[0xfe]);
    pins(&0x1234u16, &[0x34, 0x12]);
    pins(&-2i16, &[0xfe, 0xff]);
    pins(&0x1234_5678u32, &[0x78, 0x56, 0x34, 0x12]);
    pins(&-1i32, &[0xff; 4]);
    pins(&0x0102_0304_0506_0708u64, &[8, 7, 6, 5, 4, 3, 2, 1]);
    pins(&i64::MIN, &[0, 0, 0, 0, 0, 0, 0, 0x80]);
    let mut u = [0u8; 16];
    u[0] = 1;
    u[15] = 0x80;
    pins(&((1u128 << 127) | 1), &u);
    pins(&-1i128, &[0xff; 16]);
    // usize and isize are written as 64-bit
    pins(&7usize, &[7, 0, 0, 0, 0, 0, 0, 0]);
    pins(&-7isize, &(-7i64).to_le_bytes());
}

#[test]
fn floats_keep_their_bits() {
    pins(&1.0f32, &[0, 0, 0x80, 0x3f]);
    pins(&-0.0f64, &[0, 0, 0, 0, 0, 0, 0, 0x80]);
    // a NaN with a payload: compare bits, not values
    let nan = f32::from_bits(0x7fc0_1234);
    assert_eq!(to_vec(&nan, BIG).unwrap(), [0x34, 0x12, 0xc0, 0x7f]);
    let back: f32 = from_slice(&[0x34, 0x12, 0xc0, 0x7f], BIG).unwrap();
    assert_eq!(back.to_bits(), 0x7fc0_1234);
    let nan64 = f64::from_bits(0xfff8_0000_0000_0001);
    let bytes = to_vec(&nan64, BIG).unwrap();
    assert_eq!(bytes, 0xfff8_0000_0000_0001u64.to_le_bytes());
    let back: f64 = from_slice(&bytes, BIG).unwrap();
    assert_eq!(back.to_bits(), 0xfff8_0000_0000_0001);
}

#[test]
fn bool_option_unit() {
    pins(&false, &[0]);
    pins(&true, &[1]);
    pins(&None::<u16>, &[0]);
    pins(&Some(0x0102u16), &[1, 2, 1]);
    pins(&Some(None::<u8>), &[1, 0]);
    pins(&(), &[]);
    pins(&Unit, &[]);
}

#[test]
fn chars_are_their_utf8_bytes_without_a_length() {
    pins(&'a', b"a");
    pins(&'é', "é".as_bytes());
    pins(&'あ', "あ".as_bytes());
    pins(&'😀', "😀".as_bytes());
    assert_eq!("😀".len(), 4);
}

#[test]
fn strings_and_bytes_carry_a_u64_length() {
    pins(&String::new(), &[0; 8]);
    pins(
        &String::from("hé"),
        &[3, 0, 0, 0, 0, 0, 0, 0, b'h', 0xc3, 0xa9],
    );
    // a borrowed str decodes from the input without copying
    let input = [2, 0, 0, 0, 0, 0, 0, 0, b'o', b'k'];
    let s: &str = from_slice(&input, BIG).unwrap();
    assert_eq!(s, "ok");
    let raw = [0xffu8, 0, 1];
    let bytes = to_vec(&B(&raw), BIG).unwrap();
    assert_eq!(bytes, [3, 0, 0, 0, 0, 0, 0, 0, 0xff, 0, 1]);
    let back: B<'_> = from_slice(&bytes, BIG).unwrap();
    assert_eq!(back, B(&raw));
}

/// serialize / deserialize a byte slice through `serialize_bytes` / `deserialize_bytes`
mod serde_bytes_like {
    use serde::{de::Visitor, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(v: &[u8], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(v)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<&'de [u8], D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = &'de [u8];
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("borrowed bytes")
            }
            fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E> {
                Ok(v)
            }
        }
        d.deserialize_bytes(V)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Unit;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct B<'a>(#[serde(with = "serde_bytes_like")] &'a [u8]);

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Point {
    x: i16,
    y: u8,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Wrap(u32);

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Pair(u8, i8);

#[derive(Serialize, Deserialize, PartialEq, Debug)]
enum Shape {
    Empty,
    Dot(u8),
    Line(u8, u8),
    Box { w: u16, h: u16 },
}

#[test]
fn structs_tuples_and_arrays_have_no_length() {
    pins(&Point { x: -2, y: 7 }, &[0xfe, 0xff, 7]);
    pins(&Wrap(1), &[1, 0, 0, 0]);
    pins(&Pair(1, -1), &[1, 0xff]);
    pins(&(1u8, 2u16, 'z'), &[1, 2, 0, b'z']);
    pins(&[9u8, 8, 7], &[9, 8, 7]);
}

#[test]
fn enums_are_a_u32_index_then_the_fields() {
    pins(&Shape::Empty, &[0, 0, 0, 0]);
    pins(&Shape::Dot(5), &[1, 0, 0, 0, 5]);
    pins(&Shape::Line(1, 2), &[2, 0, 0, 0, 1, 2]);
    pins(&Shape::Box { w: 3, h: 0x0102 }, &[3, 0, 0, 0, 3, 0, 2, 1]);
}

#[test]
fn seqs_and_maps_carry_a_u64_count() {
    pins(&Vec::<u16>::new(), &[0; 8]);
    pins(&vec![1u16, 2], &[2, 0, 0, 0, 0, 0, 0, 0, 1, 0, 2, 0]);
    let mut m = BTreeMap::new();
    m.insert(2u8, 'b');
    m.insert(1u8, 'a');
    pins(&m, &[2, 0, 0, 0, 0, 0, 0, 0, 1, b'a', 2, b'b']);
    pins(
        &vec![Some(Shape::Dot(1)), None],
        &[2, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 1, 0],
    );
}

#[test]
fn a_nested_value_matches_its_hand_layout() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Scene {
        version: String,
        shapes: Vec<Shape>,
        origin: Option<(f32, f32)>,
        tags: BTreeMap<String, u8>,
    }
    let mut tags = BTreeMap::new();
    tags.insert("k".to_string(), 9);
    let scene = Scene {
        version: "7".into(),
        shapes: vec![Shape::Empty, Shape::Box { w: 1, h: 2 }],
        origin: Some((1.0, -0.0)),
        tags,
    };
    let mut expected = vec![1, 0, 0, 0, 0, 0, 0, 0, b'7'];
    expected.extend_from_slice(&[2, 0, 0, 0, 0, 0, 0, 0]);
    expected.extend_from_slice(&[0, 0, 0, 0]);
    expected.extend_from_slice(&[3, 0, 0, 0, 1, 0, 2, 0]);
    expected.extend_from_slice(&[1, 0, 0, 0x80, 0x3f, 0, 0, 0, 0x80]);
    expected.extend_from_slice(&[1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, b'k', 9]);
    pins(&scene, &expected);
}

#[test]
fn malformed_input_is_refused_by_name() {
    assert_eq!(
        from_slice::<bool>(&[2], BIG),
        Err(Bincode1Error::InvalidBool(2))
    );
    assert_eq!(
        from_slice::<Option<u8>>(&[2, 0], BIG),
        Err(Bincode1Error::InvalidOptionTag(2))
    );
    // a lone continuation byte, a lead byte followed by a non-continuation, a surrogate
    assert_eq!(
        from_slice::<char>(&[0x80], BIG),
        Err(Bincode1Error::InvalidChar)
    );
    assert_eq!(
        from_slice::<char>(&[0xc3, 0x41], BIG),
        Err(Bincode1Error::InvalidChar)
    );
    // a 2-byte sequence cut short is the end of the input, or the limit, not a bad char
    assert_eq!(
        from_slice::<char>(&[0xc3], BIG),
        Err(Bincode1Error::UnexpectedEnd)
    );
    assert_eq!(
        from_slice_prefix::<char>(&[0xc3, 0xa9], 1),
        Err(Bincode1Error::LimitExceeded { limit: 1 })
    );
    assert_eq!(
        from_slice::<char>(&[0xed, 0xa0, 0x80], BIG),
        Err(Bincode1Error::InvalidChar)
    );
    assert_eq!(
        from_slice::<String>(&[1, 0, 0, 0, 0, 0, 0, 0, 0xff], BIG),
        Err(Bincode1Error::InvalidUtf8)
    );
    assert!(matches!(
        from_slice::<Shape>(&[4, 0, 0, 0], BIG),
        Err(Bincode1Error::Custom(_))
    ));
    assert_eq!(
        from_slice::<u32>(&[1, 2, 3], BIG),
        Err(Bincode1Error::UnexpectedEnd)
    );
    assert_eq!(
        from_slice::<u8>(&[], BIG),
        Err(Bincode1Error::UnexpectedEnd)
    );
}

#[test]
fn a_length_is_checked_against_the_bytes_that_remain() {
    // claims 2^62 elements with 0 bytes after it: refused before any allocation
    let mut huge = (1u64 << 62).to_le_bytes().to_vec();
    assert!(matches!(
        from_slice::<Vec<u64>>(&huge, BIG),
        Err(Bincode1Error::Length(_))
    ));
    assert!(matches!(
        from_slice::<String>(&huge, BIG),
        Err(Bincode1Error::Length(_))
    ));
    assert!(matches!(
        from_slice::<BTreeMap<u8, u8>>(&huge, BIG),
        Err(Bincode1Error::Length(_))
    ));
    // a seq of zero-size elements longer than the remaining input is refused too
    huge = 3u64.to_le_bytes().to_vec();
    assert!(matches!(
        from_slice::<Vec<()>>(&huge, BIG),
        Err(Bincode1Error::Length(_))
    ));
    // ... and accepted when it fits
    let mut fits = 3u64.to_le_bytes().to_vec();
    fits.extend_from_slice(&[0, 0, 0]);
    assert_eq!(
        from_slice_prefix::<Vec<()>>(&fits, BIG),
        Ok((vec![(), (), ()], 8))
    );
}

#[test]
fn from_slice_refuses_trailing_bytes_and_the_prefix_form_reports_them() {
    assert_eq!(
        from_slice::<u16>(&[1, 0, 9], BIG),
        Err(Bincode1Error::TrailingBytes { used: 2, len: 3 })
    );
    assert_eq!(from_slice_prefix::<u16>(&[1, 0, 9], BIG), Ok((1, 2)));
}

#[test]
fn nothing_is_read_or_written_past_the_limit() {
    let v = vec![1u8, 2, 3];
    let bytes = to_vec(&v, BIG).unwrap();
    assert_eq!(bytes.len(), 11);
    assert_eq!(to_vec(&v, 11).unwrap(), bytes);
    assert_eq!(
        to_vec(&v, 10),
        Err(Bincode1Error::LimitExceeded { limit: 10 })
    );
    assert_eq!(from_slice::<Vec<u8>>(&bytes, 11).unwrap(), v);
    // the value needs byte 11; the limit stops the read at 10
    assert_eq!(
        from_slice_prefix::<Vec<u8>>(&bytes, 10),
        Err(Bincode1Error::LimitExceeded { limit: 10 })
    );
    // a length that fits the input but not the limit is refused by the limit, before the
    // string is sized
    assert_eq!(
        from_slice_prefix::<String>(&[3, 0, 0, 0, 0, 0, 0, 0, b'a', b'b', b'c'], 10),
        Err(Bincode1Error::LimitExceeded { limit: 10 })
    );
    // a value that fits under the limit, with more input after the limit (byte 0 is
    // the low byte of the length prefix, 3)
    assert_eq!(from_slice_prefix::<u8>(&bytes, 1), Ok((3, 1)));
    assert_eq!(
        from_slice::<u8>(&bytes, 1),
        Err(Bincode1Error::TrailingBytes { used: 1, len: 11 })
    );
    // encode_into appends, and its limit counts only what it appends
    let mut out = vec![0xaa; 5];
    assert_eq!(encode_into(&mut out, &0x0102u16, 2), Ok(2));
    assert_eq!(out, [0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 2, 1]);
    assert_eq!(
        encode_into(&mut out, &0x0102u16, 1),
        Err(Bincode1Error::LimitExceeded { limit: 1 })
    );
    assert_eq!(out.len(), 7, "a refused encode leaves the buffer as it was");
    // refused part-way: the u8 fits, the u16 after it does not; the u8 is taken back
    assert_eq!(
        encode_into(&mut out, &(9u8, 0x0102u16), 2),
        Err(Bincode1Error::LimitExceeded { limit: 2 })
    );
    assert_eq!(out, [0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 2, 1]);
}

#[test]
fn layouts_that_do_not_describe_themselves_are_refused() {
    // an untagged enum needs deserialize_any
    #[derive(Deserialize, Debug)]
    #[serde(untagged)]
    #[allow(dead_code)]
    enum Either {
        A(u8),
        B(String),
    }
    // a seq with no known length cannot be written
    struct Unsized;
    impl Serialize for Unsized {
        fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            use serde::ser::SerializeSeq;
            let mut seq = s.serialize_seq(None)?;
            seq.serialize_element(&1u8)?;
            seq.end()
        }
    }
    assert_eq!(
        to_vec(&Unsized, BIG),
        Err(Bincode1Error::SequenceMustHaveLength)
    );
    assert_eq!(
        from_slice::<Either>(&[1], BIG).unwrap_err(),
        Bincode1Error::DeserializeAnyNotSupported
    );
}

#[test]
fn the_layout_id_is_the_hash_of_the_layout_text() {
    assert!(
        !BINCODE1_LAYOUT.contains('\r'),
        "BINCODE1_LAYOUT has CR bytes: check .gitattributes"
    );
    let digest: [u8; 32] = Sha256::digest(BINCODE1_LAYOUT.as_bytes()).into();
    assert_eq!(digest, BINCODE1_LAYOUT_ID);
    assert!(BINCODE1_LAYOUT.starts_with("alice-serial bincode1 layout 1\n"));
    assert!(BINCODE1_LAYOUT.contains(&format!("nested more than {DEFAULT_MAX_DEPTH} deep")));
}

/// A type that recurses through an enum: `More` levels then `End`.
#[derive(Serialize, Deserialize, PartialEq, Debug)]
enum Nest {
    End,
    More(Box<Nest>),
}

/// `levels` x `More` then `End`: `levels + 1` nested enums, 4 bytes each.
fn nest_bytes(levels: usize) -> Vec<u8> {
    let mut b = [1u8, 0, 0, 0].repeat(levels);
    b.extend_from_slice(&[0, 0, 0, 0]);
    b
}

/// A type that recurses through `Option`.
#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct OptChain(Option<Box<OptChain>>);

/// A type that recurses through a sequence.
#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct SeqChain(Vec<SeqChain>);

#[test]
fn nesting_deeper_than_the_depth_limit_is_refused_without_exhausting_the_stack() {
    let max = DEFAULT_MAX_DEPTH;
    // exactly max nested enums decode; one more is refused
    assert!(from_slice::<Nest>(&nest_bytes(max - 1), BIG).is_ok());
    assert_eq!(
        from_slice::<Nest>(&nest_bytes(max), BIG),
        Err(Bincode1Error::TooDeep { max_depth: max })
    );
    // about 1 MiB of input asking for ~260k levels: refused at the limit, no stack overflow
    assert_eq!(
        from_slice::<Nest>(&nest_bytes(1 << 18), 1 << 21),
        Err(Bincode1Error::TooDeep { max_depth: max })
    );
    // Option and sequences count too: 1 byte (Some) or 8 bytes (a 1-element count) per level
    let opt = [1u8].repeat(1 << 16);
    assert_eq!(
        from_slice_prefix::<OptChain>(&opt, BIG),
        Err(Bincode1Error::TooDeep { max_depth: max })
    );
    let seq = [1u8, 0, 0, 0, 0, 0, 0, 0].repeat(1 << 14);
    assert_eq!(
        from_slice_prefix::<SeqChain>(&seq, BIG),
        Err(Bincode1Error::TooDeep { max_depth: max })
    );
    // a caller that needs deeper data names the depth
    let deep = nest_bytes(max + 10);
    assert!(from_slice_with_depth::<Nest>(&deep, BIG, max + 11).is_ok());
    assert_eq!(
        from_slice_prefix_with_depth::<Nest>(&deep, BIG, max + 10),
        Err(Bincode1Error::TooDeep {
            max_depth: max + 10
        })
    );
    // depth is released on the way out: many nested values side by side are each one deep
    let side_by_side: Vec<Option<u8>> = vec![Some(1); 4 * max];
    let bytes = to_vec(&side_by_side, BIG).unwrap();
    assert_eq!(
        from_slice::<Vec<Option<u8>>>(&bytes, BIG).unwrap(),
        side_by_side
    );
    // fixed-shape nesting (tuples, structs, newtypes) cannot recurse and does not count
    assert_eq!(
        from_slice_with_depth::<((((u8,),),),)>(&[7], BIG, 0).map(|v| v.0 .0 .0 .0),
        Ok(7)
    );
}

#[test]
#[allow(
    clippy::zero_sized_map_values,
    reason = "a map of zero-size entries is the input this test refuses"
)]
fn a_sequence_that_would_not_decode_is_refused_on_encode() {
    // n zero-size elements take fewer than n bytes, so the decoder (which needs a byte per
    // element) could not read them back: the encoder refuses them instead of writing them
    assert_eq!(
        to_vec(&vec![(), ()], BIG),
        Err(Bincode1Error::ZeroSizeElements { count: 2 })
    );
    assert_eq!(
        to_vec(&vec![Unit], BIG),
        Err(Bincode1Error::ZeroSizeElements { count: 1 })
    );
    let mut m = BTreeMap::new();
    m.insert((), ());
    assert_eq!(
        to_vec(&m, BIG),
        Err(Bincode1Error::ZeroSizeElements { count: 1 })
    );
    // an empty one, and one whose elements take at least a byte each, are written and read back
    assert_eq!(to_vec(&Vec::<()>::new(), BIG).unwrap(), [0; 8]);
    let mixed: Vec<Option<()>> = vec![Some(()), None];
    let bytes = to_vec(&mixed, BIG).unwrap();
    assert_eq!(from_slice::<Vec<Option<()>>>(&bytes, BIG).unwrap(), mixed);
}
