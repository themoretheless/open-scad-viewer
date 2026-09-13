//! Canonical ModelGraph validation and execution planning without a JavaScript evaluator.
#![feature(
    try_blocks,
    gen_blocks,
    yield_expr,
    super_let,
    deref_patterns,
    yeet_expr
)]
#![allow(unused_features)]
mod assembly;
pub mod emit;
pub mod eval;
mod mechanical;
pub use mechanical::{thread_geometry, thread_radius};
pub mod nurbs;
mod profiles;
pub mod range;
pub mod schema;
mod sketch;
pub mod units;
use value_codec::Value;
#[derive(Debug, Clone)]
pub struct Error {
    pub code: String,
    pub path: String,
    pub message: String,
    pub details: Option<Value>,
}
impl value_codec::Serialize for Error {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert("code".into(), value_codec::Serialize::to_value(&self.code));
        object.insert("path".into(), value_codec::Serialize::to_value(&self.path));
        object.insert(
            "message".into(),
            value_codec::Serialize::to_value(&self.message),
        );
        if self.details.is_some() {
            object.insert(
                "details".into(),
                value_codec::Serialize::to_value(&self.details),
            );
        }
        value_codec::Value::Object(object)
    }
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
