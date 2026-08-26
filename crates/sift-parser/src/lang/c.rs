//! C symbol extraction (`tree-sitter-c`).

use sift_core::{Symbol, SymbolKind};
use tree_sitter::Node;

use crate::engine::{self};
use crate::util;

pub(crate) fn extract(source: &[u8]) -> Vec<Symbol> {
    let Some(tree) = util::parse(tree_sitter_c::LANGUAGE.into(), source) else {
        return Vec::new();
    };
    engine::collect(tree.root_node(), source, classify)
}

pub(super) fn classify(node: Node<'_>, source: &[u8]) -> Vec<(String, SymbolKind)> {
    match node.kind() {
        "function_definition" => {
            let name = node
                .child_by_field_name("declarator")
                .and_then(|decl| util::declarator_name(decl, source));
            engine::one(name, SymbolKind::Function)
        }
        "struct_specifier" if named_with_body(node) => {
            engine::one(util::field_text(node, source, "name"), SymbolKind::Struct)
        }
        "enum_specifier" if named_with_body(node) => {
            engine::one(util::field_text(node, source, "name"), SymbolKind::Enum)
        }
        "preproc_def" | "preproc_function_def" => {
            engine::one(util::field_text(node, source, "name"), SymbolKind::Constant)
        }
        "declaration" => {
            let mut out = Vec::new();
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if (child.kind() == "function_declarator" || util::is_function_declarator(child))
                    && let Some(name) = util::declarator_name(child, source)
                {
                    out.push((name, SymbolKind::Function));
                }
            }
            if !out.is_empty() {
                out
            } else if node
                .parent()
                .is_some_and(|p| p.kind() == "translation_unit")
            {
                const_qualified_declarations(node, source)
            } else {
                Vec::new()
            }
        }
        _ => Vec::new(),
    }
}

fn named_with_body(node: Node<'_>) -> bool {
    node.child_by_field_name("name").is_some() && node.child_by_field_name("body").is_some()
}

/// Top-level `const` declarations (`const int limit = 5;`) become constants.
fn const_qualified_declarations(node: Node<'_>, source: &[u8]) -> Vec<(String, SymbolKind)> {
    let mut cursor = node.walk();
    let is_const_qualified = node.children(&mut cursor).any(|child| {
        child.kind() == "type_qualifier" && child.utf8_text(source).is_ok_and(|t| t == "const")
    });
    if !is_const_qualified {
        return Vec::new();
    }
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .filter(|child| child.kind() == "init_declarator")
        .filter_map(|decl| util::declarator_name(decl, source))
        .map(|name| (name, SymbolKind::Constant))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract_symbols;
    use sift_core::Language;

    #[test]
    fn extracts_c_symbols() {
        let source = r#"#include <stdlib.h>

struct Point {
    int x;
};

enum Color { RED };

int area(struct Point p) {
    return p.x;
}

#define MAX_POINTS 128

const int limit = 5;
"#;
        let symbols = extract_symbols(Language::C, source.as_bytes());
        let found: Vec<(String, SymbolKind)> =
            symbols.iter().map(|s| (s.name.clone(), s.kind)).collect();

        assert_eq!(
            found,
            vec![
                ("Point".to_string(), SymbolKind::Struct),
                ("Color".to_string(), SymbolKind::Enum),
                ("area".to_string(), SymbolKind::Function),
                ("MAX_POINTS".to_string(), SymbolKind::Constant),
                ("limit".to_string(), SymbolKind::Constant),
            ]
        );
    }
}
