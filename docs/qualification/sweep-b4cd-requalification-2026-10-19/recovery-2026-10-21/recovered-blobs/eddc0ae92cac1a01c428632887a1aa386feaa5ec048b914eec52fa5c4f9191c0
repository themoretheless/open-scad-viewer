//! Ruled transitions between authored section profiles.
use crate::{Result, check, curve::Curve, surface::Surface};

pub mod circle;
pub mod circle_rectangle;
pub mod ellipse;

/// Shared skeleton of two-section ruled transitions: both profiles must
/// differ, then a linear `surface::loft` over the pair.
pub(crate) fn ruled_loft_two(a: Curve, b: Curve, distinct_message: &str) -> Result<Surface> {
    check(a.control_points != b.control_points, distinct_message)?;
    crate::surface::loft(&[a, b])
}
