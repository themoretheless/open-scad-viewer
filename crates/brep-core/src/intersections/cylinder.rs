//! Exact recognition of the canonical analytic cylinder (four ruled quadrant
//! sides, two planar caps) on a retained model.
use super::sphere_sphere::{ARC_WEIGHT, RECOGNITION};
use super::*;
use crate::Model;

const QUADRANTS: [[f64; 2]; 4] = [[1., 0.], [0., 1.], [-1., 0.], [0., -1.]];
const TAU: f64 = std::f64::consts::TAU;

#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct CanonicalCylinder {
    /// Midpoint of the axis segment.
    pub center: [f64; 3],
    /// Unit axis, bottom cap toward top cap.
    pub axis: [f64; 3],
    pub radius: f64,
    pub half_height: f64,
    /// Observed structural deviation bound; feeds the outward classification.
    pub error: f64,
    /// In-plane orthonormal ring frame: x = quadrant-0 direction, y = axis x x.
    pub frame: [[f64; 3]; 2],
    /// Side face indices ordered by quadrant.
    pub sides: [usize; 4],
    /// Cap face indices: [bottom, top].
    #[allow(dead_code)]
    pub caps: [usize; 2],
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

fn point_of(jet_point: &[f64]) -> [f64; 3] {
    [jet_point[0], jet_point[1], jet_point[2]]
}

