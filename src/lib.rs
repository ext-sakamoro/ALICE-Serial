//! ALICE-Serial: Binary serialization (VarInt/ZigZag/MessagePack-like).

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

pub mod encoder;
pub mod errors;
pub mod prelude;
pub mod value;
pub mod varint;
pub mod zigzag;

#[cfg(test)]
mod integration_tests;

pub use crate::encoder::*;
pub use crate::errors::*;
pub use crate::value::*;
pub use crate::varint::*;
pub use crate::zigzag::*;
