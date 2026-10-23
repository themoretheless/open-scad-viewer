//! Frozen contacts: preserves the original arithmetic and proof budgets.
use super::*;

#[cfg(test)]
pub(super) fn frozen_inspect_impl(
    model: &Model,
    tolerance_uv: f64,
    limits: Limits,
    hulls: &[crate::boundary_hull_contact::Certificate],
    caps: &[[usize; 3]],
    sweep_hulls: bool,
    joint_exact_domain: bool,
    prepared: Option<(Option<crate::face_contact_groups::Cover>, usize)>,
) -> Result<Report> {
    model.validate_boundary_diagnostic_inputs()?;
    if !(1..=100_000).contains(&limits.pairs)
        || !(1..=1_000_000).contains(&limits.cells)
        || !(1..=8_000_000).contains(&limits.domain_cells)
        || !(1..=100_000).contains(&limits.cells_per_pair)
        || !(1..=1_000_000).contains(&limits.domain_cells_per_pair)
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
    {
        return Err(Error::new(
            "BREP_CONTACT_BUDGET",
            "Use positive bounded pair, geometry and UV budgets and finite positive UV tolerance",
        ));
    }
    // Prepare each chart once, rather than validate/copy the model per pair.
    let domains = (0..model.faces.len())
        .map(|f| FaceDomain::new(model, f, tolerance_uv))
        .collect::<Result<Vec<_>>>()?;
    let source_volume_context = joint_exact_domain.then(|| source_volume_contact::Prepared::new(model));
    let n = model.faces.len();
    let total_pairs = n
        .checked_mul(n.saturating_sub(1))
        .ok_or_else(|| Error::new("BREP_CONTACT_BUDGET", "Too many face pairs"))?
        / 2;
    let mut out = Report {
        pairs: Vec::new(),
        disjoint_groups: Vec::new(),
        grouped_pairs: 0,
        group_cells: 0,
        total_pairs,
        next_pair: None,
        cells: 0,
        domain_cells: 0,
        all_pairs_disjoint: true,
        all_pairs_classified: true,
    };
    let cover = if sweep_hulls && n > 1 {
        let (cover, cells) = prepared
            .unwrap_or_else(|| crate::face_contact_groups::Cover::prepare(model, limits.cells));
        out.cells += cells;
        out.group_cells += cells;
        cover
    } else {
        None
    };
    for a in 0..n {
        let mut pending = cover.as_ref().map(|c| vec![c.root]).unwrap_or_default();
        let mut scalar_next = a + 1;
        loop {
            let b = if let Some(cover) = &cover {
                let Some(index) = pending.pop() else {
                    break;
                };
                let node = &cover.nodes[index];
                if node.range[1] <= a + 1 {
                    continue;
                }
                if node.range[0] > a && node.children.is_some() {
                    if out.cells == limits.cells {
                        out.next_pair = Some([a, node.range[0]]);
                        out.all_pairs_disjoint = false;
                        out.all_pairs_classified = false;
                        return Ok(out);
                    }
                    out.cells += 1;
                    out.group_cells += 1;
                    if let Some(certificate) = cover.certify(a, index) {
                        out.grouped_pairs += certificate.range[1] - certificate.range[0];
                        out.disjoint_groups.push(certificate);
                        continue;
                    }
                }
                if let Some([left, right]) = node.children {
                    pending.push(right);
                    pending.push(left);
                    continue;
                }
                node.range[0]
            } else {
                if scalar_next == n {
                    break;
                }
                let b = scalar_next;
                scalar_next += 1;
                b
            };
            if out.pairs.len() == limits.pairs
                || out.cells == limits.cells
                || out.domain_cells == limits.domain_cells
            {
                out.next_pair = Some([a, b]);
                out.all_pairs_disjoint = false;
                out.all_pairs_classified = false;
                return Ok(out);
            }
            let sa = &model.faces[a].surface;
            let sb = &model.faces[b].surface;
            let mut boundary = if let Some(c) = caps
                .iter()
                .find(|c| [c[0].min(c[1]), c[0].max(c[1])] == [a, b])
            {
                Some(SharedBoundary::SweepCap {
                    cap: c[0],
                    wall: c[1],
                    edge: c[2],
                })
            } else if let Some(c) = hulls.iter().find(|c| c.faces == [a, b]) {
                Some(SharedBoundary::ExactHull(c.clone()))
            } else if let Some(c) = joint_exact_domain
                .then(|| crate::boundary_hull_contact::certify(model, [a, b]))
                .flatten()
            {
                // Only the joint embedding audit authorizes this recomputation.
                // The same individual-pair limit bounds all such attempts,
                // including pairs beyond the old lexicographic prefix.
                Some(SharedBoundary::ExactHull(c))
            } else if let Some(c) = crate::shared_boundary::certify(model, [a, b]) {
                Some(SharedBoundary::PlanarFace(c))
            } else if let Some(c) = joint_exact_domain.then(|| crate::shared_boundary::bilinear_graph::certify(model, [a,b], 131072)).flatten() {
                Some(SharedBoundary::BilinearGraph(c))
            } else {
                crate::shared_boundary::certify_opposite(model, [a, b])?
                    .map(SharedBoundary::OppositeSides)
            };
            let mut boundary_cells = 0;
            if boundary.is_none() && sweep_hulls && joint_exact_domain {
                let allowance = (limits.cells - out.cells)
                    .min(limits.cells_per_pair)
                    .min(100000);
                let (certificate, used) =
                    crate::shared_boundary::certify_opposite_with_budget(model, [a, b], allowance);
                boundary_cells = used;
                out.cells += used;
                if let Some(c) = certificate {
                    boundary = Some(SharedBoundary::OppositeSides(c));
                }
            }
            let periodic = sa.periodic_u || sa.periodic_v || sb.periodic_u || sb.periodic_v;
            let pair_allowance = limits.cells_per_pair.saturating_sub(boundary_cells);
            let mut source_chart_cells=0;
            let mut volume_separation=None;
            // Quintic polygon corner charts need a source-volume proof: their
            // piecewise profile weights need not have one global chart map.
            // Try the exact four-wall cell before spending the pair budget on
            // a stitched surface. Other chart families keep their ordering.
            let volume_first=sa.degree_u==1 && sa.degree_v==5 && sb.degree_u==1 && sb.degree_v==5;
            if volume_first && boundary.is_none()&&!periodic&&joint_exact_domain&&out.cells<limits.cells {
                let separated=source_volume_context.as_ref().unwrap().certify([a,b],
                    (limits.cells-out.cells).min(pair_allowance))?;
                source_chart_cells=separated.cells;out.cells+=separated.cells;
                if let Some(c)=separated.certificate {
                    if c.shared_edge.is_some() {boundary=Some(SharedBoundary::SourceVolumeBoundary(c));}
                    else {volume_separation=Some(c);}
                }
            }
            if boundary.is_none()&&volume_separation.is_none()&&!periodic&&joint_exact_domain&&source_chart_cells<pair_allowance&&out.cells<limits.cells {
                let joined=boundary_stitched_contact::certify(model,[a,b],
                    (limits.cells-out.cells).min(pair_allowance-source_chart_cells))?;
                source_chart_cells+=joined.cells;out.cells+=joined.cells;
                boundary=joined.certificate.map(SharedBoundary::JoinedSourceCharts);
            }
            if !volume_first&&boundary.is_none()&&!periodic&&joint_exact_domain&&source_chart_cells<pair_allowance&&out.cells<limits.cells {
                let separated=source_volume_context.as_ref().unwrap().certify([a,b],
                    (limits.cells-out.cells).min(pair_allowance-source_chart_cells))?;
                source_chart_cells+=separated.cells;out.cells+=separated.cells;
                if let Some(c)=separated.certificate {
                    if c.shared_edge.is_some() {boundary=Some(SharedBoundary::SourceVolumeBoundary(c));}
                    else {volume_separation=Some(c);}
                }
            }

            // Exact control-hull separation is the fast path only for sweep
            // certification, including closed no-cap bodies: the plain face-contact
            // API keeps the frozen trim-search work accounting byte-for-byte.
            let (hull_disjoint, hull_cells) = if boundary.is_none() && volume_separation.is_none() && !periodic && (sweep_hulls || joint_exact_domain) {
                let allowance = (limits.cells - out.cells)
                    .min(limits.cells_per_pair.saturating_sub(boundary_cells + source_chart_cells) / 2)
                    .min(64);
                let cached = if allowance > 0 {
                    cover.as_ref().and_then(|c| c.pair_separated(a, b))
                } else {
                    None
                };
                match cached {
                    Some(true) => (true, 1),
                    Some(false) => {
                        let (separated, work) =
                            crate::control_hull_separation::inspect(sa, sb, allowance - 1);
                        (separated, work + 1)
                    }
                    None => crate::control_hull_separation::inspect(sa, sb, allowance),
                }
            } else {
                (false, 0)
            };
            out.cells += hull_cells;
            let result = if boundary.is_some()
                || hull_disjoint
                || volume_separation.is_some()
                || periodic
                || out.cells == limits.cells
                || boundary_cells + source_chart_cells + hull_cells == limits.cells_per_pair
            {
                None
            } else {
                let search = if sweep_hulls {
                    surface_contact_search::search_trimmed_with_control_hulls
                } else {
                    surface_contact_search::search_trimmed
                };
                Some(search(
                    sa,
                    sb,
                    [&domains[a].region, &domains[b].region],
                    (limits.cells - out.cells)
                        .min(limits.cells_per_pair - boundary_cells - source_chart_cells - hull_cells),
                    (limits.domain_cells - out.domain_cells).min(limits.domain_cells_per_pair),
                )?)
            };
            let reason = match &result {
                None if hull_disjoint || volume_separation.is_some() => "pair-disjoint",
                None if boundary.is_some() => "shared-boundary",
                None if sa.periodic_u || sa.periodic_v || sb.periodic_u || sb.periodic_v => {
                    "periodic-trim-not-supported"
                }
                None => "pair-unresolved",
                Some(r) if r.absence_proven => "pair-disjoint",
                Some(r) if r.contact.is_some() => "interior-contact",
                Some(_) => "pair-unresolved",
            };
            out.all_pairs_classified &= boundary.is_some()
                || hull_disjoint
                || volume_separation.is_some()
                || result.as_ref().is_some_and(|r| r.absence_proven);
            out.all_pairs_disjoint &=
                hull_disjoint || volume_separation.is_some() || result.as_ref().is_some_and(|r| r.absence_proven);
            if let Some(r) = &result {
                out.cells += r.cells;
                out.domain_cells += r.domain_cells;
            }
            out.pairs.push(Pair {
                faces: [a, b],
                result,
                boundary,
                hull_disjoint,
                hull_cells,
                boundary_cells,
                source_chart_cells,
                volume_separation,
                reason,
            });
        }
    }
    debug_assert_eq!(out.pairs.len() + out.grouped_pairs, out.total_pairs);
    Ok(out)
}
