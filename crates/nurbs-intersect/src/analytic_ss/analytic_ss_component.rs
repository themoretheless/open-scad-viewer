use super::*;

pub(super) fn invalid(message: &str) -> Error {
    Error::new("BREP_ANALYTIC_SS_INVALID", message)
}
fn unsupported(message: &str) -> Error {
    Error::new("BREP_ANALYTIC_SS_UNSUPPORTED", message)
}


pub(super) fn normalize(a: [f64; 3]) -> Result<[f64; 3]> {
    let n = norm(a);
    if !n.is_finite() || n == 0. {
        return Err(invalid("Expected a finite nonzero direction"));
    }
    Ok(mul(a, 1. / n))
}

#[derive(Clone, Debug)]
pub enum AnalyticSsComponent {
    Empty,
    Point {
        point: [f64; 3],
        contact: Contact,
    },
    Line {
        start: [f64; 3],
        end: [f64; 3],
    },
    Circle {
        center: [f64; 3],
        normal: [f64; 3],
        radius: f64,
    },
    Ellipse {
        center: [f64; 3],
        major: [f64; 3],
        minor: [f64; 3],
    },
}

#[derive(Clone, Copy, Debug)]
pub struct FiniteCylinder {
    pub origin: [f64; 3],
    pub direction: [f64; 3],
    pub radius: f64,
    pub height: [f64; 2],
}

#[derive(Clone, Copy, Debug)]
pub struct FiniteCone {
    pub apex: [f64; 3],
    pub axis: [f64; 3],
    pub radius: f64,
    pub height: f64,
}

pub(super) fn complete_report<T>(components: Vec<T>) -> Report<T> {
    Report {
        components,
        unresolved: Vec::new(),
        boxes_visited: 1,
        bernstein_excluded: 0,
        coverage: Coverage::Complete,
    }
}

pub(super) fn incomplete_report<T>(domain: Vec<f64>, reason: UnresolvedReason) -> Report<T> {
    let mut report = Report::default();
    report.unresolved(domain, reason);
    report
}

/// Degree-one line segment × plane with algebraic Complete coverage.
pub fn complete_line_plane(
    start: [f64; 3],
    end: [f64; 3],
    plane: Plane,
    options: Options,
) -> Result<Report<CurvePlaneComponent>> {
    let options = options.validate()?;
    let plane = plane.normalized()?;
    if !start.iter().chain(&end).all(|x| x.is_finite()) {
        return Err(invalid("Line/plane requires finite endpoints"));
    }
    let d0 = plane.distance(start);
    let d1 = plane.distance(end);
    if !d0.is_finite() || !d1.is_finite() {
        return Err(invalid("Line/plane distances must be finite"));
    }
    let tol = options.distance_tolerance;
    if d0.abs() <= tol && d1.abs() <= tol {
        return Ok(complete_report(vec![CurvePlaneComponent::Overlap {
            parameter_interval: [0., 1.],
            control_residual: d0.abs().max(d1.abs()),
        }]));
    }
    if d0.abs() <= tol {
        return Ok(complete_report(vec![CurvePlaneComponent::Point(
            CurvePoint {
                parameter: 0.,
                parameter_interval: [0., 0.],
                point: start,
                plane_residual: d0.abs(),
                contact: Contact::Boundary,
            },
        )]));
    }
    if d1.abs() <= tol {
        return Ok(complete_report(vec![CurvePlaneComponent::Point(
            CurvePoint {
                parameter: 1.,
                parameter_interval: [1., 1.],
                point: end,
                plane_residual: d1.abs(),
                contact: Contact::Boundary,
            },
        )]));
    }
    if d0.signum() == d1.signum() {
        return Ok(complete_report(vec![]));
    }
    let t = d0 / (d0 - d1);
    if !t.is_finite() || !(0. ..=1.).contains(&t) {
        return Ok(incomplete_report(
            vec![0., 1.],
            UnresolvedReason::NearCoincidence,
        ));
    }
    let point = add(start, mul(sub(end, start), t));
    let residual = plane.distance(point).abs();
    if residual > tol {
        return Ok(incomplete_report(
            vec![0., 1.],
            UnresolvedReason::NearCoincidence,
        ));
    }
    let Ok(tolerance_context) =
        cad_predicates::ToleranceContext::from_brep_tolerance_mm(options.distance_tolerance)
    else {
        return Ok(incomplete_report(
            vec![0., 1.],
            UnresolvedReason::NearCoincidence,
        ));
    };
    if crate::predicate_evidence::require_transverse_line_plane_evidence_in_context(
        start,
        end,
        plane,
        &tolerance_context,
    )
    .is_err()
    {
        return Ok(incomplete_report(
            vec![0., 1.],
            UnresolvedReason::NearCoincidence,
        ));
    }
    Ok(complete_report(vec![CurvePlaneComponent::Point(
        CurvePoint {
            parameter: t,
            parameter_interval: [t, t],
            point,
            plane_residual: residual,
            contact: Contact::Transverse,
        },
    )]))
}

