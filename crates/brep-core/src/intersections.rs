//! Exact analytic recognition (`Model` → canonical sphere, cylinder, cone,
//! torus or plane) and the sphere–sphere pair intersection the analytic
//! Booleans use. The remaining pair intersections live in `brep-intersect`;
//! the curve/surface engine behind all of them is `nurbs-intersect`.
use nurbs_core::{Result, curve::Curve};
use nurbs_intersect::*;
use nurbs_intersect::vec3::norm;

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
