use anyhow::Result;
use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};
use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_rust() -> *const ();
}

const BOUNDARY_REFERENCE_QUERY: &str = r#"
[
  (use_declaration)
  (type_identifier)
  (scoped_identifier)
  (call_expression)
] @reference
"#;

pub fn symbol_exists(source: &str, symbol: &str, query_source: &str) -> Result<bool> {
    Ok(find_symbol_span(source, symbol, query_source)?.is_some())
}

pub fn boundary_references(source: &str) -> Result<Vec<String>> {
    let language_fn = unsafe { LanguageFn::from_raw(tree_sitter_rust) };
    let language = tree_sitter::Language::from(language_fn);
    let mut parser = Parser::new();
    parser.set_language(&language)?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| anyhow::anyhow!("tree-sitter failed to parse Rust source"))?;

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

pub fn nearest_symbol(source: &str, symbol: &str, query_source: &str) -> Result<Option<String>> {
    let language_fn = unsafe { LanguageFn::from_raw(tree_sitter_rust) };
    let symbols = captured_symbols_with_language(language_fn, source, query_source)?;
    Ok(crate::nearest_symbol_name(
        symbol_leaf_name(symbol),
        symbols,
    ))
}

pub fn symbol_arity(source: &str, symbol: &str, query_source: &str) -> Result<Option<usize>> {
    let language_fn = unsafe { LanguageFn::from_raw(tree_sitter_rust) };
    let language = tree_sitter::Language::from(language_fn);

    let mut parser = Parser::new();
    parser.set_language(&language)?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| anyhow::anyhow!("tree-sitter failed to parse Rust source"))?;

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

            let Some(function_item) = enclosing_function_item(capture.node) else {
                continue;
            };
            let Some(parameters) = function_item.child_by_field_name("parameters") else {
                continue;
            };

            let mut params_cursor = parameters.walk();
            let arity = parameters
                .named_children(&mut params_cursor)
                .filter(|node| matches!(node.kind(), "parameter" | "variadic_parameter"))
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

pub fn find_symbol_span(
    source: &str,
    symbol: &str,
    query_source: &str,
) -> Result<Option<crate::ViolationSpan>> {
    let language_fn = unsafe { LanguageFn::from_raw(tree_sitter_rust) };
    let language = tree_sitter::Language::from(language_fn);

    let mut parser = Parser::new();
    parser.set_language(&language)?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| anyhow::anyhow!("tree-sitter failed to parse Rust source"))?;

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
        .ok_or_else(|| anyhow::anyhow!("tree-sitter failed to parse Rust source"))?;

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
    symbol.split("::").last().unwrap_or(symbol)
}

fn enclosing_function_item(mut node: tree_sitter::Node<'_>) -> Option<tree_sitter::Node<'_>> {
    loop {
        if node.kind() == "function_item" {
            return Some(node);
        }
        node = node.parent()?;
    }
}
