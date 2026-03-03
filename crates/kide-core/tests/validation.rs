use kide_core::{
    check_file, definition_at, ViolationSeverity, CODE_BINDING_FILE_EMPTY,
    CODE_BINDING_FILE_NOT_FOUND, CODE_BINDING_HASH_INVALID_FORMAT, CODE_BINDING_HASH_MISMATCH,
    CODE_BINDING_SYMBOL_MISSING, CODE_BINDING_SYMBOL_NOT_FOUND,
    CODE_BINDING_SYMBOL_UNVERIFIED_DEPENDENCY, CODE_COMMAND_BINDING_ARITY_MISMATCH,
    CODE_COMMAND_BINDING_INTENT_SUSPICIOUS, CODE_CONTEXT_BOUNDARY_DUPLICATE_FORBID,
    CODE_CONTEXT_BOUNDARY_FORBIDDEN, CODE_CONTEXT_BOUNDARY_SELF_FORBID,
    CODE_DICTIONARY_DUPLICATE_KEY, CODE_DICTIONARY_TERM_FORBIDDEN, CODE_DICTIONARY_TERM_PREFERRED,
    DOCS_BINDING_FILE_EMPTY, DOCS_BINDING_HASH_INVALID_FORMAT, DOCS_BINDING_HASH_MISMATCH,
    DOCS_BINDING_SYMBOL_MISSING, DOCS_BINDING_SYMBOL_NOT_FOUND,
    DOCS_COMMAND_BINDING_ARITY_MISMATCH, DOCS_COMMAND_BINDING_INTENT_SUSPICIOUS,
    DOCS_CONTEXT_BOUNDARY_DUPLICATE_FORBID, DOCS_CONTEXT_BOUNDARY_FORBIDDEN,
    DOCS_CONTEXT_BOUNDARY_SELF_FORBID, DOCS_DICTIONARY_DUPLICATE_KEY,
    DOCS_DICTIONARY_TERM_FORBIDDEN, DOCS_DICTIONARY_TERM_PREFERRED,
};
use std::path::Path;
use tempfile::TempDir;

#[test]
fn missing_bound_file_produces_error_and_dependent_warning() {
    let temp_dir = TempDir::new().unwrap();
    let kide_path = temp_dir.path().join("main.kide");
    std::fs::write(
        &kide_path,
        r#"
context SalesContext {
  aggregate Order {
    command ship() bound to "src/domain/order.rs" symbol "Order::ship"
  }
}
"#,
    )
    .unwrap();

    let report = check_file(&kide_path).unwrap();

    let missing_file = report
        .violations
        .iter()
        .find(|violation| {
            violation.code == CODE_BINDING_FILE_NOT_FOUND
                && violation.severity == ViolationSeverity::Error
        })
        .unwrap();
    assert!(missing_file.hint.is_some());
    assert!(missing_file.docs_uri.is_some());
    assert!(missing_file.span.is_some());
    assert_eq!(missing_file.span.unwrap().start_line, 4);

    let dependent_symbol_warning = report
        .violations
        .iter()
        .find(|violation| {
            violation.code == CODE_BINDING_SYMBOL_UNVERIFIED_DEPENDENCY
                && violation.severity == ViolationSeverity::Warning
        })
        .unwrap();
    assert!(dependent_symbol_warning.span.is_some());
    assert_eq!(dependent_symbol_warning.span.unwrap().start_line, 4);
}

#[test]
fn missing_symbol_in_existing_file_produces_symbol_error() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("src/domain/order.rs"),
        r#"
impl Order {
    pub fn something_else(&mut self) {}
}
"#,
    );
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"
context SalesContext {
  aggregate Order {
    command ship() bound to "src/domain/order.rs" symbol "Order::ship"
  }
}
"#,
    );

    let report = check_file(&kide_path).unwrap();

    let missing_symbol = report
        .violations
        .iter()
        .find(|violation| {
            violation.code == CODE_BINDING_SYMBOL_NOT_FOUND
                && violation.severity == ViolationSeverity::Error
        })
        .unwrap();
    assert_eq!(
        missing_symbol.hint.as_deref(),
        Some("check the symbol name and ensure it is declared in the bound file")
    );
    assert!(missing_symbol.docs_uri.is_some());
    assert!(missing_symbol.span.is_some());
    assert_eq!(missing_symbol.span.unwrap().start_line, 4);
}

#[test]
fn rust_missing_symbol_with_near_match_suggests_symbol() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("src/domain/order.rs"),
        r#"
