#!/usr/bin/env bash
# Link and run the existing C Utils tests against the Rust libcjson_utils
# (built on top of rust/cjson from the core migration branch).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CRATE="$ROOT/rust/cjson_utils"
OUT="${OUT_DIR:-/tmp/cjson-utils-rust-tests}"
mkdir -p "$OUT"
cp -a "$ROOT/tests/json-patch-tests" "$OUT/"

cd "$CRATE"
cargo build --release
LIB_DIR="$CRATE/target/release"

# Also need Rust core shared/static artifacts when linking tests that call core
# APIs directly. Prefer the utils cdylib (embeds core via path dep) plus explicit
# libcjson if present from a sibling build.
CORE_DIR="$ROOT/rust/cjson/target/release"
if [[ ! -f "$CORE_DIR/libcjson.so" && ! -f "$CORE_DIR/libcjson.a" ]]; then
  (cd "$ROOT/rust/cjson" && cargo build --release)
fi

cc_link() {
  local name="$1"
  local libs=(-L"$LIB_DIR" -lcjson_utils)
  if [[ -f "$CORE_DIR/libcjson.so" || -f "$CORE_DIR/libcjson.a" ]]; then
    libs+=(-L"$CORE_DIR" -lcjson)
  fi
  gcc -o "$OUT/$name" "$ROOT/tests/${name}.c" "$ROOT/tests/unity/src/unity.c" \
    -I"$ROOT" -I"$ROOT/tests" -I"$ROOT/tests/unity/src" \
    "${libs[@]}" -lpthread -ldl -lm \
    -Wl,-rpath,"$LIB_DIR" -Wl,-rpath,"$CORE_DIR"
}

cc_link misc_utils_tests
cc_link json_patch_tests
cc_link old_utils_tests

cd "$OUT"
echo "=== misc_utils_tests ==="
./misc_utils_tests
echo "=== json_patch_tests ==="
./json_patch_tests
echo "=== old_utils_tests ==="
./old_utils_tests
echo "All Utils C tests passed against Rust cjson_utils (+ rust/cjson)."
