use super::*;

/// AF-01 fillet across a chain of vertical cuboid edges.
/// Cutters are authored from the original solid (durable XY), then applied as one
/// compound difference so intermediate non-cuboid solids never re-enter AF-01.
pub fn analytic_fillet_chain(
    model: &Model,
    edges: &[usize],
    radius: f64,
) -> Result<(Model, FeatureCertificate)> {
    if edges.is_empty() {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Fillet chain requires at least one edge",
        ));
    }
    if edges.len() > 8 {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Fillet chain exceeds 8-edge budget",
        ));
    }
    if !(radius.is_finite() && radius > 0.) {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Fillet radius must be finite and positive",
        ));
    }
    if !is_axis_aligned_cuboid(model) {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "analytic-fillet/1 admits axis-aligned planar cuboids only (AF-N1)",
        ));
    }
    let (min, max) = model_bounds(model);
    if max[0] - min[0] <= 2. * radius || max[1] - min[1] <= 2. * radius {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Cuboid extents are too small for the fillet chain radius",
        ));
    }
    let mut rounded = [false; 4];
    let mut seen = std::collections::BTreeSet::new();
    for &e in edges {
        let xy = vertical_edge_xy(model, e).ok_or_else(|| {
            refuse(
                "BREP_ANALYTIC_FILLET_REFUSED",
                "Fillet chain admits vertical cuboid edges only",
            )
        })?;
        let key = ((xy[0] * 1e9).round() as i64, (xy[1] * 1e9).round() as i64);
        if !seen.insert(key) {
            continue;
        }
        let idx = remap_vertical_edge(model, xy, radius.max(1e-6))?;
        let edge_ref = &model.edges[idx];
        let a = model.vertices[edge_ref.vertices[0]].point;
        let b = model.vertices[edge_ref.vertices[1]].point;
        let height = (a[2] - b[2]).abs();
        if !(height.is_finite() && height > radius * 2.) {
            return Err(refuse(
                "BREP_ANALYTIC_FILLET_REFUSED",
                "Edge too short for the requested fillet radius",
            ));
        }
        let x = (a[0] + b[0]) * 0.5;
        let y = (a[1] + b[1]) * 0.5;
        let at_max_x = (x - max[0]).abs() <= (x - min[0]).abs();
        let at_max_y = (y - max[1]).abs() <= (y - min[1]).abs();
        let corner = match (at_max_x, at_max_y) {
            (false, false) => 0,
            (true, false) => 1,
            (true, true) => 2,
            (false, true) => 3,
        };
        rounded[corner] = true;
    }
    if !rounded.iter().any(|rounded| *rounded) {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Fillet chain selected no distinct vertical corners",
        ));
    }
    let result =
        crate::imprint_pipeline::rounded_cuboid_vertical_edges(model, min, max, rounded, radius)?;
    result.validate()?;
    Ok((
        result,
        FeatureCertificate {
            capability: "analytic-fillet/1",
            complete: true,
            notes: vec!["af01_fillet_chain_remapped", "af01_exact_arc_profile"],
        },
    ))
}

