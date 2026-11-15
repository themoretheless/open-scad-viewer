use super::*;

/// Cap one exact planar boundary cycle of an open patch network. Orientation
/// is selected to oppose the side uses; curves are retained, not fitted again.
/// Leaves shell/body solid qualification separate.
pub fn cap_patch_network_plane(
    source: &Model,
    z_plane: f64,
    max_verification_cells: usize,
) -> Result<(Model, usize)> {
    cap_patch_network_impl(source, z_plane, max_verification_cells, None)
}
/// Cap a boundary containing general NURBS curves, with explicit trim audit budgets.
pub fn cap_patch_network_plane_general(
    source: &Model,
    z_plane: f64,
    max_trim_pairs: usize,
    max_trim_cells: usize,
    max_domain_cells: usize,
    max_verification_cells: usize,
) -> Result<(Model, usize)> {
    cap_patch_network_impl(
        source,
        z_plane,
        max_verification_cells,
        Some([max_trim_pairs, max_trim_cells, max_domain_cells]),
    )
}
pub(super) fn cap_patch_network_impl(
    source: &Model,
    z_plane: f64,
    max_verification_cells: usize,
    trim_limits: Option<[usize; 3]>,
) -> Result<(Model, usize)> {
    source.validate()?;
    if !z_plane.is_finite() || max_verification_cells > 100000 {
        return Err(Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Cap needs finite plane and at most 100000 cells",
        ));
    }
    let mut incidence = vec![0; source.edges.len()];
    for c in source.loops.iter().flat_map(|l| &l.coedges) {
        incidence[c.edge] += 1;
    }
    let mut addresses = Vec::new();
    let mut curves = Vec::new();
    for (wire, l) in source.loops.iter().enumerate() {
        for (index, c) in l.coedges.iter().enumerate() {
            let edge = &source.edges[c.edge];
            if incidence[c.edge] == 1
                && !c.reversed
                && edge.curve.control_points.iter().all(|p| p[2] == z_plane)
            {
                addresses.push([wire, index]);
                curves.push(edge.curve.clone());
            }
        }
    }
    if curves.is_empty() {
        return Err(Error::new(
            "BREP_THREAD_PATCH_UNRESOLVED",
            "No exact planar cap boundary",
        ));
    }
    let chain = nurbs_core::curve_chain::assemble(&curves, source.tolerance_mm, 65536)?;
    if !chain.all_endpoints_equal
        || chain.cyclic_chains.len() != 1
        || !chain.unresolved_curves.is_empty()
    {
        return Err(Error::new(
            "BREP_THREAD_PATCH_UNRESOLVED",
            "Cap requires one exactly closed boundary cycle",
        ));
    }
    let cycle = &chain.cyclic_chains[0];
    let side_wire: Vec<_> = cycle.iter().map(|&i| curves[i].clone()).collect();
    let (mut cap, mut used, side_positive) = if let Some([pairs, cells, domain]) = trim_limits {
        let report = crate::planar_cap::build_sheet_general_oriented(
            &[side_wire],
            z_plane,
            source.tolerance_mm,
            crate::planar_cap::GeneralCapLimits {
                max_trim_pairs: pairs,
                max_trim_cells: cells,
                max_domain_cells: domain,
                max_agreement_cells: max_verification_cells,
            },
        )?;
        let cap = report.model.ok_or_else(|| {
            Error::new(
                "BREP_THREAD_PATCH_UNRESOLVED",
                "General cap trim region is unproved",
            )
        })?;
        (cap, report.agreement_cells, !report.orientation_reversed)
    } else {
        let area: f64 = side_wire
            .iter()
            .flat_map(|c| c.control_points.windows(2))
            .map(|p| p[0][0] * p[1][1] - p[1][0] * p[0][1])
            .sum();
        if !area.is_finite() || area == 0. {
            return Err(Error::new(
                "BREP_THREAD_PATCH_UNRESOLVED",
                "Cap orientation is unresolved",
            ));
        }
        let wire = if area > 0. {
            side_wire
        } else {
            side_wire
                .iter()
                .rev()
                .map(|c| c.reverse())
                .collect::<Result<Vec<_>>>()?
        };
        let (cap, cells) = crate::planar_cap::build_sheet_with_report(
            &[wire],
            z_plane,
            source.tolerance_mm,
            max_verification_cells,
        )?;
        (cap, cells, area > 0.)
    };
    if side_positive {
        let cap_surface=&cap.faces[0].surface;
        let u_ends=[cap_surface.knots_u[cap_surface.degree_u],cap_surface.knots_u[cap_surface.control_points.len()]];
        cap.faces[0].surface.control_points.reverse();
        cap.faces[0].surface.weights.reverse();
        for e in &mut cap.edges {
            e.curve = e.curve.reverse()?;
            e.vertices.swap(0, 1);
        }
        cap.loops[0].coedges.reverse();
        for c in &mut cap.loops[0].coedges {
            c.pcurve = c.pcurve.reverse()?;
            for p in &mut c.pcurve.control_points {
                p[0] = u_ends[0] + (u_ends[1] - p[0]);
            }
        }
        cap.rebuild_topology_ids();
        cap.validate()?;
    }
    let pairs: Vec<_> = cycle
        .iter()
        .rev()
        .enumerate()
        .map(|(target, &i)| (addresses[i], target))
        .collect();
    let (result, cells) =
        attach_patch_sheet_edges(source, &cap, &pairs, max_verification_cells - used)?;
    used += cells;
    Ok((result, used))
}

