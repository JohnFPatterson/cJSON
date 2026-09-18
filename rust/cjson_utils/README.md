# cjson_utils (Rust C-ABI)

Rust drop-in for [`cJSON_Utils.h`](../../cJSON_Utils.h). Public symbols match the C header;
behavior follows `cJSON_Utils.c` (including known C quirks).

## Scope

- Implements **Utils only** — does not reimplement core `cJSON`
- Depends on sibling [`rust/cjson`](../cjson) (Rust core C-ABI from PR #1)
- Leaves `cJSON_Utils.c` in-tree until the harness track switches the build

## Build

Requires the core crate at `../cjson` (this branch is based on
`cursor/cjson-rust-migration-68e6`).

```bash
cd rust/cjson_utils
cargo build --release
# artifacts: target/release/libcjson_utils.{a,so}
```

## Smoke-test against existing C Utils suite

```bash
./run_c_tests.sh
```

Runs `misc_utils_tests`, `json_patch_tests`, and `old_utils_tests` linked to
`libcjson_utils.so` (which pulls in Rust core via the path dependency).

## Still needed (optional follow-up)

- Prefer Utils `cdylib` with `DT_NEEDED` on `libcjson` rather than embedding the
  core rlib (CMake/Make currently whole-archive `libcjson_utils.a`, which embeds
  core; Utils tests link only `cjson_utils` to avoid duplicate symbols).
