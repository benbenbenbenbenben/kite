#!/usr/bin/env bash
set -euo pipefail

ensure_wasi_sysroot() {
  if [[ -n "${WASI_INCLUDE:-}" ]]; then
    return
  fi

  if [[ -d "/usr/include/wasm32-wasi" ]]; then
    WASI_INCLUDE="/usr/include/wasm32-wasi"
  else
    mkdir -p .toolchain/wasi
    if ! ls .toolchain/wasi/wasi-libc_*.deb >/dev/null 2>&1; then
      (cd .toolchain/wasi && apt download wasi-libc >/dev/null)
    fi
    local deb
    deb="$(ls -1 .toolchain/wasi/wasi-libc_*.deb | head -n 1)"
    rm -rf .toolchain/wasi/sysroot
    dpkg-deb -x "${deb}" .toolchain/wasi/sysroot
    WASI_INCLUDE="${PWD}/.toolchain/wasi/sysroot/usr/include/wasm32-wasi"
  fi

  export WASI_INCLUDE
}

build_wasi_artifacts() {
  mkdir -p dist/wasm

  export CC_wasm32_wasip1="${CC_wasm32_wasip1:-clang}"
  export AR_wasm32_wasip1="${AR_wasm32_wasip1:-ar}"

  ensure_wasi_sysroot

  export CFLAGS_wasm32_wasip1="${CFLAGS_wasm32_wasip1:--isystem ${WASI_INCLUDE}}"

  rustup target add wasm32-wasip1

  cargo build --release --target wasm32-wasip1 -p kite-cli
  cp target/wasm32-wasip1/release/kite.wasm dist/wasm/kite-cli.wasm

  cargo build --release --target wasm32-wasip1 -p kite
  cp target/wasm32-wasip1/release/kite.wasm dist/wasm/kite.wasm
}

build_wasi_artifacts "$@"