/// Finite right circular cylinder × plane (axis-aligned height interval).
pub fn plane_cylinder(
    plane: Plane,
    axis_origin: [f64; 3],
    axis_dir: [f64; 3],
    radius: f64,
    height: [f64; 2],
    options: Options,
) -> Result<Report<AnalyticSsComponent>> {
    let options = options.validate()?;
    let plane = plane.normalized()?;
    let axis = normalize(axis_dir)?;
    if !(radius.is_finite() && radius > 0.)
        || !height.iter().all(|z| z.is_finite())
        || height[1] <= height[0]
        || !axis_origin.iter().all(|x| x.is_finite())
    {
        return Err(invalid("Cylinder parameters must be finite and ordered"));
    }
    let cos = dot(plane.normal, axis).abs();
    let tol = options.distance_tolerance;
    // Parallel: |n·axis| ≈ 0 → generators are lines (0/1/2).
    if cos <= 1e-12 {
        let center_dist = plane.distance(axis_origin).abs();
        if (center_dist - radius).abs() <= tol {
            // Tangent generator — refuse as degenerate contact for this matrix.
            return Ok(incomplete_report(
                vec![height[0], height[1]],
                UnresolvedReason::TangencyOrMultipleRoot,
            ));
        }
        if center_dist > radius + tol {
            return Ok(complete_report(vec![AnalyticSsComponent::Empty]));
        }
        if center_dist + tol < radius {
            let offset = (radius * radius - center_dist * center_dist).sqrt();
            if !offset.is_finite() {
                return Ok(incomplete_report(
                    vec![height[0], height[1]],
                    UnresolvedReason::NearCoincidence,
                ));
            }
            let lateral = normalize(cross(plane.normal, axis))?;
            let toward = mul(plane.normal, -plane.distance(axis_origin).signum());
            let base = add(axis_origin, mul(toward, center_dist));
            let mut components = Vec::new();
            for sign in [-1., 1.] {
                let foot = add(base, mul(lateral, sign * offset));
                let start = add(foot, mul(axis, height[0]));
                let end = add(foot, mul(axis, height[1]));
                components.push(AnalyticSsComponent::Line { start, end });
            }
            return Ok(complete_report(components));
        }
        return Ok(incomplete_report(
            vec![height[0], height[1]],
            UnresolvedReason::NearCoincidence,
        ));
    }
    // Perpendicular: circle in the cutting plane, clipped to height.
    if (cos - 1.).abs() <= 1e-12 {
        let t = (plane.offset - dot(plane.normal, axis_origin)) / dot(plane.normal, axis);
        if !t.is_finite() {
            return Ok(incomplete_report(
                vec![height[0], height[1]],
                UnresolvedReason::NearCoincidence,
            ));
        }
        if t < height[0] - tol || t > height[1] + tol {
            return Ok(complete_report(vec![AnalyticSsComponent::Empty]));
        }
        if (t - height[0]).abs() <= tol || (t - height[1]).abs() <= tol {
            return Ok(complete_report(vec![AnalyticSsComponent::Circle {
                center: add(axis_origin, mul(axis, t.clamp(height[0], height[1]))),
                normal: axis,
                radius,
            }]));
        }
        return Ok(complete_report(vec![AnalyticSsComponent::Circle {
            center: add(axis_origin, mul(axis, t)),
            normal: axis,
            radius,
        }]));
    }
    // Oblique: ellipse in plane; only admit when the full ellipse lies in height.
    let n = plane.normal;
    let center_line_t = (plane.offset - dot(n, axis_origin)) / dot(n, axis);
    if !center_line_t.is_finite() {
        return Ok(incomplete_report(
            vec![height[0], height[1]],
            UnresolvedReason::NearCoincidence,
        ));
    }
    let center = add(axis_origin, mul(axis, center_line_t));
    let sin = (1. - cos * cos).sqrt();
    if !sin.is_finite() || sin <= 1e-12 {
        return Ok(incomplete_report(
            vec![height[0], height[1]],
            UnresolvedReason::NearCoincidence,
        ));
    }
    let major_dir = normalize(cross(n, cross(axis, n)))?;
    let minor_dir = normalize(cross(n, major_dir))?;
    let major_r = radius / sin;
    let minor_r = radius;
    let half_extent = major_r * cos / sin;
    if !half_extent.is_finite()
        || center_line_t - half_extent < height[0] - tol
        || center_line_t + half_extent > height[1] + tol
    {
        // Clipped ellipse is out of the frozen matrix.
        return Ok(incomplete_report(
            vec![height[0], height[1]],
            UnresolvedReason::UnsupportedSurface,
        ));
    }
    Ok(complete_report(vec![AnalyticSsComponent::Ellipse {
        center,
        major: mul(major_dir, major_r),
        minor: mul(minor_dir, minor_r),
    }]))
}

