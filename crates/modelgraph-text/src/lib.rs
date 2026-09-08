//! Bounded ModelGraph Text frontend. No host evaluation or geometry execution.
mod lower;
mod parser;
mod value;
use serde_json::Value;

/// Compile source into the portable ModelGraph authoring graph and editor controls.
pub fn compile(source: &str) -> Result<Value, String> {
    let statements = parser::parse(source)?;
    lower::compile(statements)
}
