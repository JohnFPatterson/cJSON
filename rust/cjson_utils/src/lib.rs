//! Rust C-ABI drop-in for `cJSON_Utils`.
//!
//! Public symbols match `cJSON_Utils.h`. Core `cJSON` comes from the sibling
//! `cjson` crate (C-ABI); this crate does not reimplement core.

#![allow(clippy::missing_safety_doc)]
#![allow(clippy::too_many_arguments)]
#![allow(non_snake_case)]

mod utils;

pub use cjson::{cJSON, cJSON_bool};
pub use utils::*;
