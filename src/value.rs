//! value.

use alloc::string::String;
use alloc::vec::Vec;

// Binary Value
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    U64(u64),
    I64(i64),
    /// Carried bit-exact (NaN payloads and `-0.0` included).
    F32(f32),
    /// Carried bit-exact (NaN payloads and `-0.0` included).
    F64(f64),
    Bytes(Vec<u8>),
    /// Packed little-endian `f32` words, bit-exact.
    F32Array(Vec<f32>),
    /// Packed little-endian `f64` words, bit-exact.
    F64Array(Vec<f64>),
    /// Packed little-endian `u32` words.
    U32Array(Vec<u32>),
    /// Packed little-endian `u64` words.
    U64Array(Vec<u64>),
    /// Packed little-endian `i64` words.
    I64Array(Vec<i64>),
    Str(String),
    Array(Vec<Self>),
    Map(Vec<(Self, Self)>),
}
