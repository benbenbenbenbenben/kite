#![allow(dead_code)]

#[path = "../src/grammar_registry.rs"]
mod grammar_registry;

use grammar_registry::{AdapterRuntime, GrammarRegistry};
use tempfile::TempDir;

#[test]
fn adapter_config_loads_and_selects_requested_runtime() {
    let temp_dir = TempDir::new().unwrap();
    let grammar_dir = temp_dir.path().join("rust");
    std::fs::create_dir_all(&grammar_dir).unwrap();
    std::fs::write(
        grammar_dir.join("manifest.toml"),
        r#"
language = "rust"

[adapter]
wasm_fallback_to_native = true

[adapter.native]
backend_kind = "wasmtime_wasm"
module = "kide.adapters.rust.native"

[adapter.wasm]
backend_kind = "js_bridge"
module = "kide.adapters.rust.wasm"

[queries]
symbol_exists = "queries/symbol_exists.scm"
"#,
    )
    .unwrap();

    let registry = GrammarRegistry::load(temp_dir.path()).unwrap();
    let native = registry
        .adapter_for("rust", AdapterRuntime::Native)
        .unwrap();
    let wasm = registry.adapter_for("rust", AdapterRuntime::Wasm).unwrap();

    assert_eq!(native.backend_kind, "wasmtime_wasm");
    assert_eq!(native.module, "kide.adapters.rust.native");
    assert_eq!(wasm.backend_kind, "js_bridge");
    assert_eq!(wasm.module, "kide.adapters.rust.wasm");
}

#[test]
fn wasm_runtime_can_fallback_to_native_when_configured() {
    let temp_dir = TempDir::new().unwrap();
    let grammar_dir = temp_dir.path().join("typescript");
    std::fs::create_dir_all(&grammar_dir).unwrap();
    std::fs::write(
        grammar_dir.join("manifest.toml"),
        r#"
language = "typescript"

[adapter]
wasm_fallback_to_native = true

[adapter.native]
backend_kind = "wasmtime_wasm"
module = "kide.adapters.typescript.native"

[queries]
symbol_exists = "queries/symbol_exists.scm"
"#,
    )
    .unwrap();

    let registry = GrammarRegistry::load(temp_dir.path()).unwrap();
    let wasm = registry
        .adapter_for("typescript", AdapterRuntime::Wasm)
        .unwrap();

    assert_eq!(wasm.backend_kind, "wasmtime_wasm");
    assert_eq!(wasm.module, "kide.adapters.typescript.native");
}

#[test]
fn wasm_runtime_returns_none_without_config_or_fallback() {
    let temp_dir = TempDir::new().unwrap();
    let grammar_dir = temp_dir.path().join("prisma");
    std::fs::create_dir_all(&grammar_dir).unwrap();
    std::fs::write(
        grammar_dir.join("manifest.toml"),
        r#"
language = "prisma"

[adapter]
wasm_fallback_to_native = false

[adapter.native]
backend_kind = "wasmtime_wasm"
module = "kide.adapters.prisma.native"

[queries]
symbol_exists = "queries/symbol_exists.scm"
"#,
    )
    .unwrap();

    let registry = GrammarRegistry::load(temp_dir.path()).unwrap();
    let wasm = registry.adapter_for("prisma", AdapterRuntime::Wasm);

    assert!(wasm.is_none());
}
