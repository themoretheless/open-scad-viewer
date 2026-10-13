use super::*;

/// Radius witness for the finite exact-cylinder tessellation cell. Kept here so
/// bridge code cannot depend on private analytic recognizer internals.
pub fn certified_cylinder_radius(model: &Model) -> Result<Option<f64>> {
    Ok(crate::intersections::recognize_cylinder(model)?.map(|cylinder| cylinder.radius))
}

pub(super) fn affine_surface(surface: &Surface, tolerance: f64) -> bool {
    surface.degree_u == 1
        && surface.degree_v == 1
        && surface.control_points.len() == 2
        && surface.control_points.iter().all(|row| row.len() == 2)
        && surface
            .weights
            .iter()
            .flatten()
            .all(|weight| (*weight - surface.weights[0][0]).abs() <= 16. * f64::EPSILON)
        && (0..3).all(|axis| {
            (surface.control_points[0][0][axis] + surface.control_points[1][1][axis]
                - surface.control_points[0][1][axis]
                - surface.control_points[1][0][axis])
                .abs()
                <= tolerance
        })
}

/// Closed-form upper deviation bound for the finite certified tessellation
/// successor. Every owned shell is independently recognized; unsupported
/// rational/freeform shells refuse instead of inheriting a sampled estimate.
pub fn certified_tessellation_deviation(model: &Model, segments: usize) -> Result<f64> {
    if !(1..=32).contains(&segments) {
        return Err(Error::new(
            "BREP_TESSELLATION_BUDGET_EXHAUSTED",
            "Certified analytic tessellation needs 1..32 subdivisions per patch",
        ));
    }
    let mut maximum: f64 = 0.;
    for body in &model.bodies {
        for (&shell, cavity) in std::iter::once((&body.outer_shell, false))
            .chain(body.inner_shells.iter().map(|shell| (shell, true)))
        {
            let component = crate::solid_audit::isolated_outward_shell(model, shell, cavity)?;
            let n = segments as f64;
            let bound = if component
                .faces
                .iter()
                .all(|face| affine_surface(&face.surface, model.tolerance_mm))
            {
                0.
            } else if let Some(sphere) = crate::intersections::recognize_sphere(&component)? {
                // Stereographic quarter patches: radial angle is at most
                // 2 atan(1/n), azimuth at most pi/(2n). Their spherical-cap
                // sum bounds every patch triangle and both directed distances.
                let angle = (2. * (1. / n).atan()).hypot(std::f64::consts::FRAC_PI_2 / n);
                sphere.radius * (1. - angle.cos())
            } else if let Some(cone) = crate::intersections::recognize_cone(&component)? {
                cone.r_bottom.max(cone.r_top) * (1. - (std::f64::consts::FRAC_PI_4 / n).cos())
            } else if let Some(torus) = crate::intersections::recognize_torus(&component)? {
                let sagitta = 1. - (std::f64::consts::FRAC_PI_4 / n).cos();
                (torus.major + 2. * torus.minor) * sagitta
            } else if let Some(cylinder) = crate::intersections::recognize_cylinder(&component)? {
                cylinder.radius * (1. - (std::f64::consts::FRAC_PI_4 / n).cos())
            } else {
                return Err(Error::new(
                    "BREP_CERTIFIED_TESSELLATION_REFUSED",
                    "Certified tessellation /2 admits planar, sphere, cone/frustum, torus, and cylinder shells; generic rational/freeform deviation is not certified",
                ));
            };
            maximum = maximum.max(next_up(bound));
        }
    }
    Ok(maximum)
}

/// Append-only freeform tessellation finite cell. Equal-weight clamped Bezier
/// shells (degree ≤ 3) get a Bernstein second-difference enclosure; varying
/// weights, periodicity, loft/sweep authorship, and higher degree remain refuse.
pub const FREEFORM_TESSELLATION_CAPABILITY: &str =
    "certified-generic-rational-freeform-tessellation/1";

fn point3(point: &[f64]) -> Option<[f64; 3]> {
    (point.len() >= 3 && point.iter().take(3).all(|c| c.is_finite()))
        .then(|| [point[0], point[1], point[2]])
}

fn point_sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn point_norm(a: [f64; 3]) -> f64 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}

