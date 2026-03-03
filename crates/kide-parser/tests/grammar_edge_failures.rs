use kide_parser::parse;

const PARSE_ERROR_PREFIX: &str = "failed to parse .kide file:";

fn assert_edge_case_parse_failure(name: &str, snippet: &str) {
    let error = parse(snippet)
        .expect_err("snippet should fail to parse")
        .to_string();

    assert!(
        error.starts_with(PARSE_ERROR_PREFIX),
        "{name} should include parse() error prefix, got: {error}"
    );
    assert!(
        error.contains("ParseError"),
        "{name} should include parser error context, got: {error}"
    );
}

#[test]
fn parse_reports_edge_case_failures_with_stable_error_context() {
    let invalid_snippets = [
        (
            "invalid context identifier starts with digit",
            r#"context 1Sales { aggregate Order { id: String } }"#,
        ),
        (
            "unterminated dictionary string literal",
            r#"context Sales { dictionary { "legacy => forbidden } }"#,
        ),
        (
            "unterminated binding string literal",
            r#"context Sales { aggregate Order bound to "src/order.rs { id: String } }"#,
        ),
        (
            "missing command parameter comma separator",
            r#"context Sales { aggregate Order { command Create(id: Int status: String) bound to "src/order.rs" } }"#,
        ),
        (
            "missing command parameter colon",
            r#"context Sales { aggregate Order { command Create(id Int, status: String) bound to "src/order.rs" } }"#,
        ),
        (
            "nested braces in invariant block",
            r#"context Sales { aggregate Order { invariant KeepShape { if ready { enforce } } } }"#,
        ),
    ];

    for (name, snippet) in invalid_snippets {
        assert_edge_case_parse_failure(name, snippet);
    }
}
