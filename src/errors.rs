//! errors.

// Error
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SerialError {
    InvalidFormat,
    BufferTooShort,
    Utf8Error,
}

impl core::fmt::Display for SerialError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidFormat => write!(f, "invalid format"),
            Self::BufferTooShort => write!(f, "buffer too short"),
            Self::Utf8Error => write!(f, "utf8 error"),
        }
    }
}
