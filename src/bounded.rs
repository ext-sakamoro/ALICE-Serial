//! Lengths read from untrusted bytes, bounded before they size an allocation.
//!
//! A length or a count decoded from input is a claim about the input, not a size: allocating on
//! its word lets a few bytes ask for gigabytes. [`len_within`] accepts a length only when that many
//! elements fit in the input that remains, so an allocation sized by it never exceeds the bytes it
//! describes. [`len_at_most`] bounds a length read from a stream, where the rest of the input is not
//! known yet, by a limit the caller names. Both use checked arithmetic and return [`LenError`]; they
//! never panic.
//!
//! ```
//! use alice_serial::bounded::{len_within, LenErrorKind};
//!
//! // a count of 4-byte words, read from a header, with 12 bytes left after it
//! assert_eq!(len_within(3, 4, 12), Ok(3));
//! assert_eq!(len_within(4, 4, 12).unwrap_err().kind, LenErrorKind::Remaining);
//! assert_eq!(len_within(u64::MAX, 8, 12).unwrap_err().kind, LenErrorKind::Overflow);
//! ```

use core::fmt;

/// Why a length was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LenErrorKind {
    /// That many elements do not fit in the input that remains.
    Remaining,
    /// The length is over the limit the caller named.
    Limit,
    /// The length in bytes does not fit in `usize`.
    Overflow,
    /// The element size is zero, so the input cannot bound the count.
    ZeroElementSize,
}

/// A length refused by [`len_within`] or [`len_at_most`]: the length read, the bound it was
/// checked against (the remaining bytes or the limit) and why.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LenError {
    /// The length as it was read.
    pub len: u64,
    /// The remaining bytes ([`len_within`]) or the limit ([`len_at_most`]).
    pub bound: usize,
    /// Why it was refused.
    pub kind: LenErrorKind,
}

impl fmt::Display for LenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (len, bound) = (self.len, self.bound);
        match self.kind {
            LenErrorKind::Remaining => write!(
                f,
                "length {len} does not fit in the {bound} bytes that remain"
            ),
            LenErrorKind::Limit => write!(f, "length {len} is over the limit {bound}"),
            LenErrorKind::Overflow => write!(f, "length {len} overflows usize in bytes"),
            LenErrorKind::ZeroElementSize => {
                write!(f, "length {len} of zero-size elements cannot be bounded")
            }
        }
    }
}

impl core::error::Error for LenError {}

/// `len` as a count of `elem_size`-byte elements, when `len · elem_size ≤ remaining`.
///
/// # Errors
///
/// [`LenErrorKind::ZeroElementSize`] when `elem_size` is 0, [`LenErrorKind::Overflow`] when
/// `len · elem_size` does not fit in `usize`, and [`LenErrorKind::Remaining`] when it is more
/// than `remaining`.
pub fn len_within(len: u64, elem_size: usize, remaining: usize) -> Result<usize, LenError> {
    let refuse = |kind| LenError {
        len,
        bound: remaining,
        kind,
    };
    if elem_size == 0 {
        return Err(refuse(LenErrorKind::ZeroElementSize));
    }
    let count = usize::try_from(len).map_err(|_| refuse(LenErrorKind::Overflow))?;
    let bytes = count
        .checked_mul(elem_size)
        .ok_or_else(|| refuse(LenErrorKind::Overflow))?;
    if bytes > remaining {
        return Err(refuse(LenErrorKind::Remaining));
    }
    Ok(count)
}

/// `len` as a `usize`, when `len ≤ limit`: for a length read from a stream, bounded by a limit
/// the caller names because the rest of the input is not known yet.
///
/// # Errors
///
/// [`LenErrorKind::Limit`] when `len` is more than `limit`.
pub fn len_at_most(len: u64, limit: usize) -> Result<usize, LenError> {
    match usize::try_from(len) {
        Ok(n) if n <= limit => Ok(n),
        _ => Err(LenError {
            len,
            bound: limit,
            kind: LenErrorKind::Limit,
        }),
    }
}
