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

## Tooling & CI

- [x] Add `kite fmt` — auto-formatter for `.kite` files
- [x] Add `kite init` — scaffold a new domain file from an existing codebase
- [x] CI: run `cargo test` + `kite check examples/` on every PR
- [ ] Publish VS Code extension to marketplace (defer, do not do this)