/// Unit sphere centered at origin × plane.
pub fn plane_sphere(
    plane: Plane,
    center: [f64; 3],
    radius: f64,
    options: Options,
) -> Result<Report<AnalyticSsComponent>> {
    let options = options.validate()?;
    let plane = plane.normalized()?;
    if !(radius.is_finite() && radius > 0.) || !center.iter().all(|x| x.is_finite()) {
        return Err(invalid("Sphere parameters must be finite and positive"));
    }
    let d = plane.distance(center);
    let tol = options.distance_tolerance;
    if d.abs() > radius + tol {
        return Ok(complete_report(vec![AnalyticSsComponent::Empty]));
    }
    if (d.abs() - radius).abs() <= tol {
        return Ok(complete_report(vec![AnalyticSsComponent::Point {
            point: sub(center, mul(plane.normal, d.signum() * radius)),
            contact: Contact::Boundary,
        }]));
    }
    if d.abs() + tol < radius {
        let circle_r = (radius * radius - d * d).sqrt();
        if !circle_r.is_finite() {
            return Ok(incomplete_report(vec![], UnresolvedReason::NearCoincidence));
        }
        return Ok(complete_report(vec![AnalyticSsComponent::Circle {
            center: sub(center, mul(plane.normal, d)),
            normal: plane.normal,
            radius: circle_r,
        }]));
    }
    Ok(incomplete_report(vec![], UnresolvedReason::NearCoincidence))
}

/// Finite right circular cone (apex at origin tip along +axis, slope r/h) × plane.
/// Only the perpendicular mid-cut (circle) and empty/parallel miss are Complete.
pub fn plane_cone(
    plane: Plane,
    apex: [f64; 3],
    axis_dir: [f64; 3],
    base_radius: f64,
    height: f64,
    options: Options,
) -> Result<Report<AnalyticSsComponent>> {
    let options = options.validate()?;
    let plane = plane.normalized()?;
    let axis = normalize(axis_dir)?;
    if !(base_radius.is_finite() && base_radius > 0. && height.is_finite() && height > 0.)
        || !apex.iter().all(|x| x.is_finite())
    {
        return Err(invalid("Cone parameters must be finite and positive"));
    }
    let cos = dot(plane.normal, axis).abs();
    let tol = options.distance_tolerance;
    if cos <= 1e-12 {
        // Parallel to axis: out of frozen matrix (hyperbolic generators).
        return Ok(incomplete_report(
            vec![0., height],
            UnresolvedReason::UnsupportedSurface,
        ));
    }
    if (cos - 1.).abs() > 1e-12 {
        return Ok(incomplete_report(
            vec![0., height],
            UnresolvedReason::UnsupportedSurface,
        ));
    }
    let t = (plane.offset - dot(plane.normal, apex)) / dot(plane.normal, axis);
    if !t.is_finite() {
        return Ok(incomplete_report(
            vec![0., height],
            UnresolvedReason::NearCoincidence,
        ));
    }
    if t < -tol || t > height + tol {
        return Ok(complete_report(vec![AnalyticSsComponent::Empty]));
    }
    if t.abs() <= tol {
        return Ok(complete_report(vec![AnalyticSsComponent::Point {
            point: apex,
            contact: Contact::Boundary,
        }]));
    }
    let clamped = t.clamp(0., height);
    let radius = base_radius * (clamped / height);
    Ok(complete_report(vec![AnalyticSsComponent::Circle {
        center: add(apex, mul(axis, clamped)),
        normal: axis,
        radius,
    }]))
}

