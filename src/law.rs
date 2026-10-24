//! law: the identifier of the encoding this build reads and writes.

/// The rules of format [`crate::FORMAT_VERSION`], as text.
pub const FORMAT_SPEC: &str = include_str!("format-1.txt");

/// SHA-256 of [`FORMAT_SPEC`]. A store that keeps bytes this crate wrote can
/// keep this identifier next to them and compare it with the reader's before
/// trusting the bytes to mean the same values.
pub const SERIAL_SEMANTICS_ID: [u8; 32] = [
    0x5d, 0xff, 0x99, 0xa2, 0x0f, 0x6a, 0x4c, 0xea, 0x01, 0x98, 0x66, 0xdd, 0x35, 0xd7, 0x49, 0xb8,
    0x68, 0x0c, 0x3e, 0xc9, 0x4f, 0x5e, 0x3e, 0x94, 0x00, 0x3e, 0x2e, 0x0b, 0x01, 0x06, 0xd2, 0xf6,
];
