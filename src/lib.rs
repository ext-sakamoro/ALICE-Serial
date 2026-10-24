//! ALICE-Serial: Binary serialization (VarInt/ZigZag/MessagePack-like).
//!
//! Every value has one encoding and the decoder accepts only that one, so the
//! bytes can be hashed or signed and compared. See [`FORMAT_SPEC`].
//!
//! ```
//! use alice_serial::{decode, encode, Value};
//!
//! let v = Value::Map(vec![
//!     (Value::Str("b".into()), Value::U64(2)),
//!     (Value::Str("a".into()), Value::U64(1)),
//! ]);
//! let bytes = encode(&v)?;
//! let back = decode(&bytes)?; // keys come back in canonical order
//! assert_eq!(encode(&back)?, bytes);
//! # Ok::<(), alice_serial::SerialError>(())
//! ```

#![no_std]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(
    clippy::module_name_repetitions,
    clippy::doc_markdown,
    clippy::wildcard_imports,
    clippy::too_many_lines,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::similar_names,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::return_self_not_must_use
)]

extern crate alloc;

pub mod bounded;
pub mod encoder;
pub mod errors;
pub mod law;
pub mod prelude;
pub mod value;
pub mod varint;
pub mod zigzag;

#[cfg(test)]
mod integration_tests;

pub use crate::encoder::*;
pub use crate::errors::*;
pub use crate::law::*;
pub use crate::value::*;
pub use crate::varint::*;
pub use crate::zigzag::*;