impl Order {
    pub fn ship(&mut self) {}
}
"#,
    );
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"
context SalesContext {
  aggregate Order {
    command ship() bound to "src/domain/order.rs" symbol "Order::shpi"
  }
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    let missing_symbol = report
        .violations
        .iter()
        .find(|violation| violation.code == CODE_BINDING_SYMBOL_NOT_FOUND)
        .unwrap();
    assert_eq!(missing_symbol.hint.as_deref(), Some("did you mean 'ship'?"));
}

#[test]
fn symbol_found_produces_no_violations() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("src/domain/order.rs"),
        r#"
impl Order {
    pub fn ship(&mut self) {}
}
"#,
    );
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"
context SalesContext {
  aggregate Order {
    command ship() bound to "src/domain/order.rs" symbol "Order::ship"
  }
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    assert!(report.violations.is_empty());
}

#[test]
fn grammar_with_wasm_file_resolves_symbol() {
    let temp_dir = TempDir::new().unwrap();
    create_rust_grammar(temp_dir.path());
    create_file(
        &temp_dir.path().join("src/domain/order.rs"),
        r#"
impl Order {
    pub fn ship(&mut self) {}
}
"#,
    );
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"
context SalesContext {
  aggregate Order {
    command ship() bound to "src/domain/order.rs" symbol "Order::ship"
  }
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    assert!(report.violations.is_empty());
}

#[test]
fn rust_command_arity_mismatch_produces_error() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("src/domain/order.rs"),
        r#"
pub fn ship(order_id: i32, priority: i32) {}
"#,
    );
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"
context SalesContext {
  aggregate Order {
    command ship(order_id: Int) bound to "src/domain/order.rs" symbol "ship"
  }
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    let violation = report
        .violations
        .iter()
        .find(|violation| violation.code == CODE_COMMAND_BINDING_ARITY_MISMATCH)
        .unwrap();
    assert_eq!(violation.severity, ViolationSeverity::Error);
    assert_eq!(
        violation.docs_uri,
        Some(DOCS_COMMAND_BINDING_ARITY_MISMATCH)
    );
    assert!(violation.message.contains("Rust symbol"));
    assert!(violation.message.contains("declares 1 parameter(s)"));
    assert!(violation.message.contains("expects 2 parameter(s)"));
    assert!(violation.hint.is_some());
    assert!(violation.span.is_some());
    assert_eq!(violation.span.unwrap().start_line, 4);
}

#[test]
fn rust_command_arity_ignores_self_parameter() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("src/domain/order.rs"),
        r#"
impl Order {
    pub fn ship(&mut self, order_id: i32) {}
}
"#,
    );
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"
context SalesContext {
  aggregate Order {
    command ship(order_id: Int) bound to "src/domain/order.rs" symbol "Order::ship"
  }
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    assert!(report
        .violations
        .iter()
        .all(|violation| violation.code != CODE_COMMAND_BINDING_ARITY_MISMATCH));
}

#[test]
fn rust_command_arity_match_produces_no_mismatch() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("src/domain/order.rs"),
        r#"
pub fn ship(order_id: i32) {}
"#,
    );
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"
context SalesContext {
  aggregate Order {
    command ship(order_id: Int) bound to "src/domain/order.rs" symbol "ship"
  }
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    assert!(report
        .violations
        .iter()
        .all(|violation| violation.code != CODE_COMMAND_BINDING_ARITY_MISMATCH));
}

#[test]
fn write_command_bound_to_read_symbol_produces_warning() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("src/domain/order.rs"),
        r#"
impl Order {
    pub fn get_order(&self) {}
}
"#,
    );
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"
context SalesContext {
  aggregate Order {
    command ship() bound to "src/domain/order.rs" symbol "Order::get_order"
  }
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    let violation = report
        .violations
        .iter()
        .find(|violation| {
            violation.code == CODE_COMMAND_BINDING_INTENT_SUSPICIOUS
                && violation.severity == ViolationSeverity::Warning
        })
        .unwrap();
    assert_eq!(
        violation.docs_uri,
        Some(DOCS_COMMAND_BINDING_INTENT_SUSPICIOUS)
    );
    assert!(violation.hint.is_some());
    assert!(violation.span.is_some());
    assert_eq!(violation.span.unwrap().start_line, 4);
}

#[test]
fn write_command_bound_to_write_symbol_has_no_intent_warning() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("src/domain/order.rs"),
        r#"
impl Order {
    pub fn ship(&mut self) {}
}
"#,
    );
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"
context SalesContext {
  aggregate Order {
    command ship() bound to "src/domain/order.rs" symbol "Order::ship"
  }
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    assert!(report
        .violations
        .iter()
        .all(|violation| violation.code != CODE_COMMAND_BINDING_INTENT_SUSPICIOUS));
}