fn bezier_clamped_knots(knots: &[f64], degree: usize, controls: usize) -> bool {
    if degree == 0 || controls != degree + 1 || knots.len() != 2 * (degree + 1) {
        return false;
    }
    let lo = knots[0];
    let hi = knots[knots.len() - 1];
    if !(lo.is_finite() && hi.is_finite() && hi > lo) {
        return false;
    }
    let span = hi - lo;
    knots
        .iter()
        .take(degree + 1)
        .all(|k| (*k - lo).abs() <= 16. * f64::EPSILON * span.max(1.))
        && knots
            .iter()
            .skip(degree + 1)
            .all(|k| (*k - hi).abs() <= 16. * f64::EPSILON * span.max(1.))
}

fn equal_positive_weights(surface: &Surface) -> bool {
    let Some(first) = surface.weights.first().and_then(|row| row.first()).copied() else {
        return false;
    };
    first.is_finite()
        && first > 0.
        && surface.weights.iter().flatten().all(|w| {
            w.is_finite() && *w > 0. && (*w - first).abs() <= 16. * f64::EPSILON * first.max(1.)
        })
}

fn control_net_coplanar(surface: &Surface, tolerance: f64) -> bool {
    let pts: Vec<[f64; 3]> = surface
        .control_points
        .iter()
        .flatten()
        .filter_map(|p| point3(p))
        .collect();
    if pts.len() < 3 {
        return false;
    }
    let o = pts[0];
    let mut normal = [0., 0., 0.];
    for i in 1..pts.len() {
        for j in (i + 1)..pts.len() {
            let a = point_sub(pts[i], o);
            let b = point_sub(pts[j], o);
            let c = [
                a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0],
            ];
            let len = point_norm(c);
            if len > tolerance {
                normal = [c[0] / len, c[1] / len, c[2] / len];
                break;
            }
        }
        if normal != [0., 0., 0.] {
            break;
        }
    }
    if normal == [0., 0., 0.] {
        return true;
    }
    pts.iter().all(|p| {
        let d = (p[0] - o[0]) * normal[0] + (p[1] - o[1]) * normal[1] + (p[2] - o[2]) * normal[2];
        d.abs() <= tolerance
    })
}

fn admitted_freeform_bezier_surface(surface: &Surface) -> bool {
    (1..=3).contains(&surface.degree_u)
        && (1..=3).contains(&surface.degree_v)
        && !surface.periodic_u
        && !surface.periodic_v
        && surface.control_points.len() == surface.degree_u + 1
        && surface
            .control_points
            .iter()
            .all(|row| row.len() == surface.degree_v + 1 && row.iter().all(|p| point3(p).is_some()))
        && surface.weights.len() == surface.control_points.len()
        && surface
            .weights
            .iter()
            .zip(&surface.control_points)
            .all(|(w_row, p_row)| w_row.len() == p_row.len())
        && bezier_clamped_knots(
            &surface.knots_u,
            surface.degree_u,
            surface.control_points.len(),
        )
        && bezier_clamped_knots(
            &surface.knots_v,
            surface.degree_v,
            surface.control_points[0].len(),
        )
        && equal_positive_weights(surface)
}

/// Bernstein second-difference coefficient M = ‖S_uu‖ + 2‖S_uv‖ + ‖S_vv‖.
fn bernstein_second_diff_coeff(surface: &Surface) -> Result<f64> {
    if !admitted_freeform_bezier_surface(surface) {
        return Err(Error::new(
            "BREP_CERTIFIED_TESSELLATION_REFUSED",
            "Freeform tessellation /1 admits equal-weight clamped Bezier faces of degree ≤3 only",
        ));
    }
    if control_net_coplanar(surface, 1e-8) {
        return Ok(0.);
    }
    let pu = surface.degree_u as f64;
    let pv = surface.degree_v as f64;
    let rows = surface.control_points.len();
    let cols = surface.control_points[0].len();
    let pt = |i: usize, j: usize| -> [f64; 3] {
        point3(&surface.control_points[i][j]).expect("admitted net is finite 3D")
    };
    let mut max_uu: f64 = 0.;
    if rows >= 3 {
        for i in 0..rows - 2 {
            for j in 0..cols {
                let d2 = point_sub(
                    point_sub(pt(i + 2, j), pt(i + 1, j)),
                    point_sub(pt(i + 1, j), pt(i, j)),
                );
                max_uu = max_uu.max(point_norm(d2));
            }
        }
    }
    let mut max_vv: f64 = 0.;
    if cols >= 3 {
        for i in 0..rows {
            for j in 0..cols - 2 {
                let d2 = point_sub(
                    point_sub(pt(i, j + 2), pt(i, j + 1)),
                    point_sub(pt(i, j + 1), pt(i, j)),
                );
                max_vv = max_vv.max(point_norm(d2));
            }
        }
    }
    let mut max_uv: f64 = 0.;
    if rows >= 2 && cols >= 2 {
        for i in 0..rows - 1 {
            for j in 0..cols - 1 {
                let mixed = point_sub(
                    point_sub(pt(i + 1, j + 1), pt(i + 1, j)),
                    point_sub(pt(i, j + 1), pt(i, j)),
                );
                max_uv = max_uv.max(point_norm(mixed));
            }
        }
    }
    let suu = pu * (pu - 1.).max(0.) * max_uu;
    let svv = pv * (pv - 1.).max(0.) * max_vv;
    let suv = pu * pv * max_uv;
    Ok(suu + 2. * suv + svv)
}

