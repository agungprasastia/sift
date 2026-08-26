//! Core data model and language detection for Sift.
//!
//! Rust owns all long-lived data: [`SourceFile`], [`Symbol`] and
//! [`Language`] are the shared vocabulary of the whole workspace.

mod error;
mod language;
mod model;

pub use error::{Error, Result};
pub use language::Language;
pub use model::{SourceFile, Symbol, SymbolKind};

/// Non-cryptographic FNV-1a 64-bit hash implemented in pure safe Rust.
#[must_use]
pub fn fnv1a_hash(data: &[u8]) -> u64 {
    const BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    data.iter().fold(BASIS, |acc, &byte| {
        (acc ^ u64::from(byte)).wrapping_mul(PRIME)
    })
}
