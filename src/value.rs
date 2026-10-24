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
    F64(f64),
    Bytes(Vec<u8>),
    Str(String),
    Array(Vec<Self>),
    Map(Vec<(Self, Self)>),
}