/// Two-sided freeform tessellation deviation for equal-weight Bezier solids.
/// Bound: M / (8 n²) with M the Bernstein second-difference coefficient.
pub fn certified_freeform_tessellation_deviation(model: &Model, segments: usize) -> Result<f64> {
    if !(1..=32).contains(&segments) {
        return Err(Error::new(
            "BREP_TESSELLATION_BUDGET_EXHAUSTED",
            "Certified freeform tessellation needs 1..32 subdivisions per patch",
        ));
    }
    if model.bodies.is_empty() || model.shells.is_empty() {
        return Err(Error::new(
            "BREP_CERTIFIED_TESSELLATION_REFUSED",
            "Freeform tessellation /1 requires closed solid bodies",
        ));
    }
    if !model.shells.iter().all(|shell| shell.closed) {
        return Err(Error::new(
            "BREP_CERTIFIED_TESSELLATION_REFUSED",
            "Freeform tessellation /1 requires closed shells",
        ));
    }
    let n = segments as f64;
    let mut maximum: f64 = 0.;
    for face in &model.faces {
        let coeff = bernstein_second_diff_coeff(&face.surface)?;
        maximum = maximum.max(coeff / (8. * n * n));
    }
    Ok(next_up(maximum))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CertifiedInterval {
    pub lower: f64,
    pub upper: f64,
}
fn next_up(value: f64) -> f64 {
    if value == f64::INFINITY {
        value
    } else if value == 0. {
        f64::from_bits(1)
    } else if value > 0. {
        f64::from_bits(value.to_bits() + 1)
    } else {
        f64::from_bits(value.to_bits() - 1)
    }
}
fn next_down(value: f64) -> f64 {
    -next_up(-value)
}
#[derive(Clone, Debug)]
pub struct CertifiedMassProperties {
    pub capability: &'static str,
    pub surface_area_mm2: CertifiedInterval,
    pub volume_mm3: CertifiedInterval,
    pub centroid: [CertifiedInterval; 3],
    pub inertia_mm5: [[CertifiedInterval; 3]; 3],
    pub context: ToleranceSpecIdentity,
    pub evidence: ComposedEvidence,
    pub audit: SolidAuditCertificate,
    pub change_set: ChangeSet,
    pub naming_complete: bool,
    pub component_count: usize,
    pub cavity_count: usize,
    pub proof: &'static str,
}
impl value_codec::Serialize for CertifiedInterval {
    fn to_value(&self) -> value_codec::Value {
        value_codec::json!({"lower":self.lower,"upper":self.upper})
    }
}
impl value_codec::Serialize for CertifiedMassProperties {
    fn to_value(&self) -> value_codec::Value {
        value_codec::json!({
            "capability":self.capability,
            "status":"certified_enclosure",
            "surfaceAreaMm2":self.surface_area_mm2,
            "volumeMm3":self.volume_mm3,
            "centroid":self.centroid,
            "inertiaMm5":self.inertia_mm5,
            "context":self.context,
            "evidenceClaimCount":self.evidence.claims.len(),
            "audit":{
                "ok":self.audit.ok,
                "bodyCount":self.audit.body_count,
                "shellCount":self.audit.shell_count,
                "selfIntersectionPairsCandidate": self.audit.self_intersection_pairs_candidate,
                "selfIntersectionComplete": self.audit.self_intersection_complete,
                "selfIntersectionPairsChecked":self.audit.self_intersection_pairs_checked
            },
            "changeSet":self.change_set,
            "namingComplete":self.naming_complete,
            "composition":{
                "componentCount":self.component_count,
                "cavityCount":self.cavity_count,
                "signedShellComposition":true
            },
            "proof":self.proof
        })
    }
}

pub(super) fn enclosure(value: f64, scale: f64) -> CertifiedInterval {
    let error = (value.abs() + scale.abs() + 1.) * f64::EPSILON * 128.;
    CertifiedInterval {
        lower: next_down(value - error),
        upper: next_up(value + error),
    }
}
pub(super) fn certified_tensor(value: [[f64; 3]; 3], scale: f64) -> [[CertifiedInterval; 3]; 3] {
    value.map(|row| row.map(|entry| enclosure(entry, scale)))
}
fn axis_aligned_box(model: &Model) -> Option<([f64; 3], [f64; 3])> {
    if model.vertices.len() != 8
        || model.faces.len() != 6
        || model.bodies.len() != 1
        || model
            .faces
            .iter()
            .any(|face| !affine_surface(&face.surface, model.tolerance_mm))
        || model.edges.iter().any(|edge| edge.curve.degree != 1)
    {
        return None;
    }
    axis_aligned_corners(model)
}

/// AA cuboid recovered from equal-weight planar Bezier freeform faces (elevated
/// bicubic cuboids). Used by the freeform mass finite cell only.
pub(super) fn axis_aligned_freeform_box(model: &Model) -> Option<([f64; 3], [f64; 3])> {
    if model.vertices.len() != 8
        || model.faces.len() != 6
        || model.bodies.len() != 1
        || !model.bodies[0].inner_shells.is_empty()
        || model.edges.iter().any(|edge| edge.curve.degree != 1)
        || model.faces.iter().any(|face| {
            !admitted_freeform_bezier_surface(&face.surface)
                || !control_net_coplanar(&face.surface, 1e-8)
        })
    {
        return None;
    }
    axis_aligned_corners(model)
}

fn axis_aligned_corners(model: &Model) -> Option<([f64; 3], [f64; 3])> {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for vertex in &model.vertices {
        for axis in 0..3 {
            min[axis] = min[axis].min(vertex.point[axis]);
            max[axis] = max[axis].max(vertex.point[axis]);
        }
    }
    let mut corners = BTreeSet::new();
    for vertex in &model.vertices {
        let mut bits = 0u8;
        for axis in 0..3 {
            if (vertex.point[axis] - max[axis]).abs() <= model.tolerance_mm {
                bits |= 1 << axis;
            } else if (vertex.point[axis] - min[axis]).abs() > model.tolerance_mm {
                return None;
            }
        }
        corners.insert(bits);
    }
    (corners.len() == 8).then_some((min, max))
}
fn axis_inertia(mass: f64, radius2: f64, height: f64, axis: [f64; 3]) -> [[f64; 3]; 3] {
    let axial = mass * radius2 / 2.;
    let transverse = mass * (3. * radius2 + height * height) / 12.;
    std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            (if i == j { transverse } else { 0. }) + (axial - transverse) * axis[i] * axis[j]
        })
    })
}
fn revolution_inertia(axial: f64, transverse: f64, axis: [f64; 3]) -> [[f64; 3]; 3] {
    std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            (if i == j { transverse } else { 0. }) + (axial - transverse) * axis[i] * axis[j]
        })
    })
}

