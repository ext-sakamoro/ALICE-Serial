//! encoder.

use crate::value::*;
use alloc::string::String;
use alloc::vec::Vec;

// MessagePack-like Encoder
// ---------------------------------------------------------------------------

// Type tags
const TAG_NULL: u8 = 0xC0;
const TAG_FALSE: u8 = 0xC2;
const TAG_TRUE: u8 = 0xC3;
const TAG_U64: u8 = 0xCF;
const TAG_I64: u8 = 0xD3;
const TAG_F64: u8 = 0xCB;
const TAG_STR: u8 = 0xDB;
const TAG_BIN: u8 = 0xC6;
const TAG_ARRAY: u8 = 0xDD;
const TAG_MAP: u8 = 0xDF;

#[must_use]
pub fn encode(value: &Value) -> Vec<u8> {
    let mut buf = Vec::new();
    encode_value(value, &mut buf);
    buf
}

#[allow(clippy::cast_possible_truncation)]
fn encode_value(value: &Value, buf: &mut Vec<u8>) {
    match value {
        Value::Null => buf.push(TAG_NULL),
        Value::Bool(false) => buf.push(TAG_FALSE),
        Value::Bool(true) => buf.push(TAG_TRUE),
        Value::U64(n) => {
            if *n <= 127 {
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
        Value::F64(f) => {
            buf.push(TAG_F64);
            buf.extend_from_slice(&f.to_be_bytes());
        }
        Value::Str(s) => {
            buf.push(TAG_STR);
            let bytes = s.as_bytes();
            buf.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
            buf.extend_from_slice(bytes);
        }
        Value::Bytes(b) => {
            buf.push(TAG_BIN);
            buf.extend_from_slice(&(b.len() as u32).to_be_bytes());
            buf.extend_from_slice(b);
        }
        Value::Array(arr) => {
            buf.push(TAG_ARRAY);
            buf.extend_from_slice(&(arr.len() as u32).to_be_bytes());
            for item in arr {
                encode_value(item, buf);
            }
        }
        Value::Map(m) => {
            buf.push(TAG_MAP);
            buf.extend_from_slice(&(m.len() as u32).to_be_bytes());
            for (k, v) in m {
                encode_value(k, buf);
                encode_value(v, buf);
            }
        }
    }
}

#[must_use]
pub fn decode(data: &[u8]) -> Option<(Value, usize)> {
    if data.is_empty() {
        return None;
    }
    let tag = data[0];
    match tag {
        TAG_NULL => Some((Value::Null, 1)),
        TAG_FALSE => Some((Value::Bool(false), 1)),
        TAG_TRUE => Some((Value::Bool(true), 1)),
        TAG_U64 => {
            if data.len() < 9 {
                return None;
            }
            let n = u64::from_be_bytes([
                data[1], data[2], data[3], data[4], data[5], data[6], data[7], data[8],
            ]);
            Some((Value::U64(n), 9))
        }
        TAG_I64 => {
            if data.len() < 9 {
                return None;
            }
            let n = i64::from_be_bytes([
                data[1], data[2], data[3], data[4], data[5], data[6], data[7], data[8],
            ]);
            Some((Value::I64(n), 9))
        }
        TAG_F64 => {
            if data.len() < 9 {
                return None;
            }
            let f = f64::from_be_bytes([
                data[1], data[2], data[3], data[4], data[5], data[6], data[7], data[8],
            ]);
            Some((Value::F64(f), 9))
        }
        TAG_STR => {
            if data.len() < 5 {
                return None;
            }
            let len = u32::from_be_bytes([data[1], data[2], data[3], data[4]]) as usize;
            if data.len() < 5 + len {
                return None;
            }
            let s = core::str::from_utf8(&data[5..5 + len]).ok()?;
            Some((Value::Str(String::from(s)), 5 + len))
        }
        TAG_BIN => {
            if data.len() < 5 {
                return None;
            }
            let len = u32::from_be_bytes([data[1], data[2], data[3], data[4]]) as usize;
            if data.len() < 5 + len {
                return None;
            }
            Some((Value::Bytes(data[5..5 + len].to_vec()), 5 + len))
        }
        TAG_ARRAY => {
            if data.len() < 5 {
                return None;
            }
            let count = u32::from_be_bytes([data[1], data[2], data[3], data[4]]) as usize;
            let mut offset = 5;
            let mut arr = Vec::new();
            for _ in 0..count {
                let (val, consumed) = decode(&data[offset..])?;
                arr.push(val);
                offset += consumed;
            }
            Some((Value::Array(arr), offset))
        }
        TAG_MAP => {
            if data.len() < 5 {
                return None;
            }
            let count = u32::from_be_bytes([data[1], data[2], data[3], data[4]]) as usize;
            let mut offset = 5;
            let mut map = Vec::new();
            for _ in 0..count {
                let (k, kc) = decode(&data[offset..])?;
                offset += kc;
                let (v, vc) = decode(&data[offset..])?;
                offset += vc;
                map.push((k, v));
            }
            Some((Value::Map(map), offset))
        }
        0..=127 => Some((Value::U64(u64::from(tag)), 1)), // positive fixint
        _ => None,
    }
}
