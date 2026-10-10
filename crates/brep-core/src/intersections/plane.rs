//! Exact recognition of a single planar bilinear patch on a retained model.
use super::sphere_sphere::RECOGNITION;
use super::*;
use crate::Model;

#[doc(hidden)]
/// A recognized canonical planar patch: an exact affine rectangular surface
/// `P(u,v) = origin + u*U + v*V` over `(u,v) in [0,1]^2`, with U and V
/// orthogonal within the recognition band.
#[derive(Clone, Debug)]
pub struct CanonicalPlane {
    /// Surface control point [0][0]: the (0,0) corner.
    pub origin: [f64; 3],
    /// Edge vector to the (1,0) corner.
    pub u: [f64; 3],
    /// Edge vector to the (0,1) corner.
    pub v: [f64; 3],
    /// Unit normal u x v / |u x v|.
    pub normal: [f64; 3],
    pub u_len: f64,
    pub v_len: f64,
    /// Observed structural deviation bound; feeds the outward classification.
    pub error: f64,
}

#[doc(hidden)]
/// Recognizes a canonical planar patch model: one body, one open shell, one
/// bilinear affine face over the unit square with exact unit weights, the
/// four unit-square boundary trims each exactly once, four corner vertices
/// matching the surface corners, an exactly-affine fourth corner, and edge
/// vectors orthogonal within the recognition band. Rigid affine placement is
/// admitted; anything else returns `None`.
pub fn recognize_plane(model: &Model) -> Result<Option<CanonicalPlane>> {
    model.validate()?;
    // Bodies require closed shells, so the freestanding patch is a single
    // open shell with no body at all.
    if !model.bodies.is_empty()
        || model.shells.len() != 1
        || model.faces.len() != 1
        || model.vertices.len() != 4
        || model.edges.len() != 4
        || model.loops.len() != 1
    {
        return Ok(None);
    }
    let shell = &model.shells[0];
    if shell.closed || shell.faces.len() != 1 {
        return Ok(None);
    }
    let face = &model.faces[0];
    let surface = &face.surface;
    if surface.degree_u != 1
        || surface.degree_v != 1
        || surface.periodic_u
        || surface.periodic_v
        || surface.knots_u != [0., 0., 1., 1.]
        || surface.knots_v != [0., 0., 1., 1.]
        || surface.control_points.len() != 2
        || surface.control_points.iter().any(|row| row.len() != 2)
        || surface
            .control_points
            .iter()
            .flatten()
            .any(|p| p.len() != 3)
        || surface.weights != [[1., 1.], [1., 1.]]
        || !face.holes.is_empty()
    {
        return Ok(None);
    }
    if !super::recognize::unit_square_boundary(model, 0) {
        return Ok(None);
    }
    let p = &surface.control_points;
    let p00 = point3(&p[0][0]);
    let p10 = point3(&p[1][0]);
    let p01 = point3(&p[0][1]);
    let p11 = point3(&p[1][1]);
    let u = sub(p10, p00);
    let v = sub(p01, p00);
    let u_len = u[0].hypot(u[1]).hypot(u[2]);
    let v_len = v[0].hypot(v[1]).hypot(v[2]);
    if !(1e-5..=1e6).contains(&u_len) || !(1e-5..=1e6).contains(&v_len) {
        return Ok(None);
    }
    let scale = u_len.max(v_len);
    let mut error: f64 = 0.;
    // Affine fourth corner: p11 == p00 + u + v.
    let deviation = {
        let d = [
            p11[0] - (p00[0] + u[0] + v[0]),
            p11[1] - (p00[1] + u[1] + v[1]),
            p11[2] - (p00[2] + u[2] + v[2]),
        ];
        d[0].hypot(d[1]).hypot(d[2])
    };
    if !deviation.is_finite() || deviation > RECOGNITION * scale + 1e-12 {
        return Ok(None);
    }
    error = error.max(deviation);
    // Rectangularity: the edge vectors must be orthogonal within the band.
    let skew = dot(u, v);
    let skew_model = skew.abs() / u_len.min(v_len);
    if !skew_model.is_finite() || skew_model > RECOGNITION * scale {
        return Ok(None);
    }
    error = error.max(skew_model);
    let normal = cross(u, v);
    let n_len = normal[0].hypot(normal[1]).hypot(normal[2]);
    if !n_len.is_finite() || n_len <= 0. {
        return Ok(None);
    }
    let normal = normal.map(|x| x / n_len);
    // Every vertex coincides with one surface corner, each exactly once.
    let corners = [p00, p10, p11, p01];
    let mut used = [false; 4];
    for vertex in &model.vertices {
        let mut hit = false;
        for (k, corner) in corners.iter().enumerate() {
            let d = sub(vertex.point, *corner);
            let deviation = d[0].hypot(d[1]).hypot(d[2]);
            if !used[k] && deviation <= RECOGNITION * scale {
                used[k] = true;
                hit = true;
                error = error.max(deviation);
                break;
            }
        }
        if !hit {
            return Ok(None);
        }
    }
    if used.into_iter().any(|hit| !hit) {
        return Ok(None);
    }
    Ok(Some(CanonicalPlane {
        origin: p00,
        u,
        v,
        normal,
        u_len,
        v_len,
        error,
    }))
}
