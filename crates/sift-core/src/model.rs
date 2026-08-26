//! The data model owned by Rust. C never sees or owns these types.

use std::path::PathBuf;

use crate::language::Language;

/// One extracted declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    /// 1-based line number where the declaration starts.
    pub line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Function,
    Method,
    Struct,
    Class,
    Enum,
    Trait,
    Interface,
    Constant,
    Variable,
    Module,
}

impl SymbolKind {
    /// Long label used by `sift find` (`kind: function`).
    #[must_use]
    pub fn long_label(self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Method => "method",
            Self::Struct => "struct",
            Self::Class => "class",
            Self::Enum => "enum",
            Self::Trait => "trait",
            Self::Interface => "interface",
            Self::Constant => "constant",
            Self::Variable => "variable",
            Self::Module => "module",
        }
    }

    /// Short token used by `sift map` (`fn login`, `struct User`).
    #[must_use]
    pub fn short_label(self) -> &'static str {
        match self {
            Self::Function => "fn",
            Self::Method => "method",
            Self::Struct => "struct",
            Self::Class => "class",
            Self::Enum => "enum",
            Self::Trait => "trait",
            Self::Interface => "interface",
            Self::Constant => "const",
            Self::Variable => "var",
            Self::Module => "mod",
        }
    }
}

/// A scanned source file and its symbols.
///
/// `content_hash` is a non-cryptographic FNV-1a hash of the file bytes
/// computed through the native accelerator; it gives M0 cheap change
/// detection for later milestones.
#[derive(Debug, Clone)]
pub struct SourceFile {
    pub path: PathBuf,
    pub language: Language,
    pub symbols: Vec<Symbol>,
    /// FNV-1a 64-bit hash of the raw file content.
    pub content_hash: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_cover_every_kind() {
        let kinds = [
            SymbolKind::Function,
            SymbolKind::Method,
            SymbolKind::Struct,
            SymbolKind::Class,
            SymbolKind::Enum,
            SymbolKind::Trait,
            SymbolKind::Interface,
            SymbolKind::Constant,
            SymbolKind::Variable,
            SymbolKind::Module,
        ];
        for kind in kinds {
            assert!(!kind.long_label().is_empty());
            assert!(!kind.short_label().is_empty());
        }
        assert_eq!(SymbolKind::Function.long_label(), "function");
        assert_eq!(SymbolKind::Function.short_label(), "fn");
        assert_eq!(SymbolKind::Constant.short_label(), "const");
    }
}
