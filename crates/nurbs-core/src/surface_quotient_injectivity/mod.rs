//! Sufficient global injectivity on a rectangular chart with one collapsed
//! parameter boundary. Only that entire boundary is identified to one point.
use crate::{Result, check, distance_bounds::Interval as I, surface::Surface};
type Poly = Vec<Vec<I>>;

mod report;
#[cfg(test)]
mod tests;
pub use report::*;
