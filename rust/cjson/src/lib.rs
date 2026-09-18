//! cJSON core — C-ABI drop-in implemented in Rust.
//!
//! Utils (`cJSON_Utils`) are intentionally out of this crate so a sibling
//! crate/library can link against these symbols later.

#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_assignments)]
#![allow(unused_mut)]
#![allow(clippy::all)]
#![allow(improper_ctypes)]

mod cjson;

pub use cjson::*;
