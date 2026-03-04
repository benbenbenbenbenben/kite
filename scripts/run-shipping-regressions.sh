#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CASES=(
  "shipping-co/domain/regressions/expected-pass-minimal.kite:pass"
  "shipping-co/domain/regressions/expected-fail-missing-symbol.kite:fail"
  "shipping-co/domain/regressions/expected-fail-arity-mismatch.kite:fail"
  "shipping-co/domain/regressions/expected-fail-boundary-violation.kite:fail"
)

failures=0
for case_def in "${CASES[@]}"; do
  file_rel="${case_def%%:*}"
  expect="${case_def##*:}"
  file_path="$ROOT_DIR/$file_rel"

  set +e
  output="$(cargo run -q -p kite-cli -- check "$file_path" 2>&1)"
  exit_code=$?
  set -e

  if [[ "$expect" == "pass" && $exit_code -eq 0 ]]; then
    echo "PASS (expected pass): $file_rel"
  elif [[ "$expect" == "fail" && $exit_code -ne 0 ]]; then
    echo "PASS (expected fail): $file_rel"
  else
    echo "FAIL (expected $expect, got exit $exit_code): $file_rel"
    echo "$output"
    failures=$((failures + 1))
  fi
done

if [[ $failures -ne 0 ]]; then
  echo "Regression corpus check failed: $failures mismatch(es)."
  exit 1
fi

echo "All shipping regression corpus scenarios matched expected outcomes."
