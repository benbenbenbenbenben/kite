#!/usr/bin/env bash
set -euo pipefail

export RUST_MIN_STACK=33554432

echo "=== cargo test ==="
cargo test --workspace

echo ""
echo "=== kite fmt (idempotency check) ==="
for f in examples/shipping-co/domain/*.kite; do
    cargo run -p kite-cli -- fmt "$f" 2>/dev/null > /tmp/kite-fmt-out.kite
    cargo run -p kite-cli -- fmt /tmp/kite-fmt-out.kite 2>/dev/null > /tmp/kite-fmt-out2.kite
    diff /tmp/kite-fmt-out.kite /tmp/kite-fmt-out2.kite || { echo "FAIL: fmt not idempotent on $f"; exit 1; }
    echo "  ✅ $f idempotent"
done

echo ""
echo "=== kite check: main.kite ==="
cargo run -p kite-cli -- check examples/shipping-co/domain/main.kite

echo ""
echo "=== kite check: infra.kite ==="
cargo run -p kite-cli -- check examples/shipping-co/domain/infra.kite

echo ""
echo "=== kite check: demos.kite (expected violations) ==="
# demos.kite intentionally produces errors — we just verify it parses and runs
cargo run -p kite-cli -- check examples/shipping-co/domain/demos.kite || true

echo ""
echo "=== kite init (smoke test) ==="
cargo run -p kite-cli -- init examples/shipping-co/src -o /tmp/kite-init-test.kite
echo "  ✅ scaffold generated"

echo ""
echo "✅ CI checks complete"
