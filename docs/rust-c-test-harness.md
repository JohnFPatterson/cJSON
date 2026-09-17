# C tests against the Rust C-ABI drop-in

Core lives in `rust/cjson` ([PR #1](https://github.com/JohnFPatterson/cJSON/pull/1)).
This harness keeps the **existing C Unity / `test.c` suite** as the source of
truth and wires CMake/Make/CI so those tests link against the Rust-produced
C ABI (with `cJSON_Utils` still C until the Utils track lands).

## How to run

```bash
# Shared libcjson.so + Utils ON (default merge-shaped config)
./scripts/run-c-tests-against-rust.sh

# Static libcjson.a + Utils ON
./scripts/run-c-tests-against-rust.sh --static
```

Or manually:

```bash
cmake -S . -B build-rust \
  -DENABLE_CJSON_TEST=ON \
  -DENABLE_CJSON_UTILS=ON \
  -DBUILD_SHARED_LIBS=ON
cmake --build build-rust -j
ctest --test-dir build-rust --output-on-failure
```

Requires `cargo` / `rustc` on `PATH`.

## Merge gate (must pass vs Rust core)

With `ENABLE_CJSON_UTILS=ON` the full suite is **22** CTest targets:

| # | CTest name | Role |
|---|---|---|
| 1 | `cJSON_test` | `test.c` public API |
| 2–8 | `parse_examples` `parse_number` `parse_hex4` `parse_string` `parse_array` `parse_object` `parse_value` | parse |
| 9–13 | `print_string` `print_number` `print_array` `print_object` `print_value` | print |
| 14–19 | `misc_tests` `parse_with_opts` `compare_tests` `cjson_add` `readme_examples` `minify_tests` | misc / public |
| 20–22 | `json_patch_tests` `old_utils_tests` `misc_utils_tests` | Utils (C Utils + Rust core) |

Core-only (`ENABLE_CJSON_UTILS=OFF`) is the 19-test subset without rows 20–22.

White-box Unity tests call internals via `cJSON_internals.h` (no longer
`#include "cJSON.c"`); those symbols are exported from the Rust core.

## Shared vs static

CMake always whole-archives `cargo-cjson/release/libcjson.a` into the native
`cjson` target. That is required for `BUILD_SHARED_LIBS=ON`: a Rust `cdylib`
cannot be `--whole-archive`d into `libcjson.so` and still export the C ABI to
test binaries.
