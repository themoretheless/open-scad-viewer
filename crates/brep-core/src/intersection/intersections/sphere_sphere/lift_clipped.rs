use super::*;

/// Like `lift`, but further clipped to the given parameter intervals of the
/// exact 3D section circle. `arcs` holds ccw parameter intervals and
/// `point_at(phi)` must evaluate the exact 3D section point at that
/// parameter; the parameterization must be monotonic with the geometric
/// angle of the section circle. Interval endpoints and midpoints are mapped
/// into each patch UV through the exact rational inversion, which fixes both
/// the sub-span trims and the angular direction without any fitting.
pub fn lift_clipped(
    sphere: &CanonicalSphere,
    normal: [f64; 3],
    middle: [f64; 3],
    arcs: &[(f64, f64)],
    point_at: impl Fn(f64) -> [f64; 3],
) -> Vec<SpherePatchCircle> {
    if sphere.pole_surfaces.is_some() {
        return pole_charts::clipped(sphere,normal,middle,arcs,point_at);
    }
    let mut lifted = Vec::new();
    for (patch, section) in patch_sections(sphere, normal, middle) {
        let frame = &sphere.patches[patch];
        let mut out: Vec<Curve> = Vec::new();
        for &(a, b) in arcs {
            let sweep = b - a;
            if !sweep.is_finite() || sweep <= 0. {
                continue;
            }
            let qa = invert_patch(sphere, frame, point_at(a));
            let qb = invert_patch(sphere, frame, point_at(b));
            let qm = invert_patch(sphere, frame, point_at(a + sweep / 2.));
            match section {
                PatchUvSection::Circle {
                    center,
                    radius,
                    start,
                    end,
                } => {
                    if qa.iter().chain(&qb).chain(&qm).any(|x| x.abs() >= 1e30) {
                        continue;
                    }
                    let psi = |q: [f64; 2]| (q[1] - center[1]).atan2(q[0] - center[0]);
                    let (pa, pb, pm) = (
                        psi(qa).rem_euclid(TAU),
                        psi(qb).rem_euclid(TAU),
                        psi(qm).rem_euclid(TAU),
                    );
                    // The stereographic map is a Möbius map of the section
                    // circle: the image of the parameter interval is one of
                    // the two ccw arcs between the endpoint images; the
                    // midpoint image decides the direction.
                    let forward = (pb - pa).rem_euclid(TAU);
                    let image = if (pm - pa).rem_euclid(TAU) <= forward {
                        (pa, forward)
                    } else {
                        (pb, (pa - pb).rem_euclid(TAU))
                    };
                    let span = (start, (end - start).min(TAU));
                    for (s0, sw) in ccw_intersect(span, image) {
                        // Defensive: the emitted sub-arc midpoint must lie in
                        // the quarter disk (the intersection above is exact,
                        // so this only filters antipode-degenerate cases).
                        let mid_angle = s0 + sw / 2.;
                        let q = [
                            center[0] + radius * mid_angle.cos(),
                            center[1] + radius * mid_angle.sin(),
                        ];
                        if in_quarter_disk(q) {
                            out.extend(circle_arcs(center, radius, s0, s0 + sw));
                        }
                    }
                }
                PatchUvSection::Line { start, end } => {
                    let d = [end[0] - start[0], end[1] - start[1]];
                    let dd = d[0] * d[0] + d[1] * d[1];
                    if !dd.is_finite() || dd <= 0. {
                        continue;
                    }
                    let t =
                        |q: [f64; 2]| ((q[0] - start[0]) * d[0] + (q[1] - start[1]) * d[1]) / dd;
                    let (ta, tb, tm) = (t(qa), t(qb), t(qm));
                    if !(-1e-9..=1. + 1e-9).contains(&tm) {
                        continue;
                    }
                    let lo = ta.min(tb).max(0.);
                    let hi = ta.max(tb).min(1.);
                    if hi - lo > 1e-12 {
                        let p = |s: f64| [start[0] + s * d[0], start[1] + s * d[1]];
                        out.push(Curve {
                            degree: 1,
                            knots: vec![0., 0., 1., 1.],
                            control_points: vec![p(lo).to_vec(), p(hi).to_vec()],
                            weights: vec![1., 1.],
                            periodic: false,
                        });
                    }
                }
            }
        }
        if !out.is_empty() {
            lifted.push(SpherePatchCircle { patch, arcs: out, implicit:None });
        }
    }
    lifted
}

