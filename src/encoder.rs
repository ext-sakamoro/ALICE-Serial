//! encoder.
//!
//! Every [`Value`] has exactly one encoding, and the decoder accepts only that
//! one: integers in their shortest form, map keys in strictly ascending order
//! of their encodings with no duplicates, nesting no
//! deeper than [`MAX_DEPTH`], and (for [`decode`] and [`decode_value`]'s
//! callers that want it) no bytes after the value. So
//! `encode(decode(b)?) == b` for every `b` that decodes.

use crate::errors::SerialError;
use crate::value::*;
use alloc::string::String;
use alloc::vec::Vec;

// Format header
// ---------------------------------------------------------------------------

/// First four bytes of an [`encode`] byte string.
pub const MAGIC: [u8; 4] = *b"ALSR";

/// Format version written after [`MAGIC`] (little-endian `u16`).
pub const FORMAT_VERSION: u16 = 1;

/// Deepest nesting of arrays and maps the encoder writes and the decoder reads.
/// A scalar at the top is depth 0; each array or map adds 1.
pub const MAX_DEPTH: usize = 64;

const HEADER_LEN: usize = MAGIC.len() + 2;

// MessagePack-like Encoder
// ---------------------------------------------------------------------------

// Type tags
const TAG_NULL: u8 = 0xC0;
const TAG_FALSE: u8 = 0xC2;
const TAG_TRUE: u8 = 0xC3;
const TAG_U64: u8 = 0xCF;
const TAG_I64: u8 = 0xD3;
const TAG_F32: u8 = 0xCA;
const TAG_F64: u8 = 0xCB;
const TAG_F32_ARRAY: u8 = 0xD4;
const TAG_F64_ARRAY: u8 = 0xD5;
const TAG_U32_ARRAY: u8 = 0xD6;
const TAG_U64_ARRAY: u8 = 0xD7;
const TAG_I64_ARRAY: u8 = 0xD8;
const TAG_STR: u8 = 0xDB;
const TAG_BIN: u8 = 0xC6;
const TAG_ARRAY: u8 = 0xDD;
const TAG_MAP: u8 = 0xDF;

/// Largest `u64` written as a one-byte positive fixint.
const FIXINT_MAX: u64 = 0x7F;

/// [`MAGIC`], [`FORMAT_VERSION`], then the value.
///
/// # Errors
///
/// [`SerialError::TooLong`], [`SerialError::TooDeep`] or
/// [`SerialError::DuplicateKey`], as for [`encode_value`].
pub fn encode(value: &Value) -> Result<Vec<u8>, SerialError> {
    let mut buf = Vec::with_capacity(HEADER_LEN);
    buf.extend_from_slice(&MAGIC);
    buf.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    encode_into(value, 0, &mut buf)?;
    Ok(buf)
}

/// The value alone, without the header, for a container that carries its own.
///
/// Map entries are written in ascending order of their encoded keys, whatever
/// their order in the [`Value::Map`]. Floats are written bit-exact.
///
/// # Errors
///
/// [`SerialError::TooLong`] for a length over `u32::MAX`,
/// [`SerialError::TooDeep`] past [`MAX_DEPTH`], and
/// [`SerialError::DuplicateKey`] for a map with two equal keys.
pub fn encode_value(value: &Value) -> Result<Vec<u8>, SerialError> {
    let mut buf = Vec::new();
    encode_into(value, 0, &mut buf)?;
    Ok(buf)
}

fn put_len(buf: &mut Vec<u8>, tag: u8, len: usize) -> Result<(), SerialError> {
    let len = u32::try_from(len).map_err(|_| SerialError::TooLong)?;
    buf.push(tag);
    buf.extend_from_slice(&len.to_be_bytes());
    Ok(())
}

