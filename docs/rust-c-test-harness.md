# C tests against the Rust C-ABI drop-in

Stack: core ([PR #1](https://github.com/JohnFPatterson/cJSON/pull/1)) →
Utils ([PR #2](https://github.com/JohnFPatterson/cJSON/pull/2)) → this harness.

| Library | Crate | Header |
|---|---|---|
| `libcjson` | `rust/cjson` | `cJSON.h` (+ `cJSON_internals.h` for Unity white-box) |
| `libcjson_utils` | `rust/cjson_utils` | `cJSON_Utils.h` |

`cJSON_Utils.c` remains in-tree but is **not** built by CMake/Make anymore.

## How to run

```bash
# Full C suite, shared libs, Utils ON (default merge-shaped config)
./scripts/run-c-tests-against-rust.sh

# Static
./scripts/run-c-tests-against-rust.sh --static

# Utils-only smoke (from Utils track)
./rust/cjson_utils/run_c_tests.sh
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

## Merge gate (22 with Utils ON)

| # | CTest name | Role |
|---|---|---|
| 1 | `cJSON_test` | `test.c` public API |
| 2–8 | `parse_examples` `parse_number` `parse_hex4` `parse_string` `parse_array` `parse_object` `parse_value` | parse |
| 9–13 | `print_string` `print_number` `print_array` `print_object` `print_value` | print |
| 14–19 | `misc_tests` `parse_with_opts` `compare_tests` `cjson_add` `readme_examples` `minify_tests` | misc / public |
| 20–22 | `json_patch_tests` `old_utils_tests` `misc_utils_tests` | Utils |

## Shared vs static / embedding note

CMake whole-archives each crate’s **staticlib** into the matching native target
(`libcjson` ← `libcjson.a`, `libcjson_utils` ← `libcjson_utils.a`).

Today `rust/cjson_utils` path-depends on `rust/cjson`, so **`libcjson_utils`
embeds core symbols**. Utils CTest targets therefore link **only**
`cjson_utils` (not also `cjson`) to avoid duplicate `global_hooks` /
`cJSON_*`. A follow-up can make the Utils `cdylib` `DT_NEEDED` `libcjson`
instead of embedding the core rlib.
