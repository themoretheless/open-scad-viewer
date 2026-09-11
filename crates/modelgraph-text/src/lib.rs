//! Bounded ModelGraph Text frontend. No host evaluation or geometry execution.
#![feature(
    try_blocks,
    gen_blocks,
    yield_expr,
    super_let,
    deref_patterns,
    yeet_expr
)]
#![allow(unused_features)]
mod lower;
mod ordered_map;
mod parser;
mod value;
use value_codec::Value;

pub use math_core::{Error, Result};
pub(crate) const INVALID_INPUT: &str = "MODELGRAPH_TEXT_INVALID_INPUT";
pub(crate) fn error(message: impl Into<String>) -> Error {
    Error::new(INVALID_INPUT, message)
}

/// Compile source into the portable ModelGraph authoring graph and editor controls.
pub fn compile(source: &str) -> Result<Value> {
    let statements = parser::parse(source)?;
    lower::compile(statements)
}
