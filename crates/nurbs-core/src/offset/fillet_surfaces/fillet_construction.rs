//! Fillet construction: preserves the original arithmetic and proof budgets.
use super::*;

/// Simplified hold-line fillet: sections are tangent cubics through the given
/// hold-line points on the two supports (tangent to both supports at the hold
/// points). Both hold lines are resampled by arc length for a consistent
/// station correspondence. The spine/radius fields of the report are not
/// meaningful here; `radius_range` is `[0, 0]`.
pub fn hold_line_fillet(
    a: &Surface,
    b: &Surface,
    line_a: &[[f64; 3]],
    line_b: &[[f64; 3]],
    options: &FilletOptions,
) -> Result<FilletReport> {
    a.validate()?;
    b.validate()?;
    options.validated()?;
    let n = line_a.len().min(line_b.len()).clamp(2, MAX_SECTIONS).min(options.max_sections.max(2));
    let stations_a = resample_polyline(line_a, n)?;
    let stations_b = resample_polyline(line_b, n)?;
    let warnings = Vec::new();
    let mut sections = Vec::with_capacity(n);
    let mut spine = Vec::with_capacity(n);
    for k in 0..n {
        let t = k as f64 / (n - 1) as f64;
        let pa = project_point(a, stations_a[k], options.tolerance)?;
        let pb = project_point(b, stations_b[k], options.tolerance)?;
        let (Some((uva, pa_pt, na)), Some((uvb, pb_pt, nb))) = (pa, pb) else {
            return Ok(FilletReport::failed(
                FailureReason::MarchingDiverged,
                Some(t),
                format!("hold-line station {k} does not project onto its support within tolerance"),
                warnings,
            ));
        };
        let chord = sub(pb_pt, pa_pt);
        let chord_len = norm(chord);
        if chord_len <= 1e-12 {
            return Ok(FilletReport::failed(
                FailureReason::SectionSelfIntersection,
                Some(t),
                "hold-line station contacts coincide",
                warnings,
            ));
        }
        // Tangent directions: chord projected into each support tangent plane.
        let ta = unit(sub(chord, scale(na, dot(chord, na))));
        let tb = unit(sub(chord, scale(nb, dot(chord, nb))));
        let (Some(ta), Some(tb)) = (ta, tb) else {
            return Ok(FilletReport::failed(
                FailureReason::SectionSelfIntersection,
                Some(t),
                "hold-line chord is normal to a support; no tangent cubic exists",
                warnings,
            ));
        };
        let handle = chord_len / 3.;
        let p1 = add(pa_pt, scale(ta, handle));
        let p2 = sub(pb_pt, scale(tb, handle));
        let curve = Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![pa_pt.to_vec(), p1.to_vec(), p2.to_vec(), pb_pt.to_vec()],
            weights: vec![1.; 4],
            periodic: false,
        };
        curve.validate()?;
        sections.push(curve);
        spine.push(SpineSample {
            t,
            center: scale(add(pa_pt, pb_pt), 0.5),
            contact_a: pa_pt,
            contact_b: pb_pt,
            radius: 0.,
            uv_a: uva,
            uv_b: uvb,
        });
    }
    let params: Vec<f64> = (0..n).map(|k| k as f64).collect();
    match natural_loft::interpolate(&sections, &params) {
        Ok(surface) => Ok(FilletReport {
            surface: Some(surface),
            contact_pcurves: None,
            failure: None,
            spine,
            radius_range: [0.; 2],
            warnings,
            skin_deviation_estimate: 0.,
            closed_spine: false,
        }),
        Err(e) => Ok(FilletReport::failed(
            FailureReason::AssemblyFailed,
            None,
            format!("natural loft of hold-line sections failed: {e}"),
            warnings,
        )),
    }
}

