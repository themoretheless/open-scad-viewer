use super::*;

#[derive(Debug)]
pub struct TurnSheet {
    pub model: Model,
    pub verification_cells: usize,
    pub max_endpoint_adjustment_upper_bound: f64,
}

/// Assemble the retained finite thread patches as one open connected network.
/// Adjacency comes from source patch roles; no nearest-edge matching is used.
/// Missing/disconnected or unresolved patches refuse instead of dropping faces.
pub fn clipped_thread_network(
    spec: nurbs_core::thread::Spec,
    z_limits: [f64; 2],
    max_uv_error: f64,
    tolerance: f64,
    max_spans: usize,
    max_curve_change: f64,
    max_verification_cells: usize,
) -> Result<TurnSheet> {
    if max_verification_cells > 10_000_000 {
        return Err(Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Network budget exceeds 10000000 cells",
        ));
    }
    let trims = nurbs_core::thread::trim_candidates(spec, z_limits, tolerance)?;
    if !trims.unresolved.is_empty() || trims.candidates.is_empty() || trims.candidates.len() > 254 {
        return Err(Error::new(
            "BREP_THREAD_PATCH_UNRESOLVED",
            "Network needs 1..254 retained patches without unresolved clipping",
        ));
    }
    let mut sheets = Vec::new();
    let mut used = 0;
    let mut adjustment: f64 = 0.;
    for c in &trims.candidates {
        let mut planar = false;
        for &z in &z_limits {
            planar |= !c.plane_boundary_indices(z)?.is_empty();
        }
        let remaining = (max_verification_cells - used).min(100000);
        if planar {
            let sheet = trimmed_patch_sheet_with_planar_ends(
                c,
                &z_limits,
                max_uv_error,
                tolerance,
                max_spans,
                max_curve_change,
                remaining,
            )?;
            used += sheet.verification_cells;
            adjustment = adjustment.max(sheet.max_curve_change_upper_bound);
            sheets.push(sheet.model);
        } else {
            let sheet = trimmed_patch_sheet(c, max_uv_error, tolerance, max_spans, remaining)?;
            used += sheet.verification_cells;
            sheets.push(sheet.model);
        }
    }
    let key = |p: &nurbs_core::thread::Patch| (p.start, p.turn, p.quarter, p.profile_segment);
    let neighbour = |p: &nurbs_core::thread::Patch, axis: usize| {
        if axis == 0 {
            if p.profile_segment < 4 {
                (p.start, p.turn, p.quarter, p.profile_segment + 1)
            } else if p.start + 1 < spec.starts {
                (p.start + 1, p.turn, p.quarter, 0)
            } else {
                (0, p.turn + 1, p.quarter, 0)
            }
        } else if p.quarter < 3 {
            (p.start, p.turn, p.quarter + 1, p.profile_segment)
        } else {
            (p.start, p.turn + 1, 0, p.profile_segment)
        }
    };
    let boundary = |m: &Model, wire: usize, axis: usize, value: f64| {
        m.loops[wire]
            .coedges
            .iter()
            .position(|c| c.pcurve.control_points.iter().all(|p| p[axis] == value))
    };
    let mut order = vec![0];
    let mut model = sheets[0].clone();
    while order.len() < sheets.len() {
        let mut next = None;
        for j in 0..sheets.len() {
            if order.contains(&j) {
                continue;
            }
            let mut pairs = Vec::new();
            for (wire, &i) in order.iter().enumerate() {
                for axis in 0..2 {
                    let p = &trims.candidates[i].patch;
                    let q = &trims.candidates[j].patch;
                    let values = if neighbour(p, axis) == key(q) {
                        Some((1., 0.))
                    } else if neighbour(q, axis) == key(p) {
                        Some((0., 1.))
                    } else {
                        None
                    };
                    if let Some((sv, tv)) = values {
                        if let (Some(s), Some(t)) = (
                            boundary(&model, wire, axis, sv),
                            boundary(&sheets[j], 0, axis, tv),
                        ) {
                            pairs.push(([wire, s], t));
                        }
                    }
                }
            }
            if !pairs.is_empty() {
                next = Some((j, pairs));
                break;
            }
        }
        let (j, pairs) = next.ok_or_else(|| {
            Error::new(
                "BREP_THREAD_PATCH_UNRESOLVED",
                "Retained patch network is disconnected",
            )
        })?;
        let assignments: Vec<_> = pairs
            .iter()
            .map(|&(s, t)| (t, &model.edges[model.loops[s[0]].coedges[s[1]].edge].curve))
            .collect();
        let aligned = align_patch_endpoints_many(
            &sheets[j],
            &assignments,
            max_curve_change,
            (max_verification_cells - used).min(100000),
        )?;
        used += aligned.verification_cells;
        adjustment = adjustment.max(aligned.max_curve_change_upper_bound);
        let (assembled, cells) = attach_patch_sheet_edges(
            &model,
            &aligned.model,
            &pairs,
            (max_verification_cells - used).min(100000),
        )?;
        used += cells;
        model = assembled;
        order.push(j);
    }
    Ok(TurnSheet {
        model,
        verification_cells: used,
        max_endpoint_adjustment_upper_bound: adjustment,
    })
}

