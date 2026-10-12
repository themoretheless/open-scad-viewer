//! Local bridge operations, one router per operation family; `dispatch_local` tries them
//! in turn and leaves unowned operations to `fallback`.
pub(crate) mod mesh;
pub(crate) mod intersections;
pub(crate) mod sweep;
pub(crate) mod interchange;
pub(crate) mod nurbs;
pub(crate) mod cad;
pub(crate) mod brep;
pub(crate) mod general;
pub(crate) mod fallback;

use bridge_codec::Router;

pub(crate) const ROUTES: &[Router] = &[
    mesh::route,
    intersections::route,
    sweep::route,
    interchange::route,
    nurbs::route,
    cad::route,
    brep::route,
    general::route,
];