#[derive(Clone, Copy)]
pub(super) struct ExactProperties {
    pub(super) area: f64,
    pub(super) volume: f64,
    pub(super) centroid: [f64; 3],
    pub(super) inertia: [[f64; 3]; 3],
    pub(super) scale: f64,
}

fn polynomial_integral(coefficients: &[f64], height: f64) -> f64 {
    coefficients
        .iter()
        .enumerate()
        .map(|(power, coefficient)| {
            coefficient * height.powi(power as i32 + 1) / (power + 1) as f64
        })
        .sum()
}

fn conical_properties(cone: crate::intersections::CanonicalCone) -> ExactProperties {
    let (a, b, h) = (
        cone.r_bottom,
        (cone.r_top - cone.r_bottom) / cone.height,
        cone.height,
    );
    let r2 = [a * a, 2. * a * b, b * b];
    let r4 = [
        a.powi(4),
        4. * a.powi(3) * b,
        6. * a * a * b * b,
        4. * a * b.powi(3),
        b.powi(4),
    ];
    let volume = std::f64::consts::PI * polynomial_integral(&r2, h);
    let first = std::f64::consts::PI * polynomial_integral(&[0., r2[0], r2[1], r2[2]], h);
    let z = first / volume;
    let axial = std::f64::consts::PI * polynomial_integral(&r4, h) / 2.;
    let shifted_r2 = [
        z * z * r2[0],
        z * z * r2[1] - 2. * z * r2[0],
        z * z * r2[2] - 2. * z * r2[1] + r2[0],
        -2. * z * r2[2] + r2[1],
        r2[2],
    ];
    let transverse = std::f64::consts::PI
        * (polynomial_integral(&r4, h) / 4. + polynomial_integral(&shifted_r2, h));
    let centroid = std::array::from_fn(|i| cone.bottom[i] + z * cone.axis[i]);
    ExactProperties {
        area: std::f64::consts::PI
            * ((a + cone.r_top) * h.hypot(cone.r_top - a) + a * a + cone.r_top.powi(2)),
        volume,
        centroid,
        inertia: revolution_inertia(axial, transverse, cone.axis),
        scale: h.max(a).max(cone.r_top),
    }
}

