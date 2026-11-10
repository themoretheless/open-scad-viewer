//! Bounded, geometry-only intersection queries on retained rational definitions.
//!
//! This is the numerical foundation for sectioning and future curved Booleans,
//! not a Boolean topology oracle. Bernstein sign bounds and parameter boxes are
//! retained, but knot insertion/evaluation roundoff and branch adjacency do not
//! yet have independent coverage certificates. Even `NumericallyResolved` MUST
//! NOT be consumed as a certificate authorizing a topology change. No triangles,
//! fitted curves, snapping, or tessellation participate in these queries.
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};

mod cone_cone;
mod cone_torus;
mod cylinder_cylinder;
mod cylinder_torus;
mod plane_cone;
mod plane_cylinder;
mod plane_sphere;
mod plane_torus;
pub(crate) mod recognize;
mod sphere_cone;
mod sphere_cylinder;
pub mod sphere_sphere;
mod sphere_torus;
#[cfg(test)]
mod test_utils;
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
pub use sphere_sphere::{SpherePatchCircle, SphereSphereComponent, intersect_sphere_sphere};
pub use sphere_torus::{SphereTorusComponent, intersect_sphere_torus};
pub use torus_torus::{TorusTorusComponent, intersect_torus_torus};

pub use plane_cone::{CanonicalCone, recognize_cone};
pub use plane_torus::recognize_torus;
pub use sphere_cylinder::{CanonicalCylinder, recognize_cylinder};
pub use sphere_sphere::recognize as recognize_sphere;

/// Shared query vocabulary; `nurbs-intersect` owns these types (ADR 0011).
pub use nurbs_intersect::{Contact, Coverage, CurvePoint, Options, Plane, Report, Unresolved, UnresolvedReason};
/// The query engine (curve and surface queries against planes, curves and surfaces) is owned
/// by `nurbs-intersect` (ADR 0011). The analytic pair solvers above stay in the kernel (M3).
pub use nurbs_intersect::{
    CurveCurveComponent, CurvePlaneComponent, CurveRuledSurfaceComponent, CurveSegmentComponent,
    CurveSurfaceComponent, OverlapCorrespondence, OverlapCorrespondenceKind, SurfacePlaneComponent,
    SurfacePoint, SurfaceSurfaceComponent, SurfaceTrace, curve_curve, curve_plane, curve_ruled_surface,
    curve_segment, curve_surface, surface_plane, surface_surface,
};
pub(crate) use nurbs_intersect::{cross, dot, point3, sub};
#[cfg(test)]
#[path = "intersections/tests/tests_integration.rs"]
mod tests;

#[doc(hidden)]
pub use plane_sphere::{CanonicalPlane, recognize_plane};
#[doc(hidden)]
pub use plane_torus::CanonicalTorus;
