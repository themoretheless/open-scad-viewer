use super::*;

/// Exact rational circular arc section from contact A to contact B.
pub(super) fn arc_section(sample: &SpineSample) -> Result<std::result::Result<Curve, FailureReason>> {
    let a_vec = sub(sample.contact_a, sample.center);
    let b_vec = sub(sample.contact_b, sample.center);
    let phi = angle_between(a_vec, b_vec);
    if !(1e-6..std::f64::consts::PI - 1e-9).contains(&phi) || !phi.is_finite() {
        return Ok(Err(FailureReason::SectionSelfIntersection));
    }
    let Some(a_hat) = unit(a_vec) else {
        return Ok(Err(FailureReason::SectionSelfIntersection));
    };
    let perp = sub(b_vec, scale(a_hat, dot(b_vec, a_hat)));
    let Some(v_hat) = unit(perp) else {
        return Ok(Err(FailureReason::SectionSelfIntersection));
    };
    match primitives::ellipse_arc(
        sample.center,
        a_vec,
        scale(v_hat, sample.radius),
        0.,
        phi.to_degrees(),
    ) {
        Ok(c) => Ok(Ok(c)),
        Err(_) => Ok(Err(FailureReason::AssemblyFailed)),
    }
}

/// Signed normal curvature of `surface` at `uv` in the unit direction `dir`
/// (Meusnier; sign relative to the `S_u × S_v` normal).
pub(super) fn normal_curvature(surface: &Surface, uv: [f64; 2], dir: [f64; 3]) -> Result<Option<f64>> {
    let e = surface.evaluate(uv[0], uv[1])?;
    let (Some(n), Some((su, sv)), Some((suu, suv, svv))) =
        (e.unit_normal(), e.first_derivatives(), e.second_derivatives())
    else {
        return Ok(None);
    };
    let (ee, ff, gg) = (dot(su, su), dot(su, sv), dot(sv, sv));
    let det = ee * gg - ff * ff;
    if det <= 1e-300 {
        return Ok(None);
    }
    let (ru, rv) = (dot(dir, su), dot(dir, sv));
    let alpha = (gg * ru - ff * rv) / det;
    let beta = (-ff * ru + ee * rv) / det;
    let (ll, mm, nn) = (dot(suu, n), dot(suv, n), dot(svv, n));
    Ok(Some(ll * alpha * alpha + 2. * mm * alpha * beta + nn * beta * beta))
}

/// G2 quintic Bézier section: position, tangent and curvature vector matched
/// to both support normal sections at the contacts.
pub(super) fn g2_section(m: &Marcher<'_>, sample: &SpineSample) -> Result<std::result::Result<Curve, FailureReason>> {
    let a_vec = sub(sample.contact_a, sample.center);
    let b_vec = sub(sample.contact_b, sample.center);
    let Some(section_normal) = unit(cross(a_vec, b_vec)) else {
        return Ok(Err(FailureReason::SectionSelfIntersection));
    };
    let Some((_, na)) = point_normal(m.a, sample.uv_a[0], sample.uv_a[1])? else {
        return Ok(Err(FailureReason::AssemblyFailed));
    };
    let Some((_, nb)) = point_normal(m.b, sample.uv_b[0], sample.uv_b[1])? else {
        return Ok(Err(FailureReason::AssemblyFailed));
    };
    let chord = sub(sample.contact_b, sample.contact_a);
    let chord_len = norm(chord);
    if chord_len <= 1e-12 {
        return Ok(Err(FailureReason::SectionSelfIntersection));
    }
    let tangent_at = |n: [f64; 3]| -> Option<[f64; 3]> {
        let mut t = unit(cross(section_normal, n))?;
        if dot(t, chord) < 0. {
            t = scale(t, -1.);
        }
        Some(t)
    };
    let (Some(t0), Some(t1)) = (tangent_at(na), tangent_at(nb)) else {
        return Ok(Err(FailureReason::AssemblyFailed));
    };
    let k0 = normal_curvature(m.a, sample.uv_a, t0)?.unwrap_or(0.);
    let k1 = normal_curvature(m.b, sample.uv_b, t1)?.unwrap_or(0.);
    let handle = 0.35 * chord_len;
    // Bézier end condition: curvature vector K = (4/5)·(P2−P1)⊥/a² — solve
    // for the inner controls with K matched to k_n·n of each support.
    let k0_vec = scale(na, k0);
    let k1_vec = scale(nb, k1);
    let p0 = sample.contact_a;
    let p5 = sample.contact_b;
    let p1 = add(p0, scale(t0, handle));
    let p4 = sub(p5, scale(t1, handle));
    let p2 = add(add(p1, scale(t0, handle)), scale(k0_vec, 1.25 * handle * handle));
    let p3 = add(sub(p4, scale(t1, handle)), scale(k1_vec, 1.25 * handle * handle));
    let curve = Curve {
        degree: 5,
        knots: vec![0.; 6].into_iter().chain(vec![1.; 6]).collect(),
        control_points: vec![p0, p1, p2, p3, p4, p5]
            .iter()
            .map(|p| p.to_vec())
            .collect(),
        weights: vec![1.; 6],
        periodic: false,
    };
    match curve.validate() {
        Ok(()) => Ok(Ok(curve)),
        Err(_) => Ok(Err(FailureReason::AssemblyFailed)),
    }
}

