//! Exact recognition of the canonical analytic cone (four ruled quadrant sides,
//! optional planar caps) on a retained model.
use super::sphere_sphere::{ARC_WEIGHT, RECOGNITION};
use super::*;
use crate::Model;

const TAU: f64 = std::f64::consts::TAU;
const QUARTER: f64 = std::f64::consts::FRAC_PI_2;
const QUADRANTS: [[f64; 2]; 4] = [[1., 0.], [0., 1.], [-1., 0.], [0., -1.]];

#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct CanonicalCone {
    /// Bottom ring center (the apex itself when the bottom radius is zero).
    pub bottom: [f64; 3],
    /// Unit axis, bottom ring toward top ring.
    pub axis: [f64; 3],
    pub r_bottom: f64,
    pub r_top: f64,
    pub height: f64,
    /// Radius slope (r_top - r_bottom) / height, certified nonzero.
    pub slope: f64,
    /// Observed structural deviation bound; feeds the outward classification.
    pub error: f64,
    /// In-plane orthonormal ring frame: x = quadrant-0 direction, y = axis x x.
    pub frame: [[f64; 3]; 2],
    /// Side face indices ordered by quadrant.
    pub sides: [usize; 4],
    /// Cap face indices [bottom, top]; None for an apex ring.
    #[allow(dead_code)]
    pub caps: [Option<usize>; 2],
}

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



