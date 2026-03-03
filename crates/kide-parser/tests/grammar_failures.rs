use kide_parser::{parse, parse_file};
use std::path::PathBuf;

const VALID_SNIPPET: &str = r#"
context Sales {
    aggregate Order {
        id: String
    }
}
"#;

fn unique_temp_file_path(prefix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();

    path.push(format!("{prefix}-{}-{timestamp}.kide", std::process::id()));
    path
}

#[test]
fn parse_reports_failure_prefix_for_invalid_snippets() {
    let invalid_snippets = [
        (
            "missing closing brace",
            "context Sales { aggregate Order { id: String }",
        ),
        (
            "missing context keyword",
            "Sales { aggregate Order { id: String } }",
        ),
        (
            "malformed binding",
            "context Sales { aggregate Order bound to { id: String } }",
        ),
        (
            "malformed field binding",
            "context Sales { aggregate Order { id String } }",
        ),
    ];

    for (name, snippet) in invalid_snippets {
        let error = parse(snippet)
            .expect_err("snippet should fail to parse")
            .to_string();

        assert!(
            error.starts_with("failed to parse .kide file:"),
            "{name} should include parse() error prefix, got: {error}"
        );
        assert_ne!(
            error, "failed to parse .kide file:",
            "{name} should include parser error context"
        );
    }
}

#[test]
fn parse_file_succeeds_for_valid_input() {
    let path = unique_temp_file_path("kide-parser-success");
    std::fs::write(&path, VALID_SNIPPET).expect("temporary file should be writable");

    let result = parse_file(&path);
    let _ = std::fs::remove_file(&path);

    assert!(result.is_ok(), "valid file should parse successfully");
}

#[test]
fn parse_file_missing_file_returns_not_found_error() {
    let path = unique_temp_file_path("kide-parser-missing");
    let error = parse_file(&path).expect_err("missing file should fail");
    let io_error = error
        .downcast_ref::<std::io::Error>()
        .expect("error should preserve io::Error");

    assert_eq!(io_error.kind(), std::io::ErrorKind::NotFound);
}
