use super::*;

/// FrameLaw production sweep: Frenet / rotation-minimizing / fixed frames along a
/// polyline path, authored as a ruled solid (no mesh faceted_sweep).
pub fn frame_law_ruled_sweep(
    profile: &[[f64; 2]],
    path: &[[f64; 3]],
    frame_law: &str,
) -> Result<(Model, FeatureCertificate)> {
    if profile.len() < 3 || path.len() < 2 {
        return Err(refuse(
            "BREP_FRAME_LAW_REFUSED",
            "FrameLaw sweep requires a closed-capable profile (≥3) and a path (≥2)",
        ));
    }
    if profile.len() > 64 || path.len() > 64 {
        return Err(refuse(
            "BREP_FRAME_LAW_REFUSED",
            "FrameLaw sweep exceeded station/profile budget (64)",
        ));
    }
    let law = match frame_law {
        "frenet" | "rotation-minimizing" | "rmf" | "fixed" => frame_law,
        _ => {
            return Err(refuse(
                "BREP_FRAME_LAW_REFUSED",
                "FrameLaw must be frenet, rotation-minimizing, or fixed",
            ));
        }
    };
    let mut area = 0.;
    for i in 0..profile.len() {
        let j = (i + 1) % profile.len();
        area += profile[i][0] * profile[j][1] - profile[j][0] * profile[i][1];
        if !profile[i][0].is_finite() || !profile[i][1].is_finite() {
            return Err(refuse(
                "BREP_FRAME_LAW_REFUSED",
                "Profile points must be finite",
            ));
        }
    }
    if area <= 0. {
        return Err(refuse(
            "BREP_FRAME_LAW_REFUSED",
            "Profile must be counter-clockwise",
        ));
    }
    for w in path.windows(2) {
        let d = [w[1][0] - w[0][0], w[1][1] - w[0][1], w[1][2] - w[0][2]];
        if d[0].hypot(d[1]).hypot(d[2]) <= 1e-9 {
            return Err(refuse(
                "BREP_FRAME_LAW_REFUSED",
                "Path has a collapsed station; refuse Frenet singularity",
            ));
        }
        if !w[0].iter().chain(&w[1]).all(|x| x.is_finite()) {
            return Err(refuse(
                "BREP_FRAME_LAW_REFUSED",
                "Path points must be finite",
            ));
        }
    }
    // Station planes must stay parallel for ruled_loft caps (translation/extrusion
    // FrameLaw). Bent paths with non-parallel stations refuse typed — no mesh sweep.
    if path.len() >= 2 {
        let t0 = [
            path[1][0] - path[0][0],
            path[1][1] - path[0][1],
            path[1][2] - path[0][2],
        ];
        for w in path.windows(2).skip(1) {
            let t = [w[1][0] - w[0][0], w[1][1] - w[0][1], w[1][2] - w[0][2]];
            let c = [
                t0[1] * t[2] - t0[2] * t[1],
                t0[2] * t[0] - t0[0] * t[2],
                t0[0] * t[1] - t0[1] * t[0],
            ];
            if c[0].hypot(c[1]).hypot(c[2]) > 1e-6 * t0[0].hypot(t0[1]).hypot(t0[2]).max(1.) {
                return Err(refuse(
                    "BREP_FRAME_LAW_REFUSED",
                    "Non-parallel path stations are outside ruled FrameLaw production; refuse mesh sweep",
                ));
            }
        }
    }
    pub(super) fn norm3(v: [f64; 3]) -> f64 {
        v[0].hypot(v[1]).hypot(v[2])
    }
    pub(super) fn unit3(v: [f64; 3]) -> Option<[f64; 3]> {
        let n = norm3(v);
        if n <= 1e-15 {
            None
        } else {
            Some([v[0] / n, v[1] / n, v[2] / n])
        }
    }
    pub(super) fn cross3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    }
    pub(super) fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
    }
    let mut frames: Vec<([f64; 3], [f64; 3], [f64; 3])> = Vec::new();
    let mut prev_n: Option<[f64; 3]> = None;
    for i in 0..path.len() {
        let t = if i + 1 < path.len() {
            unit3([
                path[i + 1][0] - path[i][0],
                path[i + 1][1] - path[i][1],
                path[i + 1][2] - path[i][2],
            ])
        } else {
            unit3([
                path[i][0] - path[i - 1][0],
                path[i][1] - path[i - 1][1],
                path[i][2] - path[i - 1][2],
            ])
        }
        .ok_or_else(|| refuse("BREP_FRAME_LAW_REFUSED", "Degenerate path tangent"))?;
        let n = if law == "fixed" {
            let helper = if t[2].abs() < 0.9 {
                [0., 0., 1.]
            } else {
                [1., 0., 0.]
            };
            unit3(cross3(cross3(t, helper), t))
                .ok_or_else(|| refuse("BREP_FRAME_LAW_REFUSED", "Fixed frame collapsed"))?
        } else if let Some(pn) = prev_n {
            // Rotation-minimizing / Frenet: project previous normal onto plane ⊥ T.
            let n0 = [
                pn[0] - dot3(pn, t) * t[0],
                pn[1] - dot3(pn, t) * t[1],
                pn[2] - dot3(pn, t) * t[2],
            ];
            unit3(n0).ok_or_else(|| {
                refuse(
                    "BREP_FRAME_LAW_REFUSED",
                    "Frenet/RMF normal collapsed (inflection)",
                )
            })?
        } else {
            let helper = if t[2].abs() < 0.9 {
                [0., 0., 1.]
            } else {
                [1., 0., 0.]
            };
            unit3(cross3(cross3(t, helper), t))
                .ok_or_else(|| refuse("BREP_FRAME_LAW_REFUSED", "Seed normal collapsed"))?
        };
        let bvec = unit3(cross3(t, n))
            .ok_or_else(|| refuse("BREP_FRAME_LAW_REFUSED", "Binormal collapsed"))?;
        prev_n = Some(n);
        frames.push((t, n, bvec));
    }
    let mut sections = Vec::new();
    for (station, (_t, n, bvec)) in path.iter().zip(frames.iter()) {
        let mut pts = Vec::new();
        for p in profile {
            pts.push([
                station[0] + p[0] * n[0] + p[1] * bvec[0],
                station[1] + p[0] * n[1] + p[1] * bvec[1],
                station[2] + p[0] * n[2] + p[1] * bvec[2],
            ]);
        }
        sections.push(pts);
    }
    let result = ruled_loft(&sections)?;
    result.validate()?;
    Ok((
        result,
        FeatureCertificate {
            capability: "analytic-solid-loft/1",
            complete: true,
            notes: vec!["frame_law_parallel_translation_walking_slice"],
        },
    ))
}

