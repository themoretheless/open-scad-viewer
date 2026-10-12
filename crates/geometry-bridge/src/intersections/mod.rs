//! Read-only native intersection queries; reports never authorize topology edits.
use super::{Result, Value, encode, field, input};
use nurbs_intersect::{self as queries, Options, SurfaceTrace};

mod dispatch;
#[cfg(test)]
mod tests;
pub use dispatch::*;
