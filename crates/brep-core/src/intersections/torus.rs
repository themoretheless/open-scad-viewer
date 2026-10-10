//! Exact recognition of the canonical analytic torus (sixteen rational quadrant
//! patches) on a retained model.
use super::sphere_sphere::{ARC_WEIGHT, RECOGNITION};
use super::*;
use crate::Model;

const TAU: f64 = std::f64::consts::TAU;
const QUARTER: f64 = std::f64::consts::FRAC_PI_2;
const QUADRANTS: [[f64; 2]; 4] = [[1., 0.], [0., 1.], [-1., 0.], [0., -1.]];

#[doc(hidden)]
/// A recognized canonical ring torus: the exact sixteen-patch solid of
/// `analytic::torus`, optionally rigidly placed.
#[derive(Clone, Debug)]
pub struct CanonicalTorus {
    /// Torus center (the center of the symmetry-plane circle of radius R).
    pub center: [f64; 3],
    /// Unit symmetry axis; the profile circle rises toward +axis.
    pub axis: [f64; 3],
    pub major: f64,
    pub minor: f64,
    /// Observed structural deviation bound; feeds the outward classification.
    pub error: f64,
    /// In-plane orthonormal ring frame: x = quadrant-0 direction, y = axis x x.
    pub frame: [[f64; 3]; 2],
    /// Face index by [profile quadrant][revolution quadrant].
    pub patches: [[usize; 4]; 4],
}

/// Profile-span control points (radial, axial) of the canonical torus
/// profile circle of radius `minor` centered at radial `major`: four
/// 90-degree arcs starting at the outer equator point, turning toward +axis.
fn profile_controls(major: f64, minor: f64) -> [[[f64; 2]; 3]; 4] {
    let (r, m) = (major, minor);
    [
        [[r + m, 0.], [r + m, m], [r, m]],
        [[r, m], [r - m, m], [r - m, 0.]],
        [[r - m, 0.], [r - m, -m], [r, -m]],
        [[r, -m], [r + m, -m], [r + m, 0.]],
    ]
}

