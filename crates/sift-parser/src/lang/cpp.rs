//! C++ symbol extraction (`tree-sitter-cpp`), extending the C rules with
//! classes, namespaces and methods.

use sift_core::{Symbol, SymbolKind};
use tree_sitter::Node;

use crate::engine::{self};
use crate::lang::c;
use crate::util;

pub(crate) fn extract(source: &[u8]) -> Vec<Symbol> {
    let Some(tree) = util::parse(tree_sitter_cpp::LANGUAGE.into(), source) else {
        return Vec::new();
    };
    engine::collect(tree.root_node(), source, classify)
}

fn classify(node: Node<'_>, source: &[u8]) -> Vec<(String, SymbolKind)> {
    match node.kind() {
        "class_specifier" if named_with_body(node) => {
            engine::one(util::field_text(node, source, "name"), SymbolKind::Class)
        }
        "namespace_definition" => {
            engine::one(util::field_text(node, source, "name"), SymbolKind::Module)
        }
        "function_definition" => {
            let name = node
                .child_by_field_name("declarator")
                .and_then(|decl| util::declarator_name(decl, source));
            let kind = if inside_class_body(node) {
                SymbolKind::Method
            } else {
                SymbolKind::Function
            };
            engine::one(name, kind)
        }
        "declaration" | "field_declaration" => {
            let mut out = Vec::new();
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if (child.kind() == "function_declarator" || util::is_function_declarator(child))
                    && let Some(name) = util::declarator_name(child, source)
                {
                    let kind = if inside_class_body(node) {
                        SymbolKind::Method
                    } else {
                        SymbolKind::Function
                    };
                    out.push((name, kind));
                }
            }
            if !out.is_empty() {
                out
            } else {
                c::classify(node, source)
            }
        }
        _ => c::classify(node, source),
    }
}

fn named_with_body(node: Node<'_>) -> bool {
    node.child_by_field_name("name").is_some() && node.child_by_field_name("body").is_some()
}

fn inside_class_body(mut node: Node<'_>) -> bool {
    while let Some(parent) = node.parent() {
        match parent.kind() {
            "class_specifier" => return true,
            "translation_unit" | "namespace_definition" => return false,
            _ => {}
        }
        node = parent;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract_symbols;
    use sift_core::Language;

    #[test]
    fn extracts_cpp_symbols() {
        let source = r#"namespace geo {

class Shape {
public:
    virtual double area() const { return 0.0; }
};

double total(const Shape& s);

}

class Widget {
    void draw();
};

void free_fn();

struct Plain {
    int size;
};
"#;
        let symbols = extract_symbols(Language::Cpp, source.as_bytes());
        let found: Vec<(String, SymbolKind)> =
            symbols.iter().map(|s| (s.name.clone(), s.kind)).collect();

        assert_eq!(
            found,
            vec![
                ("geo".to_string(), SymbolKind::Module),
                ("Shape".to_string(), SymbolKind::Class),
                ("area".to_string(), SymbolKind::Method),
                ("total".to_string(), SymbolKind::Function),
                ("Widget".to_string(), SymbolKind::Class),
                ("draw".to_string(), SymbolKind::Method),
                ("free_fn".to_string(), SymbolKind::Function),
                ("Plain".to_string(), SymbolKind::Struct),
            ]
        );
    }

    #[test]
    fn destructor_names_lose_the_tilde() {
        let source = "class Gate {\npublic:\n    ~Gate() {}\n};\n";
        let symbols = extract_symbols(Language::Cpp, source.as_bytes());
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "Gate" && s.kind == SymbolKind::Method)
        );
    }
}
