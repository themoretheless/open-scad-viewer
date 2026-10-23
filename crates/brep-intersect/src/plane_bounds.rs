//! The rectangular UV bounds of a recognised plane patch as four halfplanes,
//! shared by every plane/quadric pair that clips its conic in plane UV.
use super::plane_sphere::Halfplane2;
use brep_core::intersections::CanonicalPlane;

/// Rectangle half-planes of the patch domain with per-axis bands.
pub(crate) fn rect_halfplanes(plane: &CanonicalPlane, band: f64) -> [Halfplane2; 4] {
    [
        Halfplane2 {
            normal: [-1., 0.],
            offset: 0.,
            band: band / plane.u_len,
        },
        Halfplane2 {
            normal: [1., 0.],
            offset: 1.,
            band: band / plane.u_len,
        },
        Halfplane2 {
            normal: [0., -1.],
            offset: 0.,
            band: band / plane.v_len,
        },
        Halfplane2 {
            normal: [0., 1.],
            offset: 1.,
            band: band / plane.v_len,
        },
    ]
}
