//! Small helpers shared by the per-language modules.

/// Parses `source` with the given tree-sitter language.
///
/// Returns `None` when the language cannot be loaded (never expected for the
/// statically linked grammars) or the parse produced no tree.
pub(super) fn parse(language: tree_sitter::Language, source: &[u8]) -> Option<tree_sitter::Tree> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&language).ok()?;
    parser.parse(source, None)
}

/// Text of a named field child, if present and valid UTF-8.
pub(super) fn field_text(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    field: &str,
) -> Option<String> {
    Some(
        node.child_by_field_name(field)?
            .utf8_text(source)
            .ok()?
            .to_string(),
    )
}

/// Whether any direct child has the given node kind.
pub(super) fn has_child_kind(node: tree_sitter::Node<'_>, kind: &str) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor).any(|child| child.kind() == kind)
}

/// Whether any ancestor node has one of the given kinds.
pub(super) fn ancestors_contain(mut node: tree_sitter::Node<'_>, kinds: &[&str]) -> bool {
    while let Some(parent) = node.parent() {
        if kinds.contains(&parent.kind()) {
            return true;
        }
        node = parent;
    }
    false
}

/// Whether a declarator AST node represents a function declarator (signature or definition).
pub(super) fn is_function_declarator(mut node: tree_sitter::Node<'_>) -> bool {
    loop {
        match node.kind() {
            "function_declarator" => return true,
            "pointer_declarator"
            | "reference_declarator"
            | "parenthesized_declarator"
            | "attributed_declarator"
            | "init_declarator" => {
                if let Some(inner) = node
                    .child_by_field_name("declarator")
                    .or_else(|| first_named_child(node))
                {
                    node = inner;
                } else {
                    return false;
                }
            }
            _ => return false,
        }
    }
}

fn first_named_child(node: tree_sitter::Node<'_>) -> Option<tree_sitter::Node<'_>> {
    let mut cursor = node.walk();
    node.children(&mut cursor).find(|child| child.is_named())
}

/// Name of a C/C++ declarator, descending through declarator wrappers
/// (`function_declarator`, pointers, arrays, parens, initializers) down to
/// the terminal identifier. Destructors keep their `~` stripped and
/// qualified names are reduced to their last segment.
pub(super) fn declarator_name(node: tree_sitter::Node<'_>, source: &[u8]) -> Option<String> {
    let text_of = |n: tree_sitter::Node<'_>| n.utf8_text(source).ok().map(str::to_string);

    match node.kind() {
        "identifier" | "field_identifier" | "type_identifier" | "property_identifier" => {
            text_of(node)
        }
        "destructor_name" => text_of(node).map(|t| t.trim_start_matches('~').to_string()),
        "qualified_identifier" => {
            let mut cursor = node.walk();
            let last = node.children(&mut cursor).last()?;
            declarator_name(last, source)
        }
        "function_declarator"
        | "pointer_declarator"
        | "array_declarator"
        | "reference_declarator"
        | "parenthesized_declarator"
        | "attributed_declarator"
        | "init_declarator" => {
            let inner = node
                .child_by_field_name("declarator")
                .or_else(|| first_named_child(node))?;
            declarator_name(inner, source)
        }
        _ => None,
    }
}
