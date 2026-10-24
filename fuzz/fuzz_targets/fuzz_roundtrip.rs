#![no_main]

use alice_serial::{decode, encode, SerialError, Value, MAX_DEPTH};
use libfuzzer_sys::fuzz_target;

/// A value read off the fuzz bytes, one choice per byte, nesting bounded a
/// little past `MAX_DEPTH` so the encoder's bound is reached too.
fn build(data: &mut &[u8], depth: usize) -> Value {
    let Some((&c, rest)) = data.split_first() else {
        return Value::Null;
    };
    *data = rest;
    let mut word = || {
        let mut w = [0u8; 8];
        let n = data.len().min(8);
        w[..n].copy_from_slice(&data[..n]);
        *data = &data[n..];
        w
    };
    let small = |data: &mut &[u8]| -> usize {
        let Some((&n, rest)) = data.split_first() else {
            return 0;
        };
        *data = rest;
        usize::from(n % 5)
    };
    match c % 15 {
        0 => Value::Null,
        1 => Value::Bool(c & 0x10 != 0),
        2 => Value::U64(u64::from_le_bytes(word())),
        3 => Value::I64(i64::from_le_bytes(word())),
        4 => Value::F64(f64::from_le_bytes(word())),
        5 => {
            let n = small(data);
            let (b, rest) = data.split_at(n.min(data.len()));
            *data = rest;
            Value::Bytes(b.to_vec())
        }
        6 => {
            let n = small(data);
            let (b, rest) = data.split_at(n.min(data.len()));
            *data = rest;
            Value::Str(String::from_utf8_lossy(b).into_owned())
        }
        7 if depth <= MAX_DEPTH + 2 => {
            let n = small(data);
            Value::Array((0..n).map(|_| build(data, depth + 1)).collect())
        }
        8 if depth <= MAX_DEPTH + 2 => {
            let n = small(data);
            Value::Map(
                (0..n)
                    .map(|_| (build(data, depth + 1), build(data, depth + 1)))
                    .collect(),
            )
        }
        9 => Value::F32(f32::from_bits(u32::from_le_bytes(word()[..4].try_into().unwrap()))),
        10 => Value::F32Array(words(data, |w| f32::from_bits(u32::from_le_bytes(w[..4].try_into().unwrap())))),
        11 => Value::F64Array(words(data, |w| f64::from_bits(u64::from_le_bytes(w)))),
        12 => Value::U32Array(words(data, |w| u32::from_le_bytes(w[..4].try_into().unwrap()))),
        13 => Value::U64Array(words(data, u64::from_le_bytes)),
        14 => Value::I64Array(words(data, i64::from_le_bytes)),
        _ => Value::Null,
    }
}

/// Up to 4 words of 8 fuzz bytes each, mapped by `f`.
fn words<T>(data: &mut &[u8], f: impl Fn([u8; 8]) -> T) -> Vec<T> {
    let Some((&n, rest)) = data.split_first() else {
        return Vec::new();
    };
    *data = rest;
    (0..n % 5)
        .map(|_| {
            let mut w = [0u8; 8];
            let k = data.len().min(8);
            w[..k].copy_from_slice(&data[..k]);
            *data = &data[k..];
            f(w)
        })
        .collect()
}

fuzz_target!(|data: &[u8]| {
    let mut input = data;
    let v = build(&mut input, 0);
    match encode(&v) {
        Ok(bytes) => {
            let back = decode(&bytes).expect("an encoded value decodes");
            let again = encode(&back).expect("a decoded value encodes");
            assert_eq!(again, bytes, "encode(decode(encode(v))) != encode(v)");
        }
        Err(SerialError::TooDeep | SerialError::DuplicateKey) => {}
        Err(e) => panic!("unexpected encode error {e:?}"),
    }
});
