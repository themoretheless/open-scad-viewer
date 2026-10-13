//! Progressive profile transport with simultaneous scale and angular twist.
//!
//! Each level is a retained piecewise-linear NURBS patch set. Refinement compares
//! profile controls against fourfold finer transport; this is sampled evidence,
//! not a continuous RMF, injectivity, or rounding-inclusive certificate.
use crate::{Result, check, sweep_support::vec3_ext::norm, curve::Curve, surface::Surface};
use math_core::{cross, dot, sub};
use std::f64::consts::TAU;

type V = [f64; 3];


#[cfg(feature = "codec")]
mod serialization;

#[cfg(test)]
mod tests;

mod orientation;
mod sweep;
mod approximate;
pub use orientation::*;
pub use approximate::*;