/// Chordal fillet: the chord `chord` between the two contact points is fixed;
/// the radius is recovered station-wise as `r = c / (2 sin(φ/2))` from the
/// local dihedral angle `φ` between the oriented support normals.
pub fn chordal_fillet(
    a: &Surface,
    b: &Surface,
    chord: f64,
    options: &FilletOptions,
) -> Result<ChordalFilletReport> {
    require_finite_f64(chord, "chord")?;
    check(chord > 0., "Chordal fillet chord must be positive")?;
    let (m, _, _) = prepare(a, b, options)?;
    let minimum = minimum_radius(a, b, m.tol);
    if chord * 0.5 < minimum {
        let report = FilletReport::failed(
            FailureReason::RadiusBelowMinimum { radius: chord * 0.5, minimum },
            None,
            "chord is below the numerical minimum for this surface pair",
            Vec::new(),
        );
        return Ok(ChordalFilletReport { fillet: report, radius_range: [0.; 2], dihedral_range: [0.; 2] });
    }
    let warnings = Vec::new();
    // First pass: guess radius from a 60° dihedral, measure the true angles.
    let guess = chord / (2. * (std::f64::consts::FRAC_PI_3).sin());
    let (raw, seed, _) = trace_spine(&m, guess, options.max_march_steps)?;
    if seed.is_none() || raw.len() < 2 {
        let report = FilletReport::failed(
            FailureReason::SeedNotFound,
            None,
            "no converging offset intersection for the chordal seed pass",
            warnings,
        );
        return Ok(ChordalFilletReport { fillet: report, radius_range: [0.; 2], dihedral_range: [0.; 2] });
    }
    let n = options.max_sections.clamp(2, MAX_SECTIONS);
    let probe_law = RadiusLaw::Constant(guess);
    let probes = stations(&m, &raw, n, &probe_law)?;
    let mut law_points = Vec::with_capacity(n);
    let mut dihedral_range = [f64::INFINITY, f64::NEG_INFINITY];
    for sample in &probes {
        let na = point_normal(m.a, sample.uv_a[0], sample.uv_a[1])?.map(|(_, n)| n);
        let nb = point_normal(m.b, sample.uv_b[0], sample.uv_b[1])?.map(|(_, n)| n);
        let (Some(na), Some(nb)) = (na, nb) else {
            let report = FilletReport::failed(
                FailureReason::MarchingDiverged,
                Some(sample.t),
                "normal unavailable at a chordal probe station",
                warnings,
            );
            return Ok(ChordalFilletReport { fillet: report, radius_range: [0.; 2], dihedral_range: [0.; 2] });
        };
        let phi = angle_between(scale(na, m.sa), scale(nb, m.sb));
        dihedral_range[0] = dihedral_range[0].min(phi);
        dihedral_range[1] = dihedral_range[1].max(phi);
        let half = (phi * 0.5).sin();
        if half <= 1e-9 {
            let report = FilletReport::failed(
                FailureReason::SectionSelfIntersection,
                Some(sample.t),
                format!("dihedral angle {phi:.6} rad cannot carry a finite chordal radius"),
                warnings,
            );
            return Ok(ChordalFilletReport { fillet: report, radius_range: [0.; 2], dihedral_range });
        }
        law_points.push((sample.t, chord / (2. * half)));
    }
    let law = RadiusLaw::Spline(law_points);
    let [r_min, r_max, _] = law.sampled_range();
    let radius_range = [r_min, r_max];
    let fillet = variable_radius_fillet(a, b, law, options)?;
    Ok(ChordalFilletReport { fillet, radius_range, dihedral_range })
}

/// Rolling-ball fillet of constant `radius` between two NURBS surfaces.
///
/// The spine is the intersection of the two offset surfaces; sections are
/// exact circular arcs of `radius` tangent to both supports at the contact
/// points (the projections of the ball center onto the supports).
pub fn rolling_ball_fillet(
    a: &Surface,
    b: &Surface,
    radius: f64,
    options: &FilletOptions,
) -> Result<FilletReport> {
    variable_radius_fillet(a, b, RadiusLaw::Constant(radius), options)
}

/// Variable-radius fillet: the radius follows `law` along the normalized
/// spine arc parameter. Radii must stay positive (validated); when the
/// sampled maximum exceeds a support's certified fold-free reach a warning
/// names the limiting radius and location.
pub fn variable_radius_fillet(
    a: &Surface,
    b: &Surface,
    law: RadiusLaw,
    options: &FilletOptions,
) -> Result<FilletReport> {
    variable_radius_sections(a,b,law,options,SectionKind::Arc)
}

/// Variable nominal rolling-ball radii with quintic endpoint-G2 section
/// proposals. Skinning, spline-law positivity, whole-seam G2 and the actual
/// skin radius are separate qualifications; sampled stations grant none of them.
pub fn variable_g2_fillet(
    a: &Surface, b: &Surface, law: RadiusLaw, options: &FilletOptions,
) -> Result<FilletReport> {
    variable_radius_sections(a,b,law,options,SectionKind::G2)
}

