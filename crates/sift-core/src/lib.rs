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