#[test]
fn read_command_bound_to_read_symbol_has_no_intent_warning() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("src/domain/order.rs"),
        r#"
impl Order {
    pub fn get_order(&self) {}
}
"#,
    );
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"
context SalesContext {
  aggregate Order {
    command get_order() bound to "src/domain/order.rs" symbol "Order::get_order"
  }
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    assert!(report
        .violations
        .iter()
        .all(|violation| violation.code != CODE_COMMAND_BINDING_INTENT_SUSPICIOUS));
}

#[test]
fn typescript_symbol_found_produces_no_violations() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("src/domain/order.ts"),
        r#"
export function ship() {}
"#,
    );
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"
context SalesContext {
  aggregate Order {
    command ship() bound to "src/domain/order.ts" symbol "ship"
  }
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    assert!(report.violations.is_empty());
}

#[test]
fn typescript_command_arity_mismatch_produces_error() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("src/domain/order.ts"),
        r#"
export function ship(orderId: number, priority: number) {}
"#,
    );
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"
context SalesContext {
  aggregate Order {
    command ship(order_id: Int) bound to "src/domain/order.ts" symbol "ship"
  }
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    let violation = report
        .violations
        .iter()
        .find(|violation| violation.code == CODE_COMMAND_BINDING_ARITY_MISMATCH)
        .unwrap();
    assert_eq!(violation.severity, ViolationSeverity::Error);
    assert!(violation.message.contains("TypeScript symbol"));
    assert!(violation.message.contains("declares 1 parameter(s)"));
    assert!(violation.message.contains("expects 2 parameter(s)"));
    assert!(violation
        .hint
        .as_deref()
        .unwrap_or_default()
        .contains("TypeScript symbol"));
}

#[test]
fn typescript_command_arity_match_has_no_mismatch() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("src/domain/order.ts"),
        r#"
export function ship(orderId: number) {}
"#,
    );
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"
context SalesContext {
  aggregate Order {
    command ship(order_id: Int) bound to "src/domain/order.ts" symbol "ship"
  }
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    assert!(report
        .violations
        .iter()
        .all(|violation| violation.code != CODE_COMMAND_BINDING_ARITY_MISMATCH));
}

#[test]
fn typescript_missing_symbol_produces_symbol_error() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("src/domain/order.ts"),
        r#"
export function something_else() {}
"#,
    );
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"
context SalesContext {
  aggregate Order {
    command ship() bound to "src/domain/order.ts" symbol "ship"
  }
}
"#,
    );

    let report = check_file(&kide_path).unwrap();

    let missing_symbol = report
        .violations
        .iter()
        .find(|violation| {
            violation.code == CODE_BINDING_SYMBOL_NOT_FOUND
                && violation.severity == ViolationSeverity::Error
        })
        .unwrap();
    assert!(missing_symbol.hint.is_some());
    assert!(missing_symbol.docs_uri.is_some());
    assert!(missing_symbol.span.is_some());
    assert_eq!(missing_symbol.span.unwrap().start_line, 4);
}

#[test]
fn typescript_missing_symbol_with_near_match_suggests_symbol() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("src/domain/order.ts"),
        r#"
export function processOrder() {}
"#,
    );
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"
context SalesContext {
  aggregate Order {
    command ship() bound to "src/domain/order.ts" symbol "processOder"
  }
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    let missing_symbol = report
        .violations
        .iter()
        .find(|violation| violation.code == CODE_BINDING_SYMBOL_NOT_FOUND)
        .unwrap();
    assert_eq!(
        missing_symbol.hint.as_deref(),
        Some("did you mean 'processOrder'?")
    );
}

#[test]
fn prisma_symbol_found_produces_no_violations() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("prisma/schema.prisma"),
        r#"
model Order {
  id Int @id
}
"#,
    );
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"
context SalesContext {
  aggregate Order {
    command ship() bound to "prisma/schema.prisma" symbol "Order"
  }
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    assert!(report.violations.is_empty());
}

#[test]
fn prisma_command_arity_mismatch_for_declaration_symbol_produces_error() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("prisma/schema.prisma"),
        r#"
model Order {
  id Int @id
}
"#,
    );
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"
context SalesContext {
  aggregate Order {
    command ship(order_id: Int) bound to "prisma/schema.prisma" symbol "Order"
  }
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    let violation = report
        .violations
        .iter()
        .find(|violation| violation.code == CODE_COMMAND_BINDING_ARITY_MISMATCH)
        .unwrap();
    assert_eq!(violation.severity, ViolationSeverity::Error);
    assert!(violation.message.contains("Prisma symbol"));
    assert!(violation.message.contains("declares 1 parameter(s)"));
    assert!(violation.message.contains("expects 0 parameter(s)"));
}

