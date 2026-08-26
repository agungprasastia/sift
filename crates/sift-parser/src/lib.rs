//! Tree-sitter based modular symbol extraction.
//!
//! Each supported language lives in its own module under [`lang`]; adding a
//! language means adding one module plus one match arm here and one variant
//! in `sift_core::Language`. No core architecture changes required.

mod engine;
mod lang;
mod util;

use sift_core::{Language, Symbol};

/// Extract top-level and nested declarations from `source`.
///
/// `source` must be valid UTF-8 (the scanner guarantees this); parsing never
/// fails hard — malformed code simply yields whatever declarations are
/// recognizable.
#[must_use]
pub fn extract_symbols(language: Language, source: &[u8]) -> Vec<Symbol> {
    match language {
        Language::Rust => lang::rust::extract(source),
        Language::C => lang::c::extract(source),
        Language::Cpp => lang::cpp::extract(source),
        Language::Go => lang::go::extract(source),
        Language::JavaScript => lang::javascript::extract(source),
        Language::TypeScript => lang::typescript::extract(source),
        Language::Python => lang::python::extract(source),
    }
}
