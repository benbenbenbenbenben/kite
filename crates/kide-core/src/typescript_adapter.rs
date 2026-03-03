use anyhow::Result;
use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};
use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_typescript() -> *const ();
    fn tree_sitter_tsx() -> *const ();
}

const BOUNDARY_REFERENCE_QUERY: &str = r#"
[
  (import_statement)
  (type_identifier)
  (call_expression)
  (new_expression)
] @reference
"#;

pub fn symbol_exists(source: &str, symbol: &str, query_source: &str) -> Result<bool> {
    Ok(find_symbol_span(source, symbol, query_source)?.is_some())
}

pub fn symbol_exists_tsx(source: &str, symbol: &str, query_source: &str) -> Result<bool> {
    Ok(find_symbol_span_tsx(source, symbol, query_source)?.is_some())
}

pub fn boundary_references(source: &str) -> Result<Vec<String>> {
    let language_fn = unsafe { LanguageFn::from_raw(tree_sitter_typescript) };
    boundary_references_with_language(language_fn, source)
}

pub fn boundary_references_tsx(source: &str) -> Result<Vec<String>> {
    let language_fn = unsafe { LanguageFn::from_raw(tree_sitter_tsx) };
    boundary_references_with_language(language_fn, source)
}

pub fn nearest_symbol(source: &str, symbol: &str, query_source: &str) -> Result<Option<String>> {
    let language_fn = unsafe { LanguageFn::from_raw(tree_sitter_typescript) };
    let symbols = captured_symbols_with_language(language_fn, source, query_source)?;
    Ok(crate::nearest_symbol_name(
        symbol_leaf_name(symbol),
        symbols,
    ))
}

pub fn nearest_symbol_tsx(
    source: &str,
    symbol: &str,
    query_source: &str,
) -> Result<Option<String>> {
    let language_fn = unsafe { LanguageFn::from_raw(tree_sitter_tsx) };
    let symbols = captured_symbols_with_language(language_fn, source, query_source)?;
    Ok(crate::nearest_symbol_name(
        symbol_leaf_name(symbol),
        symbols,
    ))
}

pub fn symbol_arity(source: &str, symbol: &str, query_source: &str) -> Result<Option<usize>> {
    let language_fn = unsafe { LanguageFn::from_raw(tree_sitter_typescript) };
    symbol_arity_with_language(language_fn, source, symbol, query_source)
}

pub fn symbol_arity_tsx(source: &str, symbol: &str, query_source: &str) -> Result<Option<usize>> {
    let language_fn = unsafe { LanguageFn::from_raw(tree_sitter_tsx) };
    symbol_arity_with_language(language_fn, source, symbol, query_source)
}

pub fn find_symbol_span(
    source: &str,
    symbol: &str,
    query_source: &str,
) -> Result<Option<crate::ViolationSpan>> {
    let language_fn = unsafe { LanguageFn::from_raw(tree_sitter_typescript) };
    find_symbol_span_with_language(language_fn, source, symbol, query_source)
}

pub fn find_symbol_span_tsx(
    source: &str,
    symbol: &str,
    query_source: &str,
) -> Result<Option<crate::ViolationSpan>> {
    let language_fn = unsafe { LanguageFn::from_raw(tree_sitter_tsx) };
    find_symbol_span_with_language(language_fn, source, symbol, query_source)
}

fn find_symbol_span_with_language(
    language_fn: LanguageFn,
    source: &str,
    symbol: &str,
    query_source: &str,
) -> Result<Option<crate::ViolationSpan>> {
    let language = tree_sitter::Language::from(language_fn);
    let mut parser = Parser::new();
    parser.set_language(&language)?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| anyhow::anyhow!("tree-sitter failed to parse TypeScript source"))?;

    let query = Query::new(&language, query_source)?;
    let mut query_cursor = QueryCursor::new();
    let target_symbol = symbol_leaf_name(symbol);
    let capture_names = query.capture_names();

    let mut query_matches = query_cursor.matches(&query, tree.root_node(), source.as_bytes());
    while let Some(query_match) = query_matches.next() {
        for capture in query_match.captures {
            let capture_name = capture_names[capture.index as usize];
            if capture_name != "name" {
                continue;
            }

            if let Ok(captured_text) = capture.node.utf8_text(source.as_bytes()) {
                if captured_text == target_symbol {
                    let start = capture.node.start_position();
                    let end = capture.node.end_position();
                    return Ok(Some(crate::ViolationSpan {
                        start_line: start.row + 1,
                        start_column: start.column + 1,
                        end_line: end.row + 1,
                        end_column: end.column + 1,
                    }));
                }
            }
        }
    }

    Ok(None)
}