#[test]
fn prisma_missing_symbol_produces_symbol_error() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("prisma/schema.prisma"),
        r#"
model Customer {
  id Int @id
}
"#,
    );
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"
context SalesContext {
  aggregate Order {
    command ship() bound to "prisma/schema.prisma" symbol "Order"
  }
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    let missing_symbol = report
        .violations
        .iter()
        .find(|violation| {
            violation.code == CODE_BINDING_SYMBOL_NOT_FOUND
                && violation.severity == ViolationSeverity::Error
        })
        .unwrap();
    assert!(missing_symbol.hint.is_some());
    assert_eq!(missing_symbol.docs_uri, Some(DOCS_BINDING_SYMBOL_NOT_FOUND));
    assert!(missing_symbol.span.is_some());
    assert_eq!(missing_symbol.span.unwrap().start_line, 4);
}

#[test]
fn tsx_symbol_found_produces_no_violations() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("src/domain/order.tsx"),
        r#"
export function ShipButton() { return <button />; }
"#,
    );
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"
context SalesContext {
  aggregate Order {
    command ship() bound to "src/domain/order.tsx" symbol "ShipButton"
  }
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    assert!(report.violations.is_empty());
}

#[test]
fn tsx_command_arity_mismatch_produces_error() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("src/domain/order.tsx"),
        r#"
export function ShipButton(orderId: number, priority: number) { return <button />; }
"#,
    );
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"
context SalesContext {
  aggregate Order {
    command ship(order_id: Int) bound to "src/domain/order.tsx" symbol "ShipButton"
  }
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    let violation = report
        .violations
        .iter()
        .find(|violation| violation.code == CODE_COMMAND_BINDING_ARITY_MISMATCH)
        .unwrap();
    assert_eq!(violation.severity, ViolationSeverity::Error);
    assert!(violation.message.contains("TypeScript symbol"));
    assert!(violation.message.contains("expects 2 parameter(s)"));
}

#[test]
fn forbidden_dictionary_term_in_bound_source_produces_error() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("src/domain/order.rs"),
        r#"
pub struct User {}
"#,
    );
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"context SalesContext {
  dictionary {
    "User" => forbidden
  }

  aggregate Order bound to "src/domain/order.rs" {}
}
"#,
    );

    let report = check_file(&kide_path).unwrap();

    let violation = report
        .violations
        .iter()
        .find(|violation| {
            violation.code == CODE_DICTIONARY_TERM_FORBIDDEN
                && violation.severity == ViolationSeverity::Error
        })
        .unwrap();
    assert!(violation.message.contains("src/domain/order.rs"));
    assert_eq!(
        violation.hint.as_deref(),
        Some("remove 'User' from bound source files")
    );
    assert_eq!(violation.docs_uri, Some(DOCS_DICTIONARY_TERM_FORBIDDEN));
    assert!(violation.span.is_some());
    assert_eq!(violation.span.unwrap().start_line, 3);
}

#[test]
fn preferred_dictionary_term_in_bound_source_produces_warning_with_hint() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("src/domain/order.rs"),
        r#"
pub struct User {}
"#,
    );
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"context SalesContext {
  dictionary {
    "User" => "Customer"
  }

  aggregate Order bound to "src/domain/order.rs" {}
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    let violation = report
        .violations
        .iter()
        .find(|violation| {
            violation.code == CODE_DICTIONARY_TERM_PREFERRED
                && violation.severity == ViolationSeverity::Warning
        })
        .unwrap();
    assert_eq!(
        violation.hint.as_deref(),
        Some("use 'Customer' instead of 'User'")
    );
    assert!(violation.message.contains("src/domain/order.rs"));
    assert_eq!(violation.docs_uri, Some(DOCS_DICTIONARY_TERM_PREFERRED));
    assert!(violation.span.is_some());
    assert_eq!(violation.span.unwrap().start_line, 3);
}

#[test]
fn dictionary_validation_checks_command_bound_sources() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("src/domain/order.rs"),
        r#"
pub struct Order {}
"#,
    );
    create_file(
        &temp_dir.path().join("src/domain/order_command.rs"),
        r#"
pub struct User {}
"#,
    );
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"context SalesContext {
  dictionary {
    "User" => forbidden
  }

  aggregate Order bound to "src/domain/order.rs" {
    command ship() bound to "src/domain/order_command.rs"
  }
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    let violation = report
        .violations
        .iter()
        .find(|violation| violation.code == CODE_DICTIONARY_TERM_FORBIDDEN)
        .unwrap();
    assert!(violation.message.contains("src/domain/order_command.rs"));
}

