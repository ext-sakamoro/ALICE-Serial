//! bincode1: the bincode 1 default layout, through serde.
//!
//! The bytes `bincode::serialize` (1.x) and `bincode::serde::encode_to_vec` with
//! `config::legacy()` (2.x) write: fixed-width little-endian integers, `u64`
//! lengths, a `u32` variant index, a `u8` tag for `Option` (the full rules are
//! [`BINCODE1_LAYOUT`]). Stores that already hold such bytes keep reading and
//! writing them unchanged.
//!
//! Every call takes a limit: an encode writes at most that many bytes and a
//! decode reads at most that many. A length or count read from the input is
//! checked against the bytes that remain ([`crate::bounded::len_within`])
//! before it sizes anything, so a few bytes cannot ask for a large allocation.
//!
//! ```
//! use alice_serial::bincode1::{from_slice, to_vec};
//!
//! let bytes = to_vec(&(7u16, "ab"), 1 << 20)?;
//! assert_eq!(bytes, [7, 0, 2, 0, 0, 0, 0, 0, 0, 0, b'a', b'b']);
//! let back: (u16, String) = from_slice(&bytes, 1 << 20)?;
//! assert_eq!(back, (7, "ab".to_string()));
//! # Ok::<(), alice_serial::bincode1::Bincode1Error>(())
//! ```

mod de;
mod ser;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

use serde::{Deserialize, Serialize};

use crate::bounded::LenError;

/// The rules of the layout, as text.
pub const BINCODE1_LAYOUT: &str = include_str!("../bincode1-layout.txt");

/// SHA-256 of [`BINCODE1_LAYOUT`]. A store that keeps bytes in this layout can
/// keep this identifier next to them and compare it with the reader's.
pub const BINCODE1_LAYOUT_ID: [u8; 32] = [
    0x63, 0x44, 0x79, 0x91, 0x6e, 0x68, 0xd2, 0x86, 0x27, 0xb8, 0x63, 0xfe, 0xdd, 0x01, 0x86, 0x8a,
    0xd2, 0xf1, 0x24, 0xc6, 0x90, 0x4d, 0x62, 0xd9, 0x64, 0x26, 0x96, 0x25, 0x93, 0x36, 0xe0, 0xe5,
];

/// Why an encode or a decode in this layout failed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Bincode1Error {
    /// The input ended before the value did.
    UnexpectedEnd,
    /// The value needs more bytes than the limit allows.
    LimitExceeded {
        /// The limit the caller gave.
        limit: usize,
    },
    /// [`from_slice`] decoded a value that did not use all of the input.
    TrailingBytes {
        /// The bytes the value used.
        used: usize,
        /// The length of the input.
        len: usize,
    },
    /// A `bool` byte other than 0 or 1.
    InvalidBool(u8),
    /// An `Option` tag other than 0 or 1.
    InvalidOptionTag(u8),
    /// A `char` whose bytes are not one valid UTF-8 scalar.
    InvalidChar,
    /// A string whose bytes are not valid UTF-8.
    InvalidUtf8,
    /// A length or count that does not fit in the bytes that remain.
    Length(LenError),
    /// A sequence or map written without a known length.
    SequenceMustHaveLength,
    /// A type that needs the input to describe itself (`deserialize_any`,
    /// `deserialize_identifier`, `deserialize_ignored_any`; untagged or
    /// internally tagged enums, `flatten`).
    DeserializeAnyNotSupported,
    /// An error raised by the type being encoded or decoded (an unknown variant
    /// index, a value its own checks refuse).
    Custom(String),
}

impl fmt::Display for Bincode1Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEnd => f.write_str("the input ended before the value"),
            Self::LimitExceeded { limit } => {
                write!(f, "the value needs more than the limit of {limit} bytes")
            }
            Self::TrailingBytes { used, len } => {
                write!(f, "the value used {used} of the {len} input bytes")
            }
            Self::InvalidBool(b) => write!(f, "bool byte {b} is neither 0 nor 1"),
            Self::InvalidOptionTag(t) => write!(f, "Option tag {t} is neither 0 nor 1"),
            Self::InvalidChar => f.write_str("char bytes are not one UTF-8 scalar"),
            Self::InvalidUtf8 => f.write_str("string bytes are not UTF-8"),
            Self::Length(e) => write!(f, "{e}"),
            Self::SequenceMustHaveLength => {
                f.write_str("a sequence or map needs a known length in this layout")
            }
            Self::DeserializeAnyNotSupported => {
                f.write_str("this layout does not describe itself (deserialize_any)")
            }
            Self::Custom(m) => f.write_str(m),
        }
    }
}

impl core::error::Error for Bincode1Error {}

impl serde::ser::Error for Bincode1Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::Custom(msg.to_string())
    }
}

impl serde::de::Error for Bincode1Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::Custom(msg.to_string())
    }
}

impl From<LenError> for Bincode1Error {
    fn from(e: LenError) -> Self {
        Self::Length(e)
    }
}

/// `value` in this layout, at most `limit` bytes.
///
/// # Errors
///
/// [`Bincode1Error::LimitExceeded`] when the bytes would pass `limit`,
/// [`Bincode1Error::SequenceMustHaveLength`] for a sequence or map of unknown
/// length, or the error the type raised.
pub fn to_vec<T: Serialize + ?Sized>(value: &T, limit: usize) -> Result<Vec<u8>, Bincode1Error> {
    let mut out = Vec::new();
    encode_into(&mut out, value, limit)?;
    Ok(out)
}

/// Append `value` in this layout to `out`, at most `limit` bytes; returns how
/// many bytes were appended. On an error `out` is left as it was.
///
/// # Errors
///
/// As [`to_vec`].
pub fn encode_into<T: Serialize + ?Sized>(
    out: &mut Vec<u8>,
    value: &T,
    limit: usize,
) -> Result<usize, Bincode1Error> {
    let start = out.len();
    let mut ser = ser::Serializer::new(out, limit);
    match value.serialize(&mut ser) {
        Ok(()) => Ok(out.len() - start),
        Err(e) => {
            out.truncate(start);
            Err(e)
        }
    }
}

/// The value `bytes` holds, which must be all of `bytes`; at most `limit`
/// bytes are read.
///
/// # Errors
///
/// [`Bincode1Error::TrailingBytes`] when the value ends before the input does,
/// and the errors of [`from_slice_prefix`].
pub fn from_slice<'de, T: Deserialize<'de>>(
    bytes: &'de [u8],
    limit: usize,
) -> Result<T, Bincode1Error> {
    let (value, used) = from_slice_prefix(bytes, limit)?;
    if used != bytes.len() {
        return Err(Bincode1Error::TrailingBytes {
            used,
            len: bytes.len(),
        });
    }
    Ok(value)
}

/// The value at the start of `bytes` and how many bytes it used; at most
/// `limit` bytes are read. For input that carries more data after the value.
///
/// # Errors
///
/// [`Bincode1Error::UnexpectedEnd`] when the input ends first,
/// [`Bincode1Error::LimitExceeded`] when the value needs bytes past `limit`,
/// [`Bincode1Error::Length`] for a length that does not fit the bytes that
/// remain, the malformed-byte errors, or the error the type raised.
pub fn from_slice_prefix<'de, T: Deserialize<'de>>(
    bytes: &'de [u8],
    limit: usize,
) -> Result<(T, usize), Bincode1Error> {
    let mut de = de::Deserializer::new(bytes, limit);
    let value = T::deserialize(&mut de)?;
    Ok((value, de.position()))
}
