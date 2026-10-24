//! errors.

// Error
// ---------------------------------------------------------------------------

/// Why an encode or a decode refused its input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SerialError {
    /// The bytes end before the value does.
    BufferTooShort,
    /// A tag byte this format does not define.
    InvalidTag(u8),
    /// A string is not UTF-8.
    Utf8Error,
    /// The bytes are not the one encoding of their value: an integer or a
    /// varint in a longer form than it needs.
    NonCanonical,
    /// Map keys are not in strictly ascending order of their encodings.
    UnsortedKeys,
    /// Two map entries have the same key.
    DuplicateKey,
    /// Arrays and maps are nested deeper than [`crate::MAX_DEPTH`].
    TooDeep,
    /// A string, byte string, array or map has more than `u32::MAX` items.
    TooLong,
    /// Bytes remain after the value.
    TrailingBytes,
    /// The bytes do not start with [`crate::MAGIC`].
    BadMagic,
    /// The format version is not [`crate::FORMAT_VERSION`].
    UnsupportedVersion(u16),
}

impl core::fmt::Display for SerialError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BufferTooShort => write!(f, "buffer too short"),
            Self::InvalidTag(t) => write!(f, "invalid tag {t:#04x}"),
            Self::Utf8Error => write!(f, "utf8 error"),
            Self::NonCanonical => write!(f, "value is not in its canonical encoding"),
            Self::UnsortedKeys => write!(f, "map keys are not in ascending order"),
            Self::DuplicateKey => write!(f, "map has a duplicate key"),
            Self::TooDeep => write!(f, "nesting is deeper than {}", crate::MAX_DEPTH),
            Self::TooLong => write!(f, "length does not fit in u32"),
            Self::TrailingBytes => write!(f, "bytes remain after the value"),
            Self::BadMagic => write!(f, "missing format magic"),
            Self::UnsupportedVersion(v) => {
                write!(f, "format version {v} is not {}", crate::FORMAT_VERSION)
            }
        }
    }
}

impl core::error::Error for SerialError {}