#[test]
fn dictionary_validation_uses_word_boundaries() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("src/domain/order.rs"),
        r#"
pub struct UserAccount {}
"#,
    );
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"context SalesContext {
  dictionary {
    "User" => forbidden
  }

  aggregate Order bound to "src/domain/order.rs" {}
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    assert!(report
        .violations
        .iter()
        .all(|violation| violation.code == CODE_BINDING_SYMBOL_MISSING));
}

#[test]
fn duplicate_dictionary_key_in_same_block_produces_error() {
    let temp_dir = TempDir::new().unwrap();
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"context SalesContext {
  dictionary {
    "User" => forbidden
    "User" => "Customer"
  }
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    let violation = report
        .violations
        .iter()
        .find(|violation| {
            violation.code == CODE_DICTIONARY_DUPLICATE_KEY
                && violation.severity == ViolationSeverity::Error
        })
        .unwrap();
    assert_eq!(violation.docs_uri, Some(DOCS_DICTIONARY_DUPLICATE_KEY));
    assert_eq!(
        violation.hint.as_deref(),
        Some("remove or merge duplicate key 'User'")
    );
    assert!(violation.span.is_some());
    assert_eq!(violation.span.unwrap().start_line, 4);
}

#[test]
fn dictionary_duplicate_key_rule_has_no_false_positive_with_unique_keys() {
    let temp_dir = TempDir::new().unwrap();
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"context SalesContext {
  dictionary {
    "User" => forbidden
    "Order" => "PurchaseOrder"
  }
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    assert!(report
        .violations
        .iter()
        .all(|violation| violation.code != CODE_DICTIONARY_DUPLICATE_KEY));
}

#[test]
fn forbidden_boundary_context_in_bound_source_produces_error() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("src/domain/order.rs"),
        r#"
pub struct OtherContext {}
"#,
    );
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"context SalesContext {
  boundary {
    forbid OtherContext
  }

  aggregate Order bound to "src/domain/order.rs" {}
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    let violation = report
        .violations
        .iter()
        .find(|violation| {
            violation.code == CODE_CONTEXT_BOUNDARY_FORBIDDEN
                && violation.severity == ViolationSeverity::Error
        })
        .unwrap();
    assert!(violation.message.contains("src/domain/order.rs"));
    assert_eq!(violation.docs_uri, Some(DOCS_CONTEXT_BOUNDARY_FORBIDDEN));
    assert_eq!(
        violation.hint.as_deref(),
        Some("remove references to 'OtherContext' from files bound in this context")
    );
}

#[test]
fn boundary_detects_rust_use_type_and_call_references() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("src/domain/order.rs"),
        r#"
use crate::billing::BillingContext;

pub fn ship(context: BillingContext) {
    BillingContext::sync();
}
"#,
    );
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"context SalesContext {
  boundary {
    forbid BillingContext
  }

  aggregate Order bound to "src/domain/order.rs" {}
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    assert!(report
        .violations
        .iter()
        .any(|violation| violation.code == CODE_CONTEXT_BOUNDARY_FORBIDDEN));
}

#[test]
fn boundary_detects_typescript_import_type_and_call_references() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("src/domain/order.ts"),
        r#"
import { BillingContext } from "./billing";

export function ship(context: BillingContext) {
  BillingContext.sync();
}
"#,
    );
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"context SalesContext {
  boundary {
    forbid BillingContext
  }

  aggregate Order bound to "src/domain/order.ts" {}
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    assert!(report
        .violations
        .iter()
        .any(|violation| violation.code == CODE_CONTEXT_BOUNDARY_FORBIDDEN));
}

#[test]
fn boundary_detects_tsx_import_type_and_new_references() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("src/domain/order.tsx"),
        r#"
import { BillingContext } from "./billing";

type Props = { context: BillingContext };
export function ShipButton(_: Props) {
  return <div>{new BillingContext().name}</div>;
}
"#,
    );
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"context SalesContext {
  boundary {
    forbid BillingContext
  }

  aggregate Order bound to "src/domain/order.tsx" {}
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    assert!(report
        .violations
        .iter()
        .any(|violation| violation.code == CODE_CONTEXT_BOUNDARY_FORBIDDEN));
}