#[doc(hidden)]
/// Recognizes a canonical conical frustum solid as built by
/// `analytic::frustum`, including the true-apex cone (exactly one radius
/// zero: the canonical constructor collapses that ring onto one shared
/// pole vertex and skips its cap). Certifies the side/cap surface
/// structure, exact weights and trim pcurves, every control point against
/// the exact construction in the recovered ring frame, a globally
/// consistent quadrant tiling, both ring vertex sets, and a
/// certified-nonzero taper (an equal-radius frustum is a cylinder and is
/// refused here). Rigid affine placement is admitted; anything else
/// returns `None`.
pub fn recognize_cone(model: &Model) -> Result<Option<CanonicalCone>> {
    model.validate()?;
    if model.bodies.len() != 1 || model.shells.len() != 1 {
        return Ok(None);
    }
    // Frustum: 4 sides + 2 caps, 8 ring vertices. Apex cone: 4 sides + 1
    // cap, 4 ring vertices + 1 shared apex vertex.
    if !((model.faces.len() == 6 && model.vertices.len() == 8)
        || (model.faces.len() == 5 && model.vertices.len() == 5))
    {
        return Ok(None);
    }
    let Some((sides, caps)) = recognize::classify_analytic_faces(model) else {
        return Ok(None);
    };
    if sides.len() != 4 || caps.is_empty() || caps.len() > 2 {
        return Ok(None);
    }
    // Side trims: the unit-square boundary, each edge exactly once.
    for &index in &sides {
        if !recognize::unit_square_boundary(model, index) {
            return Ok(None);
        }
    }
    // Ring centers: each ring vertex is shared by two adjacent side patches,
    // so the eight evaluated corners average to the ring center. On an apex
    // ring every corner evaluates to the one shared apex vertex.
    let mut bottom = [0.; 3];
    let mut top = [0.; 3];
    for &index in &sides {
        let surface = &model.faces[index].surface;
        for u in [0., 1.] {
            let b = point3(&surface.evaluate(u, 0.)?.point);
            let t = point3(&surface.evaluate(u, 1.)?.point);
            for k in 0..3 {
                bottom[k] += b[k] / 8.;
                top[k] += t[k] / 8.;
            }
        }
    }
    let axis_vec = sub(top, bottom);
    let height = norm(axis_vec);
    if !height.is_finite() || !(1e-5..=1e6).contains(&height) {
        return Ok(None);
    }
    let axis = axis_vec.map(|x| x / height);
    let radial = |point: [f64; 3], from: [f64; 3]| {
        let d = sub(point, from);
        let axial = dot(d, axis);
        sub(d, axis.map(|x| x * axial))
    };
    // Ring radii from the four arc midpoints (u = v = 1/2 sits on the
    // 45-degree point of the quarter arc at that height's radius).
    let mut r_bottom = 0.;
    let mut r_top = 0.;
    let mut mid_radii = Vec::with_capacity(8);
    for &index in &sides {
        let pb = point3(&model.faces[index].surface.evaluate(0.5, 0.)?.point);
        let pt = point3(&model.faces[index].surface.evaluate(0.5, 1.)?.point);
        let rb = norm(radial(pb, bottom));
        let rt = norm(radial(pt, top));
        mid_radii.push((rb, false));
        mid_radii.push((rt, true));
        r_bottom += rb / 4.;
        r_top += rt / 4.;
    }
    let r_scale = r_bottom.max(r_top);
    if !r_scale.is_finite() || !(1e-5..=1e6).contains(&r_scale) {
        return Ok(None);
    }
    let mut error: f64 = 0.;
    for (r, is_top) in &mid_radii {
        let ring = if *is_top { r_top } else { r_bottom };
        let deviation = (r - ring).abs();
        if deviation > RECOGNITION * r_scale + 1e-12 {
            return Ok(None);
        }
        error = error.max(deviation);
    }
    // An apex ring is exactly zero in the canonical construction; snap a
    // rounding-scale radius to zero and record the deviation.
    let (r_bottom, r_top) = (
        if r_bottom <= RECOGNITION * r_scale + 1e-12 {
            error = error.max(r_bottom);
            0.
        } else {
            r_bottom
        },
        if r_top <= RECOGNITION * r_scale + 1e-12 {
            error = error.max(r_top);
            0.
        } else {
            r_top
        },
    );
    // Certified-nonzero taper: an equal-radius frustum is a cylinder and is
    // refused here (the plane/cylinder cell owns it).
    if (r_top - r_bottom).abs() <= RECOGNITION * r_scale + 1e-12 {
        return Ok(None);
    }
    // In-plane frame from a nonzero ring: x is the quadrant-0 direction.
    let (ring_center, ring_v) = if r_bottom > 0. {
        (bottom, 0.)
    } else {
        (top, 1.)
    };
    let start = point3(&model.faces[sides[0]].surface.evaluate(0., ring_v)?.point);
    let x_perp = radial(start, ring_center);
    let x_length = norm(x_perp);
    if !x_length.is_finite() || x_length <= 0. {
        return Ok(None);
    }
    let x_dir = x_perp.map(|x| x / x_length);
    let y_dir = cross(axis, x_dir);
    // Per-patch quadrant tiling and exact control-point certification
    // against the linearly interpolated ring radii (apex row: the apex).
    let mut seen = [false; 4];
    let mut ordered = [0usize; 4];
    for &index in &sides {
        let surface = &model.faces[index].surface;
        let point = point3(&surface.evaluate(0., ring_v)?.point);
        let perp = radial(point, ring_center);
        let angle = dot(perp, y_dir).atan2(dot(perp, x_dir));
        let quadrant = (angle / QUARTER).round() as i64;
        let quadrant = quadrant.rem_euclid(4) as usize;
        let residual = (angle - quadrant as f64 * QUARTER + std::f64::consts::PI).rem_euclid(TAU)
            - std::f64::consts::PI;
        if residual.abs() > RECOGNITION {
            return Ok(None);
        }
        if seen[quadrant] {
            return Ok(None);
        }
        seen[quadrant] = true;
        ordered[quadrant] = index;
        let qa = QUADRANTS[quadrant];
        let qb = QUADRANTS[(quadrant + 1) % 4];
        let pattern = [
            [qa[0], qa[1]],
            [qa[0] + qb[0], qa[1] + qb[1]],
            [qb[0], qb[1]],
        ];
        for (k, expected_xy) in pattern.iter().enumerate() {
            for (j, (ring_r, zz)) in [(r_bottom, 0.), (r_top, height)].iter().enumerate() {
                let expected: [f64; 3] = std::array::from_fn(|a| {
                    bottom[a]
                        + ring_r * (expected_xy[0] * x_dir[a] + expected_xy[1] * y_dir[a])
                        + zz * axis[a]
                });
                let actual = &surface.control_points[k][j];
                if actual.len() != 3 {
                    return Ok(None);
                }
                let d = [
                    actual[0] - expected[0],
                    actual[1] - expected[1],
                    actual[2] - expected[2],
                ];
                let deviation = norm(d);
                if !deviation.is_finite() || deviation > RECOGNITION * r_scale + 1e-12 {
                    return Ok(None);
                }
                error = error.max(deviation);
            }
        }
    }
    if seen.into_iter().any(|hit| !hit) {
        return Ok(None);
    }
    // Caps: axial slot from any corner, exact bilinear corners in the ring
    // frame at that ring's radius, and the four inscribed quarter-arc trims
    // each exactly once. An apex ring has no cap.
    let mut cap_assigned = [false; 2];
    let mut cap_ids: [Option<usize>; 2] = [None, None];
    for &index in &caps {
        let surface = &model.faces[index].surface;
        let corner = point3(&surface.evaluate(0., 0.)?.point);
        let axial = dot(sub(corner, bottom), axis);
        let slot = if axial.abs() <= RECOGNITION * height {
            0
        } else if (axial - height).abs() <= RECOGNITION * height {
            1
        } else {
            return Ok(None);
        };
        error = error.max(if slot == 0 {
            axial.abs()
        } else {
            (axial - height).abs()
        });
        let ring_r = if slot == 0 { r_bottom } else { r_top };
        if ring_r == 0. {
            return Ok(None);
        }
        if cap_assigned[slot] {
            return Ok(None);
        }
        cap_assigned[slot] = true;
        cap_ids[slot] = Some(index);
        let ring_c = if slot == 0 { bottom } else { top };
        for (i, row) in surface.control_points.iter().take(2).enumerate() {
            for (j, actual) in row.iter().take(2).enumerate() {
                let expected: [f64; 3] = std::array::from_fn(|a| {
                    ring_c[a]
                        + ring_r
                            * ((2. * i as f64 - 1.) * x_dir[a] + (2. * j as f64 - 1.) * y_dir[a])
                });
                if actual.len() != 3 {
                    return Ok(None);
                }
                let d = [
                    actual[0] - expected[0],
                    actual[1] - expected[1],
                    actual[2] - expected[2],
                ];
                let deviation = norm(d);
                if !deviation.is_finite() || deviation > RECOGNITION * r_scale + 1e-12 {
                    return Ok(None);
                }
                error = error.max(deviation);
            }
        }
        let loop_ = &model.loops[model.faces[index].outer];
        if loop_.coedges.len() != 4 {
            return Ok(None);
        }
        let mut seen_quadrant = [false; 4];
        for coedge in &loop_.coedges {
            let mut hit = false;
            for (quadrant, seen) in seen_quadrant.iter_mut().enumerate() {
                if !*seen && cap_quarter_arc(&coedge.pcurve, quadrant) {
                    *seen = true;
                    hit = true;
                    break;
                }
            }
            if !hit {
                return Ok(None);
            }
        }
        if seen_quadrant.into_iter().any(|hit| !hit) {
            return Ok(None);
        }
    }
    if cap_assigned[0] != (r_bottom > 0.) || cap_assigned[1] != (r_top > 0.) {
        return Ok(None);
    }
    // Every vertex on one of the two rings (the apex ring: the pole).
    for vertex in &model.vertices {
        let d = sub(vertex.point, bottom);
        let axial = dot(d, axis);
        let perp = sub(d, axis.map(|x| x * axial));
        let slot = if axial.abs() <= RECOGNITION * height {
            0
        } else if (axial - height).abs() <= RECOGNITION * height {
            1
        } else {
            return Ok(None);
        };
        let ring_r = if slot == 0 { r_bottom } else { r_top };
        let dev_r = (norm(perp) - ring_r).abs();
        if dev_r > RECOGNITION * r_scale + 1e-12 {
            return Ok(None);
        }
        error = error.max(dev_r);
    }
    Ok(Some(CanonicalCone {
        bottom,
        axis,
        r_bottom,
        r_top,
        height,
        slope: (r_top - r_bottom) / height,
        error,
        frame: [x_dir, y_dir],
        sides: ordered,
        caps: cap_ids,
    }))
}