/// Certified successor entry point for the only exact frame-law slice authored
/// here: collinear stations with a constant parallel frame.
pub fn audited_parallel_frame_sweep(
    profile: &[[f64; 2]],
    path: &[[f64; 3]],
    frame_law: &str,
) -> Result<AuditedFeatureResult> {
    if !matches!(frame_law, "fixed" | "rotation-minimizing" | "rmf") {
        return Err(refuse(
            "BREP_FRAME_LAW_REFUSED",
            "Certified successor sweep admits fixed/parallel RMF translation only",
        ));
    }
    let (result, mut feature) = frame_law_ruled_sweep(profile, path, frame_law)?;
    feature.capability = EXACT_PARALLEL_FRAME_SWEEP_CAPABILITY;
    let context = result.tolerance_context()?;
    let evidence = compose_predicate_evidence(
        &context,
        [
            PredicateEvidence::correspondence(&context, 0., 1.)?,
            PredicateEvidence::topology_preservation(
                &context,
                "parallel sweep shared section incidence",
                result.persistent_naming_complete(),
            )?,
        ],
    )?;
    let audit = audit_solid(&result)?;
    let naming_complete = result.persistent_naming_complete()
        && result.1.faces.len() == result.faces.len()
        && result.1.edges.len() == result.edges.len();
    if !naming_complete {
        return Err(refuse(
            "BREP_FRAME_LAW_REFUSED",
            "Certified successor sweep lacks complete ChangeSet/naming evidence",
        ));
    }
    Ok(AuditedFeatureResult {
        change_set: result.1.change_set.clone(),
        model: result,
        feature,
        context: context.spec_identity(),
        evidence,
        audit,
        naming_complete,
    })
}

