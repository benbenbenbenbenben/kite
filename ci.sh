#!/usr/bin/env bash
set -euo pipefail

export RUST_MIN_STACK=33554432

echo "=== cargo test ==="
cargo test --workspace

echo ""
echo "=== kide fmt (idempotency check) ==="
for f in examples/shipping-co/domain/*.kide; do
    cargo run -p kide-cli -- fmt "$f" 2>/dev/null > /tmp/kide-fmt-out.kide
    cargo run -p kide-cli -- fmt /tmp/kide-fmt-out.kide 2>/dev/null > /tmp/kide-fmt-out2.kide
    diff /tmp/kide-fmt-out.kide /tmp/kide-fmt-out2.kide || { echo "FAIL: fmt not idempotent on $f"; exit 1; }
    echo "  ✅ $f idempotent"
done

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
echo "=== kide init (smoke test) ==="
cargo run -p kide-cli -- init examples/shipping-co/src -o /tmp/kide-init-test.kide
echo "  ✅ scaffold generated"

echo ""
echo "✅ CI checks complete"