/// Audited successor wrapper around the exact multi-edge fillet author.
/// The result is published only after context-bound correspondence evidence,
/// global solid audit, and persistent naming/ChangeSet evidence agree.
pub fn audited_multi_edge_fillet(
    model: &Model,
    edges: &[usize],
    radius: f64,
) -> Result<AuditedFeatureResult> {
    let (result, mut feature) = analytic_fillet_chain(model, edges, radius)?;
    feature.capability = AUDITED_MULTI_EDGE_FILLET_CAPABILITY;
    let context = result.tolerance_context()?;
    let evidence = compose_predicate_evidence(
        &context,
        [
            PredicateEvidence::correspondence(&context, 0., radius)?,
            PredicateEvidence::topology_preservation(
                &context,
                "multi-edge fillet authored shared-edge incidence",
                result.persistent_naming_complete() && !result.1.faces.is_empty(),
            )?,
        ],
    )?;
    let audit = audit_solid(&result)?;
    let naming_complete = result.persistent_naming_complete()
        && result.1.faces.len() == result.faces.len()
        && result.1.edges.len() == result.edges.len();
    if !naming_complete {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Audited multi-edge fillet lacks complete ChangeSet/naming evidence",
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
pub fn analytic_chamfer(
    model: &Model,
    edge: usize,
    distance: f64,
) -> Result<(Model, FeatureCertificate)> {
    model.validate()?;
    if !(distance.is_finite() && distance > 0.) {
        return Err(refuse(
            "BREP_ANALYTIC_CHAMFER_REFUSED",
            "Chamfer distance must be finite and positive",
        ));
    }
    if !is_axis_aligned_cuboid(model) {
        return Err(refuse(
            "BREP_ANALYTIC_CHAMFER_REFUSED",
            "analytic-chamfer/1 admits axis-aligned planar cuboids only",
        ));
    }
    if edge >= model.edges.len() {
        return Err(refuse(
            "BREP_ANALYTIC_CHAMFER_REFUSED",
            "Edge index out of range",
        ));
    }
    let edge_ref = &model.edges[edge];
    if edge_ref.curve.degree != 1 {
        return Err(refuse(
            "BREP_ANALYTIC_CHAMFER_REFUSED",
            "Curved edges are outside analytic-chamfer/1",
        ));
    }
    let a = model.vertices[edge_ref.vertices[0]].point;
    let b = model.vertices[edge_ref.vertices[1]].point;
    let dir = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let len = dir[0].hypot(dir[1]).hypot(dir[2]);
    if !(len.is_finite() && len > distance * 2.) {
        return Err(refuse(
            "BREP_ANALYTIC_CHAMFER_REFUSED",
            "Edge too short for the requested chamfer distance",
        ));
    }
    let (min, max) = model_bounds(model);
    let span = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
    if span.iter().any(|s| *s <= distance * 2.) {
        return Err(refuse(
            "BREP_ANALYTIC_CHAMFER_REFUSED",
            "Cuboid extents too small for chamfer distance",
        ));
    }
    let vertical = dir[0].abs() <= 1e-12 && dir[1].abs() <= 1e-12 && dir[2].abs() > 1e-12;
    if !vertical {
        return Err(refuse(
            "BREP_ANALYTIC_CHAMFER_REFUSED",
            "AF-01 walking slice admits vertical (+Z) cuboid edges only",
        ));
    }
    let z0 = a[2].min(b[2]);
    let height = len;
    let x = (a[0] + b[0]) * 0.5;
    let y = (a[1] + b[1]) * 0.5;
    let sx = if (x - max[0]).abs() <= 1e-9 {
        1.
    } else if (x - min[0]).abs() <= 1e-9 {
        -1.
    } else {
        return Err(refuse(
            "BREP_ANALYTIC_CHAMFER_REFUSED",
            "Selected edge is not on a cuboid corner",
        ));
    };
    let sy = if (y - max[1]).abs() <= 1e-9 {
        1.
    } else if (y - min[1]).abs() <= 1e-9 {
        -1.
    } else {
        return Err(refuse(
            "BREP_ANALYTIC_CHAMFER_REFUSED",
            "Selected edge is not on a cuboid corner",
        ));
    };
    // Right-triangular cutter at the selected corner. Keep its profile CCW.
    let mut profile = [[x - sx * distance, y], [x, y - sy * distance], [x, y]];
    let area2 = (profile[1][0] - profile[0][0]) * (profile[2][1] - profile[0][1])
        - (profile[1][1] - profile[0][1]) * (profile[2][0] - profile[0][0]);
    if area2 < 0. {
        profile.swap(0, 1);
    }
    let cutter = extrude_polygon(&profile, z0, z0 + height)?;
    let result = boolean(model, &cutter, "difference")?;
    result.validate()?;
    // Chamfer faces remain planar (degree 1). Faceted mesh bevels must not be claimed here.
    if result
        .faces
        .iter()
        .any(|f| f.surface.degree_u > 1 || f.surface.degree_v > 1)
    {
        return Err(refuse(
            "BREP_ANALYTIC_CHAMFER_REFUSED",
            "Unexpected curved face in planar chamfer result",
        ));
    }
    Ok((
        result,
        FeatureCertificate {
            capability: "analytic-chamfer/1",
            complete: true,
            notes: vec!["af01_planar_chamfer_edge", "not_mesh_bevel"],
        },
    ))
}

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
    fn norm3(v: [f64; 3]) -> f64 {
        v[0].hypot(v[1]).hypot(v[2])
    }
    fn unit3(v: [f64; 3]) -> Option<[f64; 3]> {
        let n = norm3(v);
        if n <= 1e-15 {
            None
        } else {
            Some([v[0] / n, v[1] / n, v[2] / n])
        }
    }
    fn cross3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    }
    fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
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
