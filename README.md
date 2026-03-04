Here is the `README.md` for the repository. It pitches the philosophy, explains the Tree-sitter magic, and gives a clear "getting started" vibe.

---

# 🪁 Kite

**Kite is a continuous architecture enforcement tool for Domain-Driven Design (DDD).** It provides a Domain-Specific Language (`.kite`) to define your Bounded Contexts, Aggregates, and Sagas. But unlike traditional Model-Driven tools, **Kite does not generate code.** Instead, it uses [Tree-sitter](https://tree-sitter.github.io/tree-sitter/) (via [this `rust-sitter` fork](https://github.com/benbenbenbenbenben/krust-sitter)) to parse your actual implementation files (currently Rust and TypeScript/TSX) and validates that your codebase structurally matches your architectural design.

If your code drifts from your domain model, the Kite compiler fails. **Technical debt is now a syntax error.**

---

## The Problem: Architecture Drift

You start a project with a beautiful whiteboard session. You define strict Bounded Contexts, clear Aggregates, and a Ubiquitous Language.

Six months later:

* An "Aggregate Root" has public setters everywhere.
* The `Logistics` context is directly querying the `Identity` database.
* The codebase uses the term "User", but the business team calls them "Patrons".

The whiteboard lied. The code is the only truth.

## The Solution: Binding Contracts

Kite flips the script. You define the rules of the domain in a `.kite` file, and **bind** those rules to your implementation files. Kite acts as a Meta-Language Server, constantly diffing your Domain Abstract Syntax Tree against your Code Concrete Syntax Tree.

### 1. Write the Domain Spec (`sales.kite`)

```kite
context SalesContext {
    dictionary {
        "User" => forbidden // We use 'Customer' here
    }

    aggregate Order bound to "src/domain/order.rs" {
        
        // State mutations must be explicit commands
        command ship() 
            bound symbol "Order::ship";
            
        // Invariants must be explicitly handled in the code
        invariant MustHaveItems 
            bound symbol "Order::verify_not_empty";
    }
}

```

### 2. Write your Code (`src/domain/order.rs`)

Write your code however you like. Kite only cares about the structural contract.

```rust
impl Order {
    // Kite verifies the bound symbol exists in the target file
    pub fn ship(&mut self) -> Result<(), DomainError> {
        self.verify_not_empty()?;
        self.status = OrderStatus::Shipped;
        Ok(())
    }

    pub fn verify_not_empty(&self) -> Result<(), DomainError> {
        if self.items.is_empty() {
            Err(DomainError::EmptyOrder)
        } else {
            Ok(())
        }
    }
}

```

### 3. Run the Verifier

Run Kite in your CI/CD pipeline or as an LSP in your editor.

```bash
$ kite check

🔍 Analyzing Domain: SalesContext
✅ Dictionary verified.
✅ Aggregate 'Order' found in src/domain/order.rs.
✅ Command 'ship()' signature matches implementation.
✅ Invariant 'MustHaveItems' verified.

🪁 All contexts verified. 0 Drift detected.

```

---

## What happens when you drift?

Imagine a junior developer tries to add a shortcut to the Rust code by adding arguments to the `ship` function, bypassing the domain rules.

```rust
// Developer modifies the Rust code:
pub fn ship(&mut self, bypass_checks: bool) { ... }

```

Kite catches this instantly using structural AST diffing:

```bash
$ kite check

❌ DRIFT DETECTED IN SalesContext

🔗 Binding Violation in aggregate 'Order'
   -> src/domain/order.rs

The bound method `Order::ship` signature does not match the Domain Spec.
  Expected: ship()
  Found:    ship(bypass_checks: bool)

Architectural rule broken: State mutation commands cannot accept arbitrary control flags.
Update your .kite file if the business rules have changed, or revert the code.

```

---

## How it Works (Under the Hood)

Kite is built in **Rust** and leverages `rust-sitter`.

1. **The Parser**: Parses `.kite` files into a strongly-typed Domain AST.
2. **The Adapter Engine**: Reads the `bound to` directives and loads the appropriate Tree-sitter grammar (e.g., `tree-sitter-rust`, `tree-sitter-typescript`).
3. **The Query Engine**: Runs pre-compiled S-expression (`.scm`) queries against your source files to find classes, structs, methods, and parameters.
4. **The Validator**: Compares the shapes. If the Domain expects an Immutable Value Object, Kite verifies the Rust struct has no mutable `&mut self` methods exposed.

---

## The Ecosystem

Kite is part of the **kodus ecosystem**, a suite of tools designed for high-assurance, easily modeled distributed systems:

* **Kodus**: The secure server runtime.
* **Kettu**: The agile, WASM-native implementation language.
* **Karu**: The strict security and authorization policy language.
* **Kite**: The structural domain and architecture verifier.

*(Note: Kite works perfectly as a standalone tool for existing Rust, Go, or TypeScript projects!)*

---

## Getting Started

### Installation

```bash
cargo build --release -p kite-cli
```

### Usage

Validate a `.kite` file:

```bash
kite check domain/main.kite
```

Start the integrated LSP server (stdio transport):

```bash
kite start-lsp
```

### Diagnostics and editor metadata

- `kite check` emits stable diagnostic codes with severity, e.g. `error [BINDING_SYMBOL_NOT_FOUND] ...`.
- Dictionary rules run against source files bound within each context (`aggregate`, `command`, and `invariant` bindings).
- `"Term" => forbidden` emits `error [DICTIONARY_TERM_FORBIDDEN]`.
- `"Term" => "Preferred"` emits `warning [DICTIONARY_TERM_PREFERRED]` with a replacement hint.
- Duplicate dictionary keys in the same block emit `error [DICTIONARY_DUPLICATE_KEY]`.
- `boundary { forbid OtherContext }` emits `error [CONTEXT_BOUNDARY_FORBIDDEN]` when forbidden context dependencies are referenced from bound Rust/TypeScript/TSX source files (imports/uses, type references, and call/new references); if structured extraction fails, Kite falls back to token matching.
- Duplicate `forbid` entries in the same boundary block emit `error [CONTEXT_BOUNDARY_DUPLICATE_FORBID]`.
- `boundary` self-forbid entries (`forbid CurrentContextName`) emit `warning [CONTEXT_BOUNDARY_SELF_FORBID]` with a fix hint.
- If a `bound to` file is missing, Kite emits `error [BINDING_FILE_NOT_FOUND]` plus `warning [BINDING_SYMBOL_UNVERIFIED_DEPENDENCY]` for dependent `symbol` checks.
- `hash "<value>"` must be lowercase SHA-256 hex (`64` chars) or Kite emits `error [BINDING_HASH_INVALID_FORMAT]`.
- If a bound file exists and `hash` is present, Kite compares the declared hash against file contents and emits `error [BINDING_HASH_MISMATCH]` when they differ.
- `error [BINDING_SYMBOL_NOT_FOUND]` includes a nearest-symbol hint when a close declaration exists in supported bound files.
- `error [COMMAND_BINDING_ARITY_MISMATCH]` is emitted when a command's parameter count does not match a bound symbol's arity in Rust (`.rs`), TypeScript/TSX (`.ts`, `.tsx`), or Prisma (`.prisma` declarations are treated as zero-arity).
- `warning [COMMAND_BINDING_INTENT_SUSPICIOUS]` is emitted when a write-oriented command (e.g. `create`, `ship`, `delete`) is bound to a read-oriented symbol (e.g. `get*`, `list*`, `find*`, `read*`).
- The LSP publishes `Diagnostic.code` for rule IDs and `Diagnostic.codeDescription.href` when docs are available.
- When present, diagnostic metadata is included in `Diagnostic.data` as `{ "code", "hint", "docsUri" }` to improve editor UX.

### Symbol validation coverage

- `bound symbol` validation is implemented for Rust (`.rs`), TypeScript (`.ts`, `.tsx`), and Prisma (`.prisma`).
- TypeScript symbol matching accepts scoped spellings like `Order::ship`, `Order.ship`, and `Order#ship` (resolved to the leaf symbol name).
- Prisma symbol matching validates declaration names from `model`, `enum`, `type`, `view`, `datasource`, and `generator` blocks.

### Supported targets

Native single-file executables:

- Linux: `x86_64-unknown-linux-musl` (static)
- Windows: `x86_64-pc-windows-msvc` (static CRT)
- macOS: `x86_64-apple-darwin`, `aarch64-apple-darwin`

WASI artifacts:

- `kite-cli.wasm` (CLI flavor, built from `kite-cli`)
- `kite.wasm` (runtime flavor, built from `kite`)

Build scripts:

```bash
./scripts/build-native.sh
./scripts/build-wasm.sh
```

`build-wasm.sh` expects `clang`/`ar` and WASI headers (`wasi-libc`); override via `WASI_INCLUDE`, `CC_wasm32_wasip1`, `AR_wasm32_wasip1`, and `CFLAGS_wasm32_wasip1` if needed.

### VS Code extension

`vscode-kite/` contains the extension that launches `kite start-lsp`.

- Debug config: `.vscode/launch.json`
- Build/watch tasks: `.vscode/tasks.json`

Run extension compile manually:

```bash
cd vscode-kite
npm install
npm run compile
```

### Grammar registry

Kite loads Tree-sitter query mappings from `grammars/<language>/manifest.toml`.

- Rust grammar assets are vendored in `grammars/rust/tree-sitter-rust-0.24.0`.
- `grammars/rust/queries/symbol_exists.scm` is used for symbol existence checks.
- `grammars/typescript/queries/symbol_exists.scm` is used for TypeScript/TSX symbol existence checks.
- Prisma grammar assets are vendored in `grammars/prisma/tree-sitter-prisma-1.6.0`.

### Runtime-configurable adapters

Grammar manifests can declare adapter runtime metadata under `[adapter]`, with runtime-specific entries (`[adapter.native]`, `[adapter.wasm]`) that provide `backend_kind` and `module` identifiers.

- **Native path**: use `backend_kind = "wasmtime_wasm"` to run wasm adapters via a Wasmtime host in native builds.
- **Wasm target path**: use `backend_kind = "js_bridge"` to route adapter calls through a JavaScript bridge when running wasm targets.
- **Compatibility/fallback**: set `wasm_fallback_to_native = true` to use native adapter metadata when no explicit wasm adapter entry is present.

---

## Contributing

We are currently expanding support for new languages via the **Grammar Registry**.

To add support for a new language (e.g., Go, Python, C#):

1. **Add the Grammar**: Place the `tree-sitter-<lang>.wasm` file in `grammars/<lang>/`.
2. **Define the Manifest**: Create `grammars/<lang>/manifest.toml` declaring the extensions and query paths.
3. **Write Queries**:
   * `symbol_exists.scm`: A Tree-sitter query that captures the `@name` of declarations (classes, functions, etc.).
   * `boundary_references`: (Optional) A query in the manifest or a separate file that captures `@reference` tokens for architectural boundary enforcement.

Check out the existing [Rust](grammars/rust/) or [TypeScript](grammars/typescript/) definitions for examples.

**License**: MIT
