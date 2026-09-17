#!/usr/bin/env bash
# Run the existing C test suite against the Rust C-ABI core (and C Utils).
#
# Based on rust/cjson from the core migration track. This script is the
# harness entrypoint for shared/static + Utils-on gating.
#
# Usage:
#   ./scripts/run-c-tests-against-rust.sh
#   ./scripts/run-c-tests-against-rust.sh --static
#   BUILD_DIR=/tmp/cjson-rs ./scripts/run-c-tests-against-rust.sh
#
# Requires: cmake, a C toolchain, cargo/rustc on PATH.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="${BUILD_DIR:-${ROOT}/build-rust}"
SHARED=ON

for arg in "$@"; do
  case "$arg" in
    --static) SHARED=OFF ;;
    --shared) SHARED=ON ;;
    -h|--help)
      sed -n '1,20p' "$0"
      exit 0
      ;;
    *)
      echo "Unknown argument: $arg" >&2
      exit 2
      ;;
  esac
done

mkdir -p "${BUILD_DIR}"
cmake -S "${ROOT}" -B "${BUILD_DIR}" \
  -DENABLE_CJSON_TEST=ON \
  -DENABLE_CJSON_UTILS=ON \
  -DBUILD_SHARED_LIBS="${SHARED}" \
  -DENABLE_SANITIZERS=OFF \
  -DENABLE_VALGRIND=OFF \
  -DENABLE_SAFE_STACK=OFF

cmake --build "${BUILD_DIR}" --parallel "$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 4)"

export CTEST_OUTPUT_ON_FAILURE=1
ctest --test-dir "${BUILD_DIR}" --output-on-failure

echo
echo "Merge gate: all of the following CTest names must pass (22 with Utils ON):"
echo "  cJSON_test"
echo "  parse_examples parse_number parse_hex4 parse_string parse_array parse_object parse_value"
echo "  print_string print_number print_array print_object print_value"
echo "  misc_tests parse_with_opts compare_tests cjson_add readme_examples minify_tests"
echo "  json_patch_tests old_utils_tests misc_utils_tests"
