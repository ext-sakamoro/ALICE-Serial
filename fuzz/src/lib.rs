//! The differential check of `alice_serial::bincode1` against bincode itself.
//!
//! [`check`] takes arbitrary bytes and asserts two things:
//! - a value built from the bytes ([`Zoo`], every serde data-model kind the
//!   layout covers) encodes to the same bytes with alice-serial, bincode 1.3.3
//!   (`bincode::serialize`) and bincode 2.0.1 (`config::legacy()`), and decodes
//!   back to a value that encodes to those bytes again
//! - the bytes decoded as a `Zoo` are accepted or refused alike by the three,
//!   and when accepted they give the same value (compared by its bytes) and
//!   alice-serial and bincode 2 report the same number of bytes used
//!
//! The fuzz target and the stable test in `tests/` call the same function. The
//! types below are test fixtures (one per serde data-model kind), not API, so
//! their fields are public.

use std::collections::BTreeMap;

use arbitrary::{Arbitrary, Unstructured};
use serde::{Deserialize, Serialize};

/// The limit every side is given; far above any fuzz input.
pub const LIMIT: usize = 1 << 20;

/// A nesting depth that holds any value a fuzz input can describe (a level takes at
/// least a byte, and inputs stay far below this many bytes).
pub const DEEP: usize = 1 << 16;

/// A unit struct: no bytes.
#[derive(Serialize, Deserialize, Arbitrary, Debug)]
pub struct UnitStruct;

/// A newtype struct: the inner value's bytes.
#[derive(Serialize, Deserialize, Arbitrary, Debug)]
pub struct Newtype(pub i32);

/// A tuple struct: the fields in order, no length.
#[derive(Serialize, Deserialize, Arbitrary, Debug)]
pub struct TupleStruct(pub u8, pub Option<char>);

/// Every enum variant kind, one recursive.
#[derive(Serialize, Deserialize, Arbitrary, Debug)]
pub enum Kind {
    Unit,
    Newtype(u32),
    Tuple(i8, Option<bool>),
    Struct { x: f32, y: Vec<u8> },
    Nested(Option<Box<Kind>>),
}

/// One field per data-model kind the layout covers.
#[derive(Serialize, Deserialize, Arbitrary, Debug)]
pub struct Zoo {
    pub b: bool,
    pub u8_: u8,
    pub i8_: i8,
    pub u16_: u16,
    pub i16_: i16,
    pub u32_: u32,
    pub i32_: i32,
    pub u64_: u64,
    pub i64_: i64,
    pub u128_: u128,
    pub i128_: i128,
    pub usize_: usize,
    pub f32_: f32,
    pub f64_: f64,
    pub c: char,
    pub s: String,
    pub o: Option<u16>,
    pub unit: (),
    pub unit_struct: UnitStruct,
    pub newtype: Newtype,
    pub tuple: (u8, i64, char),
    pub tuple_struct: TupleStruct,
    pub array: [u16; 3],
    pub seq: Vec<Kind>,
    pub map: BTreeMap<u8, String>,
    pub nested: Option<Vec<Option<String>>>,
}

fn ours(v: &Zoo) -> Vec<u8> {
    alice_serial::bincode1::to_vec(v, LIMIT).expect("alice-serial encode")
}

fn bincode1(v: &Zoo) -> Vec<u8> {
    bincode1_ref::serialize(v).expect("bincode 1 encode")
}

fn bincode2(v: &Zoo) -> Vec<u8> {
    bincode2_ref::serde::encode_to_vec(v, bincode2_ref::config::legacy()).expect("bincode 2 encode")
}

/// What one call compared: values encoded by all three, and raw inputs decoded by all three.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Compared {
    /// Values built from the input whose bytes the three encoders agreed on.
    pub encoded: usize,
    /// Raw inputs the three decoders all accepted with the same value.
    pub accepted: usize,
    /// Raw inputs the three decoders all refused.
    pub refused: usize,
    /// Raw inputs alice-serial refused at the default depth, which bincode has no bound
    /// for; each is compared again at `DEEP` and also counted as accepted or refused.
    pub too_deep: usize,
}

/// One differential case; panics on any disagreement.
pub fn check(data: &[u8]) -> Compared {
    let mut compared = Compared::default();

    // 1. a value: the three encoders agree, and our decode round-trips it
    if let Ok(v) = Zoo::arbitrary(&mut Unstructured::new(data)) {
        let a = ours(&v);
        assert_eq!(a, bincode1(&v), "alice-serial vs bincode 1: {v:?}");
        assert_eq!(a, bincode2(&v), "alice-serial vs bincode 2: {v:?}");
        // a value built from the input can nest past the default depth; its own bytes
        // decode with a depth that holds it
        let back: Zoo = alice_serial::bincode1::from_slice_with_depth(&a, LIMIT, DEEP)
            .expect("decode own bytes");
        assert_eq!(ours(&back), a, "round trip changed the bytes");
        compared.encoded += 1;
    }

    // 2. the raw bytes: the three decoders accept and refuse alike
    let mut a = alice_serial::bincode1::from_slice_prefix::<Zoo>(data, LIMIT);
    if matches!(
        a,
        Err(alice_serial::bincode1::Bincode1Error::TooDeep { .. })
    ) {
        // refused for depth alone: compare again with a depth that holds it, so the rest
        // of the decode is still checked against bincode
        compared.too_deep += 1;
        a = alice_serial::bincode1::from_slice_prefix_with_depth::<Zoo>(data, LIMIT, DEEP);
        assert!(
            !matches!(
                a,
                Err(alice_serial::bincode1::Bincode1Error::TooDeep { .. })
            ),
            "deeper than {DEEP} levels: raise DEEP"
        );
    }
    let b1 = bincode1_ref::deserialize::<Zoo>(data);
    let b2 = bincode2_ref::serde::decode_from_slice::<Zoo, _>(
        data,
        bincode2_ref::config::legacy().with_limit::<LIMIT>(),
    );
    assert_eq!(
        a.is_ok(),
        b1.is_ok(),
        "accept differs from bincode 1: ours {a:?} / bincode 1 {b1:?}"
    );
    assert_eq!(
        a.is_ok(),
        b2.is_ok(),
        "accept differs from bincode 2: ours {a:?} / bincode 2 {b2:?}"
    );
    match (a, b1, b2) {
        (Ok((va, used_a)), Ok(vb1), Ok((vb2, used_b2))) => {
            let bytes = ours(&va);
            assert_eq!(bytes, ours(&vb1), "decoded value differs from bincode 1");
            assert_eq!(bytes, ours(&vb2), "decoded value differs from bincode 2");
            assert_eq!(used_a, used_b2, "bytes used differ from bincode 2");
            // the value need not re-encode to its input: a map read with keys out of
            // order or repeated comes back sorted and deduplicated by BTreeMap, on all
            // three sides alike
            compared.accepted += 1;
        }
        _ => compared.refused += 1,
    }
    compared
}
