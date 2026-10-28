//! Polyline stroke expansion: caps, joins, optional dash → filled outline paths.
use crate::path::BezierPath;
use crate::{Result, check};
use math_core::{add2, norm2, scale2, sub2, unit2};

#[path = "../stroke_simple.rs"]
mod simple;

mod line_cap;
mod stroke_pieces;
#[cfg(test)]
mod tests;
pub use line_cap::*;
use stroke_pieces::*;
