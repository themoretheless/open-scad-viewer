//! Analytic sphere/cylinder intersection for canonical solids in the axial
//! configuration (sphere center certified on the cylinder axis).
//!
//! The sphere operand must be the exact canonical stereographic sphere of
//! `analytic::sphere` (recognized by `sphere_sphere::recognize`); the cylinder
//! operand must be the exact canonical six-face cylinder of
//! `analytic::cylinder` (four rational circular side patches, two bilinear
//! caps with inscribed-circle trims), either optionally carried through a
//! rigid affine placement. Anything else is an explicit `UnsupportedSurface`
//! region — this cell never falls back to numerical surface/surface
//! subdivision. Axiality is certified, never forced: a perpendicular offset
//! at pure rounding scale snaps to the axis, an offset inside the
//! recognition-scale band reports a `NearCoincidence` region, and a clearly
//! off-axis pair is `UnsupportedSurface` (the general sphere/cylinder pair is
//! a quartic, out of scope). Classification uses outward binary64 bands
//! widened by the observed recognition deviation. Resolved contacts are exact
//! rational circles (four 90-degree arcs, weights cos(pi/4)) with UV lifts on
//! both surfaces: iso-v lines across all four cylinder side patches, an exact
//! UV circle on a cap face, and per-patch UV circles/lines on the sphere.
//! Every tangency — cap-plane touch, rim (cap circle radius == cylinder
//! radius), the r == R coincident band — stays an explicit unresolved region;
//! tangent contacts are never reported as point or guessed-circle components,
//! matching the house tangency discipline. Nothing here authorizes a topology
//! change.
use super::sphere_sphere::{
    self, ARC_WEIGHT, CanonicalSphere, RECOGNITION, SpherePatchCircle, circle_arcs, circle_curve,
    lift,
};
use super::*;
use crate::Model;

const QUADRANTS: [[f64; 2]; 4] = [[1., 0.], [0., 1.], [-1., 0.], [0., -1.]];
const TAU: f64 = std::f64::consts::TAU;

/// One cylinder face's share of an intersection circle in that face's UV.
#[derive(Clone, Debug)]
pub struct CylinderPatchCurve {
    /// Face index in the source cylinder model.
    pub patch: usize,
    /// Side patches carry one degree-1 iso-v segment; a cap patch carries
    /// four exact 90-degree rational arcs of the UV circle.
    pub arcs: Vec<Curve>,
}