pub(super) fn certify_loft_sweep(
    result: Model,
    capability: &'static str,
    notes: Vec<&'static str>,
    scale: f64,
) -> Result<AuditedFeatureResult> {
    result.validate()?;
    let context = result.tolerance_context()?;
    let naming_complete = result.persistent_naming_complete()
        && result.1.faces.len() == result.faces.len()
        && result.1.edges.len() == result.edges.len();
    let evidence = compose_predicate_evidence(
        &context,
        [
            PredicateEvidence::correspondence(&context, 0., scale.max(1e-12))?,
            PredicateEvidence::topology_preservation(
                &context,
                "exact indexed section correspondence and shared span-boundary edges",
                naming_complete,
            )?,
        ],
    )?;
    let audit = audit_solid(&result).map_err(|error| {
        refuse(
            "BREP_LOFT_SWEEP_AUDIT_REFUSED",
            &format!(
                "Closed loft/sweep failed sew, orientation, or self-intersection audit: {}",
                error.message
            ),
        )
    })?;
    if !naming_complete {
        return Err(refuse(
            "BREP_LOFT_SWEEP_NAMING_REFUSED",
            "Closed loft/sweep lacks complete persistent names or ChangeSet ownership",
        ));
    }
    Ok(AuditedFeatureResult {
        change_set: result.1.change_set.clone(),
        model: result,
        feature: FeatureCertificate {
            capability,
            complete: true,
            notes,
        },
        context: context.spec_identity(),
        evidence,
        audit,
        naming_complete,
    })
}

pub(super) fn convex_section_area(section: &[[f64; 3]]) -> Result<f64> {
    if section.len() < 3 {
        return Err(refuse(
            "BREP_LOFT_SECTION_COLLAPSE_REFUSED",
            "A section needs at least three control vertices",
        ));
    }
    let origin = section[0];
    let u = unit3(sub3(section[1], origin)).ok_or_else(|| {
        refuse(
            "BREP_LOFT_SECTION_COLLAPSE_REFUSED",
            "Section has a collapsed first NURBS edge",
        )
    })?;
    let normal = unit3(cross3(u, sub3(section[2], origin))).ok_or_else(|| {
        refuse(
            "BREP_LOFT_SECTION_COLLAPSE_REFUSED",
            "Section has a singular support plane",
        )
    })?;
    let v = cross3(normal, u);
    let profile = section
        .iter()
        .map(|point| {
            let delta = sub3(*point, origin);
            if dot3(delta, normal).abs() > 1e-7 {
                return Err(refuse(
                    "BREP_LOFT_SECTION_TOPOLOGY_REFUSED",
                    "Section control polygon is not planar",
                ));
            }
            Ok([dot3(delta, u), dot3(delta, v)])
        })
        .collect::<Result<Vec<_>>>()?;
    let mut area2 = 0.;
    for i in 0..profile.len() {
        let a = profile[i];
        let b = profile[(i + 1) % profile.len()];
        let c = profile[(i + 2) % profile.len()];
        let turn = (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0]);
        if turn <= 1e-9 {
            return Err(refuse(
                "BREP_LOFT_SECTION_TOPOLOGY_REFUSED",
                "Section topology must be one simple strictly-convex counter-clockwise NURBS loop",
            ));
        }
        area2 += a[0] * b[1] - b[0] * a[1];
    }
    if area2 <= 2e-9 {
        return Err(refuse(
            "BREP_LOFT_SECTION_COLLAPSE_REFUSED",
            "Section area Bernstein lower bound is not positive",
        ));
    }
    Ok(area2 * 0.5)
}

