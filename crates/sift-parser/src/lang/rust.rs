//! Rust symbol extraction (`tree-sitter-rust`).

use sift_core::{Symbol, SymbolKind};
use tree_sitter::Node;

use crate::engine::{self};
use crate::util;

pub(crate) fn extract(source: &[u8]) -> Vec<Symbol> {
    let Some(tree) = util::parse(tree_sitter_rust::LANGUAGE.into(), source) else {
        return Vec::new();
    };
    engine::collect(tree.root_node(), source, classify)
}

fn classify(node: Node<'_>, source: &[u8]) -> Vec<(String, SymbolKind)> {
    match node.kind() {
        "function_item" | "function_signature_item" => engine::one(
            util::field_text(node, source, "name"),
            if util::ancestors_contain(node, &["impl_item", "trait_item"]) {
                SymbolKind::Method
            } else {
                SymbolKind::Function
            },
        ),
        "struct_item" => engine::one(util::field_text(node, source, "name"), SymbolKind::Struct),
        "enum_item" => engine::one(util::field_text(node, source, "name"), SymbolKind::Enum),
        "trait_item" => engine::one(util::field_text(node, source, "name"), SymbolKind::Trait),
        "const_item" | "static_item" => {
            engine::one(util::field_text(node, source, "name"), SymbolKind::Constant)
        }
        "mod_item" => engine::one(util::field_text(node, source, "name"), SymbolKind::Module),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract_symbols;
    use sift_core::Language;

    #[test]
    fn extracts_rust_symbols() {
        let source = r#"
mod inner {
    pub fn helper() {}
}

pub struct User { pub id: u32 }

impl User {
    pub fn new() -> Self { Self { id: 0 } }
}

pub enum Color { Red }

pub trait Visitor {
    fn visit(&mut self);
}

pub const MAX: u32 = 10;

pub static NAME: &str = "x";
"#;
        let symbols = extract_symbols(Language::Rust, source.as_bytes());
        let found: Vec<(String, SymbolKind)> =
            symbols.iter().map(|s| (s.name.clone(), s.kind)).collect();

        assert_eq!(
            found,
            vec![
                ("inner".to_string(), SymbolKind::Module),
                ("helper".to_string(), SymbolKind::Function),
                ("User".to_string(), SymbolKind::Struct),
                ("new".to_string(), SymbolKind::Method),
                ("Color".to_string(), SymbolKind::Enum),
                ("Visitor".to_string(), SymbolKind::Trait),
                ("visit".to_string(), SymbolKind::Method),
                ("MAX".to_string(), SymbolKind::Constant),
                ("NAME".to_string(), SymbolKind::Constant),
            ]
        );
    }

    #[test]
    fn reports_one_based_line_numbers() {
        let source = "fn alpha() {\n}\n\nfn beta() {\n}\n";
        let symbols = extract_symbols(Language::Rust, source.as_bytes());
        let lines: Vec<usize> = symbols.iter().map(|s| s.line).collect();
        assert_eq!(lines, vec![1, 4]);
    }
}