/// Skin the sections and estimate the inter-section deviation from the true
/// pipe of radius `r(t)` around the spine.
pub(super) fn assemble(
    sections: Vec<Curve>,
    spine: &[SpineSample],
    stations_s: &[f64],
) -> Result<(Surface, f64)> {
    let params: Vec<f64> = stations_s.to_vec();
    let surface = natural_loft::interpolate(&sections, &params)?;
    // Deviation estimate: mid-span samples against the spine chord.
    let mut deviation = 0_f64;
    let (du, dv) = domain(&surface);
    for k in 0..spine.len().saturating_sub(1) {
        let v_mid = (stations_s[k] + stations_s[k + 1]) * 0.5;
        let v = dv[0] + (dv[1] - dv[0]) * (v_mid - params[0]) / (params[params.len() - 1] - params[0]);
        let r_mid = (spine[k].radius + spine[k + 1].radius) * 0.5;
        for &u_frac in &[0.25, 0.5, 0.75] {
            let u = du[0] + (du[1] - du[0]) * u_frac;
            let Ok(e) = surface.evaluate(u, v) else { continue };
            let p = e.point;
            let c0 = spine[k].center;
            let c1 = spine[k + 1].center;
            let axis = sub(c1, c0);
            let len2 = dot(axis, axis);
            let w = if len2 > 1e-300 { (dot(sub(p, c0), axis) / len2).clamp(0., 1.) } else { 0. };
            let nearest = add(c0, scale(axis, w));
            deviation = deviation.max((norm(sub(p, nearest)) - r_mid).abs());
        }
    }
    Ok((surface, deviation))
}

/// Shared tail: stations, section construction, skinning, report assembly.
pub(super) fn build_report(
    m: &Marcher<'_>,
    raw: &[RawPoint],
    law: &RadiusLaw,
    options: &FilletOptions,
    warnings: Vec<String>,
    closed: bool,
    section_kind: SectionKind,
) -> Result<FilletReport> {
    let mut warnings = warnings;
    let length = raw.last().map(|p| p.s).unwrap_or(0.);
    if raw.len() < 2 || length <= 1e-12 {
        return Ok(FilletReport::failed(
            FailureReason::MarchingDiverged,
            Some(0.),
            "spine trace produced fewer than two points",
            warnings,
        ));
    }
    let n = options.max_sections.clamp(2, MAX_SECTIONS);
    let spine = stations(m, raw, n, law)?;
    // Result boundary: spine stations must be finite before section building.
    for sample in &spine {
        require_finite_point(&sample.center, "spine_center")?;
        require_finite_point(&sample.contact_a, "spine_contact_a")?;
        require_finite_point(&sample.contact_b, "spine_contact_b")?;
        require_finite_f64(sample.radius, "spine_radius")?;
    }
    let stations_s: Vec<f64> = spine.iter().map(|s| s.t * length).collect();
    let minimum = minimum_radius(m.a, m.b, m.tol);
    let mut sections = Vec::with_capacity(n);
    let mut radius_range = [f64::INFINITY, f64::NEG_INFINITY];
    for sample in &spine {
        radius_range[0] = radius_range[0].min(sample.radius);
        radius_range[1] = radius_range[1].max(sample.radius);
        if sample.radius < minimum {
            return Ok(FilletReport::failed(
                FailureReason::RadiusBelowMinimum { radius: sample.radius, minimum },
                Some(sample.t),
                format!("section radius {:.3e} below the numerical minimum {:.3e}", sample.radius, minimum),
                warnings,
            ));
        }
        let built = match section_kind {
            SectionKind::Arc => arc_section(sample)?,
            SectionKind::G2 => g2_section(m, sample)?,
        };
        let curve = match built {
            Ok(c) => c,
            Err(reason) => {
                return Ok(FilletReport::failed(
                    reason,
                    Some(sample.t),
                    format!("section construction failed at t = {:.4}", sample.t),
                    warnings,
                ));
            }
        };
        // Result boundary: no section with non-finite control points leaves.
        for p in &curve.control_points {
            crate::foundation::guards::require_finite_slice(p, "section_control_points")?;
        }
        sections.push(curve);
    }
    match assemble(sections, &spine, &stations_s) {
        Ok((surface, deviation)) => Ok(FilletReport {
            surface: Some(surface),
            failure: None,
            spine,
            radius_range,
            warnings,
            skin_deviation_estimate: deviation,
            closed_spine: closed,
        }),
        Err(e) => {
            warnings.push(format!("skinning error: {e}"));
            Ok(FilletReport::failed(
                FailureReason::AssemblyFailed,
                None,
                format!("natural loft of the sections failed: {e}"),
                warnings,
            ))
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum SectionKind {
    Arc,
    G2,
}

pub(super) fn prepare<'a>(
    a: &'a Surface,
    b: &'a Surface,
    options: &FilletOptions,
) -> Result<(Marcher<'a>, f64, f64)> {
    a.validate()?;
    b.validate()?;
    let (sa, sb) = options.validated()?;
    Ok((Marcher::new(a, b, sa, sb, options.tolerance), sa, sb))
}

pub(super) fn reachability_warnings(a: &Surface, b: &Surface, sa: f64, sb: f64, r_max: f64) -> Vec<String> {
    let mut warnings = Vec::new();
    if let Some(w) = reachability_warning("support A", a, r_max * sa) {
        warnings.push(w);
    }
    if let Some(w) = reachability_warning("support B", b, r_max * sb) {
        warnings.push(w);
    }
    warnings
}