fn encode_into(value: &Value, depth: usize, buf: &mut Vec<u8>) -> Result<(), SerialError> {
    match value {
        Value::Null => buf.push(TAG_NULL),
        Value::Bool(false) => buf.push(TAG_FALSE),
        Value::Bool(true) => buf.push(TAG_TRUE),
        Value::U64(n) => {
            if *n <= FIXINT_MAX {
                buf.push(*n as u8); // positive fixint
            } else {
                buf.push(TAG_U64);
                buf.extend_from_slice(&n.to_be_bytes());
            }
        }
        Value::I64(n) => {
            buf.push(TAG_I64);
            buf.extend_from_slice(&n.to_be_bytes());
        }
        Value::F32(f) => {
            buf.push(TAG_F32);
            buf.extend_from_slice(&f.to_bits().to_be_bytes());
        }
        Value::F64(f) => {
            buf.push(TAG_F64);
            buf.extend_from_slice(&f.to_bits().to_be_bytes());
        }
        Value::F32Array(a) => {
            put_len(buf, TAG_F32_ARRAY, a.len())?;
            for x in a {
                buf.extend_from_slice(&x.to_bits().to_le_bytes());
            }
        }
        Value::F64Array(a) => {
            put_len(buf, TAG_F64_ARRAY, a.len())?;
            for x in a {
                buf.extend_from_slice(&x.to_bits().to_le_bytes());
            }
        }
        Value::U32Array(a) => {
            put_len(buf, TAG_U32_ARRAY, a.len())?;
            for x in a {
                buf.extend_from_slice(&x.to_le_bytes());
            }
        }
        Value::U64Array(a) => {
            put_len(buf, TAG_U64_ARRAY, a.len())?;
            for x in a {
                buf.extend_from_slice(&x.to_le_bytes());
            }
        }
        Value::I64Array(a) => {
            put_len(buf, TAG_I64_ARRAY, a.len())?;
            for x in a {
                buf.extend_from_slice(&x.to_le_bytes());
            }
        }
        Value::Str(s) => {
            put_len(buf, TAG_STR, s.len())?;
            buf.extend_from_slice(s.as_bytes());
        }
        Value::Bytes(b) => {
            put_len(buf, TAG_BIN, b.len())?;
            buf.extend_from_slice(b);
        }
        Value::Array(arr) => {
            if depth >= MAX_DEPTH {
                return Err(SerialError::TooDeep);
            }
            put_len(buf, TAG_ARRAY, arr.len())?;
            for item in arr {
                encode_into(item, depth + 1, buf)?;
            }
        }
        Value::Map(m) => {
            if depth >= MAX_DEPTH {
                return Err(SerialError::TooDeep);
            }
            put_len(buf, TAG_MAP, m.len())?;
            let mut entries = Vec::with_capacity(m.len());
            for (k, v) in m {
                let mut key = Vec::new();
                encode_into(k, depth + 1, &mut key)?;
                entries.push((key, v));
            }
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            if entries.windows(2).any(|w| w[0].0 == w[1].0) {
                return Err(SerialError::DuplicateKey);
            }
            for (key, v) in entries {
                buf.extend_from_slice(&key);
                encode_into(v, depth + 1, buf)?;
            }
        }
    }
    Ok(())
}

/// The value of an [`encode`] byte string: the header, then exactly one value
/// in its canonical encoding, and nothing after it.
///
/// # Errors
///
/// [`SerialError::BadMagic`] or [`SerialError::UnsupportedVersion`] for the
/// header, [`SerialError::TrailingBytes`] for bytes after the value, and
/// anything [`decode_value`] refuses.
pub fn decode(data: &[u8]) -> Result<Value, SerialError> {
    if data.len() < HEADER_LEN {
        return Err(SerialError::BufferTooShort);
    }
    if data[..MAGIC.len()] != MAGIC {
        return Err(SerialError::BadMagic);
    }
    let version = u16::from_le_bytes([data[4], data[5]]);
    if version != FORMAT_VERSION {
        return Err(SerialError::UnsupportedVersion(version));
    }
    let body = &data[HEADER_LEN..];
    let (value, used) = decode_value(body)?;
    if used != body.len() {
        return Err(SerialError::TrailingBytes);
    }
    Ok(value)
}

/// One value without a header from the front of `data`, and the number of
/// bytes it took. Bytes after it are left to the caller.
///
/// # Errors
///
/// [`SerialError::BufferTooShort`], [`SerialError::InvalidTag`],
/// [`SerialError::Utf8Error`], [`SerialError::NonCanonical`],
/// [`SerialError::UnsortedKeys`], [`SerialError::DuplicateKey`] or
/// [`SerialError::TooDeep`].
pub fn decode_value(data: &[u8]) -> Result<(Value, usize), SerialError> {
    let mut r = Reader { data, at: 0 };
    let value = r.value(0)?;
    Ok((value, r.at))
}

const fn le4(c: &[u8]) -> [u8; 4] {
    let mut w = [0u8; 4];
    w.copy_from_slice(c);
    w
}

const fn le8(c: &[u8]) -> [u8; 8] {
    let mut w = [0u8; 8];
    w.copy_from_slice(c);
    w
}

struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], SerialError> {
        if n > self.data.len() - self.at {
            return Err(SerialError::BufferTooShort);
        }
        let out = &self.data[self.at..self.at + n];
        self.at += n;
        Ok(out)
    }

    fn word(&mut self) -> Result<[u8; 8], SerialError> {
        let mut w = [0u8; 8];
        w.copy_from_slice(self.take(8)?);
        Ok(w)
    }

    fn len(&mut self) -> Result<usize, SerialError> {
        let mut w = [0u8; 4];
        w.copy_from_slice(self.take(4)?);
        Ok(u32::from_be_bytes(w) as usize)
    }

    /// `n` words of `width` bytes each, `n` read from the length prefix; the
    /// byte length is checked against the bytes left before anything is sized.
    fn packed<T>(
        &mut self,
        width: usize,
        word: impl Fn(&[u8]) -> T,
    ) -> Result<Vec<T>, SerialError> {
        let n = self.len()?;
        let bytes = n.checked_mul(width).ok_or(SerialError::BufferTooShort)?;
        Ok(self.take(bytes)?.chunks_exact(width).map(word).collect())
    }

    fn value(&mut self, depth: usize) -> Result<Value, SerialError> {
        let tag = self.take(1)?[0];
        match tag {
            TAG_NULL => Ok(Value::Null),
            TAG_FALSE => Ok(Value::Bool(false)),
            TAG_TRUE => Ok(Value::Bool(true)),
            TAG_U64 => {
                let n = u64::from_be_bytes(self.word()?);
                if n <= FIXINT_MAX {
                    return Err(SerialError::NonCanonical);
                }
                Ok(Value::U64(n))
            }
            TAG_I64 => Ok(Value::I64(i64::from_be_bytes(self.word()?))),
            TAG_F32 => {
                let mut w = [0u8; 4];
                w.copy_from_slice(self.take(4)?);
                Ok(Value::F32(f32::from_bits(u32::from_be_bytes(w))))
            }
            TAG_F64 => Ok(Value::F64(f64::from_bits(u64::from_be_bytes(self.word()?)))),
            TAG_F32_ARRAY => {
                Ok(Value::F32Array(self.packed(4, |c| {
                    f32::from_bits(u32::from_le_bytes(le4(c)))
                })?))
            }
            TAG_F64_ARRAY => {
                Ok(Value::F64Array(self.packed(8, |c| {
                    f64::from_bits(u64::from_le_bytes(le8(c)))
                })?))
            }
            TAG_U32_ARRAY => Ok(Value::U32Array(
                self.packed(4, |c| u32::from_le_bytes(le4(c)))?,
            )),
            TAG_U64_ARRAY => Ok(Value::U64Array(
                self.packed(8, |c| u64::from_le_bytes(le8(c)))?,
            )),
            TAG_I64_ARRAY => Ok(Value::I64Array(
                self.packed(8, |c| i64::from_le_bytes(le8(c)))?,
            )),
            TAG_STR => {
                let len = self.len()?;
                let s =
                    core::str::from_utf8(self.take(len)?).map_err(|_| SerialError::Utf8Error)?;
                Ok(Value::Str(String::from(s)))
            }
            TAG_BIN => {
                let len = self.len()?;
                Ok(Value::Bytes(self.take(len)?.to_vec()))
            }
            TAG_ARRAY => {
                if depth >= MAX_DEPTH {
                    return Err(SerialError::TooDeep);
                }
                let count = self.len()?;
                // grown per decoded item, never sized from `count`: counts nested in
                // each other would multiply memory, while every item takes a byte
                let mut arr = Vec::new();
                for _ in 0..count {
                    arr.push(self.value(depth + 1)?);
                }
                Ok(Value::Array(arr))
            }
            TAG_MAP => {
                if depth >= MAX_DEPTH {
                    return Err(SerialError::TooDeep);
                }
                let count = self.len()?;
                // grown per decoded item, never sized from `count`: counts nested in
                // each other would multiply memory, while every item takes a byte
                let mut map = Vec::new();
                let mut prev: Option<&[u8]> = None;
                for _ in 0..count {
                    let start = self.at;
                    let k = self.value(depth + 1)?;
                    let key = &self.data[start..self.at];
                    if let Some(p) = prev {
                        match p.cmp(key) {
                            core::cmp::Ordering::Less => {}
                            core::cmp::Ordering::Equal => return Err(SerialError::DuplicateKey),
                            core::cmp::Ordering::Greater => return Err(SerialError::UnsortedKeys),
                        }
                    }
                    prev = Some(key);
                    let v = self.value(depth + 1)?;
                    map.push((k, v));
                }
                Ok(Value::Map(map))
            }
            0..=0x7F => Ok(Value::U64(u64::from(tag))), // positive fixint
            _ => Err(SerialError::InvalidTag(tag)),
        }
    }
}
