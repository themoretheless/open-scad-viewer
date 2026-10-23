//! Closed rings: area, winding, planar boolean, offset.
use crate::tessellation::FillRule;
use crate::{Result, check};
use std::collections::{BTreeSet, HashMap};

pub type Rings = Vec<Vec<math_core::V2>>;
pub use math_core::{cross2, sub2};

use winding::WindingIndex;

mod winding;

mod area;
mod offset_join;
#[cfg(test)]
mod tests;
pub use area::*;
pub use offset_join::*;
