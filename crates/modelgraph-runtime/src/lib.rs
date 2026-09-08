//! Canonical ModelGraph validation and execution planning without a JavaScript evaluator.
mod assembly;
pub mod emit;
pub mod eval;
mod mechanical;
pub mod nurbs;
mod profiles;
pub mod range;
pub mod schema;
mod sketch;
pub mod units;
use serde::Serialize;
use serde_json::Value;
#[derive(Debug, Clone, Serialize)]
pub struct Error {
    pub code: String,
    pub path: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}
impl Error {
    pub fn new(
        code: impl Into<String>,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            path: path.into(),
            message: message.into(),
            details: None,
        }
    }
    pub fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }
}
pub type Result<T> = std::result::Result<T, Error>;
pub fn compile(value: Value) -> Result<Value> {
    let document = schema::validate(value)?;
    let mut compiled = emit::compile(&document)?;
    compiled["document"] = document;
    Ok(compiled)
}
