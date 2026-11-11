//! Layer 2 of the NURBS stack: foundation edits and fitting, distance bounds and certificates.
//! Split out of `nurbs-core` (refactor plan O3); `nurbs_core::{foundation, distance,
//! certificates}` and the crate-root aliases re-export every module.
// `unwrap`/`expect` in production code are counted per crate (`clippyUnwrap` pins, plan L6)
// and may only go down; prefer typed errors.
#![cfg_attr(not(test), warn(clippy::unwrap_used, clippy::expect_used))]
pub use math_core::{Error, Result};
#[doc(hidden)]
pub use nurbs_numerics::{INVALID_INPUT, NUMERIC_ERROR, RESOURCE_LIMIT, check, input, numeric, numeric_err, resource};
/// Lower layers under the paths these modules used inside `nurbs-core`.
#[doc(hidden)]
pub mod core {
    pub use nurbs_geometry::*;
}
#[doc(hidden)]
pub mod numerics {
    pub use nurbs_numerics::*;
}
pub mod foundation;
pub mod distance;
pub mod certificates;

// Crate-root aliases copied from nurbs-core for the modules of layers 0-2.
#[doc(hidden)]
pub use crate::core::affine;
#[doc(hidden)]
pub use crate::core::conditioning;
#[doc(hidden)]
pub use crate::core::coordinate_frame;
#[doc(hidden)]
pub use crate::foundation::sections;
#[doc(hidden)]
pub use crate::core::bounds;
#[doc(hidden)]
pub use crate::foundation::closed_spline;
#[doc(hidden)]
pub use crate::core::curve;
#[doc(hidden)]
pub use crate::core::geometry;
#[doc(hidden)]
pub use crate::foundation::canonical;
#[doc(hidden)]
pub use crate::distance::interval_eval;
#[doc(hidden)]
pub use crate::distance::curve_distance;
#[doc(hidden)]
pub use crate::certificates::curve_decomposition_certificate;
#[doc(hidden)]
pub use crate::core::exact_curve_segments;
#[doc(hidden)]
pub use crate::distance::curve_deviation;
#[doc(hidden)]
pub use crate::distance::curve_surface_distance;
#[doc(hidden)]
pub use crate::numerics::distance_bounds;
#[doc(hidden)]
pub use crate::foundation::grid_spline;
#[doc(hidden)]
pub use crate::foundation::hermite;
#[doc(hidden)]
pub use crate::foundation::natural_loft;
#[doc(hidden)]
pub use crate::foundation::natural_spline;
#[doc(hidden)]
pub use crate::distance_bounds::DistanceStopReason;
#[doc(hidden)]
pub use crate::distance::curve_jets;
#[doc(hidden)]
pub use crate::core::curve::Curve as NurbsCurve;
#[doc(hidden)]
pub use crate::numerics::periodic_chart;
#[doc(hidden)]
pub use crate::distance::radial_bounds;
#[doc(hidden)]
pub use crate::core::surface;
#[doc(hidden)]
pub use crate::distance::surface_distance;
#[doc(hidden)]
pub use crate::core::retained_wall_coefficients;
#[doc(hidden)]
pub use crate::numerics::exact_products;
#[doc(hidden)]
pub use crate::numerics::compensated;
#[doc(hidden)]
pub use crate::foundation::dual;
#[doc(hidden)]
pub use crate::numerics::robust_solvers;
#[doc(hidden)]
pub use crate::distance::interval_newton;
#[doc(hidden)]
pub use crate::numerics::sturm;
#[doc(hidden)]
pub use crate::distance::convex_distance;
#[doc(hidden)]
pub use crate::distance::obb_tree;
#[doc(hidden)]
pub use crate::foundation::bezier_extraction;
#[doc(hidden)]
pub use crate::distance::uv_curve_crossings;
#[doc(hidden)]
pub use crate::core::curve_surface_plane;
#[doc(hidden)]
pub use crate::core::sweep_support;
#[doc(hidden)]
pub use crate::distance::curve_axis_driver;
#[doc(hidden)]
pub use crate::core::curve_point_identity;
#[doc(hidden)]
pub use crate::core::surface_projection_jacobian;
#[doc(hidden)]
pub use crate::core::curve_quadratic_separator;
#[doc(hidden)]
pub use crate::distance::polar_surface_differential;
#[doc(hidden)]
pub use crate::numerics::bernstein_interpolation as bernstein_interpolation;
#[doc(hidden)]
pub use crate::distance::rational_pole_differential;
#[doc(hidden)]
pub use crate::distance::synchronized_boundary_normals;

/// Test fixtures that these modules' tests took from `nurbs_core::primitives`, a higher layer.
#[cfg(test)]
mod primitives {
    pub use crate::foundation::conics::circle_arc;
    /// Same curve as `nurbs_core::primitives::line`: a degree-one segment on [0, 1].
    pub fn line(start: [f64; 3], end: [f64; 3]) -> crate::Result<crate::curve::Curve> {
        let mut curve = crate::curve::Curve::from_polyline(vec![start.to_vec(), end.to_vec()])?;
        let end = curve.domain()[1];
        for knot in &mut curve.knots {
            *knot /= end
        }
        curve.validate()?;
        Ok(curve)
    }
}
