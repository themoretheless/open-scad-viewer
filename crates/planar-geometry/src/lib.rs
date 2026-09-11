//! Planar shape kernel: Bézier paths, closed rings, Pathfinder, edit.
//! Independent of triangle-mesh, NURBS and SDF kernels.
//!
//! [`rings`] is the boolean/offset engine on closed polylines.
//! [`pathfinder`] is the z-order scenario layer (divide / crop / trim / merge)
//! built on `rings::planar`.
pub mod corners;
pub mod edit;
pub mod effects;
pub mod path;
pub mod pathfinder;
pub mod rings;
pub mod stroke;

#[derive(Debug, Clone)]
pub struct Error {
    pub code: &'static str,
    pub message: String,
}
impl value_codec::Serialize for Error {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert("code".into(), value_codec::Serialize::to_value(&self.code));
        object.insert(
            "message".into(),
            value_codec::Serialize::to_value(&self.message),
        );
        value_codec::Value::Object(object)
    }
}
impl Error {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            code: "PLANAR_INVALID_INPUT",
            message: message.into(),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;
pub(crate) fn check(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(Error::new(message))
    }
}
