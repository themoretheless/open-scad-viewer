//! Transport adapter for native path sampling.
use super::{Result, Value, encode, field};
pub fn sample(v: Value) -> Result<Value> {
    encode(geometry_ops::path_sampling::sample(
        &field::<Vec<Vec<f64>>>(&v, "path")?,
        &field::<Vec<f64>>(&v, "fractions")?,
    )?)
}