#[doc(hidden)]
/// Recognizes a canonical cylinder solid as built by `analytic::cylinder`,
/// certifying the side/cap surface structure, exact weights and trim pcurves,
/// every control point against the exact construction in the recovered ring
/// frame, a globally consistent quadrant tiling, and both ring vertex sets.
/// Rigid affine placement is admitted; anything else returns `None`.
pub fn recognize_cylinder(model: &Model) -> Result<Option<CanonicalCylinder>> {
    model.validate()?;
    if model.bodies.len() != 1
        || model.shells.len() != 1
        || model.faces.len() != 6
        || model.vertices.len() != 8
    {
        return Ok(None);
    }
    let Some((sides, caps)) = recognize::classify_analytic_faces(model) else {
        return Ok(None);
    };
    if sides.len() != 4 || caps.len() != 2 {
        return Ok(None);
    }
    // Side trims: the unit-square boundary, each edge exactly once.
    for &index in &sides {
        if !recognize::unit_square_boundary(model, index) {
            return Ok(None);
        }
    }
    // Ring centers: each ring vertex is shared by two adjacent side patches,
    // so the eight evaluated corners average to the ring center.
    let mut bottom = [0.; 3];
    let mut top = [0.; 3];
    for &index in &sides {
        let surface = &model.faces[index].surface;
        for u in [0., 1.] {
            let b = point_of(&surface.evaluate(u, 0.)?.point);
            let t = point_of(&surface.evaluate(u, 1.)?.point);
            for k in 0..3 {
                bottom[k] += b[k] / 8.;
                top[k] += t[k] / 8.;
            }
        }
    }
    let axis_vec = sub(top, bottom);
    let height = axis_vec[0].hypot(axis_vec[1]).hypot(axis_vec[2]);
    if !height.is_finite() || !(1e-5..=1e6).contains(&height) {
        return Ok(None);
    }
    let axis = axis_vec.map(|x| x / height);
    let center = std::array::from_fn(|k| (bottom[k] + top[k]) / 2.);
    let radial = |point: [f64; 3], from: [f64; 3]| {
        let d = sub(point, from);
        let axial = dot(d, axis);
        sub(d, axis.map(|x| x * axial))
    };
    // Radius from the four arc midpoints (u = v = 1/2 sits on the 45-degree
    // point of the quarter arc at half height).
    let mut radius = 0.;
    let mut mid_radii = Vec::with_capacity(4);
    for &index in &sides {
        let point = point_of(&model.faces[index].surface.evaluate(0.5, 0.5)?.point);
        let perp = radial(point, center);
        let r = perp[0].hypot(perp[1]).hypot(perp[2]);
        mid_radii.push(r);
        radius += r / 4.;
    }
    if !radius.is_finite() || !(1e-5..=1e6).contains(&radius) {
        return Ok(None);
    }
    let mut error: f64 = 0.;
    for r in &mid_radii {
        let deviation = (r - radius).abs();
        if deviation > RECOGNITION * radius {
            return Ok(None);
        }
        error = error.max(deviation);
    }
    // In-plane frame: x from the first side patch's bottom start direction.
    let start = point_of(&model.faces[sides[0]].surface.evaluate(0., 0.)?.point);
    let x_perp = radial(start, bottom);
    let x_length = x_perp[0].hypot(x_perp[1]).hypot(x_perp[2]);
    if !x_length.is_finite() || x_length <= 0. {
        return Ok(None);
    }
    let x_dir = x_perp.map(|x| x / x_length);
    error = error.max((x_length - radius).abs());
    let y_dir = cross(axis, x_dir);
    // Per-patch quadrant tiling and exact control-point certification.
    let quarter = std::f64::consts::FRAC_PI_2;
    let mut seen = [false; 4];
    let mut ordered = [0usize; 4];
    for &index in &sides {
        let surface = &model.faces[index].surface;
        let point = point_of(&surface.evaluate(0., 0.)?.point);
        let perp = radial(point, bottom);
        let angle = dot(perp, y_dir).atan2(dot(perp, x_dir));
        let quadrant = (angle / quarter).round() as i64;
        let quadrant = quadrant.rem_euclid(4) as usize;
        let residual = (angle - quadrant as f64 * quarter + std::f64::consts::PI).rem_euclid(TAU)
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
            for (j, actual) in surface.control_points[k].iter().take(2).enumerate() {
                let expected: [f64; 3] = std::array::from_fn(|a| {
                    bottom[a]
                        + radius * (expected_xy[0] * x_dir[a] + expected_xy[1] * y_dir[a])
                        + height * j as f64 * axis[a]
                });
                if actual.len() != 3 {
                    return Ok(None);
                }
                let d = [
                    actual[0] - expected[0],
                    actual[1] - expected[1],
                    actual[2] - expected[2],
                ];
                let deviation = d[0].hypot(d[1]).hypot(d[2]);
                if !deviation.is_finite() || deviation > RECOGNITION * radius + 1e-12 {
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
    // frame, and the four inscribed quarter-arc trims each exactly once.
    let mut cap_assigned = [false; 2];
    let mut cap_ids = [0usize; 2];
    for &index in &caps {
        let surface = &model.faces[index].surface;
        let corner = point_of(&surface.evaluate(0., 0.)?.point);
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
        if cap_assigned[slot] {
            return Ok(None);
        }
        cap_assigned[slot] = true;
        cap_ids[slot] = index;
        for (i, row) in surface.control_points.iter().take(2).enumerate() {
            for (j, actual) in row.iter().take(2).enumerate() {
                let expected: [f64; 3] = std::array::from_fn(|a| {
                    bottom[a]
                        + radius
                            * ((2. * i as f64 - 1.) * x_dir[a] + (2. * j as f64 - 1.) * y_dir[a])
                        + if slot == 1 { height * axis[a] } else { 0. }
                });
                if actual.len() != 2 + 1 {
                    return Ok(None);
                }
                let d = [
                    actual[0] - expected[0],
                    actual[1] - expected[1],
                    actual[2] - expected[2],
                ];
                let deviation = d[0].hypot(d[1]).hypot(d[2]);
                if !deviation.is_finite() || deviation > RECOGNITION * radius + 1e-12 {
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
    if cap_assigned.into_iter().any(|hit| !hit) {
        return Ok(None);
    }
    // Every vertex on one of the two rings.
    for vertex in &model.vertices {
        let d = sub(vertex.point, center);
        let axial = dot(d, axis);
        let perp = sub(d, axis.map(|x| x * axial));
        let dev_r = (perp[0].hypot(perp[1]).hypot(perp[2]) - radius).abs();
        let dev_a = (axial.abs() - height / 2.).abs();
        if dev_r > RECOGNITION * radius || dev_a > RECOGNITION * height {
            return Ok(None);
        }
        error = error.max(dev_r).max(dev_a);
    }
    Ok(Some(CanonicalCylinder {
        center,
        axis,
        radius,
        half_height: height / 2.,
        error,
        frame: [x_dir, y_dir],
        sides: ordered,
        caps: cap_ids,
    }))
}