/// Author a clipped patch with exact planar end boundaries. Symbolic plane
/// incidence chooses edges; continuous agreement qualifies the stored geometry.
pub fn trimmed_patch_sheet_with_planar_ends(
    candidate: &TrimCandidate,
    z_planes: &[f64],
    max_uv_error: f64,
    tolerance: f64,
    max_spans: usize,
    max_curve_change: f64,
    max_verification_cells: usize,
) -> Result<EndpointAlignment> {
    if z_planes.is_empty() || z_planes.len() > 2 || max_verification_cells > 100000 {
        return Err(Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Planar ends need one or two planes and at most 100000 cells",
        ));
    }
    let mut planes = Vec::new();
    for &z in z_planes {
        for i in candidate.plane_boundary_indices(z)? {
            planes.push((i, z));
        }
    }
    if planes.is_empty() {
        return Err(Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Patch has no boundary on the requested planes",
        ));
    }
    let sheet = trimmed_patch_sheet(
        candidate,
        max_uv_error,
        tolerance,
        max_spans,
        max_verification_cells,
    )?;
    let mut aligned = planarize_patch_edges(
        &sheet.model,
        &planes,
        max_curve_change,
        max_verification_cells - sheet.verification_cells,
    )?;
    aligned.verification_cells += sheet.verification_cells;
    Ok(aligned)
}

fn invalid_planarization() -> Error {
    Error::new(
        "BREP_INVALID_THREAD_PATCH",
        "Planarization requires a clamped single-face sheet and compatible finite planes",
    )
}

/// Requested Z of every touched vertex and edge.
type PlaneAssignments = (
    std::collections::BTreeMap<usize, f64>,
    std::collections::BTreeMap<usize, f64>,
);

/// Validate the sheet and collect one compatible plane per touched vertex
/// and edge.
fn collect_plane_assignments(
    target: &Model,
    planes: &[(usize, f64)],
    max_change: f64,
    max_verification_cells: usize,
) -> Result<PlaneAssignments> {
    target.validate()?;
    if target.faces.len() != 1
        || target.loops.len() != 1
        || target.shells.len() != 1
        || target.shells[0].closed
        || !target.bodies.is_empty()
        || planes.is_empty()
        || planes.len() > target.edges.len()
        || !max_change.is_finite()
        || max_change < 0.
        || max_verification_cells > MAX_ALIGNMENT_CELLS
    {
        return Err(invalid_planarization());
    }
    let mut vertices = std::collections::BTreeMap::new();
    let mut edges = std::collections::BTreeMap::new();
    for &(index, z) in planes {
        if !z.is_finite() {
            return Err(invalid_planarization());
        }
        let c = target.loops[0]
            .coedges
            .get(index)
            .ok_or_else(invalid_planarization)?;
        if let Some(previous) = edges.insert(c.edge, z)
            && previous != z
        {
            return Err(invalid_planarization());
        }
        for v in target.edges[c.edge].vertices {
            if let Some(previous) = vertices.insert(v, z)
                && previous != z
            {
                return Err(invalid_planarization());
            }
        }
    }
    for e in &target.edges {
        let c = &e.curve;
        if !clamped_spatial(c) {
            return Err(invalid_planarization());
        }
        let first = c.control_points.first().ok_or_else(invalid_planarization)?;
        let last = c.control_points.last().ok_or_else(invalid_planarization)?;
        if first.as_slice() != target.vertices[e.vertices[0]].point.as_slice()
            || last.as_slice() != target.vertices[e.vertices[1]].point.as_slice()
        {
            return Err(invalid_planarization());
        }
    }
    Ok((vertices, edges))
}

/// Move the assigned vertices and controls onto their planes and return the
/// outward-rounded largest Z-control displacement.
fn apply_plane_assignments(
    target: &Model,
    result: &mut Model,
    (vertices, edges): &PlaneAssignments,
) -> f64 {
    for (&v, &z) in vertices {
        result.vertices[v].point[2] = z;
    }
    let mut bound: f64 = 0.;
    for (index, e) in result.0.edges.iter_mut().enumerate() {
        if let Some(&z) = edges.get(&index) {
            for p in &mut e.curve.control_points {
                p[2] = z;
            }
        }
        if let Some(&z) = vertices.get(&e.vertices[0]) {
            e.curve.control_points[0][2] = z;
        }
        if let Some(&z) = vertices.get(&e.vertices[1])
            && let Some(last) = e.curve.control_points.last_mut()
        {
            last[2] = z;
        }
        for (p, original) in e
            .curve
            .control_points
            .iter()
            .zip(&target.edges[index].curve.control_points)
        {
            if p[2] != original[2] {
                bound = bound.max((p[2] - original[2]).abs().next_up());
            }
        }
    }
    bound
}

/// Put selected boundary curves exactly on Z planes, including their shared
/// vertices and incident endpoint controls. Positive rational weights bound
/// continuous curve change by the maximum outward Z-control displacement.
pub fn planarize_patch_edges(
    target: &Model,
    planes: &[(usize, f64)],
    max_change: f64,
    max_verification_cells: usize,
) -> Result<EndpointAlignment> {
    let assignments =
        collect_plane_assignments(target, planes, max_change, max_verification_cells)?;
    let mut result = target.clone();
    let bound = apply_plane_assignments(target, &mut result, &assignments);
    if !bound.is_finite() || bound > max_change {
        return Err(Error::new(
            "BREP_THREAD_PATCH_UNRESOLVED",
            "Planar edge displacement exceeds requested budget",
        ));
    }
    let (agreements, used) = verify_edited_boundaries(
        &result,
        max_verification_cells,
        BoundaryRecheckMessages {
            exhausted: "Planarization verifier budget exhausted",
            unproved: "Planarized boundary agreement unproved",
        },
    )?;
    result.rebuild_topology_ids();
    result.inherit_topology_ids(&[target]);
    result.validate()?;
    Ok(EndpointAlignment {
        model: result,
        max_curve_change_upper_bound: bound,
        agreements,
        verification_cells: used,
    })
}
