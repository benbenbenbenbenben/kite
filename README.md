Here is the `README.md` for the repository. It pitches the philosophy, explains the Tree-sitter magic, and gives a clear "getting started" vibe.

---

# 💎 Kide

> **kide** *(Finnish)*: Crystal. The crystallized, immutable truth of a system.

**Kide is a continuous architecture enforcement tool for Domain-Driven Design (DDD).** It provides a Domain-Specific Language (`.kide`) to define your Bounded Contexts, Aggregates, and Sagas. But unlike traditional Model-Driven tools, **Kide does not generate code.** Instead, it uses [Tree-sitter](https://tree-sitter.github.io/tree-sitter/) (via [this `rust-sitter` fork](https://github.com/benbenbenbenbenben/krust-sitter)) to parse your actual implementation files (Rust, TypeScript, Go, etc.) and validates that your codebase structurally matches your architectural design.

If your code drifts from your domain model, the Kide compiler fails. **Technical debt is now a syntax error.**

---

## The Problem: Architecture Drift

You start a project with a beautiful whiteboard session. You define strict Bounded Contexts, clear Aggregates, and a Ubiquitous Language.

Six months later:

* An "Aggregate Root" has public setters everywhere.
* The `Logistics` context is directly querying the `Identity` database.
* The codebase uses the term "User", but the business team calls them "Patrons".

The whiteboard lied. The code is the only truth.

## The Solution: Binding Contracts

Kide flips the script. You define the rules of the domain in a `.kide` file, and **bind** those rules to your implementation files. Kide acts as a Meta-Language Server, constantly diffing your Domain Abstract Syntax Tree against your Code Concrete Syntax Tree.

### 1. Write the Domain Spec (`sales.kide`)

```kide
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

Write your code however you like. Kide only cares about the structural contract.

```rust
impl Order {
    // Kide's rust-sitter engine verifies this exists and takes 0 arguments
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

Run Kide in your CI/CD pipeline or as an LSP in your editor.

```bash
$ kide check

🔍 Analyzing Domain: SalesContext
✅ Dictionary verified.
✅ Aggregate 'Order' found in src/domain/order.rs.
✅ Command 'ship()' signature matches implementation.
✅ Invariant 'MustHaveItems' verified.

✨ All contexts crystallized. 0 Drift detected.

```

---

## What happens when you drift?

Imagine a junior developer tries to add a shortcut to the Rust code by adding arguments to the `ship` function, bypassing the domain rules.

```rust
// Developer modifies the Rust code:
pub fn ship(&mut self, bypass_checks: bool) { ... }

```

Kide catches this instantly using structural AST diffing:

```bash
$ kide check

❌ DRIFT DETECTED IN SalesContext

🔗 Binding Violation in aggregate 'Order'
   -> src/domain/order.rs

The bound method `Order::ship` signature does not match the Domain Spec.
  Expected: ship()
  Found:    ship(bypass_checks: bool)

Architectural rule broken: State mutation commands cannot accept arbitrary control flags.
Update your .kide file if the business rules have changed, or revert the code.

```

---

## How it Works (Under the Hood)

Kide is built in **Rust** and leverages `rust-sitter`.

1. **The Parser**: Parses `.kide` files into a strongly-typed Domain AST.
2. **The Adapter Engine**: Reads the `bound to` directives and loads the appropriate Tree-sitter grammar (e.g., `tree-sitter-rust`, `tree-sitter-typescript`).
3. **The Query Engine**: Runs pre-compiled S-expression (`.scm`) queries against your source files to find classes, structs, methods, and parameters.
4. **The Validator**: Compares the shapes. If the Domain expects an Immutable Value Object, Kide verifies the Rust struct has no mutable `&mut self` methods exposed.

---

## The Ecosystem

Kide is part of the **K-Stack**, a suite of tools designed for high-assurance, easily modeled distributed systems:

* **Kodus**: The secure server runtime (Home).
* **Kettu**: The agile, WASM-native implementation language (Fox).
* **Karu**: The strict security and authorization policy language (Bear).
* **Kide**: The structural domain and architecture verifier (Crystal).

*(Note: Kide works perfectly as a standalone tool for existing Rust, Go, or TypeScript projects!)*

---

## Getting Started

### Installation

*(Coming soon)*

```bash
cargo install kide-cli

```

### Usage

Initialize a new Kide workspace:

```bash
kide init

```

This creates a `domain/` folder with a `main.kide` entry point.

Run the verifier:

```bash
kide check --strict

```

---

## Contributing

We are currently building the core `rust-sitter` query adapters for standard languages.

* To help build the **Rust Adapter**, check out `src/adapters/rust.rs`.
* To help build the **TypeScript Adapter**, check out `src/adapters/typescript.rs`.

**License**: MIT