# Copilot instructions for Kide

## Build and test commands

### Rust workspace
- Build CLI release binary: `cargo build --release -p kide-cli`
- Check all crates: `cargo check --workspace`
- Run parser tests: `cargo test -p kide-parser`
- Run core validation suite: `cargo test -p kide-core --test validation`
- Run core adapter-manifest/runtime suite: `cargo test -p kide-core --test adapter_manifest_runtime`
- Run CLI LSP-focused tests: `cargo test -p kide-cli lsp::tests::`

### Run a single test
- Single core validation test:  
  `cargo test -p kide-core --test validation rust_command_arity_mismatch_produces_error`
- Single parser test:  
  `cargo test -p kide-parser parse_reports_edge_case_failures_with_stable_error_context`
- Single LSP unit test:  
  `cargo test -p kide-cli lsp::tests::builds_quick_fix_actions_for_missing_symbol_suggestions`

### Packaging / target builds
- Native multi-target build script: `./scripts/build-native.sh`
- WASI build script: `./scripts/build-wasm.sh`

### Shipping regression corpus
- Run expected pass/fail domain scenarios: `./scripts/run-shipping-regressions.sh`

### VS Code extension
- Install deps: `npm --prefix vscode-kide install`
- Compile extension: `npm --prefix vscode-kide run compile`

## High-level architecture

- `crates/kide-parser` defines the `.kide` grammar with `rust-sitter` (`krust-sitter` fork) and produces a strongly typed AST (`Program`, `Context`, `Aggregate`, `Binding`, etc.).  
- `crates/kide-core` is the enforcement engine:
  - parses `.kide` input through `kide-parser`
  - loads language metadata from `grammars/*/manifest.toml` via `grammar_registry.rs`
  - runs diagnostics/validation and definition lookup
  - routes language operations through `adapter_runtime.rs` (runtime-configurable adapter backend selection).
- `crates/kide-cli` is the executable:
  - `kide check` prints violations and uses exit code `1` when any error-severity violation exists
  - `kide start-lsp` hosts Tower LSP server (`crates/kide-cli/src/lsp.rs`) with diagnostics, go-to-definition, document symbols, and code actions.
- `vscode-kide` starts the language client/server pair, watches relevant files, and now augments source files with Kide-driven inlay hints/decorations based on diagnostics.

## Key repository conventions

- Diagnostics are contract-stable and code-first:
  - every rule uses a stable `CODE_*` identifier and docs URI constant in `kide-core`
  - violations carry severity, code, message, hint, docs URI, and optional span.
- Span convention: parser/core spans are 1-based; LSP conversion to 0-based happens in `crates/kide-cli/src/lsp.rs`.
- Grammar + adapter behavior is manifest-driven:
  - language query mapping and adapter runtime metadata live in `grammars/<language>/manifest.toml`
  - adapter runtime selection is configuration-based (`native` / `wasm` entries, optional wasm fallback).
- New language/rule changes are expected to update tests in the same pass:
  - parser grammar behavior: `crates/kide-parser/tests/*`
  - enforcement/diagnostics behavior: `crates/kide-core/tests/validation.rs` (and focused suites like adapter runtime tests)
  - LSP behavior: `crates/kide-cli/src/lsp.rs` unit tests.
- Shipping domain regression scenarios use explicit expected outcome fixtures under `shipping-co/domain/regressions/*.kide`; keep `scripts/run-shipping-regressions.sh` aligned when adding/removing scenarios.