#[test]
fn duplicate_boundary_forbid_in_same_block_produces_error() {
    let temp_dir = TempDir::new().unwrap();
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"context SalesContext {
  boundary {
    forbid BillingContext
    forbid BillingContext
  }
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    let violation = report
        .violations
        .iter()
        .find(|violation| {
            violation.code == CODE_CONTEXT_BOUNDARY_DUPLICATE_FORBID
                && violation.severity == ViolationSeverity::Error
        })
        .unwrap();
    assert_eq!(
        violation.docs_uri,
        Some(DOCS_CONTEXT_BOUNDARY_DUPLICATE_FORBID)
    );
    assert_eq!(
        violation.hint.as_deref(),
        Some("remove duplicate 'forbid BillingContext' entries in this boundary block")
    );
    assert!(violation.span.is_some());
    assert_eq!(violation.span.unwrap().start_line, 4);
}

#[test]
fn boundary_duplicate_forbid_rule_has_no_false_positive_with_distinct_contexts() {
    let temp_dir = TempDir::new().unwrap();
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"context SalesContext {
  boundary {
    forbid BillingContext
    forbid SupportContext
  }
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    assert!(report
        .violations
        .iter()
        .all(|violation| violation.code != CODE_CONTEXT_BOUNDARY_DUPLICATE_FORBID));
}

#[test]
fn boundary_self_forbid_produces_warning_with_hint() {
    let temp_dir = TempDir::new().unwrap();
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"context SalesContext {
  boundary {
    forbid SalesContext
  }
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    let violation = report
        .violations
        .iter()
        .find(|violation| {
            violation.code == CODE_CONTEXT_BOUNDARY_SELF_FORBID
                && violation.severity == ViolationSeverity::Warning
        })
        .unwrap();
    assert_eq!(violation.docs_uri, Some(DOCS_CONTEXT_BOUNDARY_SELF_FORBID));
    assert_eq!(
        violation.hint.as_deref(),
        Some("remove 'forbid SalesContext' or replace it with another context name")
    );
    assert!(violation.span.is_some());
    assert_eq!(violation.span.unwrap().start_line, 3);
}

#[test]
fn boundary_self_forbid_rule_has_no_false_positive_for_other_context() {
    let temp_dir = TempDir::new().unwrap();
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"context SalesContext {
  boundary {
    forbid BillingContext
  }
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    assert!(report
        .violations
        .iter()
        .all(|violation| violation.code != CODE_CONTEXT_BOUNDARY_SELF_FORBID));
}

#[test]
fn boundary_validation_no_violation_when_forbidden_context_absent() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("src/domain/order.rs"),
        r#"
pub struct Customer {}
"#,
    );
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"context SalesContext {
  boundary {
    forbid OtherContext
  }

  aggregate Order bound to "src/domain/order.rs" {}
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    assert!(report
        .violations
        .iter()
        .all(|violation| violation.code == CODE_BINDING_SYMBOL_MISSING));
}

#[test]
fn boundary_violation_span_points_to_forbid_entry() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("src/domain/order.rs"),
        r#"
pub struct BillingContext {}
"#,
    );
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"context SalesContext {
  boundary {
    forbid OtherContext
    forbid BillingContext
  }

  aggregate Order bound to "src/domain/order.rs" {}
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    let violation = report
        .violations
        .iter()
        .find(|violation| violation.code == CODE_CONTEXT_BOUNDARY_FORBIDDEN)
        .unwrap();
    assert!(violation.span.is_some());
    assert_eq!(violation.span.unwrap().start_line, 4);
}

#[test]
fn invalid_binding_hash_format_produces_error() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("src/domain/order.rs"),
        "pub struct Order {}\n",
    );
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"context SalesContext {
  aggregate Order bound to "src/domain/order.rs" hash "INVALID_HASH" {}
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    let violation = report
        .violations
        .iter()
        .find(|violation| violation.code == CODE_BINDING_HASH_INVALID_FORMAT)
        .unwrap();
    assert_eq!(violation.severity, ViolationSeverity::Error);
    assert_eq!(violation.docs_uri, Some(DOCS_BINDING_HASH_INVALID_FORMAT));
    assert!(violation.hint.is_some());
    assert!(violation.span.is_some());
}

#[test]
fn binding_hash_mismatch_produces_error() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("src/domain/order.rs"),
        "pub struct Order {}\n",
    );
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"context SalesContext {
  aggregate Order bound to "src/domain/order.rs" hash "0000000000000000000000000000000000000000000000000000000000000000" {}
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    let violation = report
        .violations
        .iter()
        .find(|violation| violation.code == CODE_BINDING_HASH_MISMATCH)
        .unwrap();
    assert_eq!(violation.severity, ViolationSeverity::Error);
    assert_eq!(violation.docs_uri, Some(DOCS_BINDING_HASH_MISMATCH));
    assert!(violation.message.contains("hash mismatch"));
    assert!(violation.span.is_some());
}