/// Two finite parallel cylinders: empty, coincident walls (empty), or two
/// generator `Line`s for transverse wall contact. Skew / non-parallel refuse.
pub fn cylinder_cylinder(
    a: FiniteCylinder,
    b: FiniteCylinder,
    options: Options,
) -> Result<Report<AnalyticSsComponent>> {
    let FiniteCylinder {
        origin: a_origin,
        direction: a_dir,
        radius: a_radius,
        height: a_height,
    } = a;
    let FiniteCylinder {
        origin: b_origin,
        direction: b_dir,
        radius: b_radius,
        height: b_height,
    } = b;
    let options = options.validate()?;
    let a_axis = normalize(a_dir)?;
    let b_axis = normalize(b_dir)?;
    if !(a_radius.is_finite() && b_radius.is_finite() && a_radius > 0. && b_radius > 0.) {
        return Err(invalid("Cylinder radii must be positive and finite"));
    }
    let parallel = cross(a_axis, b_axis);
    if norm(parallel) > 1e-12 {
        return Ok(incomplete_report(
            vec![a_height[0], a_height[1], b_height[0], b_height[1]],
            UnresolvedReason::UnsupportedSurface,
        ));
    }
    let delta = sub(b_origin, a_origin);
    let radial = sub(delta, mul(a_axis, dot(delta, a_axis)));
    let separation = norm(radial);
    let tol = options.distance_tolerance;
    let z0 = a_height[0].max(b_height[0]);
    let z1 = a_height[1].min(b_height[1]);
    if z1 < z0 - tol {
        return Ok(complete_report(vec![AnalyticSsComponent::Empty]));
    }
    if separation > a_radius + b_radius + tol {
        return Ok(complete_report(vec![AnalyticSsComponent::Empty]));
    }
    // Strict nesting without wall contact: |r_a - r_b| > separation.
    let nested = (a_radius - b_radius).abs() > separation + tol;
    if nested {
        return Ok(complete_report(vec![AnalyticSsComponent::Empty]));
    }
    // Coincident walls (same radius, shared axis): no transverse curve; Complete empty.
    if separation <= tol && (a_radius - b_radius).abs() <= tol {
        return Ok(complete_report(vec![AnalyticSsComponent::Empty]));
    }
    // Near-tangent band: refuse Incomplete (not certified for imprint).
    if (separation - (a_radius + b_radius)).abs() <= tol
        || (separation - (a_radius - b_radius).abs()).abs() <= tol
    {
        return Ok(incomplete_report(
            vec![a_height[0], a_height[1], b_height[0], b_height[1]],
            UnresolvedReason::NearCoincidence,
        ));
    }
    // Transverse parallel wall contact → two generator lines from 2D circle×circle.
    if separation <= tol {
        let _ = unsupported;
        return Ok(incomplete_report(
            vec![a_height[0], a_height[1], b_height[0], b_height[1]],
            UnresolvedReason::UnsupportedSurface,
        ));
    }
    let ux = radial[0] / separation;
    let uy = radial[1] / separation;
    let uz = radial[2] / separation;
    let a =
        (a_radius * a_radius - b_radius * b_radius + separation * separation) / (2. * separation);
    let h2 = a_radius * a_radius - a * a;
    if h2 <= 0. {
        return Ok(incomplete_report(
            vec![a_height[0], a_height[1], b_height[0], b_height[1]],
            UnresolvedReason::NearCoincidence,
        ));
    }
    let h = h2.sqrt();
    let mid = add(a_origin, mul([ux, uy, uz], a));
    // Perpendicular in the plane orthogonal to the shared axis.
    let px = a_axis[1] * uz - a_axis[2] * uy;
    let py = a_axis[2] * ux - a_axis[0] * uz;
    let pz = a_axis[0] * uy - a_axis[1] * ux;
    let pn = (px * px + py * py + pz * pz).sqrt();
    if pn <= 1e-15 {
        return Ok(incomplete_report(
            vec![a_height[0], a_height[1], b_height[0], b_height[1]],
            UnresolvedReason::UnsupportedSurface,
        ));
    }
    let (px, py, pz) = (px / pn * h, py / pn * h, pz / pn * h);
    let p0 = [mid[0] + px, mid[1] + py, mid[2] + pz];
    let p1 = [mid[0] - px, mid[1] - py, mid[2] - pz];
    let start0 = add(
        p0,
        mul(a_axis, z0 - dot(p0, a_axis) + dot(a_origin, a_axis)),
    );
    // Project intersection points onto the axis parameter range [z0, z1].
    let axial0 = |p: [f64; 3]| -> f64 { dot(sub(p, a_origin), a_axis) };
    let lift = |p: [f64; 3], z: f64| -> [f64; 3] {
        let cur = axial0(p);
        add(p, mul(a_axis, z - cur))
    };
    let _ = start0;
    Ok(complete_report(vec![
        AnalyticSsComponent::Line {
            start: lift(p0, z0),
            end: lift(p0, z1),
        },
        AnalyticSsComponent::Line {
            start: lift(p1, z0),
            end: lift(p1, z1),
        },
    ]))
}

