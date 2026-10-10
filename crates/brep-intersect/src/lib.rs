//! Analytic pair intersections on retained B-rep models: plane, sphere,
//! cylinder, cone and torus against each other, each recognised exactly on
//! the model and reported through the `nurbs-intersect` report schema. The
//! sphere–sphere pair still lives in `brep-core` beside the analytic
//! Booleans that consume it directly and is re-exported here.
use nurbs_core::{Result, curve::Curve};
use nurbs_intersect::*;

#[cfg(test)]
mod test_utils;
mod cone_cone;
mod cone_torus;
mod cylinder_cylinder;
mod cylinder_torus;
mod plane_cone;
mod plane_cylinder;
mod plane_sphere;
mod plane_torus;
mod sphere_cone;
mod sphere_cylinder;
mod sphere_torus;
mod torus_torus;
pub use cone_cone::{ConeConeComponent, intersect_cone_cone};
pub use cone_torus::{ConeTorusComponent, intersect_cone_torus};
pub use cylinder_cylinder::{CylinderCylinderComponent, intersect_cylinder_cylinder};
pub use cylinder_torus::{CylinderTorusComponent, intersect_cylinder_torus};
pub use plane_cone::{PlaneConeComponent, intersect_plane_cone};
pub use plane_cylinder::{PlaneCylinderComponent, intersect_plane_cylinder};
pub use plane_sphere::{PlanePatchCurve, PlaneSphereComponent, intersect_plane_sphere};
pub use plane_torus::{PlaneTorusComponent, TorusPatchCurve, intersect_plane_torus};
pub use sphere_cone::{SphereConeComponent, intersect_sphere_cone};
pub use sphere_cylinder::{CylinderPatchCurve, SphereCylinderComponent, intersect_sphere_cylinder};
pub use sphere_torus::{SphereTorusComponent, intersect_sphere_torus};
pub use torus_torus::{TorusTorusComponent, intersect_torus_torus};
pub use brep_core::intersections::{SpherePatchCircle, SphereSphereComponent, intersect_sphere_sphere};
