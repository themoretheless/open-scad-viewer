//! Boolean arrangements retaining exact source line/quadratic/cubic spans.
//!
//! Adaptive chords locate candidate intersections; intersections are refined on
//! the original curves, which are split with de Casteljau. Chords are never used
//! as replacement output geometry. The result has filled material on its left.
use crate::curve::{BezPath, CurveSegment, PathEl, Point, Rect, Shape};
pub use crate::tessellation::FillRule;
use crate::{Result, check};
use std::collections::{BTreeSet, HashMap};

mod binary_op;
mod binary_op_2;
#[cfg(test)]
mod tests;
pub use binary_op::*;
pub use binary_op_2::*;
