//! Fail-closed solid operations for the planar subset of the NURBS B-rep.
//!
//! Booleans accept closed, orthogonal, planar solids and return one or more
//! connected bodies, including enclosed orthogonal cavities as inner shells.
//! Chamfers and fillets accept one convex planar body. Fillets are represented
//! by planar tangent facets; `segments` controls that declared approximation.
use super::*;
use std::collections::{BTreeMap, BTreeSet};

mod boolean;
mod edge_features;
mod planar_boolean;
mod planar_edit;
mod planar_model;
mod primitives;
#[cfg(test)]
mod tests;
pub use boolean::*;
pub use edge_features::*;
pub use planar_boolean::*;
pub use planar_edit::*;
pub use planar_model::*;
pub use primitives::*;

const UNSUPPORTED: &str = "BREP_UNSUPPORTED_OPERATION";
const OPERATION_FAILED: &str = "BREP_OPERATION_FAILED";

fn unsupported(message: impl Into<String>) -> Error {
    Error::new(UNSUPPORTED, message)
}
fn failed(message: impl Into<String>) -> Error {
    Error::new(OPERATION_FAILED, message)
}
fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] + b[i])
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn mul(a: [f64; 3], s: f64) -> [f64; 3] {
    a.map(|v| v * s)
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| a[i] * b[i]).sum()
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn norm(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}
fn unit(a: [f64; 3]) -> Result<[f64; 3]> {
    let n = norm(a);
    if !n.is_finite() || n <= 1e-12 {
        return Err(unsupported("Degenerate planar face"));
    }
    Ok(mul(a, 1. / n))
}
fn close(a: [f64; 3], b: [f64; 3], tolerance: f64) -> bool {
    norm(sub(a, b)) <= tolerance
}
fn loop_vertices(model: &Model, loop_id: usize) -> Result<Vec<usize>> {
    let wire = model
        .loops
        .get(loop_id)
        .ok_or_else(|| failed("Face references an unknown loop"))?;
    Ok(wire
        .coedges
        .iter()
        .map(|c| model.edges[c.edge].vertices[usize::from(c.reversed)])
        .collect())
}