#[test]
fn binding_hash_match_produces_no_hash_diagnostics() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("src/domain/order.rs"),
        "pub struct Order {}\n",
    );
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"context SalesContext {
  aggregate Order bound to "src/domain/order.rs" hash "635bcbc8ab12031354eadb76eee11c49a2cc94afa6b5c7e3a58b0fd4e11e182f" {}
}
"#,
    );

    let report = check_file(&kide_path).unwrap();
    assert!(report.violations.iter().all(|violation| {
        violation.code != CODE_BINDING_HASH_INVALID_FORMAT
            && violation.code != CODE_BINDING_HASH_MISMATCH
    }));
}

#[test]
fn definition_at_symbol_points_to_rust_symbol() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("src/domain/order.rs"),
        r#"impl Order {
    pub fn ship(&mut self) {}
}
"#,
    );
    let kide_source = r#"context SalesContext {
  aggregate Order {
    command ship() bound to "src/domain/order.rs" symbol "Order::ship"
  }
}
"#;
    let (line, column) = find_lsp_position(kide_source, "Order::ship");

    let definition = definition_at(kide_source, temp_dir.path(), line, column)
        .unwrap()
        .unwrap();

    assert!(definition.file_path.ends_with("src/domain/order.rs"));
    assert_eq!(definition.span.start_line, 2);
}

#[test]
fn definition_at_symbol_points_to_typescript_symbol() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("src/domain/order.ts"),
        r#"export class Order {
  ship() {}
}
"#,
    );
    let kide_source = r#"context SalesContext {
  aggregate Order {
    command ship() bound to "src/domain/order.ts" symbol "Order::ship"
  }
}
"#;
    let (line, column) = find_lsp_position(kide_source, "Order::ship");

    let definition = definition_at(kide_source, temp_dir.path(), line, column)
        .unwrap()
        .unwrap();

    assert!(definition.file_path.ends_with("src/domain/order.ts"));
    assert_eq!(definition.span.start_line, 2);
}

#[test]
fn definition_at_symbol_returns_none_when_typescript_symbol_missing() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("src/domain/order.ts"),
        r#"export class Order {
  cancel() {}
}
"#,
    );
    let kide_source = r#"context SalesContext {
  aggregate Order {
    command ship() bound to "src/domain/order.ts" symbol "Order::ship"
  }
}
"#;
    let (line, column) = find_lsp_position(kide_source, "Order::ship");

    let definition = definition_at(kide_source, temp_dir.path(), line, column).unwrap();

    assert!(definition.is_none());
}

#[test]
fn definition_at_symbol_points_to_prisma_symbol() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("prisma/schema.prisma"),
        r#"model Order {
  id Int @id
}
"#,
    );
    let kide_source = r#"context SalesContext {
  aggregate Order {
    command ship() bound to "prisma/schema.prisma" symbol "Order"
  }
}
"#;
    let (line, column) = find_lsp_position(kide_source, "symbol \"Order\"");
    let column = column + "symbol \"".chars().count() as u32;

    let definition = definition_at(kide_source, temp_dir.path(), line, column)
        .unwrap()
        .unwrap();

    assert!(definition.file_path.ends_with("prisma/schema.prisma"));
    assert_eq!(definition.span.start_line, 1);
}

#[test]
fn definition_at_symbol_returns_none_when_prisma_symbol_missing() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("prisma/schema.prisma"),
        r#"model Customer {
  id Int @id
}
"#,
    );
    let kide_source = r#"context SalesContext {
  aggregate Order {
    command ship() bound to "prisma/schema.prisma" symbol "Order"
  }
}
"#;
    let (line, column) = find_lsp_position(kide_source, "symbol \"Order\"");
    let column = column + "symbol \"".chars().count() as u32;

    let definition = definition_at(kide_source, temp_dir.path(), line, column).unwrap();

    assert!(definition.is_none());
}

#[test]
fn definition_at_path_points_to_prisma_file_start() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("prisma/schema.prisma"),
        r#"model Order {
  id Int @id
}
"#,
    );
    let kide_source = r#"context SalesContext {
  aggregate Order {
    command ship() bound to "prisma/schema.prisma" symbol "Order"
  }
}
"#;
    let (line, column) = find_lsp_position(kide_source, "prisma/schema.prisma");

    let definition = definition_at(kide_source, temp_dir.path(), line, column)
        .unwrap()
        .unwrap();

    assert!(definition.file_path.ends_with("prisma/schema.prisma"));
    assert_eq!(definition.span.start_line, 1);
    assert_eq!(definition.span.start_column, 1);
}