/// Analytic sphere/sphere intersection of two canonical sphere solids.
/// Non-canonical operands are explicit `UnsupportedSurface` regions, never a
/// numerical fallback; tangency bands and coincident spheres stay unresolved.
pub fn intersect_sphere_sphere(
    first: &Model,
    second: &Model,
    options: Options,
) -> Result<Report<SphereSphereComponent>> {
    let options = options.validate()?;
    let _ = options;
    let mut report = Report::default();
    let domain = vec![0., 1., 0., 1., 0., 1., 0., 1.];
    let (Some(a), Some(b)) = (recognize_any(first)?, recognize_any(second)?) else {
        report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    };
    report.boxes_visited = 1;
    // Outward binary64 classification band: recognition deviation plus a
    // rounding allowance on the center difference, its norm, and the radii.
    let delta = sub(b.center, a.center);
    let scale = delta.iter().map(|v| v.abs()).fold(0., f64::max);
    let distance = if scale == 0. {
        0.
    } else {
        let scaled = delta.map(|x| x / scale);
        scaled[0].hypot(scaled[1]).hypot(scaled[2]) * scale
    };
    let band =
        a.error + b.error + 16. * f64::EPSILON * (distance + scale + a.radius + b.radius + 1.);
    let d_lo = (distance - band).max(0.);
    let d_hi = distance + band;
    let sum = a.radius + b.radius;
    let sum_lo = sum - band;
    let sum_hi = sum + band;
    let diff = (a.radius - b.radius).abs();
    let diff_lo = (diff - band).max(0.);
    let diff_hi = diff + band;
    // Separate: the whole distance band clears the radius sum.
    if d_lo > sum_hi {
        return Ok(report);
    }
    // Contained without contact, including concentric unequal radii.
    if d_hi < diff_lo {
        return Ok(report);
    }
    let zero = band.max(64. * f64::EPSILON * sum);
    // Concentric with radii equal within the band: coincident surfaces (or an
    // inseparable near-coincidence), never a fabricated curve.
    if d_hi <= zero && diff_lo <= zero {
        report.unresolved(domain, UnresolvedReason::CoincidentTrim);
        return Ok(report);
    }
    // External or internal tangency, or a classification band straddling one:
    // tangent contacts are never reported as points (house tangency rule).
    if d_hi >= sum_lo || d_lo <= diff_hi {
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        return Ok(report);
    }
    // Transverse pair: the circle exists with clear margin on both sides.
    let normal = delta.map(|x| x / distance);
    let along = (a.radius * a.radius - b.radius * b.radius + distance * distance) / (2. * distance);
    let h2 = a.radius * a.radius - along * along;
    if !h2.is_finite() || h2 <= 0. {
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        return Ok(report);
    }
    let radius = h2.sqrt();
    let center = std::array::from_fn(|i| a.center[i] + along * normal[i]);
    // Circle basis: cross with the coordinate axis least aligned with normal.
    let mut axis = 0;
    for candidate in 1..3 {
        if normal[candidate].abs() < normal[axis].abs() {
            axis = candidate;
        }
    }
    let unit = std::array::from_fn(|i| (i == axis) as u8 as f64);
    let e1 = cross(normal, unit);
    let e1_length = dot(e1, e1).sqrt();
    if !e1_length.is_finite() || e1_length <= 0. {
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        return Ok(report);
    }
    let e1 = e1.map(|x| x / e1_length);
    let e2 = cross(normal, e1);
    let curve = circle_curve(center, radius, e1, e2);
    let first_uv = lift(&a, normal, center);
    let second_uv = lift(&b, normal, center);
    let mut max_sample_residual = 0_f64;
    for i in 0..16 {
        let point = curve.evaluate(i as f64 / 4.)?.point;
        let point: [f64; 3] = [point[0], point[1], point[2]];
        for sphere in [&a, &b] {
            let d = sub(point, sphere.center);
            let residual = (d[0].hypot(d[1]).hypot(d[2]) - sphere.radius).abs();
            max_sample_residual = max_sample_residual.max(residual);
        }
    }
    report.components.push(SphereSphereComponent::Circle {
        curve,
        center,
        radius,
        normal,
        first_uv,
        second_uv,
        max_sample_residual,
    });
    Ok(report)
}
