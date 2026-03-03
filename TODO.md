# Kide TODO

## Language & Parser

- [ ] Validate `hash` clause in shipping-co (add SHA-256 hashes to critical bindings)
- [ ] Validate inline `block` bodies work end-to-end (alternative to `bound to`)
- [ ] Add `Decimal`, `Date`, `Void` primitive coverage to the example
- [ ] Consider optional `description` strings on aggregates, commands, invariants

## Diagnostics & Validation

- [ ] Arity check: add a shipping-co binding that intentionally mismatches to demo the diagnostic
- [ ] Intent check: add a write→read mismatch to demo `COMMAND_BINDING_INTENT_SUSPICIOUS`
- [ ] Dictionary: add a bound source that violates a forbidden term to demo the diagnostic
- [ ] Boundary: add a cross-context reference to demo `CONTEXT_BOUNDARY_FORBIDDEN`
- [ ] Explore: warn when two aggregates in different contexts bind to the same file (shared kernel smell)
- [ ] Explore: detect unused aggregate fields (declared but never referenced)

## LSP

- [x] Hover support: show aggregate/command/invariant docs on hover
- [ ] Completion: suggest context names in `forbid`, symbol names from bound files
- [ ] Rename: rename a command/invariant across the `.kide` file
- [ ] Semantic tokens: richer highlighting via LSP (complement TextMate grammar)
- [ ] Workspace diagnostics: check all `.kide` files, not just the open one

## Grammar Support

- [ ] Add Go (`tree-sitter-go`) grammar with symbol + boundary queries
- [ ] Add Python (`tree-sitter-python`) grammar
- [ ] Add C# (`tree-sitter-c-sharp`) grammar
- [ ] Prisma: add `boundary_references` query (currently unsupported)

## Example: shipping-co

- [ ] Make dictionary terms actually trigger in bound sources (add realistic violations)
- [x] Add `hash` clauses to lock down critical service files
- [ ] Add a second `.kide` file (e.g. `infra.kide`) to test multi-file workflows
- [x] Add integration test that runs `kide check` on shipping-co as a regression gate

## Tooling & CI

- [ ] Add `kide fmt` — auto-formatter for `.kide` files
- [ ] Add `kide init` — scaffold a new domain file from an existing codebase
- [ ] CI: run `cargo test` + `kide check examples/` on every PR
- [ ] Publish VS Code extension to marketplace
