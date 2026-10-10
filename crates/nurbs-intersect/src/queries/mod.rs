//! Bounded, geometry-only intersection queries on retained rational definitions.
//!
//! This is the numerical foundation for sectioning and future curved Booleans,
//! not a Boolean topology oracle. Bernstein sign bounds and parameter boxes are
//! retained, but knot insertion/evaluation roundoff and branch adjacency do not
//! yet have independent coverage certificates. Even `NumericallyResolved` MUST
//! NOT be consumed as a certificate authorizing a topology change. No triangles,
//! fitted curves, snapping, or tessellation participate in these queries.
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};

mod codec;
mod curve_coincidence;
mod curve_curve;
mod curve_plane;
mod curve_ruled_surface;
mod curve_segment;
mod curve_surface;
mod ruled_coincidence;
mod ruled_points;
mod support;
mod surface_plane;
mod surface_surface;
mod surface_trace;
mod tangents;
#[cfg(test)]
mod tests;
pub(crate) use curve_coincidence::*;
pub use curve_curve::*;
pub use curve_plane::*;
pub use curve_ruled_surface::*;
pub use curve_segment::*;
pub use curve_surface::*;
pub(crate) use ruled_coincidence::*;
pub(crate) use ruled_points::*;
pub(crate) use support::*;
pub use surface_plane::*;
pub use surface_surface::*;
pub use surface_trace::*;
pub(crate) use tangents::*;

#[derive(Clone, Copy, Debug)]
pub struct Plane {
    /// The equation is normal.dot(point) = offset; need not be normalized.
    pub normal: [f64; 3],
    pub offset: f64,
}

impl Plane {
    pub(crate) fn normalized(self) -> Result<Self> {
        if !self.normal.iter().all(|v| v.is_finite()) || !self.offset.is_finite() {
            return Err(invalid(
                "Plane needs a finite, nonzero normal and finite offset",
            ));
        }
        let scale = self.normal.iter().map(|v| v.abs()).fold(0., f64::max);
        if scale == 0. {
            return Err(invalid(
                "Plane needs a finite, nonzero normal and finite offset",
            ));
        }
        // Never form the original norm: it may overflow, or round a
        // subnormal vector's length down to a single component. The scaled
        // norm is in [1,sqrt(3)] even at binary64's extreme exponents.
        let scaled = self.normal.map(|x| x / scale);
        let length = scaled[0].hypot(scaled[1]).hypot(scaled[2]);
        // Divide the larger offset by length first when offset/scale could
        // overflow. For a smaller offset, divide by scale first so a tiny
        // offset is not lost before a subsequent division by a tiny scale.
        let scaled_offset = self.offset / scale;
        let offset = if !scaled_offset.is_finite() {
            (self.offset / length) / scale
        } else {
            scaled_offset / length
        };
        let plane = Self {
            normal: scaled.map(|x| x / length),
            offset,
        };
        if !plane.offset.is_finite() {
            return Err(invalid("Normalized plane offset is not finite"));
        }
        Ok(plane)
    }
    pub(crate) fn distance(self, point: [f64; 3]) -> f64 {
        dot(self.normal, point) - self.offset
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// Absolute model-space residual target. Does not grow during a query.
    pub distance_tolerance: f64,
    /// Absolute parameter interval target, in the input knot coordinates.
    pub parameter_tolerance: f64,
    pub max_depth: usize,
    pub max_boxes: usize,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            distance_tolerance: 1e-9,
            parameter_tolerance: 1e-10,
            max_depth: 48,
            max_boxes: 8192,
        }
    }
}
impl Options {
    #[doc(hidden)]
    pub fn validate(self) -> Result<Self> {
        if !self.distance_tolerance.is_finite()
            || self.distance_tolerance <= 0.
            || !self.parameter_tolerance.is_finite()
            || self.parameter_tolerance <= 0.
            || !(1..=64).contains(&self.max_depth)
            || !(1..=65536).contains(&self.max_boxes)
        {
            return Err(invalid(
                "Intersection tolerances must be positive and finite; depth 1..64, boxes 1..65536",
            ));
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coverage {
    /// Domain partition certified for the frozen analytic/affine matrix only.
    Complete,
    /// All parameter regions were handled numerically. NOT certified complete.
    NumericallyResolved,
    Incomplete,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnresolvedReason {
    BudgetExceeded,
    TangencyOrMultipleRoot,
    NearCoincidence,
    BoundaryCrossing,
    UnsupportedSurface,
    CoincidentTrim,
}
#[derive(Clone, Debug)]
pub struct Unresolved {
    /// Curve: [t0,t1]. Surface: [u0,u1,v0,v1]. Surface pairs concatenate both boxes.
    pub parameter_box: Vec<f64>,
    pub reason: UnresolvedReason,
}
#[derive(Clone, Debug)]
pub struct Report<T> {
    pub components: Vec<T>,
    pub unresolved: Vec<Unresolved>,
    pub boxes_visited: usize,
    pub bernstein_excluded: usize,
    pub coverage: Coverage,
}
impl<T> Default for Report<T> {
    fn default() -> Self {
        Self {
            components: Vec::new(),
            unresolved: Vec::new(),
            boxes_visited: 0,
            bernstein_excluded: 0,
            coverage: Coverage::NumericallyResolved,
        }
    }
}
impl<T> Report<T> {
    /// Always false until independent coverage and correspondence certificates
    /// exist. Keeping this query beside coverage prevents accidental promotion.
    pub fn permits_topology_change(&self) -> bool {
        false
    }
    #[doc(hidden)]
    pub fn unresolved(&mut self, domain: impl Into<Vec<f64>>, reason: UnresolvedReason) {
        self.coverage = Coverage::Incomplete;
        self.unresolved.push(Unresolved {
            parameter_box: domain.into(),
            reason,
        });
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Contact {
    Transverse,
    Boundary,
}
#[derive(Clone, Debug)]
pub struct CurvePoint {
    pub parameter: f64,
    pub parameter_interval: [f64; 2],
    pub point: [f64; 3],
    pub plane_residual: f64,
    pub contact: Contact,
}

#[doc(hidden)]
pub use crate::vec3::{cross, dot, point3, sub};
