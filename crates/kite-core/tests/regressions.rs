//! Regression tests that run `kite check` against the example fixture files
//! in `examples/shipping-co/domain/regressions/`.
//!
//! Each fixture file is named with an `expected-pass-` or `expected-fail-` prefix
//! and encodes a specific scenario. These tests verify that the checker produces
//! the correct outcome and violation codes.

use std::path::PathBuf;

fn regressions_dir() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .join("..")
        .join("..")
        .join("examples")
        .join("shipping-co")
        .join("domain")
        .join("regressions")
}

#[test]
fn regression_expected_pass_minimal() {
    let path = regressions_dir().join("expected-pass-minimal.kite");
    let report = kite_core::check_file(&path).expect("should parse without error");
    let errors: Vec<_> = report
        .violations
        .iter()
        .filter(|v| v.severity == kite_core::ViolationSeverity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "expected-pass-minimal.kite should produce no errors, got: {:#?}",
        errors
    );
}

#[test]
fn regression_expected_fail_arity_mismatch() {
    let path = regressions_dir().join("expected-fail-arity-mismatch.kite");
    let report = kite_core::check_file(&path).expect("should parse without error");
    let codes: Vec<&str> = report.violations.iter().map(|v| v.code).collect();
    assert!(
        codes.contains(&"COMMAND_BINDING_ARITY_MISMATCH"),
        "expected COMMAND_BINDING_ARITY_MISMATCH violation, got: {:?}",
        codes
    );
}

#[test]
fn regression_expected_fail_boundary_violation() {
    let path = regressions_dir().join("expected-fail-boundary-violation.kite");
    let report = kite_core::check_file(&path).expect("should parse without error");
    let codes: Vec<&str> = report.violations.iter().map(|v| v.code).collect();
    assert!(
        codes.contains(&"CONTEXT_BOUNDARY_FORBIDDEN"),
        "expected CONTEXT_BOUNDARY_FORBIDDEN violation, got: {:?}",
        codes
    );
}

#[test]
fn regression_expected_fail_missing_symbol() {
    let path = regressions_dir().join("expected-fail-missing-symbol.kite");
    let report = kite_core::check_file(&path).expect("should parse without error");
    let codes: Vec<&str> = report.violations.iter().map(|v| v.code).collect();
    assert!(
        codes.contains(&"BINDING_SYMBOL_NOT_FOUND"),
        "expected BINDING_SYMBOL_NOT_FOUND violation, got: {:?}",
        codes
    );
}