#[derive(Clone, Debug)]
pub enum SphereCylinderComponent {
    /// Transverse intersection: the exact circle plus both UV lifts.
    Circle {
        /// Exact rational full circle: degree 2, knots 0..=4, four 90-degree
        /// arcs with weights cos(pi/4).
        curve: Curve,
        center: [f64; 3],
        radius: f64,
        /// Unit cylinder axis (the circle plane normal).
        normal: [f64; 3],
        sphere_uv: Vec<SpherePatchCircle>,
        cylinder_uv: Vec<CylinderPatchCurve>,
        /// Worst residual over 16 circle samples against the sphere equation
        /// and the cylinder side or cap plane equation.
        max_sample_residual: f64,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct CanonicalCylinder {
    /// Midpoint of the axis segment.
    pub(crate) center: [f64; 3],
    /// Unit axis, bottom cap toward top cap.
    pub(crate) axis: [f64; 3],
    pub(crate) radius: f64,
    pub(crate) half_height: f64,
    /// Observed structural deviation bound; feeds the outward classification.
    pub(crate) error: f64,
    /// In-plane orthonormal ring frame: x = quadrant-0 direction, y = axis x x.
    pub(crate) frame: [[f64; 3]; 2],
    /// Side face indices ordered by quadrant.
    pub(crate) sides: [usize; 4],
    /// Cap face indices: [bottom, top].
    #[allow(dead_code)]
    pub(crate) caps: [usize; 2],
}

/// Degree-1 pcurve exactly from `from` to `to` with unit knots/weights.
fn unit_edge(curve: &Curve, from: [f64; 2], to: [f64; 2]) -> bool {
    curve.degree == 1
        && curve.knots == [0., 0., 1., 1.]
        && curve.control_points == [from.to_vec(), to.to_vec()]
        && curve.weights == [1., 1.]
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

/// Recognizes a canonical cylinder solid as built by `analytic::cylinder`,
/// certifying the side/cap surface structure, exact weights and trim pcurves,
/// every control point against the exact construction in the recovered ring
/// frame, a globally consistent quadrant tiling, and both ring vertex sets.
/// Rigid affine placement is admitted; anything else returns `None`.
pub(crate) fn recognize_cylinder(model: &Model) -> Result<Option<CanonicalCylinder>> {
    model.validate()?;
    if model.bodies.len() != 1
        || model.shells.len() != 1
        || model.faces.len() != 6
        || model.vertices.len() != 8
    {
        return Ok(None);
    }
    let mut sides: Vec<usize> = Vec::new();
    let mut caps: Vec<usize> = Vec::new();
    for (index, face) in model.faces.iter().enumerate() {
        let surface = &face.surface;
        if !face.holes.is_empty() {
            return Ok(None);
        }
        let side = surface.degree_u == 2
            && surface.degree_v == 1
            && !surface.periodic_u
            && !surface.periodic_v
            && surface.knots_u == [0., 0., 0., 1., 1., 1.]
            && surface.knots_v == [0., 0., 1., 1.]
            && surface.control_points.len() == 3
            && surface.control_points.iter().all(|row| row.len() == 2);
        let cap = surface.degree_u == 1
            && surface.degree_v == 1
            && !surface.periodic_u
            && !surface.periodic_v
            && surface.knots_u == [0., 0., 1., 1.]
            && surface.knots_v == [0., 0., 1., 1.]
            && surface.control_points.len() == 2
            && surface.control_points.iter().all(|row| row.len() == 2);
        if side {
            if surface.weights != [[1., 1.], [ARC_WEIGHT, ARC_WEIGHT], [1., 1.]] {
                return Ok(None);
            }
            sides.push(index);
        } else if cap {
            if surface.weights != [[1., 1.], [1., 1.]] {
                return Ok(None);
            }
            caps.push(index);
        } else {
            return Ok(None);
        }
    }
    if sides.len() != 4 || caps.len() != 2 {
        return Ok(None);
    }
    // Side trims: the unit-square boundary, each edge exactly once.
    let boundary = [
        ([0., 0.], [1., 0.]),
        ([1., 0.], [1., 1.]),
        ([1., 1.], [0., 1.]),
        ([0., 1.], [0., 0.]),
    ];
    for &index in &sides {
        let loop_ = &model.loops[model.faces[index].outer];
        if loop_.coedges.len() != 4 {
            return Ok(None);
        }
        let mut seen = [false; 4];
        for coedge in &loop_.coedges {
            let mut hit = false;
            for (k, &(a, b)) in boundary.iter().enumerate() {
                if !seen[k] && unit_edge(&coedge.pcurve, a, b) {
                    seen[k] = true;
                    hit = true;
                    break;
                }
            }
            if !hit {
                return Ok(None);
            }
        }
        if seen.into_iter().any(|hit| !hit) {
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
    if !(x_length > 0.) {
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
            for j in 0..2 {
                let expected: [f64; 3] = std::array::from_fn(|a| {
                    bottom[a]
                        + radius * (expected_xy[0] * x_dir[a] + expected_xy[1] * y_dir[a])
                        + height * j as f64 * axis[a]
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
        for i in 0..2 {
            for j in 0..2 {
                let expected: [f64; 3] = std::array::from_fn(|a| {
                    bottom[a]
                        + radius
                            * ((2. * i as f64 - 1.) * x_dir[a] + (2. * j as f64 - 1.) * y_dir[a])
                        + if slot == 1 { height * axis[a] } else { 0. }
                });
                let actual = &surface.control_points[i][j];
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
            for quadrant in 0..4 {
                if !seen_quadrant[quadrant] && cap_quarter_arc(&coedge.pcurve, quadrant) {
                    seen_quadrant[quadrant] = true;
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

/// Where a resolved circle sits on the cylinder boundary.
#[derive(Clone, Copy)]
enum CircleSite {
    /// On the side wall: radius is the cylinder radius.
    Side,
    /// On a cap disk with the given circle radius.
    Cap(f64),
}

/// Exact circle component at axial position `axial` relative to the cylinder
/// center, with UV lifts on both surfaces and a 16-sample residual bound.
fn circle_component(
    sphere: &CanonicalSphere,
    cylinder: &CanonicalCylinder,
    axial: f64,
    site: CircleSite,
) -> Result<SphereCylinderComponent> {
    let center = std::array::from_fn(|k| cylinder.center[k] + axial * cylinder.axis[k]);
    let (radius, cylinder_uv) = match site {
        CircleSite::Side => {
            // Side patches: u sweeps the quadrant, v runs bottom to top, so
            // the circle is the iso-v line v = (axial + h/2) / h on each.
            let v0 = (axial + cylinder.half_height) / (2. * cylinder.half_height);
            let arcs = vec![Curve {
                degree: 1,
                knots: vec![0., 0., 1., 1.],
                control_points: vec![[0., v0].to_vec(), [1., v0].to_vec()],
                weights: vec![1., 1.],
                periodic: false,
            }];
            (
                cylinder.radius,
                cylinder
                    .sides
                    .iter()
                    .map(|&patch| CylinderPatchCurve {
                        patch,
                        arcs: arcs.clone(),
                    })
                    .collect::<Vec<_>>(),
            )
        }
        CircleSite::Cap(rho) => {
            let face = if axial > 0. {
                cylinder.caps[1]
            } else {
                cylinder.caps[0]
            };
            // Cap UV maps the plane as [1/2 + x/(2R), 1/2 + y/(2R)]: the
            // circle is an exact UV circle of radius rho/(2R) about [1/2,1/2].
            (
                rho,
                vec![CylinderPatchCurve {
                    patch: face,
                    arcs: circle_arcs([0.5, 0.5], rho / (2. * cylinder.radius), 0., TAU),
                }],
            )
        }
    };
    let curve = circle_curve(center, radius, cylinder.frame[0], cylinder.frame[1]);
    let sphere_uv = lift(sphere, cylinder.axis, center);
    let mut max_sample_residual = 0_f64;
    for i in 0..16 {
        let point = point_of(&curve.evaluate(i as f64 / 4.)?.point);
        let d = sub(point, sphere.center);
        let sphere_residual = (d[0].hypot(d[1]).hypot(d[2]) - sphere.radius).abs();
        let other = match site {
            CircleSite::Side => {
                let rel = sub(point, cylinder.center);
                let a = dot(rel, cylinder.axis);
                let perp = sub(rel, cylinder.axis.map(|x| x * a));
                (perp[0].hypot(perp[1]).hypot(perp[2]) - cylinder.radius).abs()
            }
            CircleSite::Cap(_) => (dot(sub(point, cylinder.center), cylinder.axis) - axial).abs(),
        };
        max_sample_residual = max_sample_residual.max(sphere_residual).max(other);
    }
    Ok(SphereCylinderComponent::Circle {
        curve,
        center,
        radius,
        normal: cylinder.axis,
        sphere_uv,
        cylinder_uv,
        max_sample_residual,
    })
}

/// Analytic sphere/cylinder intersection of a canonical sphere solid and a
/// canonical cylinder solid, axial configuration only. Non-canonical operands
/// and clearly off-axis pairs are explicit unsupported regions, never a
/// numerical fallback; near-axial offsets, the r == R band, rim and cap-plane
/// tangencies stay unresolved — tangent contacts are never guessed.
pub fn intersect_sphere_cylinder(
    sphere_model: &Model,
    cylinder_model: &Model,
    options: Options,
) -> Result<Report<SphereCylinderComponent>> {
    let options = options.validate()?;
    let _ = options;
    let mut report = Report::default();
    let domain = vec![0., 1., 0., 1., 0., 1., 0., 1.];
    let (Some(sphere), Some(cylinder)) = (
        sphere_sphere::recognize(sphere_model)?,
        recognize_cylinder(cylinder_model)?,
    ) else {
        report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    };
    report.boxes_visited = 1;
    let offset = sub(sphere.center, cylinder.center);
    let s = dot(offset, cylinder.axis);
    let perp = sub(offset, cylinder.axis.map(|x| x * s));
    let pscale = perp.iter().map(|v| v.abs()).fold(0., f64::max);
    let d_perp = if pscale == 0. {
        0.
    } else {
        let scaled = perp.map(|x| x / pscale);
        scaled[0].hypot(scaled[1]).hypot(scaled[2]) * pscale
    };
    let terms = s.abs() + cylinder.half_height + sphere.radius + cylinder.radius + 1.;
    // Outward binary64 classification band: recognition deviation plus a
    // rounding allowance on the axial coordinates and radii.
    let band = sphere.error + cylinder.error + 16. * f64::EPSILON * terms;
    // Axiality is certified, never forced: pure-rounding offsets snap to the
    // axis, recognition-scale offsets report near_coincidence, and anything
    // larger is the unsupported general (quartic) configuration.
    let snap = 64. * f64::EPSILON * terms;
    if d_perp > snap {
        let near = RECOGNITION * (sphere.radius + cylinder.radius + cylinder.half_height);
        let reason = if d_perp <= near {
            UnresolvedReason::NearCoincidence
        } else {
            UnresolvedReason::UnsupportedSurface
        };
        report.unresolved(domain, reason);
        return Ok(report);
    }
    let r = sphere.radius;
    let big_r = cylinder.radius;
    let half = cylinder.half_height;
    // Sphere provably beyond a cap plane: no contact in any radius relation.
    if s - r > half + band || -s - r > half + band {
        return Ok(report);
    }
    // r == R within the band: side contact degenerates to a tangent circle at
    // the sphere equator and the component structure cannot be certified —
    // a coincident-band region, never a guessed circle.
    if (r - big_r).abs() <= band {
        report.unresolved(domain, UnresolvedReason::CoincidentTrim);
        return Ok(report);
    }
    let mut tangency = false;
    let mut found: Vec<(f64, CircleSite)> = Vec::new();
    if r < big_r {
        // Sphere provably inside the side wall: cap-plane crossings only, and
        // every cap circle has radius <= r < R, so it sits inside the disk.
        for &q in &[half, -half] {
            let d = (q - s).abs();
            if d > r + band {
                continue;
            }
            if (d - r).abs() <= band {
                tangency = true;
                continue;
            }
            let rho2 = r * r - d * d;
            if !rho2.is_finite() || rho2 <= 0. {
                tangency = true;
                continue;
            }
            found.push((q, CircleSite::Cap(rho2.sqrt())));
        }
    } else {
        // r > R provably: side crossings at s +- sqrt(r^2 - R^2), clipped by
        // the finite height; a crossing on a cap plane is the rim tangency.
        let delta2 = r * r - big_r * big_r;
        if !delta2.is_finite() || delta2 <= 0. {
            report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
            return Ok(report);
        }
        let delta = delta2.sqrt();
        for sign in [1., -1.] {
            let p = s + sign * delta;
            if p.abs() > half + band {
                continue;
            }
            if (p.abs() - half).abs() <= band {
                tangency = true;
                continue;
            }
            found.push((p, CircleSite::Side));
        }
        // Cap-plane contacts: a circle inside the disk (which implies a side
        // crossing within the height), the rim band, or a plane tangency.
        for &q in &[half, -half] {
            let d = (q - s).abs();
            if d > r + band {
                continue;
            }
            if (d - r).abs() <= band {
                tangency = true;
                continue;
            }
            let rho2 = r * r - d * d;
            if !rho2.is_finite() || rho2 <= 0. {
                tangency = true;
                continue;
            }
            let rho = rho2.sqrt();
            if rho > big_r + band {
                continue;
            }
            if (rho - big_r).abs() <= band {
                tangency = true;
                continue;
            }
            found.push((q, CircleSite::Cap(rho)));
        }
    }
    if tangency {
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
    }
    found.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (axial, site) in found {
        report
            .components
            .push(circle_component(&sphere, &cylinder, axial, site)?);
    }
    Ok(report)
}

impl value_codec::Serialize for CylinderPatchCurve {
    fn to_value(&self) -> value_codec::Value {
        value_codec::json!({"patch":self.patch,"arcs":self.arcs})
    }
}
impl value_codec::Serialize for SphereCylinderComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Circle {
                curve,
                center,
                radius,
                normal,
                sphere_uv,
                cylinder_uv,
                max_sample_residual,
            } => value_codec::json!({"kind":"circle","curve":curve,"center":center,"radius":radius,
                "normal":normal,"sphereUv":sphere_uv,"cylinderUv":cylinder_uv,
                "maxSampleResidual":max_sample_residual}),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn translated(model: &Model, offset: [f64; 3]) -> Model {
        crate::transform::affine(
            model,
            [
                [1., 0., 0., offset[0]],
                [0., 1., 0., offset[1]],
                [0., 0., 1., offset[2]],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap()
    }
    fn rotated_translated(model: &Model, angle: f64, offset: [f64; 3]) -> Model {
        let (sin, cos) = angle.sin_cos();
        crate::transform::affine(
            model,
            [
                [1., 0., 0., offset[0]],
                [0., cos, -sin, offset[1]],
                [0., sin, cos, offset[2]],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap()
    }
    fn only_circles(
        report: &Report<SphereCylinderComponent>,
        count: usize,
    ) -> &Report<SphereCylinderComponent> {
        assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
        assert!(report.unresolved.is_empty(), "{report:?}");
        assert!(!report.permits_topology_change());
        assert_eq!(report.components.len(), count, "{report:?}");
        report
    }
    type SphereCylinderCircle<'a> = (
        &'a Curve,
        [f64; 3],
        f64,
        [f64; 3],
        &'a [SpherePatchCircle],
        &'a [CylinderPatchCurve],
        f64,
    );

    fn circle_of(component: &SphereCylinderComponent) -> SphereCylinderCircle<'_> {
        let SphereCylinderComponent::Circle {
            curve,
            center,
            radius,
            normal,
            sphere_uv,
            cylinder_uv,
            max_sample_residual,
        } = component;
        (
            curve,
            *center,
            *radius,
            *normal,
            sphere_uv,
            cylinder_uv,
            *max_sample_residual,
        )
    }
    fn sphere_residual(point: [f64; 3], center: [f64; 3], radius: f64) -> f64 {
        (sub(point, center)[0]
            .hypot(sub(point, center)[1])
            .hypot(sub(point, center)[2])
            - radius)
            .abs()
    }
    /// Radial distance from the z axis (canonical frames in these tests).
    fn radial_z(point: [f64; 3]) -> f64 {
        point[0].hypot(point[1])
    }
    /// Worst residual of lifted UV arcs evaluated through their own surface,
    /// against the sphere equation and a per-circle predicate.
    fn uv_samples(
        sphere_model: &Model,
        cylinder_model: &Model,
        sphere_center: [f64; 3],
        sphere_radius: f64,
        sphere_uv: &[SpherePatchCircle],
        cylinder_uv: &[CylinderPatchCurve],
        cylinder_check: impl Fn([f64; 3]) -> f64,
    ) -> f64 {
        let mut worst = 0_f64;
        for lift in sphere_uv {
            let surface = &sphere_model.faces[lift.patch].surface;
            for arc in &lift.arcs {
                for k in 0..=8 {
                    let uv = arc.evaluate(k as f64 / 8.).unwrap().point;
                    let p = surface
                        .evaluate(uv[0].clamp(0., 1.), uv[1].clamp(0., 1.))
                        .unwrap()
                        .point;
                    let p = [p[0], p[1], p[2]];
                    worst = worst
                        .max(sphere_residual(p, sphere_center, sphere_radius))
                        .max(cylinder_check(p));
                }
            }
        }
        for lift in cylinder_uv {
            let surface = &cylinder_model.faces[lift.patch].surface;
            for arc in &lift.arcs {
                for k in 0..=8 {
                    let uv = arc.evaluate(k as f64 / 8.).unwrap().point;
                    assert!((0. ..=1.).contains(&uv[0]) && (0. ..=1.).contains(&uv[1]));
                    let p = surface.evaluate(uv[0], uv[1]).unwrap().point;
                    let p = [p[0], p[1], p[2]];
                    worst = worst
                        .max(sphere_residual(p, sphere_center, sphere_radius))
                        .max(cylinder_check(p));
                }
            }
        }
        worst
    }

    #[test]
    fn two_side_circles_match_the_sqrt_oracle() {
        // Cylinder R=2 spans z in 0..8; sphere r=3 centered on the axis at
        // the cylinder midpoint: side circles at z = 4 +- sqrt(9 - 4).
        let sphere = translated(&crate::analytic::sphere(3.).unwrap(), [0., 0., 4.]);
        let cylinder = crate::analytic::cylinder(2., 8.).unwrap();
        let report = intersect_sphere_cylinder(&sphere, &cylinder, Options::default()).unwrap();
        let report = only_circles(&report, 2);
        let oracle = (9_f64 - 4.).sqrt();
        for (component, z) in report.components.iter().zip([4. - oracle, 4. + oracle]) {
            let (curve, center, radius, normal, sphere_uv, cylinder_uv, sampled) =
                circle_of(component);
            assert!((radius - 2.).abs() <= 1e-12, "{radius}");
            assert!(
                sub(center, [0., 0., z]).iter().all(|x| x.abs() <= 1e-12),
                "{center:?}"
            );
            assert!(normal[2].abs() >= 1. - 1e-12, "{normal:?}");
            // Exact rational circle: four 90-degree arcs, weights cos(pi/4).
            assert_eq!(curve.degree, 2);
            assert_eq!(
                curve.knots,
                vec![0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.]
            );
            assert_eq!(curve.control_points.len(), 9);
            assert_eq!(
                curve.weights,
                vec![
                    1., ARC_WEIGHT, 1., ARC_WEIGHT, 1., ARC_WEIGHT, 1., ARC_WEIGHT, 1.
                ]
            );
            // Sixteen samples satisfy both implicit equations.
            let mut worst = 0_f64;
            for i in 0..16 {
                let p = curve.evaluate(i as f64 / 4.).unwrap().point;
                let p = [p[0], p[1], p[2]];
                worst = worst
                    .max(sphere_residual(p, [0., 0., 4.], 3.))
                    .max((radial_z(p) - 2.).abs());
            }
            assert!(worst <= 1e-12, "{worst}");
            assert!(sampled <= 1e-12, "{sampled}");
            // Side circles lift to iso-v lines on all four side patches.
            assert_eq!(cylinder_uv.len(), 4);
            let v0 = (z) / 8.;
            for lift in cylinder_uv {
                assert_eq!(lift.arcs.len(), 1);
                let arc = &lift.arcs[0];
                assert_eq!(arc.degree, 1);
                assert!((arc.control_points[0][1] - v0).abs() <= 1e-12);
                assert!((arc.control_points[1][1] - v0).abs() <= 1e-12);
            }
            assert!(!sphere_uv.is_empty());
            let uv_worst = uv_samples(
                &sphere,
                &cylinder,
                [0., 0., 4.],
                3.,
                sphere_uv,
                cylinder_uv,
                |p| (radial_z(p) - 2.).abs(),
            );
            assert!(uv_worst <= 1e-9, "{uv_worst}");
        }
    }

    #[test]
    fn one_circle_survives_clipping_by_the_finite_height() {
        // Sphere r=3 centered at the bottom cap plane: upper side circle at
        // z = sqrt(5) inside, lower one below the cylinder; the bottom cap
        // plane circle has radius 3 > R, so it lies outside the cap disk.
        let sphere = crate::analytic::sphere(3.).unwrap();
        let cylinder = crate::analytic::cylinder(2., 8.).unwrap();
        let report = intersect_sphere_cylinder(&sphere, &cylinder, Options::default()).unwrap();
        let report = only_circles(&report, 1);
        let (curve, center, radius, _, _, cylinder_uv, sampled) = circle_of(&report.components[0]);
        assert!((radius - 2.).abs() <= 1e-12);
        assert!(
            sub(center, [0., 0., 5_f64.sqrt()])
                .iter()
                .all(|x| x.abs() <= 1e-12)
        );
        let mut worst = 0_f64;
        for i in 0..16 {
            let p = curve.evaluate(i as f64 / 4.).unwrap().point;
            let p = [p[0], p[1], p[2]];
            worst = worst
                .max(sphere_residual(p, [0., 0., 0.], 3.))
                .max((radial_z(p) - 2.).abs());
        }
        assert!(worst <= 1e-12, "{worst}");
        assert!(sampled <= 1e-12);
        assert_eq!(cylinder_uv.len(), 4);
    }

    #[test]
    fn small_sphere_poking_through_a_cap_yields_a_cap_circle() {
        // r=1.5 < R=2, center 0.5 below the top cap plane z=8: circle of
        // radius sqrt(1.5^2 - 0.5^2) = sqrt(2) in the cap plane, inside the
        // disk; no side contact is possible.
        let sphere = translated(&crate::analytic::sphere(1.5).unwrap(), [0., 0., 7.5]);
        let cylinder = crate::analytic::cylinder(2., 8.).unwrap();
        let report = intersect_sphere_cylinder(&sphere, &cylinder, Options::default()).unwrap();
        let report = only_circles(&report, 1);
        let (curve, center, radius, _, sphere_uv, cylinder_uv, sampled) =
            circle_of(&report.components[0]);
        let oracle = 2_f64.sqrt();
        assert!((radius - oracle).abs() <= 1e-12, "{radius}");
        assert!(
            sub(center, [0., 0., 8.]).iter().all(|x| x.abs() <= 1e-12),
            "{center:?}"
        );
        let mut worst = 0_f64;
        for i in 0..16 {
            let p = curve.evaluate(i as f64 / 4.).unwrap().point;
            let p = [p[0], p[1], p[2]];
            worst = worst
                .max(sphere_residual(p, [0., 0., 7.5], 1.5))
                .max((p[2] - 8.).abs());
        }
        assert!(worst <= 1e-12, "{worst}");
        assert!(sampled <= 1e-12);
        // Cap lift: one face, four exact 90-degree arcs of the UV circle of
        // radius sqrt(2)/(2R) = sqrt(2)/4 about [1/2, 1/2].
        assert_eq!(cylinder_uv.len(), 1);
        let lift = &cylinder_uv[0];
        assert_eq!(lift.arcs.len(), 4);
        for arc in &lift.arcs {
            assert_eq!(arc.degree, 2);
            assert_eq!(arc.weights, vec![1., ARC_WEIGHT, 1.]);
            let endpoint = &arc.control_points[0];
            let uvr = (endpoint[0] - 0.5).hypot(endpoint[1] - 0.5);
            assert!((uvr - oracle / 4.).abs() <= 1e-12, "{uvr}");
        }
        let uv_worst = uv_samples(
            &sphere,
            &cylinder,
            [0., 0., 7.5],
            1.5,
            sphere_uv,
            cylinder_uv,
            |p| (p[2] - 8.).abs(),
        );
        assert!(uv_worst <= 1e-9, "{uv_worst}");
    }

    #[test]
    fn zero_circle_configurations_resolve_empty() {
        let cylinder = crate::analytic::cylinder(2., 8.).unwrap();
        // Small sphere strictly inside.
        let inside = translated(&crate::analytic::sphere(1.).unwrap(), [0., 0., 4.]);
        // Sphere beyond the top cap, no reach back.
        let beyond = translated(&crate::analytic::sphere(1.5).unwrap(), [0., 0., 11.]);
        // Large sphere swallowing the whole cylinder (side crossings beyond
        // the height, cap circles outside the disks).
        let swallow = translated(&crate::analytic::sphere(20.).unwrap(), [0., 0., 4.]);
        for sphere in [&inside, &beyond, &swallow] {
            let report = intersect_sphere_cylinder(sphere, &cylinder, Options::default()).unwrap();
            assert!(
                report.components.is_empty() && report.unresolved.is_empty(),
                "{report:?}"
            );
            assert_eq!(report.coverage, Coverage::NumericallyResolved);
        }
    }

    #[test]
    fn equal_radii_report_the_coincident_band() {
        let cylinder = crate::analytic::cylinder(2., 8.).unwrap();
        // Exact r == R: tangent equator circle — never a guessed component.
        let exact = translated(&crate::analytic::sphere(2.).unwrap(), [0., 0., 4.]);
        // Within the outward band: inseparable from coincidence.
        let near = crate::analytic::sphere(2. + 2e-15).unwrap();
        let near = translated(&near, [0., 0., 4.]);
        // r == R with the sphere reaching back over the top cap: the radius
        // ambiguity still forbids certifying a cap circle.
        let reaching = translated(&crate::analytic::sphere(2.).unwrap(), [0., 0., 9.]);
        for sphere in [&exact, &near, &reaching] {
            let report = intersect_sphere_cylinder(sphere, &cylinder, Options::default()).unwrap();
            assert!(report.components.is_empty(), "{report:?}");
            assert_eq!(report.coverage, Coverage::Incomplete);
            assert_eq!(report.unresolved.len(), 1);
            assert_eq!(
                report.unresolved[0].reason,
                UnresolvedReason::CoincidentTrim
            );
            assert!(!report.permits_topology_change());
        }
    }

    #[test]
    fn cap_plane_touch_stays_a_tangency_region() {
        let cylinder = crate::analytic::cylinder(2., 8.).unwrap();
        // r=1.5 sphere tangent to the top cap plane z=8 from inside.
        let sphere = translated(&crate::analytic::sphere(1.5).unwrap(), [0., 0., 6.5]);
        let report = intersect_sphere_cylinder(&sphere, &cylinder, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::TangencyOrMultipleRoot
        );
        // Just clear of the band: strictly inside, empty and resolved.
        let clear = translated(&crate::analytic::sphere(1.5).unwrap(), [0., 0., 6.5 - 1e-9]);
        let report = intersect_sphere_cylinder(&clear, &cylinder, Options::default()).unwrap();
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{report:?}"
        );
        // Just across: a small transverse cap circle.
        let across = translated(&crate::analytic::sphere(1.5).unwrap(), [0., 0., 6.5 + 1e-9]);
        let report = intersect_sphere_cylinder(&across, &cylinder, Options::default()).unwrap();
        only_circles(&report, 1);
    }

    #[test]
    fn rim_tangency_stays_unresolved() {
        // sqrt(r^2 - R^2) == h/2 exactly: both side crossings land on the cap
        // planes — the rim tangency is never a guessed circle.
        let cylinder = crate::analytic::cylinder(2., 8.).unwrap();
        let r = (4_f64 + 16.).sqrt();
        let sphere = translated(&crate::analytic::sphere(r).unwrap(), [0., 0., 4.]);
        let report = intersect_sphere_cylinder(&sphere, &cylinder, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::TangencyOrMultipleRoot
        );
    }

    #[test]
    fn near_axial_offset_within_the_band_stays_unresolved() {
        let cylinder = crate::analytic::cylinder(2., 8.).unwrap();
        let sphere = translated(&crate::analytic::sphere(3.).unwrap(), [1e-10, 0., 4.]);
        let report = intersect_sphere_cylinder(&sphere, &cylinder, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::NearCoincidence
        );
    }

    #[test]
    fn clearly_off_axis_pairs_are_unsupported_regions() {
        let cylinder = crate::analytic::cylinder(2., 8.).unwrap();
        let sphere = translated(&crate::analytic::sphere(3.).unwrap(), [0.5, 0., 4.]);
        let report = intersect_sphere_cylinder(&sphere, &cylinder, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(report.unresolved.len(), 1);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::UnsupportedSurface
        );
        assert_eq!(
            report.unresolved[0].parameter_box,
            vec![0., 1., 0., 1., 0., 1., 0., 1.]
        );
    }

    #[test]
    fn noncanonical_operands_are_explicit_unsupported_regions() {
        let sphere = crate::analytic::sphere(2.).unwrap();
        // Frustum and tube are not the canonical cylinder; cuboids and tori
        // are neither canonical operand.
        for (a, b) in [
            (
                crate::analytic::sphere(2.).unwrap(),
                crate::analytic::frustum(1., 2., 3.).unwrap(),
            ),
            (
                crate::analytic::sphere(2.).unwrap(),
                crate::analytic::tube(2., 1., 3.).unwrap(),
            ),
            (
                crate::cuboid([0., 0., 0.], [1., 1., 1.]).unwrap(),
                crate::analytic::cylinder(1., 2.).unwrap(),
            ),
            (
                crate::analytic::torus(3., 1.).unwrap(),
                crate::analytic::cylinder(1., 2.).unwrap(),
            ),
        ] {
            let report = intersect_sphere_cylinder(&a, &b, Options::default()).unwrap();
            assert!(report.components.is_empty(), "{report:?}");
            assert_eq!(report.coverage, Coverage::Incomplete);
            assert_eq!(report.unresolved.len(), 1);
            assert_eq!(
                report.unresolved[0].reason,
                UnresolvedReason::UnsupportedSurface
            );
        }
        // A canonical pair still resolves: sphere r=2 at the bottom ring of
        // cylinder(1, 4) crosses the side once at z = 4 - sqrt(3).
        let tall = crate::analytic::cylinder(1., 4.).unwrap();
        let report = intersect_sphere_cylinder(&sphere, &tall, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
        assert_eq!(report.components.len(), 1);
        // A structurally perturbed cylinder fails validation as a hard error.
        let mut perturbed = crate::analytic::cylinder(1., 2.).unwrap();
        perturbed.faces[0].surface.weights[1][0] = 0.5;
        assert!(intersect_sphere_cylinder(&sphere, &perturbed, Options::default()).is_err());
    }

    #[test]
    fn rotated_translated_placement_keeps_the_exact_circles() {
        // Rigid placement of the whole axial configuration about X by 0.5.
        let angle = 0.5;
        let offset = [0.3, -0.2, 1.1];
        let sphere = rotated_translated(
            &translated(&crate::analytic::sphere(3.).unwrap(), [0., 0., 4.]),
            angle,
            offset,
        );
        let cylinder =
            rotated_translated(&crate::analytic::cylinder(2., 8.).unwrap(), angle, offset);
        let report = intersect_sphere_cylinder(&sphere, &cylinder, Options::default()).unwrap();
        let report = only_circles(&report, 2);
        // Independent binary64 oracle in the placed frame.
        let (sin, cos) = angle.sin_cos();
        let placed = |p: [f64; 3]| {
            [
                p[0] + offset[0],
                cos * p[1] - sin * p[2] + offset[1],
                sin * p[1] + cos * p[2] + offset[2],
            ]
        };
        let sphere_center = placed([0., 0., 4.]);
        let axis = {
            let a = placed([0., 0., 1.]);
            let b = placed([0., 0., 0.]);
            sub(a, b)
        };
        let oracle = (9_f64 - 4.).sqrt();
        for (component, z) in report.components.iter().zip([4. - oracle, 4. + oracle]) {
            let (curve, center, radius, normal, sphere_uv, cylinder_uv, sampled) =
                circle_of(component);
            let expected = placed([0., 0., z]);
            assert!((radius - 2.).abs() <= 1e-12, "{radius}");
            assert!(
                sub(center, expected).iter().all(|x| x.abs() <= 1e-12),
                "{center:?}"
            );
            assert!(
                sub(normal, axis).iter().all(|x| x.abs() <= 1e-12),
                "{normal:?}"
            );
            let mut worst = 0_f64;
            for i in 0..16 {
                let p = curve.evaluate(i as f64 / 4.).unwrap().point;
                let p = [p[0], p[1], p[2]];
                worst = worst.max(sphere_residual(p, sphere_center, 3.)).max({
                    let rel = sub(p, expected);
                    let a = dot(rel, axis);
                    let perp = sub(rel, axis.map(|x| x * a));
                    (perp[0].hypot(perp[1]).hypot(perp[2]) - 2.).abs()
                });
            }
            assert!(worst <= 1e-12, "{worst}");
            assert!(sampled <= 1e-12);
            assert_eq!(cylinder_uv.len(), 4);
            assert!(!sphere_uv.is_empty());
            let axis = normal;
            let uv_worst = uv_samples(
                &sphere,
                &cylinder,
                sphere_center,
                3.,
                sphere_uv,
                cylinder_uv,
                |p| {
                    let rel = sub(p, center);
                    let a = dot(rel, axis);
                    let perp = sub(rel, axis.map(|x| x * a));
                    (perp[0].hypot(perp[1]).hypot(perp[2]) - 2.).abs()
                },
            );
            assert!(uv_worst <= 1e-9, "{uv_worst}");
        }
    }

    #[test]
    fn swapped_cap_slots_and_bottom_cap_circle() {
        // Sphere poking through the bottom cap from below: circle on the
        // bottom cap face, axial slot 0.
        let sphere = translated(&crate::analytic::sphere(1.5).unwrap(), [0., 0., 0.5]);
        let cylinder = crate::analytic::cylinder(2., 8.).unwrap();
        let report = intersect_sphere_cylinder(&sphere, &cylinder, Options::default()).unwrap();
        let report = only_circles(&report, 1);
        let (_, center, radius, _, _, cylinder_uv, _) = circle_of(&report.components[0]);
        assert!((radius - 2_f64.sqrt()).abs() <= 1e-12);
        assert!(sub(center, [0., 0., 0.]).iter().all(|x| x.abs() <= 1e-12));
        assert_eq!(cylinder_uv.len(), 1);
        // The bottom cap is a different face than the top cap; its lift still
        // evaluates onto the sphere and the cap plane.
        let lift = &cylinder_uv[0];
        let surface = &cylinder.faces[lift.patch].surface;
        for arc in &lift.arcs {
            for k in 0..=8 {
                let uv = arc.evaluate(k as f64 / 8.).unwrap().point;
                let p = surface.evaluate(uv[0], uv[1]).unwrap().point;
                let p = [p[0], p[1], p[2]];
                assert!(sphere_residual(p, [0., 0., 0.5], 1.5) <= 1e-9);
                assert!((p[2]).abs() <= 1e-9);
            }
        }
    }
}
