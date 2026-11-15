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
/// Patch UV error, geometric tolerance and span cap shared by the thread
/// sheet and network builders.
#[derive(Clone, Copy, Debug)]
pub struct PatchFit {
    pub max_uv_error: f64,
    pub tolerance: f64,
    pub max_spans: usize,
}
pub fn clipped_thread_network(
    spec: nurbs_core::thread::Spec,
    z_limits: [f64; 2],
    fit: PatchFit,
    max_curve_change: f64,
    max_verification_cells: usize,
) -> Result<TurnSheet> {
    if max_verification_cells > MAX_NETWORK_CELLS {
        return Err(Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Network budget exceeds 10000000 cells",
        ));
    }
    let trims = nurbs_core::thread::trim_candidates(spec, z_limits, fit.tolerance)?;
    assemble_clipped_network(
        &trims,
        spec.starts,
        z_limits,
        fit,
        max_curve_change,
        max_verification_cells,
    )
}

/// Profile segments and quarters of one untrimmed turn/start.
const TURN_SEGMENTS: usize = 5;
const TURN_QUARTERS: usize = 4;

/// The untrimmed source rectangle of one turn/start patch.
fn untrimmed_turn_candidate(
    trims: &nurbs_core::thread::TrimCandidates,
    start: usize,
    turn: usize,
    q: usize,
    s: usize,
) -> Result<&TrimCandidate> {
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
    Ok(candidate)
}

/// Attach the aligned sheet at quarter `q`, segment `s` to its profile
/// predecessor and/or its quarter predecessor in `source`.
fn attach_turn_sheet(
    source: &Model,
    sheet: &Model,
    q: usize,
    s: usize,
    remaining: usize,
) -> Result<(Model, usize)> {
    if s > 0 && q > 0 {
        return attach_two_edges_impl(
            source,
            sheet,
            [[q * TURN_SEGMENTS + s - 1, 1], [0, 3]],
            [[(q - 1) * TURN_SEGMENTS + s, 2], [0, 0]],
            remaining,
        );
    }
    let (wire, source_use, target_use) = if s > 0 {
        (q * TURN_SEGMENTS + s - 1, 1, 3)
    } else {
        ((q - 1) * TURN_SEGMENTS + s, 2, 0)
    };
    let (next, proof) = attach_patch_sheet(source, sheet, wire, source_use, target_use, remaining)?;
    Ok((next, proof.verification_cells))
}

/// Assemble all twenty original patches of one untrimmed thread turn/start.
/// The result is an open rectangular patch network, not a capped thread solid.
pub fn untrimmed_turn_sheet(
    spec: nurbs_core::thread::Spec,
    start: usize,
    turn: usize,
    fit: PatchFit,
    max_endpoint_change: f64,
    max_verification_cells: usize,
) -> Result<TurnSheet> {
    let PatchFit {
        max_uv_error,
        tolerance,
        max_spans,
    } = fit;
    if start >= spec.starts || turn >= spec.turns || max_verification_cells > MAX_NETWORK_CELLS {
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
    let remaining = |used: usize| (max_verification_cells - used).min(MAX_STEP_CELLS);
    for q in 0..TURN_QUARTERS {
        for s in 0..TURN_SEGMENTS {
            let candidate = untrimmed_turn_candidate(&trims, start, turn, q, s)?;
            let sheet = trimmed_patch_sheet(
                candidate,
                max_uv_error,
                tolerance,
                max_spans,
                remaining(used),
            )?;
            used += sheet.verification_cells;
            let Some(source) = model.as_ref() else {
                model = Some(sheet.model);
                continue;
            };
            let mut assignments = Vec::new();
            if s > 0 {
                let wire = q * TURN_SEGMENTS + s - 1;
                assignments.push((3, &source.edges[source.loops[wire].coedges[1].edge].curve));
            }
            if q > 0 {
                let wire = (q - 1) * TURN_SEGMENTS + s;
                assignments.push((0, &source.edges[source.loops[wire].coedges[2].edge].curve));
            }
            let aligned = align_patch_endpoints_many(
                &sheet.model,
                &assignments,
                max_endpoint_change,
                remaining(used),
            )?;
            used += aligned.verification_cells;
            adjustment = adjustment.max(aligned.max_curve_change_upper_bound);
            let (next, cells) = attach_turn_sheet(source, &aligned.model, q, s, remaining(used))?;
            used += cells;
            model = Some(next);
        }
    }
    let model = model.ok_or_else(|| {
        Error::new(
            "BREP_THREAD_PATCH_UNRESOLVED",
            "Full source patch rectangle is unavailable",
        )
    })?;
    Ok(TurnSheet {
        model,
        verification_cells: used,
        max_endpoint_adjustment_upper_bound: adjustment,
    })
}