/// Qualified exact finite successor for 2..16 explicitly corresponding,
/// simple convex degree-1 rational NURBS sections. Each adjacent pair authors
/// one rational bilinear Bezier side per edge; endpoint sections are capped.
pub fn audited_multi_section_loft(sections: &[Vec<[f64; 3]>]) -> Result<AuditedFeatureResult> {
    if sections.len() < 3 || sections.len() > 16 {
        return Err(refuse(
            "BREP_LOFT_RESOURCE_REFUSED",
            "Qualified multi-section loft requires 3..16 sections",
        ));
    }
    let count = sections[0].len();
    if !(3..=16).contains(&count) || sections.iter().any(|section| section.len() != count) {
        return Err(refuse(
            "BREP_LOFT_CORRESPONDENCE_REFUSED",
            "Exact positional section correspondence requires equal 3..16 control-vertex counts",
        ));
    }
    let areas = sections
        .iter()
        .map(|section| convex_section_area(section))
        .collect::<Result<Vec<_>>>()?;
    let first = &sections[0];
    let origin = first[0];
    let normal =
        unit3(cross3(sub3(first[1], origin), sub3(first[2], origin))).ok_or_else(|| {
            refuse(
                "BREP_LOFT_SECTION_COLLAPSE_REFUSED",
                "First section plane is singular",
            )
        })?;
    let mut prior = f64::NEG_INFINITY;
    for section in sections {
        let height = dot3(sub3(section[0], origin), normal);
        if height <= prior + 1e-7
            || section
                .iter()
                .any(|point| (dot3(sub3(*point, origin), normal) - height).abs() > 1e-7)
        {
            return Err(refuse(
                "BREP_LOFT_CORRESPONDENCE_REFUSED",
                "Qualified loft sections must remain parallel and strictly ordered; topology-changing or ambiguous correspondence is refused",
            ));
        }
        prior = height;
    }
    let result = crate::analytic::piecewise_ruled_loft(sections)?;
    certify_loft_sweep(
        result,
        EXACT_MULTI_SECTION_LOFT_CAPABILITY,
        vec![
            "exact_positional_section_correspondence",
            "degree1_rational_nurbs_sections",
            "rational_bilinear_bezier_side_spans",
            "positive_section_area_bernstein_bounds",
            "exact_c0_shared_span_boundaries",
            "closed_caps_sew_audit_and_global_self_intersection",
        ],
        areas.iter().copied().fold(0., f64::max).sqrt(),
    )
}

pub(super) fn segment_distance_squared(a: [f64; 3], b: [f64; 3], c: [f64; 3], d: [f64; 3]) -> f64 {
    let u = sub3(b, a);
    let v = sub3(d, c);
    let w = sub3(a, c);
    let aa = dot3(u, u);
    let bb = dot3(u, v);
    let cc = dot3(v, v);
    let dd = dot3(u, w);
    let ee = dot3(v, w);
    let denom = aa * cc - bb * bb;
    let (mut s, mut t) = if denom > 1e-18 {
        ((bb * ee - cc * dd) / denom, (aa * ee - bb * dd) / denom)
    } else {
        (0., if cc > 1e-18 { ee / cc } else { 0. })
    };
    s = s.clamp(0., 1.);
    t = t.clamp(0., 1.);
    let delta = sub3(
        [a[0] + s * u[0], a[1] + s * u[1], a[2] + s * u[2]],
        [c[0] + t * v[0], c[1] + t * v[1], c[2] + t * v[2]],
    );
    dot3(delta, delta)
}
