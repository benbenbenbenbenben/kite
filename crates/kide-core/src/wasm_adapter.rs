use anyhow::{Context, Result};
use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};

use crate::grammar_registry::GrammarRegistry;

pub fn symbol_exists(
    registry: &GrammarRegistry,
    language: &str,
    is_tsx: bool,
    source: &str,
    symbol: &str,
    query_source: &str,
) -> Result<Option<bool>> {
    Ok(
        find_symbol_span(registry, language, is_tsx, source, symbol, query_source)?
            .map(|_| true)
            .or(Some(false)),
    )
}

pub fn find_symbol_span(
    registry: &GrammarRegistry,
    language: &str,
    is_tsx: bool,
    source: &str,
    symbol: &str,
    query_source: &str,
) -> Result<Option<crate::ViolationSpan>> {
    let Some((mut parser, ts_language)) = create_parser(registry, language, is_tsx)? else {
        return Ok(None);
    };

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| anyhow::anyhow!("tree-sitter failed to parse {} source", language))?;

    let query = Query::new(&ts_language, query_source)?;
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

pub fn nearest_symbol(
    registry: &GrammarRegistry,
    language: &str,
    is_tsx: bool,
    source: &str,
    symbol: &str,
    query_source: &str,
) -> Result<Option<String>> {
    let symbols = captured_symbols(registry, language, is_tsx, source, query_source)?;
    let Some(symbols) = symbols else {
        return Ok(None);
    };
    Ok(crate::nearest_symbol_name(
        symbol_leaf_name(symbol),
        symbols,
    ))
}

pub fn symbol_arity(
    registry: &GrammarRegistry,
    language: &str,
    is_tsx: bool,
    source: &str,
    symbol: &str,
    query_source: &str,
) -> Result<Option<usize>> {
    // Prisma models are declaration-only (arity 0) — no function parameters
    if language == "prisma" {
        if symbol_exists(registry, language, is_tsx, source, symbol, query_source)?.unwrap_or(false)
        {
            return Ok(Some(0));
        } else {
            return Ok(None);
        }
    }

    let Some((mut parser, ts_language)) = create_parser(registry, language, is_tsx)? else {
        return Ok(None);
    };

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| anyhow::anyhow!("tree-sitter failed to parse {} source", language))?;

    let query = Query::new(&ts_language, query_source)?;
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

            let Some(function_item) = enclosing_function_like(language, capture.node) else {
                continue;
            };
            let Some(parameters) = function_item
                .child_by_field_name("parameters")
                .or_else(|| function_item.child_by_field_name("formal_parameters"))
            else {
                continue;
            };

            let mut params_cursor = parameters.walk();
            let arity = parameters
                .named_children(&mut params_cursor)
                .filter(|node| is_parameter_node(language, node))
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

pub fn boundary_references(
    registry: &GrammarRegistry,
    language: &str,
    is_tsx: bool,
    source: &str,
) -> Result<Option<Vec<String>>> {
    let Some(boundary_query_source) = registry.boundary_references_query(language) else {
        return Ok(None);
    };

    let Some((mut parser, ts_language)) = create_parser(registry, language, is_tsx)? else {
        return Ok(None);
    };

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| anyhow::anyhow!("tree-sitter failed to parse {} source", language))?;

    let query = Query::new(&ts_language, &boundary_query_source)?;
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

    Ok(Some(references))
}

pub fn captured_symbols(
    registry: &GrammarRegistry,
    language: &str,
    is_tsx: bool,
    source: &str,
    query_source: &str,
) -> Result<Option<Vec<String>>> {
    let Some((mut parser, ts_language)) = create_parser(registry, language, is_tsx)? else {
        return Ok(None);
    };

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| anyhow::anyhow!("tree-sitter failed to parse {} source", language))?;

    let query = Query::new(&ts_language, query_source)?;
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

    Ok(Some(symbols))
}

/// Create a parser configured for WASM grammar loading.
///
/// On non-wasm32 targets, this loads the grammar via WasmStore + wasmtime.
/// On wasm32 targets, this returns None (use the JS bridge adapter instead).
#[cfg(not(target_arch = "wasm32"))]
fn create_parser(
    registry: &GrammarRegistry,
    language: &str,
    is_tsx: bool,
) -> Result<Option<(Parser, tree_sitter::Language)>> {
    let wasm_bytes = if is_tsx && language == "typescript" {
        registry.tsx_wasm_bytes(language)?
    } else {
        registry.wasm_bytes(language)?
    };

    let Some(wasm_bytes) = wasm_bytes else {
        return Ok(None);
    };

    let engine = tree_sitter::wasmtime::Engine::default();
    let mut store = tree_sitter::WasmStore::new(&engine)
        .map_err(|e| anyhow::anyhow!("failed to create wasm store: {}", e))?;

    // For TSX, the WASM module exports tree_sitter_tsx, not tree_sitter_typescript
    let wasm_language_name = if is_tsx && language == "typescript" {
        "tsx"
    } else {
        language
    };

    let lang = store
        .load_language(wasm_language_name, &wasm_bytes)
        .with_context(|| format!("failed to load wasm grammar for '{}'", language))?;

    let mut parser = Parser::new();
    parser
        .set_wasm_store(store)
        .map_err(|e| anyhow::anyhow!("failed to set wasm store: {}", e))?;
    parser.set_language(&lang)?;

    Ok(Some((parser, lang)))
}

#[cfg(target_arch = "wasm32")]
fn create_parser(
    _registry: &GrammarRegistry,
    _language: &str,
    _is_tsx: bool,
) -> Result<Option<(Parser, tree_sitter::Language)>> {
    // On wasm32 targets, grammar loading is handled via JS bridge
    Ok(None)
}

fn symbol_leaf_name(symbol: &str) -> &str {
    symbol
        .rsplit(|c| matches!(c, ':' | '.' | '#'))
        .find(|segment| !segment.is_empty())
        .unwrap_or(symbol)
}

fn enclosing_function_like<'a>(
    language: &str,
    mut node: tree_sitter::Node<'a>,
) -> Option<tree_sitter::Node<'a>> {
    loop {
        let kind = node.kind();
        let is_function = match language {
            "rust" => kind == "function_item",
            "typescript" => matches!(
                kind,
                "function_declaration"
                    | "method_definition"
                    | "arrow_function"
                    | "generator_function_declaration"
                    | "function_signature"
                    | "method_signature"
                    | "abstract_method_signature"
            ),
            _ => false,
        };
        if is_function {
            return Some(node);
        }
        node = node.parent()?;
    }
}

fn is_parameter_node(language: &str, node: &tree_sitter::Node<'_>) -> bool {
    match language {
        "rust" => matches!(node.kind(), "parameter" | "variadic_parameter"),
        "typescript" => matches!(
            node.kind(),
            "required_parameter" | "optional_parameter" | "rest_pattern"
        ),
        _ => false,
    }
}
