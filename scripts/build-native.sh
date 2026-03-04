#!/usr/bin/env bash
set -euo pipefail

build_native_targets() {
  local targets=(
    "x86_64-unknown-linux-musl"
    "x86_64-pc-windows-msvc"
    "x86_64-apple-darwin"
    "aarch64-apple-darwin"
  )

  mkdir -p dist/native

  for target in "${targets[@]}"; do
    echo "Building kite for ${target}..."
    cargo build --release --target "${target}" -p kite-cli

    if [[ "${target}" == *windows* ]]; then
      cp "target/${target}/release/kite.exe" "dist/native/kite-${target}.exe"
    else
      cp "target/${target}/release/kite" "dist/native/kite-${target}"
    fi
  done
}

build_native_targets "$@"
