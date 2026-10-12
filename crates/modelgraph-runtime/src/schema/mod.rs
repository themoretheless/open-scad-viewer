//! Strict ModelGraph/1 validation and default insertion.
//!
//! Dispatch by `op`/`kind` once per object instead of probing a recursive union.
//! The input is normalized in place; expression subtrees are never cloned.
use crate::{Error, Result};
use value_codec::Value;

mod validate;
mod expression;
mod node;
#[cfg(test)]
mod tests;
pub use validate::*;
use expression::*;
use node::*;
