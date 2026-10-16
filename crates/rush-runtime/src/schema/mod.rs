//! Strict Rush IR/1 validation and default insertion.
//!
//! Dispatch by `op`/`kind` once per object instead of probing a recursive union.
//! The input is normalized in place; expression subtrees are never cloned.
use crate::{Error, Result};
use value_codec::Value;


#[cfg(test)]
#[path = "../tests/schema.rs"]
mod tests;

mod validate;
mod expression;
mod node;
pub use validate::*;
use expression::*;
use node::*;
