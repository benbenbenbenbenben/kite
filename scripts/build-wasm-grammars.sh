#!/bin/bash
# Download and build tree-sitter grammar WASM files.
#
# Requirements:
#   - tree-sitter CLI (cargo install tree-sitter-cli)
#   - emscripten (https://emscripten.org/docs/getting_started/downloads.html)
#
# Usage:
#   ./scripts/build-wasm-grammars.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
GRAMMARS_DIR="$ROOT_DIR/grammars"
TMP_DIR=$(mktemp -d)

cleanup() { rm -rf "$TMP_DIR"; }
trap cleanup EXIT

echo "Building WASM grammars..."
echo "  Temp dir: $TMP_DIR"
echo ""

# --- Rust ---
echo "  [1/4] tree-sitter-rust v0.24.0..."
RUST_DIR="$TMP_DIR/tree-sitter-rust"
git clone --depth 1 --branch v0.24.0 https://github.com/tree-sitter/tree-sitter-rust.git "$RUST_DIR" 2>/dev/null
(cd "$RUST_DIR" && tree-sitter build --wasm -o "$GRAMMARS_DIR/rust/tree-sitter-rust.wasm")
echo "    ✓ grammars/rust/tree-sitter-rust.wasm"

# --- TypeScript ---
echo "  [2/4] tree-sitter-typescript v0.23.2 (typescript)..."
TS_DIR="$TMP_DIR/tree-sitter-typescript"
git clone --depth 1 --branch v0.23.2 https://github.com/tree-sitter/tree-sitter-typescript.git "$TS_DIR" 2>/dev/null
(cd "$TS_DIR/typescript" && tree-sitter build --wasm -o "$GRAMMARS_DIR/typescript/tree-sitter-typescript.wasm")
echo "    ✓ grammars/typescript/tree-sitter-typescript.wasm"

# --- TSX ---
echo "  [3/4] tree-sitter-typescript v0.23.2 (tsx)..."
(cd "$TS_DIR/tsx" && tree-sitter build --wasm -o "$GRAMMARS_DIR/typescript/tree-sitter-tsx.wasm")
echo "    ✓ grammars/typescript/tree-sitter-tsx.wasm"

# --- Prisma ---
echo "  [4/4] tree-sitter-prisma v1.6.0..."
PRISMA_DIR="$TMP_DIR/tree-sitter-prisma"
git clone --depth 1 --branch v1.6.0 https://github.com/victorhqc/tree-sitter-prisma.git "$PRISMA_DIR" 2>/dev/null
(cd "$PRISMA_DIR" && tree-sitter build --wasm -o "$GRAMMARS_DIR/prisma/tree-sitter-prisma.wasm")
echo "    ✓ grammars/prisma/tree-sitter-prisma.wasm"

echo ""
echo "Done! All WASM grammar files are in place."
echo ""
ls -lh "$GRAMMARS_DIR"/*/tree-sitter-*.wasm 2>/dev/null || true
