//! varint.

use alloc::vec::Vec;

// ---------------------------------------------------------------------------
// VarInt Encoding (LEB128)
// ---------------------------------------------------------------------------

/// `VarInt` エンコード (unsigned LEB128)
#[must_use]
pub fn encode_varint(mut value: u64) -> Vec<u8> {
    let mut buf = Vec::new();
    loop {
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

/// `VarInt` デコード
#[must_use]
pub fn decode_varint(data: &[u8]) -> Option<(u64, usize)> {
    let mut result: u64 = 0;
    let mut shift = 0;
    for (i, &byte) in data.iter().enumerate() {
        result |= (u64::from(byte & 0x7F)) << shift;
        if byte & 0x80 == 0 {
            return Some((result, i + 1));
        }
        shift += 7;
        if shift > 63 {
            return None;
        }
    }
    None
}
