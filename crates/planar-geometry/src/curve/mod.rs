//! Compound Bézier paths and analytic curve queries.
//!
//! The familiar `Point`, `PathEl`, `BezPath` and `Shape` names form a narrow
//! migration boundary for vector-editor clients. This module is implemented
//! independently; it does not depend on or re-export another geometry engine.
//! Geometry stays as original polynomial curves until a consumer explicitly
//! requests flattening, stroking or tessellation.

use crate::path::{BezierPath, PathSegment};
use crate::{Result, check};

mod point;
mod shape;
#[cfg(test)]
mod tests;
pub use point::*;
pub use shape::*;
