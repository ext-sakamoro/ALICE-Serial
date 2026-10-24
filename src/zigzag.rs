//! zigzag.

// ZigZag Encoding (signed → unsigned)
// ---------------------------------------------------------------------------

#[must_use]
#[allow(clippy::cast_sign_loss)]
pub const fn zigzag_encode(n: i64) -> u64 {
    ((n << 1) ^ (n >> 63)) as u64
}

#[must_use]
#[allow(clippy::cast_possible_wrap)]
pub const fn zigzag_decode(n: u64) -> i64 {
    ((n >> 1) as i64) ^ -((n & 1) as i64)
}
