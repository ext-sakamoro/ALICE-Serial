//! varint.

use crate::errors::SerialError;
use alloc::vec::Vec;

// ---------------------------------------------------------------------------
// VarInt Encoding (LEB128)
// ---------------------------------------------------------------------------

/// `VarInt` エンコード (unsigned LEB128)
#[must_use]
pub fn encode_varint(mut value: u64) -> Vec<u8> {
    // a u64 takes at most 10 bytes of 7 bits, so the loop is bounded
    let mut buf = Vec::with_capacity(10);
    for _ in 0..10 {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if value == 0 {
            break;
        }
    }
    buf
}

/// `VarInt` デコード: the value and the number of bytes it took.
///
/// Only the shortest encoding of a value is accepted: a final byte of `0`
/// after the first byte, or bits past the 64th, are refused.
///
/// # Errors
///
/// [`SerialError::BufferTooShort`] when the bytes end inside the varint,
/// [`SerialError::NonCanonical`] for a longer form than the value needs, and
/// [`SerialError::TooLong`] for a value over `u64::MAX`.
pub fn decode_varint(data: &[u8]) -> Result<(u64, usize), SerialError> {
    let mut result: u64 = 0;
    for (i, &byte) in data.iter().enumerate() {
        let shift = 7 * i;
        let bits = u64::from(byte & 0x7F);
        if i == 9 && bits > 1 {
            return Err(SerialError::TooLong);
        }
        result |= bits << shift;
        if byte & 0x80 == 0 {
            if i > 0 && byte == 0 {
                return Err(SerialError::NonCanonical);
            }
            return Ok((result, i + 1));
        }
        if i == 9 {
            return Err(SerialError::TooLong);
        }
    }
    Err(SerialError::BufferTooShort)
}
