//! Exact analytic recognition (`Model` → canonical sphere, cylinder, cone,
//! torus or plane) and the sphere–sphere pair intersection the analytic
//! Booleans use. The remaining pair intersections live in `brep-intersect`;
//! the curve/surface engine behind all of them is `nurbs-intersect`.
use nurbs_core::{Result, curve::Curve};
use nurbs_intersect::*;
use nurbs_intersect::vec3::norm;
use sphere_sphere::ARC_WEIGHT;
use std::f64::consts::{FRAC_PI_2 as QUARTER, TAU};

#[cfg(test)]
mod test_utils;
mod cone;
mod cylinder;
mod plane;
pub(crate) mod recognize;
#[doc(hidden)]
pub mod sphere_sphere;
mod torus;
pub use sphere_sphere::{SpherePatchCircle, SphereSphereComponent, intersect_sphere_sphere};

#[doc(hidden)]
pub use cone::{CanonicalCone, recognize_cone};
#[doc(hidden)]
pub use cylinder::{CanonicalCylinder, recognize_cylinder};
#[doc(hidden)]
pub use plane::{CanonicalPlane, recognize_plane};
#[doc(hidden)]
pub use sphere_sphere::{CanonicalSphere, recognize as recognize_sphere};
#[doc(hidden)]
pub use torus::{CanonicalTorus, recognize_torus};

const QUADRANTS: [[f64; 2]; 4] = [[1., 0.], [0., 1.], [-1., 0.], [0., -1.]];

/// One quadrant's exact cap trim arc: the quarter circle of radius 1/2
/// centered at [1/2, 1/2] in cap UV, weights cos(pi/4).
fn cap_quarter_arc(curve: &Curve, quadrant: usize) -> bool {
    let [x, y] = QUADRANTS[quadrant];
    let [nx, ny] = QUADRANTS[(quadrant + 1) % 4];
    curve.degree == 2
        && curve.knots == [0., 0., 0., 1., 1., 1.]
        && curve.weights == [1., ARC_WEIGHT, 1.]
        && curve.control_points
            == [
                [0.5 + x / 2., 0.5 + y / 2.].to_vec(),
                [0.5 + (x + nx) / 2., 0.5 + (y + ny) / 2.].to_vec(),
                [0.5 + nx / 2., 0.5 + ny / 2.].to_vec(),
            ]
}