#[test]
fn empty_bound_file_produces_warning() {
    let temp_dir = TempDir::new().unwrap();
    create_file(&temp_dir.path().join("src/domain/order.ts"), "");
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"
context SalesContext {
  aggregate Order bound to "src/domain/order.ts" {
    command ship() bound to "src/domain/order.ts" symbol "ship"
  }
}
"#,
    );

    let report = check_file(&kide_path).unwrap();

    let violation = report
        .violations
        .iter()
        .find(|violation| {
            violation.code == CODE_BINDING_FILE_EMPTY
                && violation.severity == ViolationSeverity::Warning
        })
        .expect("expected empty file warning");
    assert!(violation.message.contains("empty"));
    assert!(violation.hint.is_some());
    assert_eq!(violation.docs_uri, Some(DOCS_BINDING_FILE_EMPTY));
    assert!(violation.span.is_some());
}

#[test]
fn binding_without_symbol_clause_produces_information() {
    let temp_dir = TempDir::new().unwrap();
    create_file(
        &temp_dir.path().join("src/domain/order.rs"),
        r#"
pub fn ship() {}
"#,
    );
    let kide_path = temp_dir.path().join("main.kide");
    create_file(
        &kide_path,
        r#"
context SalesContext {
  aggregate Order bound to "src/domain/order.rs" {}
}
"#,
    );

    let report = check_file(&kide_path).unwrap();

    let violation = report
        .violations
        .iter()
        .find(|violation| {
            violation.code == CODE_BINDING_SYMBOL_MISSING
                && violation.severity == ViolationSeverity::Information
        })
        .expect("expected missing symbol info diagnostic");
    assert!(violation.message.contains("no symbol clause"));
    assert!(violation.hint.is_some());
    assert_eq!(violation.docs_uri, Some(DOCS_BINDING_SYMBOL_MISSING));
    assert!(violation.span.is_some());
}

fn create_file(path: &Path, source: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, source).unwrap();
}

fn create_rust_grammar(base_dir: &Path) {
    // Copy the real WASM file from the workspace grammars directory
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../grammars");
    let wasm_source = workspace_root.join("rust/tree-sitter-rust.wasm");
    let grammar_dir = base_dir.join("grammars/rust");
    std::fs::create_dir_all(grammar_dir.join("queries")).unwrap();

    // Write manifest
    create_file(
        &grammar_dir.join("manifest.toml"),
        r#"language = "rust"
version = "0.24.0"
wasm_file = "tree-sitter-rust.wasm"
extensions = [".rs"]
display_name = "Rust"

[queries]
symbol_exists = "queries/symbol_exists.scm"

[queries.boundary_references]
source = """
[
  (use_declaration)
  (type_identifier)
  (scoped_identifier)
  (call_expression)
] @reference
"""
"#,
    );

    // Copy WASM file if it exists, otherwise tests that require parsing will be skipped
    if wasm_source.exists() {
        std::fs::copy(&wasm_source, grammar_dir.join("tree-sitter-rust.wasm")).unwrap();
    }

    // Write query file
    create_file(
        &grammar_dir.join("queries/symbol_exists.scm"),
        r#"
(function_item
  name: (identifier) @name)
"#,
    );
}

#[test]
fn shipping_co_example_produces_no_errors_or_warnings() {
    let example_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/shipping-co/domain/main.kide");
    if !example_path.exists() {
        eprintln!(
            "skipping shipping-co integration test: {} not found",
            example_path.display()
        );
        return;
    }

    let report = check_file(&example_path).unwrap();

    let errors_and_warnings: Vec<_> = report
        .violations
        .iter()
        .filter(|v| {
            matches!(
                v.severity,
                ViolationSeverity::Error | ViolationSeverity::Warning
            )
        })
        .collect();

    assert!(
        errors_and_warnings.is_empty(),
        "shipping-co should have no errors or warnings, but found:\n{}",
        errors_and_warnings
            .iter()
            .map(|v| format!("  {} [{}] {}", v.severity.as_str(), v.code, v.message))
            .collect::<Vec<_>>()
            .join("\n")
    );

    assert_eq!(report.contexts, 5, "shipping-co should have 5 contexts");
}

fn find_lsp_position(source: &str, needle: &str) -> (u32, u32) {
    let idx = source.find(needle).unwrap();
    let prefix = &source[..idx];
    let line = prefix.as_bytes().iter().filter(|&&b| b == b'\n').count() as u32;
    let column = prefix
        .rsplit_once('\n')
        .map(|(_, rest)| rest.chars().count() as u32)
        .unwrap_or(prefix.chars().count() as u32);
    (line, column)
}
