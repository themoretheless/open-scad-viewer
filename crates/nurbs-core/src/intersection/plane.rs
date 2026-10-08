//! Plane geometry used by native curve/surface reduction.
use super::{Result, Surface, check, cross3, dot3, norm3, point3};

pub(super) struct Plane {
    pub normal: [f64; 3],
    pub offset: f64,
}

pub(super) fn plane_of(surface: &Surface) -> Result<Option<Plane>> {
    // Affine bilinear degree-(1,1) with planar controls: exact plane residual.
    if surface.degree_u != 1 || surface.degree_v != 1 {
        return Ok(None);
    }
    let corners = [
        &surface.control_points[0][0],
        &surface.control_points[0][1],
        &surface.control_points[1][0],
        &surface.control_points[1][1],
    ];
    let o = point3(corners[0])?;
    let a = [
        corners[1][0] - o[0],
        corners[1][1] - o[1],
        corners[1][2] - o[2],
    ];
    let b = [
        corners[2][0] - o[0],
        corners[2][1] - o[1],
        corners[2][2] - o[2],
    ];
    let n = cross3(a, b);
    let nn = norm3(n);
    if !nn.is_finite() || nn <= 0. {
        return Ok(None);
    }
    let normal = n.map(|x| x / nn);
    let offset = dot3(normal, o);
    // All surface corners must lie on the plane.
    for corner in &corners {
        let p = point3(corner)?;
        if (dot3(normal, p) - offset).abs() > 1e-12 {
            return Ok(None);
        }
    }
    Ok(Some(Plane { normal, offset }))
}

pub(super) fn invert_plane_uv(surface: &Surface, point: [f64; 3]) -> Result<[f64; 2]> {
    let o = point3(&surface.control_points[0][0])?;
    let u_dir = [
        surface.control_points[1][0][0] - o[0],
        surface.control_points[1][0][1] - o[1],
        surface.control_points[1][0][2] - o[2],
    ];
    let v_dir = [
        surface.control_points[0][1][0] - o[0],
        surface.control_points[0][1][1] - o[1],
        surface.control_points[0][1][2] - o[2],
    ];
    let d = [point[0] - o[0], point[1] - o[1], point[2] - o[2]];
    let guu = dot3(u_dir, u_dir);
    let guv = dot3(u_dir, v_dir);
    let gvv = dot3(v_dir, v_dir);
    let det = guu * gvv - guv * guv;
    check(det.abs() > 0., "Degenerate planar frame")?;
    let ru = dot3(d, u_dir);
    let rv = dot3(d, v_dir);
    let [u0, u1] = [
        surface.knots_u[surface.degree_u],
        surface.knots_u[surface.knots_u.len() - surface.degree_u - 1],
    ];
    let [v0, v1] = [
        surface.knots_v[surface.degree_v],
        surface.knots_v[surface.knots_v.len() - surface.degree_v - 1],
    ];
    let su = (gvv * ru - guv * rv) / det;
    let sv = (-guv * ru + guu * rv) / det;
    Ok([u0 + su * (u1 - u0), v0 + sv * (v1 - v0)])
}
