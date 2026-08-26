//! Python symbol extraction (`tree-sitter-python`).

use sift_core::{Symbol, SymbolKind};
use tree_sitter::Node;

use crate::engine::{self};
use crate::util;

pub(crate) fn extract(source: &[u8]) -> Vec<Symbol> {
    let Some(tree) = util::parse(tree_sitter_python::LANGUAGE.into(), source) else {
        return Vec::new();
    };
    engine::collect(tree.root_node(), source, classify)
}

fn classify(node: Node<'_>, source: &[u8]) -> Vec<(String, SymbolKind)> {
    match node.kind() {
        "class_definition" => {
            engine::one(util::field_text(node, source, "name"), SymbolKind::Class)
        }
        "function_definition" => {
            let kind = if in_class_scope(node) {
                SymbolKind::Method
            } else {
                SymbolKind::Function
            };
            engine::one(util::field_text(node, source, "name"), kind)
        }
        "assignment" if is_module_level(node) => module_assignment(node, source),
        _ => Vec::new(),
    }
}

/// Nearest enclosing scope decides method-vs-function: a def directly inside
/// a class body is a method even when nested deeper inside another function.
fn in_class_scope(mut node: Node<'_>) -> bool {
    while let Some(parent) = node.parent() {
        match parent.kind() {
            "class_definition" => return true,
            "function_definition" => return false,
            _ => {}
        }
        node = parent;
    }
    false
}

fn is_module_level(node: Node<'_>) -> bool {
    !util::ancestors_contain(node, &["function_definition", "class_definition"])
}

fn module_assignment(node: Node<'_>, source: &[u8]) -> Vec<(String, SymbolKind)> {
    let Some(left) = node
        .child_by_field_name("left")
        .filter(|child| child.kind() == "identifier")
    else {
        return Vec::new();
    };
    let Ok(name) = left.utf8_text(source) else {
        return Vec::new();
    };
    // Convention heuristic: SCREAMING_CASE module bindings are constants.
    let kind = if looks_constant(name) {
        SymbolKind::Constant
    } else {
        SymbolKind::Variable
    };
    vec![(name.to_string(), kind)]
}

fn looks_constant(name: &str) -> bool {
    name.chars().any(char::is_uppercase) && !name.chars().any(char::is_lowercase)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract_symbols;
    use sift_core::Language;

    #[test]
    fn extracts_python_symbols() {
        let source = r#"class Client:
    def connect(self):
        pass


def load(path):
    def inner():
        pass
    inner()


LIMIT = 10
name = "x"
"#;
        let symbols = extract_symbols(Language::Python, source.as_bytes());
        let found: Vec<(String, SymbolKind)> =
            symbols.iter().map(|s| (s.name.clone(), s.kind)).collect();

        assert_eq!(
            found,
            vec![
                ("Client".to_string(), SymbolKind::Class),
                ("connect".to_string(), SymbolKind::Method),
                ("load".to_string(), SymbolKind::Function),
                ("inner".to_string(), SymbolKind::Function),
                ("LIMIT".to_string(), SymbolKind::Constant),
                ("name".to_string(), SymbolKind::Variable),
            ]
        );
    }
}
