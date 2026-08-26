//! Shared AST walk: depth-limited DFS feeding every node to a classifier.

use sift_core::{Symbol, SymbolKind};

/// Depth cap guarding against pathological stack blowups on deeply nested
/// generated code; real-world ASTs stay far below this.
const MAX_DEPTH: usize = 256;

/// Classifies a single node into zero or more `(name, kind)` pairs.
pub(super) type Classifier =
    for<'a> fn(tree_sitter::Node<'a>, &'a [u8]) -> Vec<(String, SymbolKind)>;

/// Walks the tree rooted at `root`, collecting classified symbols.
pub(super) fn collect(
    root: tree_sitter::Node<'_>,
    source: &[u8],
    classify: Classifier,
) -> Vec<Symbol> {
    let mut out = Vec::new();
    walk(root, source, 0, classify, &mut out);
    out
}

fn walk(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    depth: usize,
    classify: Classifier,
    out: &mut Vec<Symbol>,
) {
    if depth > MAX_DEPTH {
        return;
    }

    // The classifier yields bare names/kinds so per-language quirks stay in
    // the language modules; the engine only supplies line numbers.
    for (name, kind) in classify(node, source) {
        out.push(Symbol {
            name,
            kind,
            line: node.start_position().row + 1,
        });
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(child, source, depth + 1, classify, out);
    }
}

/// Convenience for language modules: wrap an optional name into the shape
/// [`Classifier`] expects.
pub(super) fn one(name: Option<String>, kind: SymbolKind) -> Vec<(String, SymbolKind)> {
    name.map(|n| vec![(n, kind)]).unwrap_or_default()
}