pub(super) fn exact_component_properties(model: &Model) -> Result<ExactProperties> {
    if let Some((min, max)) = axis_aligned_box(model) {
        let d = std::array::from_fn::<_, 3, _>(|i| max[i] - min[i]);
        let volume = d[0] * d[1] * d[2];
        return Ok(ExactProperties {
            area: 2. * (d[0] * d[1] + d[1] * d[2] + d[2] * d[0]),
            volume,
            centroid: std::array::from_fn(|i| (min[i] + max[i]) / 2.),
            inertia: [
                [volume * (d[1] * d[1] + d[2] * d[2]) / 12., 0., 0.],
                [0., volume * (d[0] * d[0] + d[2] * d[2]) / 12., 0.],
                [0., 0., volume * (d[0] * d[0] + d[1] * d[1]) / 12.],
            ],
            scale: d.into_iter().fold(0., f64::max),
        });
    }
    if let Some((outer, inner, height, centroid)) = canonical_tube(model) {
        let volume = std::f64::consts::PI * (outer * outer - inner * inner) * height;
        return Ok(ExactProperties {
            area: 2.
                * std::f64::consts::PI
                * ((outer + inner) * height + outer * outer - inner * inner),
            volume,
            centroid,
            inertia: axis_inertia(volume, outer * outer + inner * inner, height, [0., 0., 1.]),
            scale: height.max(outer),
        });
    }
    if let Some((area, volume, centroid, inertia, scale)) = exact_vertical_prism(model) {
        return Ok(ExactProperties {
            area,
            volume,
            centroid,
            inertia,
            scale,
        });
    }
    if let Some(sphere) = crate::intersections::recognize_sphere(model)? {
        let volume = 4. * std::f64::consts::PI * sphere.radius.powi(3) / 3.;
        let diagonal = 2. * volume * sphere.radius.powi(2) / 5.;
        return Ok(ExactProperties {
            area: 4. * std::f64::consts::PI * sphere.radius.powi(2),
            volume,
            centroid: sphere.center,
            inertia: [[diagonal, 0., 0.], [0., diagonal, 0.], [0., 0., diagonal]],
            scale: sphere.radius,
        });
    }
    if let Some(cone) = crate::intersections::recognize_cone(model)? {
        return Ok(conical_properties(cone));
    }
    if let Some(torus) = crate::intersections::recognize_torus(model)? {
        let volume = 2. * std::f64::consts::PI.powi(2) * torus.major * torus.minor.powi(2);
        let axial = volume * (torus.major.powi(2) + 0.75 * torus.minor.powi(2));
        let transverse = volume * (0.5 * torus.major.powi(2) + 0.625 * torus.minor.powi(2));
        return Ok(ExactProperties {
            area: 4. * std::f64::consts::PI.powi(2) * torus.major * torus.minor,
            volume,
            centroid: torus.center,
            inertia: revolution_inertia(axial, transverse, torus.axis),
            scale: torus.major + torus.minor,
        });
    }
    if let Some(cylinder) = crate::intersections::recognize_cylinder(model)? {
        let height = 2. * cylinder.half_height;
        let volume = std::f64::consts::PI * cylinder.radius.powi(2) * height;
        return Ok(ExactProperties {
            area: 2. * std::f64::consts::PI * cylinder.radius * (height + cylinder.radius),
            volume,
            centroid: cylinder.center,
            inertia: axis_inertia(volume, cylinder.radius.powi(2), height, cylinder.axis),
            scale: height.max(cylinder.radius),
        });
    }
    Err(Error::new(
        "BREP_CERTIFIED_MASS_REFUSED",
        "Certified mass /2 admits recognized analytic shells and exact planar prisms; generic rational/freeform quadrature remains non-certified",
    ))
}
