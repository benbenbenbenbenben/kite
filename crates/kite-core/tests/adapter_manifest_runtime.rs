#![allow(dead_code)]

#[path = "../src/grammar_registry.rs"]
mod grammar_registry;

use grammar_registry::GrammarRegistry;
use tempfile::TempDir;

#[test]
fn grammar_registry_loads_language_with_wasm_file() {
    let temp_dir = TempDir::new().unwrap();
    let grammar_dir = temp_dir.path().join("rust");
    std::fs::create_dir_all(&grammar_dir).unwrap();
    std::fs::write(
        grammar_dir.join("manifest.toml"),
        r#"
language = "rust"
version = "0.24.0"
wasm_file = "tree-sitter-rust.wasm"
extensions = [".rs"]
display_name = "Rust"

[queries]
symbol_exists = "queries/symbol_exists.scm"
"#,
    )
    .unwrap();

    let registry = GrammarRegistry::load(temp_dir.path()).unwrap();
    assert!(registry.has_language("rust"));
    assert_eq!(registry.display_name("rust"), "Rust");
}

#[test]
fn grammar_registry_resolves_language_for_path() {
    let temp_dir = TempDir::new().unwrap();
    let grammar_dir = temp_dir.path().join("typescript");
    std::fs::create_dir_all(&grammar_dir).unwrap();
    std::fs::write(
        grammar_dir.join("manifest.toml"),
        r#"
language = "typescript"
wasm_file = "tree-sitter-typescript.wasm"
tsx_wasm_file = "tree-sitter-tsx.wasm"
extensions = [".ts", ".tsx"]
display_name = "TypeScript"

[queries]
symbol_exists = "queries/symbol_exists.scm"
"#,
    )
    .unwrap();

    let registry = GrammarRegistry::load(temp_dir.path()).unwrap();
    assert_eq!(
        registry.language_for_path(std::path::Path::new("foo.ts")),
        Some("typescript")
    );
    assert_eq!(
        registry.language_for_path(std::path::Path::new("foo.tsx")),
        Some("typescript")
    );
    assert!(registry
        .language_for_path(std::path::Path::new("foo.py"))
        .is_none());
}

#[test]
fn grammar_registry_unknown_language_returns_none() {
    let temp_dir = TempDir::new().unwrap();
    let grammar_dir = temp_dir.path().join("prisma");
    std::fs::create_dir_all(&grammar_dir).unwrap();
    std::fs::write(
        grammar_dir.join("manifest.toml"),
        r#"
language = "prisma"
wasm_file = "tree-sitter-prisma.wasm"
extensions = [".prisma"]
display_name = "Prisma"

[queries]
symbol_exists = "queries/symbol_exists.scm"
"#,
    )
    .unwrap();

    let registry = GrammarRegistry::load(temp_dir.path()).unwrap();
    assert!(!registry.has_language("python"));
    assert_eq!(registry.display_name("python"), "bound");
}

#[test]
fn grammar_registry_loads_boundary_references_query() {
    let temp_dir = TempDir::new().unwrap();
    let grammar_dir = temp_dir.path().join("rust");
    std::fs::create_dir_all(&grammar_dir).unwrap();
    std::fs::write(
        grammar_dir.join("manifest.toml"),
        r#"
language = "rust"
wasm_file = "tree-sitter-rust.wasm"
extensions = [".rs"]

[queries]
symbol_exists = "queries/symbol_exists.scm"

[queries.boundary_references]
source = """
[
  (use_declaration)
  (call_expression)
] @reference
"""
"#,
    )
    .unwrap();

    let registry = GrammarRegistry::load(temp_dir.path()).unwrap();
    let query = registry.boundary_references_query("rust").unwrap();
    assert!(query.contains("use_declaration"));
    assert!(query.contains("@reference"));
    assert!(registry.boundary_references_query("python").is_none());
}
