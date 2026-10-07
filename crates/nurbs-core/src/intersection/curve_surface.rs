//! Native events and coverage for curve/surface intersection.
use super::{ContactClass, ToleranceContext, UnresolvedReason};

pub struct CurveSurfacePoint {
    pub t: f64,
    pub t_interval: [f64; 2],
    pub uv: [f64; 2],
    pub uv_box: [f64; 4],
    pub point: [f64; 3],
    pub residual: f64,
    pub contact: ContactClass,
    pub multiplicity: u32,
    pub curve_wrap: i32,
    pub geometry_enclosure: [[f64; 2]; 3],
    pub parameter_box: [f64; 6],
}
pub struct CurveSurfaceOverlap {
    pub curve_interval: [f64; 2],
    pub uv_start: [f64; 2],
    pub uv_end: [f64; 2],
    pub curve_wrap: i32,
    pub geometry_enclosure: [[f64; 2]; 3],
    pub coedge_trim: brep_topology::CoedgeTrim,
    pub samples: [[f64; 3]; 3],
}
pub enum CurveSurfaceComponent {
    Point(CurveSurfacePoint),
    Overlap(CurveSurfaceOverlap),
}
impl CurveSurfaceComponent {
    pub(super) fn parameter_start(&self) -> f64 {
        match self {
            Self::Point(point) => point.t,
            Self::Overlap(overlap) => overlap.curve_interval[0],
        }
    }
}
pub enum CurveSurfaceParameterBox {
    Curve([f64; 2]),
    CurveSurface([f64; 6]),
}
pub struct UnresolvedCurveSurface {
    pub parameter_box: CurveSurfaceParameterBox,
    pub reason: UnresolvedReason,
}
pub struct CurveSurfaceIntersection {
    /// True only when the entire report has a continuous existence/coverage
    /// proof including arithmetic rounding. The current legacy solver returns
    /// approximate candidates and cannot set this flag.
    pub certified: bool,
    pub components: Vec<CurveSurfaceComponent>,
    pub unresolved: Vec<UnresolvedCurveSurface>,
    pub boxes_visited: usize,
    pub bernstein_excluded: usize,
    pub krawczyk_isolated: usize,
    pub tolerance: ToleranceContext,
}
