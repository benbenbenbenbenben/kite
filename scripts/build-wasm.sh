#!/usr/bin/env bash
set -euo pipefail

mkdir -p dist/wasm

export CC_wasm32_wasip1="${CC_wasm32_wasip1:-clang}"
export AR_wasm32_wasip1="${AR_wasm32_wasip1:-ar}"

if [[ -z "${WASI_INCLUDE:-}" ]]; then
  if [[ -d "/usr/include/wasm32-wasi" ]]; then
    WASI_INCLUDE="/usr/include/wasm32-wasi"
  else
    mkdir -p .toolchain/wasi
    if ! ls .toolchain/wasi/wasi-libc_*.deb >/dev/null 2>&1; then
      (cd .toolchain/wasi && apt download wasi-libc >/dev/null)
    fi
    deb="$(ls -1 .toolchain/wasi/wasi-libc_*.deb | head -n 1)"
    rm -rf .toolchain/wasi/sysroot
    dpkg-deb -x "${deb}" .toolchain/wasi/sysroot
    WASI_INCLUDE="${PWD}/.toolchain/wasi/sysroot/usr/include/wasm32-wasi"
  fi
fi

export CFLAGS_wasm32_wasip1="${CFLAGS_wasm32_wasip1:--isystem ${WASI_INCLUDE}}"

rustup target add wasm32-wasip1

cargo build --release --target wasm32-wasip1 -p kide-cli
cp target/wasm32-wasip1/release/kide.wasm dist/wasm/kide-cli.wasm

cargo build --release --target wasm32-wasip1 -p kide
cp target/wasm32-wasip1/release/kide.wasm dist/wasm/kide.wasm
