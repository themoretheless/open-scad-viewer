//! Geometry and oriented contours of a patch in a clamped periodic chart.

use crate::{curve::Curve, surface::Surface};
use num_rational::BigRational;

#[derive(Clone, Debug)]
pub struct PeriodicTrimComponent {
    /// Original contour indices in the same order as `loops`.
    pub contours: Vec<usize>,
    /// Counterclockwise outer loop followed by clockwise holes.
    pub loops: Vec<Vec<Curve>>,
}

#[derive(Clone, Debug)]
pub struct PeriodicTrimPatch {
    pub surface: Surface,
    pub trim: PeriodicTrimComponent,
    /// Offset applied to the component's original UV lift to obtain this chart.
    pub uv_translation: [BigRational; 2],
}
