//! Go symbol extraction (`tree-sitter-go`).

use sift_core::{Symbol, SymbolKind};
use tree_sitter::Node;

use crate::engine::{self};
use crate::util;

pub(crate) fn extract(source: &[u8]) -> Vec<Symbol> {
    let Some(tree) = util::parse(tree_sitter_go::LANGUAGE.into(), source) else {
        return Vec::new();
    };
    engine::collect(tree.root_node(), source, classify)
}

fn classify(node: Node<'_>, source: &[u8]) -> Vec<(String, SymbolKind)> {
    match node.kind() {
        "function_declaration" => {
            engine::one(util::field_text(node, source, "name"), SymbolKind::Function)
        }
        "method_declaration" => {
            engine::one(util::field_text(node, source, "name"), SymbolKind::Method)
        }
        "type_spec" => {
            let Some(name) = util::field_text(node, source, "name") else {
                return Vec::new();
            };
            if util::has_child_kind(node, "struct_type") {
                engine::one(Some(name), SymbolKind::Struct)
            } else if util::has_child_kind(node, "interface_type") {
                engine::one(Some(name), SymbolKind::Interface)
            } else {
                Vec::new()
            }
        }
        "const_declaration" => grouped_specs(node, source, "const_spec", SymbolKind::Constant),
        "var_declaration" => grouped_specs(node, source, "var_spec", SymbolKind::Variable),
        "package_clause" => {
            let mut cursor = node.walk();
            let name = node
                .children(&mut cursor)
                .find(|child| child.kind() == "package_identifier")
                .and_then(|n| n.utf8_text(source).ok().map(str::to_string));
            engine::one(name, SymbolKind::Module)
        }
        _ => Vec::new(),
    }
}

/// Go groups several specs under one declaration
/// (`const A, B = 1, 2`); expand every spec and identifier into its own symbol.
fn grouped_specs(
    node: Node<'_>,
    source: &[u8],
    spec_kind: &str,
    kind: SymbolKind,
) -> Vec<(String, SymbolKind)> {
    let mut cursor = node.walk();
    let mut out = Vec::new();
    for spec in node
        .children(&mut cursor)
        .filter(|child| child.kind() == spec_kind)
    {
        let mut spec_cursor = spec.walk();
        for child in spec.children(&mut spec_cursor) {
            if child.kind() == "identifier"
                && let Ok(name) = child.utf8_text(source)
            {
                out.push((name.to_string(), kind));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract_symbols;
    use sift_core::Language;

    #[test]
    fn extracts_go_symbols() {
        let source = r#"package main

func main() {}

func helper(n int) int {
	return n
}

type Server struct{}

type Handler interface{ Handle() }

const A, B = 1, 2

var counter int
"#;
        let symbols = extract_symbols(Language::Go, source.as_bytes());
        let found: Vec<(String, SymbolKind)> =
            symbols.iter().map(|s| (s.name.clone(), s.kind)).collect();

        assert_eq!(
            found,
            vec![
                ("main".to_string(), SymbolKind::Module),
                ("main".to_string(), SymbolKind::Function),
                ("helper".to_string(), SymbolKind::Function),
                ("Server".to_string(), SymbolKind::Struct),
                ("Handler".to_string(), SymbolKind::Interface),
                ("A".to_string(), SymbolKind::Constant),
                ("B".to_string(), SymbolKind::Constant),
                ("counter".to_string(), SymbolKind::Variable),
            ]
        );
    }
}
