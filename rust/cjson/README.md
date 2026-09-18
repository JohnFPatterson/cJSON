# cJSON core (Rust, C-ABI)

This crate is the **core** `cJSON` implementation: a faithful C-ABI drop-in for
`cJSON.h`, built as `staticlib` + `cdylib` (`libcjson`).

## Boundaries

- **In scope:** everything in `cJSON.h` / former `cJSON.c`.
- **Out of scope:** `cJSON_Utils` — leave that to a sibling crate/library that
  links against these exported symbols. Do not add Utils sources here.

## Build

```bash
cargo build --release
# outputs target/release/libcjson.a and libcjson.so
```

CMake/Make at the repo root invoke this crate and link C tests against it.