#[doc(hidden)]
/// Recognizes a canonical ring-torus solid as built by `analytic::torus`,
/// certifying the 4x4 quadrant patch tiling, the exact tensor-product
/// weights [[1,w,1],[w,w^2,w],[1,w,1]] with w = cos(pi/4), the unit-square
/// boundary trims, every control point against the exact construction in the
/// recovered ring frame, and all sixteen ring/quadrant vertices. Rigid
/// affine placement is admitted; anything else returns `None`. Only strict
/// ring tori exist canonically (the constructor refuses horn and spindle
/// tori), and the recovered radii must keep R - r clearly positive.
pub fn recognize_torus(model: &Model) -> Result<Option<CanonicalTorus>> {
    model.validate()?;
    if model.bodies.len() != 1
        || model.shells.len() != 1
        || model.faces.len() != 16
        || model.vertices.len() != 16
        || model.edges.len() != 32
        || model.loops.len() != 16
    {
        return Ok(None);
    }
    let shell = &model.shells[0];
    if !shell.closed || shell.faces.len() != 16 {
        return Ok(None);
    }
    let w2 = ARC_WEIGHT * ARC_WEIGHT;
    let expected_weights = [
        [1., ARC_WEIGHT, 1.],
        [ARC_WEIGHT, w2, ARC_WEIGHT],
        [1., ARC_WEIGHT, 1.],
    ];
    for (face_index, face) in model.faces.iter().enumerate() {
        let surface = &face.surface;
        if surface.degree_u != 2
            || surface.degree_v != 2
            || surface.periodic_u
            || surface.periodic_v
            || surface.knots_u != [0., 0., 0., 1., 1., 1.]
            || surface.knots_v != [0., 0., 0., 1., 1., 1.]
            || surface.control_points.len() != 3
            || surface.control_points.iter().any(|row| row.len() != 3)
            || surface
                .control_points
                .iter()
                .flatten()
                .any(|p| p.len() != 3)
            || surface.weights != expected_weights
            || !face.holes.is_empty()
        {
            return Ok(None);
        }
        if !super::recognize::unit_square_boundary(model, face_index) {
            return Ok(None);
        }
    }
    // Center: each profile ring's four quadrant vertices average to the same
    // axis point, so all sixteen vertices average to the torus center.
    let mut center = [0.; 3];
    for vertex in &model.vertices {
        for k in 0..3 {
            center[k] += vertex.point[k] / 16.;
        }
    }
    let mut error: f64 = 0.;
    // The outer equator ring (rho = R + r, axial 0) is strictly the farthest
    // vertex set from the center for a ring torus: (R+r)^2 > R^2 + r^2.
    let dmax = model
        .vertices
        .iter()
        .map(|v| {
            let d = sub(v.point, center);
            d[0].hypot(d[1]).hypot(d[2])
        })
        .fold(0., f64::max);
    if !dmax.is_finite() || !(2e-5..=1e6).contains(&dmax) {
        return Ok(None);
    }
    let scale = dmax;
    let outer: Vec<[f64; 3]> = model
        .vertices
        .iter()
        .map(|v| v.point)
        .filter(|p| {
            let d = sub(*p, center);
            (d[0].hypot(d[1]).hypot(d[2]) - dmax).abs() <= RECOGNITION * scale
        })
        .collect();
    if outer.len() != 4 {
        return Ok(None);
    }
    // Axis from the outer ring plane; every outer vertex must lie in the
    // center plane within the band.
    let axis_raw = cross(sub(outer[1], outer[0]), sub(outer[2], outer[0]));
    let axis_len = axis_raw[0].hypot(axis_raw[1]).hypot(axis_raw[2]);
    if !axis_len.is_finite() || axis_len <= 0. {
        return Ok(None);
    }
    let axis = axis_raw.map(|x| x / axis_len);
    for p in &outer {
        let deviation = dot(sub(*p, center), axis).abs();
        if !deviation.is_finite() || deviation > RECOGNITION * scale {
            return Ok(None);
        }
        error = error.max(deviation);
    }
    // r is the extreme axial vertex coordinate; R = (R + r) - r.
    let minor = model
        .vertices
        .iter()
        .map(|v| dot(sub(v.point, center), axis).abs())
        .fold(0., f64::max);
    let major = dmax - minor;
    if !minor.is_finite() || !major.is_finite() || !(1e-5..=1e6).contains(&minor) || major <= minor
    {
        return Ok(None);
    }
    // Ring frame: x from the first outer-ring vertex, y = axis x x.
    let x_dir = sub(outer[0], center).map(|x| x / dmax);
    let y_dir = cross(axis, x_dir);
    let frame = [x_dir, y_dir];
    let profile = profile_controls(major, minor);
    // Quadrant of a ring-frame point; both angular residuals must sit at
    // pure rounding/recognition scale.
    let quadrant_of = |point: [f64; 3]| -> Option<(usize, usize, f64)> {
        let d = sub(point, center);
        let axial = dot(d, axis);
        let perp = sub(d, axis.map(|x| x * axial));
        let rho = perp[0].hypot(perp[1]).hypot(perp[2]);
        let alpha = axial.atan2(rho - major);
        let theta = dot(perp, y_dir).atan2(dot(perp, x_dir));
        let residual = |angle: f64, q: i64| {
            (angle - q as f64 * QUARTER + std::f64::consts::PI).rem_euclid(TAU)
                - std::f64::consts::PI
        };
        let jq = (alpha / QUARTER).round() as i64;
        let iq = (theta / QUARTER).round() as i64;
        if residual(alpha, jq).abs() > RECOGNITION || residual(theta, iq).abs() > RECOGNITION {
            return None;
        }
        Some((jq.rem_euclid(4) as usize, iq.rem_euclid(4) as usize, rho))
    };
    // Per-patch quadrant tiling and exact control-point certification.
    let mut seen = [[false; 4]; 4];
    let mut patches = [[0usize; 4]; 4];
    for (index, face) in model.faces.iter().enumerate() {
        let surface = &face.surface;
        let p00 = surface.evaluate(0., 0.)?.point;
        let Some((j, i, _)) = quadrant_of([p00[0], p00[1], p00[2]]) else {
            return Ok(None);
        };
        if seen[j][i] {
            return Ok(None);
        }
        seen[j][i] = true;
        patches[j][i] = index;
        let qa = QUADRANTS[i];
        let qb = QUADRANTS[(i + 1) % 4];
        let pattern = [
            [qa[0], qa[1]],
            [qa[0] + qb[0], qa[1] + qb[1]],
            [qb[0], qb[1]],
        ];
        for (a, dir) in pattern.iter().enumerate() {
            for (b, profile_point) in profile[j].iter().enumerate() {
                let expected: [f64; 3] = std::array::from_fn(|k| {
                    center[k]
                        + profile_point[0] * (dir[0] * x_dir[k] + dir[1] * y_dir[k])
                        + profile_point[1] * axis[k]
                });
                let actual = &surface.control_points[a][b];
                let d = [
                    actual[0] - expected[0],
                    actual[1] - expected[1],
                    actual[2] - expected[2],
                ];
                let deviation = d[0].hypot(d[1]).hypot(d[2]);
                if !deviation.is_finite() || deviation > RECOGNITION * scale + 1e-12 {
                    return Ok(None);
                }
                error = error.max(deviation);
            }
        }
    }
    if seen.iter().flatten().any(|hit| !hit) {
        return Ok(None);
    }
    // Every vertex is a ring/quadrant vertex, each exactly once, certified
    // radially and axially against the exact profile ring starts.
    let mut used = [[false; 4]; 4];
    for vertex in &model.vertices {
        let Some((j, _, rho)) = quadrant_of(vertex.point) else {
            return Ok(None);
        };
        let d = sub(vertex.point, center);
        let axial = dot(d, axis);
        let perp = sub(d, axis.map(|x| x * axial));
        let theta = dot(perp, y_dir).atan2(dot(perp, x_dir));
        let iq = (theta / QUARTER).round() as i64;
        let i = iq.rem_euclid(4) as usize;
        if used[j][i] {
            return Ok(None);
        }
        used[j][i] = true;
        let start = profile[j][0];
        let deviation = (rho - start[0]).abs().max((axial - start[1]).abs());
        if !deviation.is_finite() || deviation > RECOGNITION * scale + 1e-12 {
            return Ok(None);
        }
        error = error.max(deviation);
    }
    if used.iter().flatten().any(|hit| !hit) {
        return Ok(None);
    }
    Ok(Some(CanonicalTorus {
        center,
        axis,
        major,
        minor,
        error,
        frame,
        patches,
    }))
}