pub(super) fn variable_radius_sections(
    a: &Surface, b: &Surface, law: RadiusLaw, options: &FilletOptions, kind: SectionKind,
) -> Result<FilletReport> {
    law.validate()?;
    let (m, sa, sb) = prepare(a, b, options)?;
    let [r_min, r_max, r_mean] = if let RadiusLaw::Constant(r)=&law {
        if kind==SectionKind::G2 {[*r;3]} else {law.sampled_range()}
    } else {law.sampled_range()};
    let minimum = minimum_radius(a, b, m.tol);
    if r_min < minimum {
        return Ok(FilletReport::failed(
            FailureReason::RadiusBelowMinimum { radius: r_min, minimum },
            None,
            format!("law minimum radius {r_min:.3e} is below the numerical minimum {minimum:.3e}"),
            Vec::new(),
        ));
    }
    let warnings = reachability_warnings(a, b, sa, sb, r_max);
    let (mut raw, seed, closed) = trace_spine(&m, r_mean, options.max_march_steps)?;
    if seed.is_none() {
        return Ok(FilletReport::failed(
            FailureReason::SeedNotFound,
            None,
            "no converging intersection of the two offset surfaces (wrong side selectors, or the ball of this radius never touches both supports)",
            warnings,
        ));
    }
    let mut warnings = warnings;
    if !matches!(law, RadiusLaw::Constant(_)) {
        if let Some(failure) = polish_with_law(&m, &mut raw, &law, &mut warnings)? {
            return Ok(FilletReport {
                surface: None,
                contact_pcurves: None,
                failure: Some(failure),
                spine: Vec::new(),
                radius_range: [0.; 2],
                warnings,
                skin_deviation_estimate: 0.,
                closed_spine: false,
            });
        }
    }
    build_report(&m, &raw, &law, options, warnings, closed, kind)
}

/// G2 fillet: same rolling-ball spine, but each section is a quintic Bézier
/// with position, tangent and curvature matched to both support normal
/// sections at the sampled contacts. Complete skinned-seam G2 still requires
/// independent original-surface qualification over the whole contact curve.
pub fn g2_fillet(
    a: &Surface,
    b: &Surface,
    radius: f64,
    options: &FilletOptions,
) -> Result<FilletReport> {
    variable_g2_fillet(a,b,RadiusLaw::Constant(radius),options)
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
        let mut curve = match built {
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
        if section_kind==SectionKind::Arc {
            let last=curve.control_points.len()-1;
            curve.control_points[0]=sample.contact_a.to_vec();
            curve.control_points[last]=sample.contact_b.to_vec();
            // Impose authored coordinate-plane contact constraints on the
            // proposal. This is literal source geometry, not a near-plane fit.
            // Fresh contact, radius and tangency proofs still admit the skin.
            for (support,indices) in [(m.a,[0,1]),(m.b,[last,last-1])] {
                for coordinate in 0..3 {
                    let value=support.control_points[0][0][coordinate];
                    if support.control_points.iter().flatten().all(|p|p[coordinate]==value) {
                        for i in indices {curve.control_points[i][coordinate]=value;}
                    }
                }
            }
        }
        // Result boundary: no section with non-finite control points leaves.
        for p in &curve.control_points {
            crate::foundation::guards::require_finite_slice(p, "section_control_points")?;
        }
        sections.push(curve);
    }
    let mut contact_pcurves = None;
    let mut spine = spine;
    let assembled = if section_kind == SectionKind::G2 && !closed && !matches!(law, RadiusLaw::Constant(_))
        && !(station_hermite::affine_chart(m.a) && station_hermite::affine_chart(m.b)) {
        station_hermite::build(m, raw, law, n).and_then(|(surface, pcurves, refined)| {
            contact_pcurves = Some(pcurves);
            spine = refined;
            let deviation = estimate_skin_deviation(&surface, &spine, &stations_s)?;
            Ok((surface, deviation))
        })
    } else {
        assemble(sections, &spine, &stations_s)
    };
    match assembled {
        Ok((surface, deviation)) => Ok(FilletReport {
            surface: Some(surface),
            contact_pcurves,
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
