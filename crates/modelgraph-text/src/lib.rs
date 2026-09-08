//! Bounded ModelGraph Text frontend. No host evaluation or geometry execution.
mod lower;
mod ordered_map;
mod parser;
mod value;
use value_codec::Value;

/// Compile source into the portable ModelGraph authoring graph and editor controls.
pub fn compile(source: &str) -> Result<Value, String> {
    let statements = parser::parse(source)?;
    lower::compile(statements)
}
