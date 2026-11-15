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
//! A decode also refuses nesting deeper than [`DEFAULT_MAX_DEPTH`] (or the depth
//! the caller names), so a few bytes per level cannot exhaust the stack.
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
    0x91, 0xeb, 0xb6, 0x07, 0x27, 0xbe, 0x3a, 0x0b, 0x63, 0x50, 0xb1, 0x7f, 0x75, 0x13, 0x3f, 0xf0,
    0x7a, 0x88, 0xf3, 0xe3, 0xd8, 0xdf, 0xa2, 0x11, 0xbf, 0xd2, 0x87, 0x84, 0x52, 0xe7, 0x56, 0xd2,
];

/// How deep a decode nests enums, `Some` values, sequences and maps before it
/// refuses the input ([`Bincode1Error::TooDeep`]).
///
/// Only these can make a type recurse; tuples, structs and newtypes have a fixed shape and
/// do not count. A caller whose data nests deeper names its own depth with
/// [`from_slice_with_depth`] and decodes on a thread whose stack fits it.
pub const DEFAULT_MAX_DEPTH: usize = 128;

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
    /// A sequence or map of `count` elements written in fewer than `count` bytes
    /// (zero-size elements): a decode, which needs a byte per element to bound the
    /// count, could not read it back, so it is not written.
    ZeroSizeElements {
        /// The element count.
        count: usize,
    },
    /// Enums, `Some` values, sequences and maps nested deeper than the decode's depth.
    TooDeep {
        /// The depth the decode allowed.
        max_depth: usize,
    },
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
            Self::ZeroSizeElements { count } => write!(
                f,
                "{count} elements written in fewer than {count} bytes could not be read back"
            ),
            Self::TooDeep { max_depth } => {
                write!(f, "nesting deeper than {max_depth} levels")
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
/// bytes are read and nesting at most [`DEFAULT_MAX_DEPTH`] deep.
///
/// # Errors
///
/// [`Bincode1Error::TrailingBytes`] when the value ends before the input does,
/// and the errors of [`from_slice_prefix`].
pub fn from_slice<'de, T: Deserialize<'de>>(
    bytes: &'de [u8],
    limit: usize,
) -> Result<T, Bincode1Error> {
    from_slice_with_depth(bytes, limit, DEFAULT_MAX_DEPTH)
}

/// [`from_slice`] with the nesting depth the caller names.
///
/// # Errors
///
/// As [`from_slice`], with [`Bincode1Error::TooDeep`] at `max_depth`.
pub fn from_slice_with_depth<'de, T: Deserialize<'de>>(
    bytes: &'de [u8],
    limit: usize,
    max_depth: usize,
) -> Result<T, Bincode1Error> {
    let (value, used) = from_slice_prefix_with_depth(bytes, limit, max_depth)?;
    if used != bytes.len() {
        return Err(Bincode1Error::TrailingBytes {
            used,
            len: bytes.len(),
        });
    }
    Ok(value)
}

/// The value at the start of `bytes` and how many bytes it used; at most
/// `limit` bytes are read and nesting at most [`DEFAULT_MAX_DEPTH`] deep. For
/// input that carries more data after the value.
///
/// # Errors
///
/// [`Bincode1Error::UnexpectedEnd`] when the input ends first,
/// [`Bincode1Error::LimitExceeded`] when the value needs bytes past `limit`,
/// [`Bincode1Error::Length`] for a length that does not fit the bytes that
/// remain, [`Bincode1Error::TooDeep`], the malformed-byte errors, or the error
/// the type raised.
pub fn from_slice_prefix<'de, T: Deserialize<'de>>(
    bytes: &'de [u8],
    limit: usize,
) -> Result<(T, usize), Bincode1Error> {
    from_slice_prefix_with_depth(bytes, limit, DEFAULT_MAX_DEPTH)
}

/// [`from_slice_prefix`] with the nesting depth the caller names.
///
/// # Errors
///
/// As [`from_slice_prefix`], with [`Bincode1Error::TooDeep`] at `max_depth`.
pub fn from_slice_prefix_with_depth<'de, T: Deserialize<'de>>(
    bytes: &'de [u8],
    limit: usize,
    max_depth: usize,
) -> Result<(T, usize), Bincode1Error> {
    let mut de = de::Deserializer::new(bytes, limit, max_depth);
    let value = T::deserialize(&mut de)?;
    Ok((value, de.position()))
}
