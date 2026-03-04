# Kite TODO

## Language & Parser

- [x] Validate `hash` clause in shipping-co (add SHA-256 hashes to critical bindings)
- [x] Validate inline `block` bodies work end-to-end (alternative to `bound to`)
- [x] Add `Decimal`, `Date`, `Void` primitive coverage to the example
- [x] Consider optional `description` strings on aggregates, commands, invariants

## Diagnostics & Validation

- [x] Arity check: add a shipping-co binding that intentionally mismatches to demo the diagnostic
- [x] Intent check: add a write→read mismatch to demo `COMMAND_BINDING_INTENT_SUSPICIOUS`
- [x] Dictionary: add a bound source that violates a forbidden term to demo the diagnostic
- [x] Boundary: add a cross-context reference to demo `CONTEXT_BOUNDARY_FORBIDDEN`
- [x] Explore: warn when two aggregates in different contexts bind to the same file (shared kernel smell)
- [x] Explore: detect unused aggregate fields (declared but never referenced)

## LSP

- [x] Hover support: show aggregate/command/invariant docs on hover
- [x] Completion: suggest context names in `forbid`, symbol names from bound files
- [x] Rename: rename a command/invariant across the `.kite` file
- [x] Semantic tokens: richer highlighting via LSP (complement TextMate grammar)
- [x] Workspace diagnostics: check all `.kite` files, not just the open one
- [x] Document outline: Dictionary, Boundary, and Field elements in the symbol tree
- [x] Fix: spawn diagnostics in background to prevent request pipeline hang

## Grammar Support

- [x] Add Go (`tree-sitter-go`) grammar with symbol + boundary queries
- [x] Add Python (`tree-sitter-python`) grammar
- [x] Add C# (`tree-sitter-c-sharp`) grammar
- [x] Prisma: add `boundary_references` query (currently unsupported)

## Example: shipping-co

- [x] Make dictionary terms actually trigger in bound sources (add realistic violations)
- [x] Add `hash` clauses to lock down critical service files
- [x] Add a second `.kite` file (e.g. `infra.kite`) to test multi-file workflows
- [x] Add integration test that runs `kite check` on shipping-co as a regression gate

## Testing

- [x] Regression tests: `expected-pass` / `expected-fail` fixture files in `examples/shipping-co/domain/regressions/` assert specific violation codes (`COMMAND_BINDING_ARITY_MISMATCH`, `CONTEXT_BOUNDARY_FORBIDDEN`, `BINDING_SYMBOL_NOT_FOUND`)
- [x] VS Code integration tests: extension activation + document symbol pipeline tested in real Extension Development Host via `@vscode/test-cli` — run with `cd vscode-kite && npm test`
- [ ] Add more regression fixtures as new violation types are added
- [ ] CI: run VS Code integration tests (needs `xvfb-run` or headless Electron)

## Tooling & CI

- [x] Add `kite fmt` — auto-formatter for `.kite` files
- [x] Add `kite init` — scaffold a new domain file from an existing codebase
- [x] CI: run `cargo test` + `kite check examples/` on every PR
- [-] Publish VS Code extension to marketplace (defer, do not do this)

## Enriched Diagnostics

- [ ] Include source code snippets in diagnostic messages (e.g. for `COMMAND_BINDING_ARITY_MISMATCH`, show the function signature from the bound source file as a multiline message) — diagnostic messages support multiline via `\n`, but no markdown
- [ ] Include clickable file paths in diagnostic messages — VS Code auto-links `path/to/file.rs:42:5` format; test which format feels best in the Problems panel and hover tooltip
- [ ] Explore using `codeDescription.href` with `file:///` URIs to link directly from the diagnostic code to the offending source location — the LSP already uses `codeDescription` for docs links, could add source file links too

## Source File Decorations

The goal: make bound source files "aware" of their kite specifications. Similar to how test runners decorate tested code, but for architectural conformance. All features behind `kite.decorations.*` settings flags, defaulting to on.

- [ ] Gutter/margin indicators on bound source files showing kite association status (pass/fail/warning) — similar to test runner coverage indicators
- [ ] Explore using a small kite icon (custom `gutterIconPath`) for lines referenced by kite bindings, coloured red/amber/green by validation state
- [ ] Inlay hints on bound source symbols showing the associated kite spec (e.g. `← Order.ship`) — note: the extension already has an inlay hints provider (`sourceInlayHintsProvider`) that could be extended
- [ ] Decide: gutter icons vs inlay hints vs both — prototype both and see what feels right
- [ ] Add a "Find Related Kite Specifications" command (reverse lookup: given a source file + symbol, find all `.kite` entries that bind to it)
- [ ] Explore CodeLens as an alternative/complement — show "Referenced by: Order.ship (SalesContext)" above bound functions
