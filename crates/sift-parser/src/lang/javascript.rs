//! JavaScript symbol extraction (`tree-sitter-javascript`).

use sift_core::{Symbol, SymbolKind};
use tree_sitter::Node;

use crate::engine::{self};
use crate::util;

pub(crate) fn extract(source: &[u8]) -> Vec<Symbol> {
    let Some(tree) = util::parse(tree_sitter_javascript::LANGUAGE.into(), source) else {
        return Vec::new();
    };
    engine::collect(tree.root_node(), source, classify)
}

pub(super) fn classify(node: Node<'_>, source: &[u8]) -> Vec<(String, SymbolKind)> {
    match node.kind() {
        "function_declaration" | "generator_function_declaration" => {
            engine::one(util::field_text(node, source, "name"), SymbolKind::Function)
        }
        "class_declaration" => {
            engine::one(util::field_text(node, source, "name"), SymbolKind::Class)
        }
        "method_definition" => {
            let name = util::field_text(node, source, "name")
                .map(|n| n.trim_matches('"').trim_matches('\'').to_string());
            engine::one(name, SymbolKind::Method)
        }
        "variable_declaration" | "lexical_declaration" => variable_declarators(node, source),
        _ => Vec::new(),
    }
}

/// Expands `const LIMIT = 10;` / `var x = 1;` into one symbol per declared
/// identifier. `const` bindings become constants, the rest variables.
fn variable_declarators(node: Node<'_>, source: &[u8]) -> Vec<(String, SymbolKind)> {
    let mut cursor = node.walk();
    let is_const = node
        .children(&mut cursor)
        .any(|child| child.kind() == "const");
    let kind = if is_const {
        SymbolKind::Constant
    } else {
        SymbolKind::Variable
    };

    let mut cursor = node.walk();
    node.children(&mut cursor)
        .filter(|child| child.kind() == "variable_declarator")
        .filter_map(|decl| decl.child_by_field_name("name"))
        .filter(|name_node| name_node.kind() == "identifier")
        .filter_map(|name_node| name_node.utf8_text(source).ok().map(str::to_string))
        .map(|name| (name, kind))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract_symbols;
    use sift_core::Language;

    #[test]
    fn extracts_javascript_symbols() {
        let source = r#"export function fmt(v) {
  return v;
}

async function go() {}

const LIMIT = 10;

let counter = 0;

class Cache {
  get(k) { return k; }
}
"#;
        let symbols = extract_symbols(Language::JavaScript, source.as_bytes());
        let found: Vec<(String, SymbolKind)> =
            symbols.iter().map(|s| (s.name.clone(), s.kind)).collect();

        assert_eq!(
            found,
            vec![
                ("fmt".to_string(), SymbolKind::Function),
                ("go".to_string(), SymbolKind::Function),
                ("LIMIT".to_string(), SymbolKind::Constant),
                ("counter".to_string(), SymbolKind::Variable),
                ("Cache".to_string(), SymbolKind::Class),
                ("get".to_string(), SymbolKind::Method),
            ]
        );
    }
}
