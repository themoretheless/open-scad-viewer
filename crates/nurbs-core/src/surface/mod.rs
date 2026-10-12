use crate::curve::{Curve, basis, bounds};
use crate::{Result, check, numeric};
use math_core::{cross, dot, norm};

mod surface;
mod revolve;
#[cfg(test)]
mod construction_tests;
pub use surface::*;
pub use revolve::*;
