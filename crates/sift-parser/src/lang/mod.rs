//! Per-language extractors. Adding a language = one module here + one match
//! arm in `crate::extract_symbols`.

pub(super) mod c;
pub(super) mod cpp;
pub(super) mod go;
pub(super) mod javascript;
pub(super) mod python;
pub(super) mod rust;
pub(super) mod typescript;