fn symbol_arity_with_language(
    language_fn: LanguageFn,
    source: &str,
    symbol: &str,
    query_source: &str,
) -> Result<Option<usize>> {
    let language = tree_sitter::Language::from(language_fn);
    let mut parser = Parser::new();
    parser.set_language(&language)?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| anyhow::anyhow!("tree-sitter failed to parse TypeScript source"))?;

    let query = Query::new(&language, query_source)?;
    let mut query_cursor = QueryCursor::new();
    let target_symbol = symbol_leaf_name(symbol);
    let capture_names = query.capture_names();
    let mut arities = Vec::new();

    let mut query_matches = query_cursor.matches(&query, tree.root_node(), source.as_bytes());
    while let Some(query_match) = query_matches.next() {
        for capture in query_match.captures {
            let capture_name = capture_names[capture.index as usize];
            if capture_name != "name" {
                continue;
            }

            let Ok(captured_text) = capture.node.utf8_text(source.as_bytes()) else {
                continue;
            };
            if captured_text != target_symbol {
                continue;
            }

            let Some(function_like) = enclosing_function_like(capture.node) else {
                continue;
            };
            let Some(parameters) = function_like.child_by_field_name("parameters") else {
                continue;
            };

            let mut params_cursor = parameters.walk();
            let arity = parameters
                .named_children(&mut params_cursor)
                .filter(|node| matches!(node.kind(), "required_parameter" | "optional_parameter"))
                .count();
            arities.push(arity);
        }
    }

    let mut arities = arities.into_iter();
    let Some(first) = arities.next() else {
        return Ok(None);
    };
    if arities.all(|arity| arity == first) {
        Ok(Some(first))
    } else {
        Ok(None)
    }
}

fn boundary_references_with_language(
    language_fn: LanguageFn,
    source: &str,
) -> Result<Vec<String>> {
    let language = tree_sitter::Language::from(language_fn);
    let mut parser = Parser::new();
    parser.set_language(&language)?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| anyhow::anyhow!("tree-sitter failed to parse TypeScript source"))?;

    let query = Query::new(&language, BOUNDARY_REFERENCE_QUERY)?;
    let mut query_cursor = QueryCursor::new();
    let capture_names = query.capture_names();
    let mut query_matches = query_cursor.matches(&query, tree.root_node(), source.as_bytes());
    let mut references = Vec::new();
    while let Some(query_match) = query_matches.next() {
        for capture in query_match.captures {
            let capture_name = capture_names[capture.index as usize];
            if capture_name != "reference" {
                continue;
            }
            let Ok(captured_text) = capture.node.utf8_text(source.as_bytes()) else {
                continue;
            };
            references.push(captured_text.to_owned());
        }
    }

    Ok(references)
}

fn captured_symbols_with_language(
    language_fn: LanguageFn,
    source: &str,
    query_source: &str,
) -> Result<Vec<String>> {
    let language = tree_sitter::Language::from(language_fn);
    let mut parser = Parser::new();
    parser.set_language(&language)?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| anyhow::anyhow!("tree-sitter failed to parse TypeScript source"))?;

    let query = Query::new(&language, query_source)?;
    let mut query_cursor = QueryCursor::new();
    let capture_names = query.capture_names();
    let mut symbols = Vec::new();

    let mut query_matches = query_cursor.matches(&query, tree.root_node(), source.as_bytes());
    while let Some(query_match) = query_matches.next() {
        for capture in query_match.captures {
            let capture_name = capture_names[capture.index as usize];
            if capture_name != "name" {
                continue;
            }
            if let Ok(captured_text) = capture.node.utf8_text(source.as_bytes()) {
                symbols.push(captured_text.to_owned());
            }
        }
    }

    Ok(symbols)
}

fn symbol_leaf_name(symbol: &str) -> &str {
    symbol
        .rsplit([':', '.', '#'])
        .find(|segment| !segment.is_empty())
        .unwrap_or(symbol)
}

fn enclosing_function_like(mut node: tree_sitter::Node<'_>) -> Option<tree_sitter::Node<'_>> {
    loop {
        if matches!(
            node.kind(),
            "function_declaration"
                | "generator_function_declaration"
                | "function_signature"
                | "method_definition"
                | "method_signature"
                | "abstract_method_signature"
        ) {
            return Some(node);
        }
        node = node.parent()?;
    }
}
