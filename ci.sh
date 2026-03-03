#!/usr/bin/env bash
set -euo pipefail

echo "=== cargo test ==="
cargo test --workspace

echo ""
echo "=== kide check: main.kide ==="
cargo run -p kide-cli -- check examples/shipping-co/domain/main.kide

echo ""
echo "=== kide check: infra.kide ==="
cargo run -p kide-cli -- check examples/shipping-co/domain/infra.kide

echo ""
echo "=== kide check: demos.kide (expected violations) ==="
# demos.kide intentionally produces errors — we just verify it parses and runs
cargo run -p kide-cli -- check examples/shipping-co/domain/demos.kide || true

echo ""
echo "✅ CI checks complete"