/// Assemble all twenty original patches of one untrimmed thread turn/start.
/// The result is an open rectangular patch network, not a capped thread solid.
pub fn untrimmed_turn_sheet(
    spec: nurbs_core::thread::Spec,
    start: usize,
    turn: usize,
    max_uv_error: f64,
    tolerance: f64,
    max_spans: usize,
    max_endpoint_change: f64,
    max_verification_cells: usize,
) -> Result<TurnSheet> {
    if start >= spec.starts || turn >= spec.turns || max_verification_cells > 10_000_000 {
        return Err(Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Turn/start index or global verification budget invalid",
        ));
    }
    // trim_candidates validates Spec before authoring; the full axial range
    // retains all source patch rectangles without introducing clipping edges.
    let extent = spec.pitch * spec.starts as f64 * (spec.turns as f64 + 1.);
    let trims = nurbs_core::thread::trim_candidates(spec, [0., extent], tolerance)?;
    let mut model: Option<Model> = None;
    let mut used = 0;
    let mut adjustment: f64 = 0.;
    for q in 0..4 {
        for s in 0..5 {
            let candidate = trims
                .candidates
                .iter()
                .find(|c| {
                    c.patch.start == start
                        && c.patch.turn == turn
                        && c.patch.quarter == q
                        && c.patch.profile_segment == s
                })
                .ok_or_else(|| {
                    Error::new(
                        "BREP_THREAD_PATCH_UNRESOLVED",
                        "Full source patch rectangle is unavailable",
                    )
                })?;
            if candidate.exact_uv_vertices
                != vec![
                    nurbs_core::thread::ExactUvVertex::Corner { corner: 0 },
                    nurbs_core::thread::ExactUvVertex::Corner { corner: 1 },
                    nurbs_core::thread::ExactUvVertex::Corner { corner: 2 },
                    nurbs_core::thread::ExactUvVertex::Corner { corner: 3 },
                ]
            {
                return Err(Error::new(
                    "BREP_THREAD_PATCH_UNRESOLVED",
                    "Turn authoring requires untrimmed rectangles",
                ));
            }
            let sheet = trimmed_patch_sheet(
                candidate,
                max_uv_error,
                tolerance,
                max_spans,
                (max_verification_cells - used).min(100000),
            )?;
            used += sheet.verification_cells;
            let Some(source) = model.as_ref() else {
                model = Some(sheet.model);
                continue;
            };
            let mut assignments = Vec::new();
            if s > 0 {
                let wire = q * 5 + s - 1;
                assignments.push((3, &source.edges[source.loops[wire].coedges[1].edge].curve));
            }
            if q > 0 {
                let wire = (q - 1) * 5 + s;
                assignments.push((0, &source.edges[source.loops[wire].coedges[2].edge].curve));
            }
            let aligned = align_patch_endpoints_many(
                &sheet.model,
                &assignments,
                max_endpoint_change,
                (max_verification_cells - used).min(100000),
            )?;
            used += aligned.verification_cells;
            adjustment = adjustment.max(aligned.max_curve_change_upper_bound);
            let remaining = (max_verification_cells - used).min(100000);
            let (next, cells) = if s > 0 && q > 0 {
                attach_two_edges_impl(
                    source,
                    &aligned.model,
                    [[q * 5 + s - 1, 1], [0, 3]],
                    [[(q - 1) * 5 + s, 2], [0, 0]],
                    remaining,
                )?
            } else {
                let (wire, source_use, target_use) = if s > 0 {
                    (q * 5 + s - 1, 1, 3)
                } else {
                    ((q - 1) * 5 + s, 2, 0)
                };
                let (next, proof) = attach_patch_sheet(
                    source,
                    &aligned.model,
                    wire,
                    source_use,
                    target_use,
                    remaining,
                )?;
                (next, proof.verification_cells)
            };
            used += cells;
            model = Some(next);
        }
    }
    Ok(TurnSheet {
        model: model.unwrap(),
        verification_cells: used,
        max_endpoint_adjustment_upper_bound: adjustment,
    })
}