/// Two spheres: Complete empty / tangent point / circle; near-band Incomplete.
pub fn sphere_sphere(
    a_center: [f64; 3],
    a_radius: f64,
    b_center: [f64; 3],
    b_radius: f64,
    options: Options,
) -> Result<Report<AnalyticSsComponent>> {
    let options = options.validate()?;
    if !(a_radius.is_finite() && b_radius.is_finite() && a_radius > 0. && b_radius > 0.)
        || !a_center.iter().chain(&b_center).all(|x| x.is_finite())
    {
        return Err(invalid("Sphere parameters must be finite and positive"));
    }
    let delta = sub(b_center, a_center);
    let d = norm(delta);
    let tol = options.distance_tolerance;
    if d > a_radius + b_radius + tol {
        return Ok(complete_report(vec![AnalyticSsComponent::Empty]));
    }
    if d + tol < (a_radius - b_radius).abs() {
        return Ok(complete_report(vec![AnalyticSsComponent::Empty]));
    }
    if (d - (a_radius + b_radius)).abs() <= tol || (d - (a_radius - b_radius).abs()).abs() <= tol {
        if d <= tol {
            return Ok(incomplete_report(vec![], UnresolvedReason::NearCoincidence));
        }
        let t = a_radius / d;
        return Ok(complete_report(vec![AnalyticSsComponent::Point {
            point: add(a_center, mul(delta, t)),
            contact: Contact::Boundary,
        }]));
    }
    if d <= tol {
        return Ok(incomplete_report(vec![], UnresolvedReason::NearCoincidence));
    }
    if d + tol < a_radius + b_radius && d > (a_radius - b_radius).abs() + tol {
        let x = (a_radius * a_radius - b_radius * b_radius + d * d) / (2. * d);
        let r2 = a_radius * a_radius - x * x;
        if r2 <= 0. {
            return Ok(incomplete_report(vec![], UnresolvedReason::NearCoincidence));
        }
        let circle_r = r2.sqrt();
        let axis = normalize(delta)?;
        return Ok(complete_report(vec![AnalyticSsComponent::Circle {
            center: add(a_center, mul(axis, x)),
            normal: axis,
            radius: circle_r,
        }]));
    }
    Ok(incomplete_report(vec![], UnresolvedReason::NearCoincidence))
}
