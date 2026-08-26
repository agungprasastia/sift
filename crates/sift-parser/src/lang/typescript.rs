//! TypeScript symbol extraction (`tree-sitter-typescript`).
//!
//! TS-specific declarations are handled here; everything else falls through
//! to the JavaScript classifier. M0 limitation: `type` aliases are skipped
//! because the shared symbol model has no alias kind yet.

use sift_core::{Symbol, SymbolKind};
use tree_sitter::Node;

use crate::engine::{self};
use crate::lang::javascript;
use crate::util;

pub(crate) fn extract(source: &[u8]) -> Vec<Symbol> {
    // `.tsx` uses a different grammar; M0 routes all TypeScript through the
    // plain TS grammar and accepts that limitation.
    let Some(tree) = util::parse(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(), source) else {
        return Vec::new();
    };
    engine::collect(tree.root_node(), source, classify)
}

fn classify(node: Node<'_>, source: &[u8]) -> Vec<(String, SymbolKind)> {
    match node.kind() {
        "interface_declaration" => engine::one(
            util::field_text(node, source, "name"),
            SymbolKind::Interface,
        ),
        "enum_declaration" => engine::one(util::field_text(node, source, "name"), SymbolKind::Enum),
        "abstract_class_declaration" => {
            engine::one(util::field_text(node, source, "name"), SymbolKind::Class)
        }
        "type_alias_declaration" => Vec::new(),
        _ => javascript::classify(node, source),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract_symbols;
    use sift_core::Language;

    #[test]
    fn extracts_typescript_symbols() {
        let source = r#"interface Options {
    retries: number;
}

enum Mode { Fast, Slow }

export function retry<T>(op: () => T): T {
    return op();
}

class RetryClient implements Options {
    retries = 3;

    fetch(): void {}
}

type Alias = string;
"#;
        let symbols = extract_symbols(Language::TypeScript, source.as_bytes());
        let found: Vec<(String, SymbolKind)> =
            symbols.iter().map(|s| (s.name.clone(), s.kind)).collect();

        assert_eq!(
            found,
            vec![
                ("Options".to_string(), SymbolKind::Interface),
                ("Mode".to_string(), SymbolKind::Enum),
                ("retry".to_string(), SymbolKind::Function),
                ("RetryClient".to_string(), SymbolKind::Class),
                ("fetch".to_string(), SymbolKind::Method),
            ]
        );
    }
}
