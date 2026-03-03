use anyhow::Result;
use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};
use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_prisma() -> *const ();
}

pub fn symbol_exists(source: &str, symbol: &str, query_source: &str) -> Result<bool> {
    Ok(find_symbol_span(source, symbol, query_source)?.is_some())
}

pub fn nearest_symbol(source: &str, symbol: &str, query_source: &str) -> Result<Option<String>> {
    let language_fn = unsafe { LanguageFn::from_raw(tree_sitter_prisma) };
    let symbols = captured_symbols_with_language(language_fn, source, query_source)?;
    Ok(crate::nearest_symbol_name(
        symbol_leaf_name(symbol),
        symbols,
    ))
}

pub fn symbol_arity(source: &str, symbol: &str, query_source: &str) -> Result<Option<usize>> {
    if symbol_exists(source, symbol, query_source)? {
        Ok(Some(0))
    } else {
        Ok(None)
    }
}

pub fn find_symbol_span(
    source: &str,
    symbol: &str,
    query_source: &str,
) -> Result<Option<crate::ViolationSpan>> {
    let language_fn = unsafe { LanguageFn::from_raw(tree_sitter_prisma) };
    let language = tree_sitter::Language::from(language_fn);
    let mut parser = Parser::new();
    parser.set_language(&language)?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| anyhow::anyhow!("tree-sitter failed to parse Prisma source"))?;

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
        .ok_or_else(|| anyhow::anyhow!("tree-sitter failed to parse Prisma source"))?;

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
