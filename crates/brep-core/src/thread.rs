//! Open trimmed thread patch faces. Shell sewing and ideal-thread error remain
//! separate qualifications; this authoring preserves the stored patch surface.
use crate::{Coedge, Edge, Face, FaceUse, Loop, Model, Shell, Vertex};
use nurbs_core::{Error, Result, curve_surface_agreement::Report, thread::TrimCandidate};

#[derive(Debug)]
pub struct PatchSheet {
    pub model: Model,
    pub agreements: Vec<Report>,
    pub verification_cells: usize,
}

#[derive(Debug)]
pub struct EndpointAlignment {
    pub model: Model,
    /// Continuous change bound for every edited rational curve, from positive
    /// partition of unity and outward bounds on endpoint-control movement.
    pub max_curve_change_upper_bound: f64,
    pub agreements: Vec<Report>,
    pub verification_cells: usize,
}

#[derive(Debug)]
pub struct ClosedBodyCandidate {
    pub model: Model,
    pub mass: crate::analysis::MassProperties,
    pub orientation_reversed: bool,
    pub evaluations: usize,
    /// Quadrature and combinatorial closure do not certify embedded geometry.
    pub solid_geometry_certified: bool,
}

pub struct CandidateQualification {
    pub continuous_agreement: crate::boundary_agreement::Report,
    pub volume: crate::volume_validity::Report,
    /// Requires all boundary embedding, nesting and outward-orientation stages.
    pub solid_geometry_certified: bool,
}
/// Recompute independent geometric qualification. Numerical volume and cached
/// construction reports never authorize a solid certificate on their own.
pub fn qualify_closed_candidate(
    candidate: &ClosedBodyCandidate,
    tolerance_uv: f64,
    max_agreement_cells: usize,
    limits: crate::volume_validity::Limits,
) -> Result<CandidateQualification> {
    let agreement = crate::boundary_agreement::verify(&candidate.model, max_agreement_cells)?;
    let volume = crate::volume_validity::inspect(&candidate.model, tolerance_uv, limits)?;
    let certified = agreement.complete && volume.proven;
    Ok(CandidateQualification {
        continuous_agreement: agreement,
        volume,
        solid_geometry_certified: certified,
    })
}

/// Admit combinatorial closure, orient by the highest planar cap normal,
/// then require positive numerical signed volume.
/// Returns an explicitly unqualified body candidate; independent embedding,
/// outwardness and rounding-inclusive accuracy are still required.
pub fn closed_body_candidate(
    source: &Model,
    relative_tolerance: f64,
    max_evaluations: usize,
) -> Result<ClosedBodyCandidate> {
    let report = source.validate()?;
    if source.shells.len() != 1
        || !source.bodies.is_empty()
        || report.boundary_edge_count != 0
        || source.shells[0].faces.len() != source.faces.len()
    {
        return Err(Error::new(
            "BREP_THREAD_PATCH_UNRESOLVED",
            "Candidate needs one fully sewn shell without existing bodies",
        ));
    }
    let mut model = source.clone();
    model.shells[0].closed = true;
    model.bodies.push(crate::Body {
        outer_shell: 0,
        inner_shells: vec![],
    });
    model.rebuild_topology_ids();
    model.validate()?;
    let (face, _) = model
        .faces
        .iter()
        .enumerate()
        .filter_map(|(i, f)| {
            let z = f.surface.control_points[0][0][2];
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == z)
                .then_some((i, z))
        })
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .ok_or_else(|| {
            Error::new(
                "BREP_THREAD_PATCH_UNRESOLVED",
                "Candidate needs a planar top-cap orientation witness",
            )
        })?;
    let surface = &model.faces[face].surface;
    let domain = [
        [
            surface.knots_u[surface.degree_u],
            surface.knots_u[surface.control_points.len()],
        ],
        [
            surface.knots_v[surface.degree_v],
            surface.knots_v[surface.control_points[0].len()],
        ],
    ];
    let mut normal = nurbs_core::surface_injectivity::normal_direction_bounds(
        surface,
        domain,
        [0., 0., 1.],
        100,
    )?
    .ok_or_else(|| {
        Error::new(
            "BREP_THREAD_PATCH_UNRESOLVED",
            "Top-cap normal direction is unresolved",
        )
    })?;
    if model.shells[0]
        .faces
        .iter()
        .find(|u| u.face == face)
        .unwrap()
        .reversed
    {
        normal = [-normal[1], -normal[0]];
    }
    let reversed = if normal[1] < 0. {
        true
    } else if normal[0] > 0. {
        false
    } else {
        return Err(Error::new(
            "BREP_THREAD_PATCH_UNRESOLVED",
            "Top-cap normal sign is unresolved",
        ));
    };
    if reversed {
        for use_ in &mut model.shells[0].faces {
            use_.reversed = !use_.reversed;
        }
        model.rebuild_topology_ids();
        model.validate()?;
    }
    let mass = crate::analysis::mass_properties(&model, relative_tolerance, max_evaluations)?;
    let evaluations = mass.evaluations;
    if mass.signed_volume_mm3 <= mass.volume_error_estimate_mm3 {
        return Err(Error::new(
            "BREP_THREAD_PATCH_UNRESOLVED",
            "Positive numerical volume not established",
        ));
    }
    Ok(ClosedBodyCandidate {
        model,
        mass,
        orientation_reversed: reversed,
        evaluations,
        solid_geometry_certified: false,
    })
}

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
fn cap_patch_network_impl(
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
            pairs,
            cells,
            domain,
            max_verification_cells,
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
                p[0] = 1. - p[0];
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

/// Put selected boundary curves exactly on Z planes, including their shared
/// vertices and incident endpoint controls. Positive rational weights bound
/// continuous curve change by the maximum outward Z-control displacement.
pub fn planarize_patch_edges(
    target: &Model,
    planes: &[(usize, f64)],
    max_change: f64,
    max_verification_cells: usize,
) -> Result<EndpointAlignment> {
    use nurbs_core::curve_surface_agreement::{self, Status};
    let invalid = || {
        Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Planarization requires a clamped single-face sheet and compatible finite planes",
        )
    };
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
        || max_verification_cells > 100000
    {
        return Err(invalid());
    }
    let mut vertices = std::collections::BTreeMap::new();
    let mut edges = std::collections::BTreeMap::new();
    for &(index, z) in planes {
        if !z.is_finite() {
            return Err(invalid());
        }
        let c = target.loops[0].coedges.get(index).ok_or_else(invalid)?;
        if let Some(previous) = edges.insert(c.edge, z) {
            if previous != z {
                return Err(invalid());
            }
        }
        for v in target.edges[c.edge].vertices {
            if let Some(previous) = vertices.insert(v, z) {
                if previous != z {
                    return Err(invalid());
                }
            }
        }
    }
    for e in &target.edges {
        let c = &e.curve;
        let [a, b] = c.domain();
        if c.control_points[0].len() != 3
            || !c.knots[..=c.degree].iter().all(|&k| k == a)
            || !c.knots[c.knots.len() - c.degree - 1..]
                .iter()
                .all(|&k| k == b)
            || c.control_points.first().unwrap().as_slice()
                != target.vertices[e.vertices[0]].point.as_slice()
            || c.control_points.last().unwrap().as_slice()
                != target.vertices[e.vertices[1]].point.as_slice()
        {
            return Err(invalid());
        }
    }
    let mut result = target.clone();
    for (&v, &z) in &vertices {
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
        if let Some(&z) = vertices.get(&e.vertices[1]) {
            e.curve.control_points.last_mut().unwrap()[2] = z;
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
    if !bound.is_finite() || bound > max_change {
        return Err(Error::new(
            "BREP_THREAD_PATCH_UNRESOLVED",
            "Planar edge displacement exceeds requested budget",
        ));
    }
    let mut used = 0;
    let mut agreements = Vec::new();
    for c in &result.loops[0].coedges {
        if used == max_verification_cells {
            return Err(Error::new(
                "BREP_THREAD_PATCH_UNRESOLVED",
                "Planarization verifier budget exhausted",
            ));
        }
        let report = curve_surface_agreement::verify(
            &result.edges[c.edge].curve,
            &c.pcurve,
            &result.faces[0].surface,
            c.reversed,
            result.tolerance_mm,
            max_verification_cells - used,
        )?;
        used += report.cells;
        if report.status != Status::WithinTolerance {
            return Err(Error::new(
                "BREP_THREAD_PATCH_UNRESOLVED",
                "Planarized boundary agreement unproved",
            ));
        }
        agreements.push(report);
    }
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

/// Match a sheet boundary's endpoints to an immutable source curve, changing
/// incident endpoint controls only. Rechecks every resulting boundary use.
pub fn align_patch_endpoints(
    target: &Model,
    target_coedge: usize,
    source_curve: &nurbs_core::curve::Curve,
    max_change: f64,
    max_verification_cells: usize,
) -> Result<EndpointAlignment> {
    align_patch_endpoints_many(
        target,
        &[(target_coedge, source_curve)],
        max_change,
        max_verification_cells,
    )
}
/// Agree on all endpoint assignments before editing. Conflicting assignments
/// to the same vertex refuse, even when their difference is below tolerance.
pub fn align_patch_endpoints_many(
    target: &Model,
    boundaries: &[(usize, &nurbs_core::curve::Curve)],
    max_change: f64,
    max_verification_cells: usize,
) -> Result<EndpointAlignment> {
    use nurbs_core::curve_surface_agreement::{self, Status};
    let invalid = || {
        Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Endpoint alignment requires a clamped single-face sheet and finite movement budget",
        )
    };
    target.validate()?;
    if boundaries.is_empty() || boundaries.len() > target.edges.len() {
        return Err(invalid());
    }
    for (_, curve) in boundaries {
        curve.validate()?;
    }
    if !max_change.is_finite()
        || max_change < 0.
        || max_verification_cells > 100000
        || target.faces.len() != 1
        || target.loops.len() != 1
        || target.shells.len() != 1
        || target.shells[0].closed
        || !target.bodies.is_empty()
    {
        return Err(invalid());
    }
    let clamped = |c: &nurbs_core::curve::Curve| {
        let [a, b] = c.domain();
        c.control_points[0].len() == 3
            && c.knots[..=c.degree].iter().all(|&k| k == a)
            && c.knots[c.knots.len() - c.degree - 1..]
                .iter()
                .all(|&k| k == b)
    };
    if boundaries.iter().any(|(_, c)| !clamped(c))
        || target.edges.iter().any(|e| !clamped(&e.curve))
    {
        return Err(invalid());
    }
    for e in &target.edges {
        if e.curve.control_points.first().unwrap().as_slice()
            != target.vertices[e.vertices[0]].point.as_slice()
            || e.curve.control_points.last().unwrap().as_slice()
                != target.vertices[e.vertices[1]].point.as_slice()
        {
            return Err(invalid());
        }
    }
    let mut assignments = std::collections::BTreeMap::new();
    for &(index, curve) in boundaries {
        let coedge = target.loops[0].coedges.get(index).ok_or_else(invalid)?;
        if coedge.reversed {
            return Err(invalid());
        }
        let edge = &target.edges[coedge.edge];
        let desired = [
            curve.control_points.last().unwrap(),
            &curve.control_points[0],
        ];
        for i in 0..2 {
            let p = [desired[i][0], desired[i][1], desired[i][2]];
            if let Some(previous) = assignments.insert(edge.vertices[i], p) {
                if previous != p {
                    return Err(Error::new(
                        "BREP_INVALID_THREAD_PATCH",
                        "Conflicting endpoint assignments",
                    ));
                }
            }
        }
    }
    let mut result = target.clone();
    let mut bound: f64 = 0.;
    for (vertex, p) in assignments {
        let original = target.vertices[vertex].point;
        let mut squared: f64 = 0.;
        for axis in 0..3 {
            if original[axis] == p[axis] {
                continue;
            }
            let delta = (original[axis] - p[axis]).abs().next_up();
            squared = (squared + (delta * delta).next_up()).next_up();
        }
        let movement = if squared == 0. {
            0.
        } else {
            squared.sqrt().next_up()
        };
        bound = bound.max(movement);
        result.vertices[vertex].point = p;
    }
    if !bound.is_finite() || bound > max_change {
        return Err(Error::new(
            "BREP_THREAD_PATCH_UNRESOLVED",
            "Endpoint movement exceeds requested budget",
        ));
    }
    let vertices = &result.0.vertices;
    for e in &mut result.0.edges {
        e.curve.control_points[0] = vertices[e.vertices[0]].point.to_vec();
        *e.curve.control_points.last_mut().unwrap() = vertices[e.vertices[1]].point.to_vec();
    }
    let mut used = 0;
    let mut agreements = Vec::new();
    for c in &result.loops[0].coedges {
        if used == max_verification_cells {
            return Err(Error::new(
                "BREP_THREAD_PATCH_UNRESOLVED",
                "Alignment verification budget exhausted",
            ));
        }
        let report = curve_surface_agreement::verify(
            &result.edges[c.edge].curve,
            &c.pcurve,
            &result.faces[0].surface,
            c.reversed,
            result.tolerance_mm,
            max_verification_cells - used,
        )?;
        used += report.cells;
        if report.status != Status::WithinTolerance {
            return Err(Error::new(
                "BREP_THREAD_PATCH_UNRESOLVED",
                "Edited boundary agreement unproved",
            ));
        }
        agreements.push(report);
    }
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

#[derive(Debug)]
pub struct SharedPatchEdge {
    /// One retained world curve, oriented as the source boundary.
    pub curve: nurbs_core::curve::Curve,
    pub source_pcurve: nurbs_core::curve::Curve,
    pub target_pcurve: nurbs_core::curve::Curve,
    pub target_reversed: bool,
    pub agreements: [Report; 2],
    pub verification_cells: usize,
}

/// Sew two boundary uses within one open shell. Retains the left world curve;
/// exact opposite endpoints and continuous agreement on both faces are needed.
/// This does not certify surface embedding or turn the shell into a solid.
pub fn sew_patch_boundaries(
    model: &Model,
    left: [usize; 2],
    right: [usize; 2],
    max_verification_cells: usize,
) -> Result<(Model, SharedPatchEdge)> {
    sew_boundaries_impl(model, left, right, max_verification_cells, true)
}
fn sew_boundaries_impl(
    model: &Model,
    left: [usize; 2],
    right: [usize; 2],
    max_verification_cells: usize,
    validate_input: bool,
) -> Result<(Model, SharedPatchEdge)> {
    if validate_input {
        model.validate()?;
    }
    let invalid = || {
        Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Sew needs distinct forward boundary uses in one open shell",
        )
    };
    if model.shells.len() != 1 || model.shells[0].closed || !model.bodies.is_empty() {
        return Err(invalid());
    }
    let coedge = |address: [usize; 2]| {
        model
            .loops
            .get(address[0])
            .and_then(|l| l.coedges.get(address[1]))
            .ok_or_else(invalid)
    };
    let lc = coedge(left)?;
    let rc = coedge(right)?;
    if lc.edge == rc.edge || lc.reversed || rc.reversed {
        return Err(invalid());
    }
    for edge in [lc.edge, rc.edge] {
        if model
            .loops
            .iter()
            .flat_map(|l| &l.coedges)
            .filter(|c| c.edge == edge)
            .count()
            != 1
        {
            return Err(invalid());
        }
    }
    let owner = |wire| {
        model
            .faces
            .iter()
            .find(|f| f.outer == wire)
            .ok_or_else(invalid)
    };
    let lf = owner(left[0])?;
    let rf = owner(right[0])?;
    let le = &model.edges[lc.edge];
    let re = &model.edges[rc.edge];
    let mut vertex_map: Vec<_> = (0..model.vertices.len()).collect();
    for i in 0..2 {
        let old = re.vertices[i];
        let new = le.vertices[1 - i];
        if model.vertices[old].point != model.vertices[new].point {
            return Err(invalid());
        }
        vertex_map[old] = new;
    }
    let provisional = nurbs_core::thread::MappedEdge {
        curve: le.curve.clone(),
        pcurve: lc.pcurve.clone(),
        agreement: Report {
            status: nurbs_core::curve_surface_agreement::Status::Unresolved,
            cells: 0,
            witness: None,
            witness_distance: None,
        },
        requested_world_tolerance: model.tolerance_mm,
    };
    let proof = share_patch_edge(
        &provisional,
        &lf.surface,
        &rc.pcurve,
        &rf.surface,
        true,
        model.tolerance_mm,
        max_verification_cells,
    )?;
    let mut result = model.clone();
    result.loops[right[0]].coedges[right[1]].edge = lc.edge;
    result.loops[right[0]].coedges[right[1]].reversed = true;
    result.edges.remove(rc.edge);
    for c in result.loops.iter_mut().flat_map(|l| &mut l.coedges) {
        if c.edge > rc.edge {
            c.edge -= 1;
        }
    }
    // Resolve endpoint unions before compacting; reject cycles rather than
    // silently assigning a different coordinate or breaking incidence.
    for i in 0..vertex_map.len() {
        let mut v = i;
        let mut steps = 0;
        while vertex_map[v] != v {
            v = vertex_map[v];
            steps += 1;
            if steps > vertex_map.len() {
                return Err(invalid());
            }
        }
        vertex_map[i] = v;
    }
    let mut compact = vec![usize::MAX; vertex_map.len()];
    let mut vertices = Vec::new();
    for (i, &root) in vertex_map.iter().enumerate() {
        if i == root {
            compact[i] = vertices.len();
            vertices.push(model.vertices[i].clone());
        }
    }
    for edge in &mut result.edges {
        edge.vertices = edge.vertices.map(|v| compact[vertex_map[v]]);
    }
    result.vertices = vertices;
    result.rebuild_topology_ids();
    if validate_input {
        result.inherit_topology_ids(&[model]);
    }
    if validate_input {
        result.validate()?;
    }
    Ok((result, proof))
}

/// Join two single-face patch sheets using one retained source edge. Opposite
/// loop traversal and exact endpoint equality are required; no vertex healing.
/// All source inputs remain immutable, including on failed verification.
pub fn join_patch_sheets(
    source: &Model,
    target: &Model,
    source_coedge: usize,
    target_coedge: usize,
    max_verification_cells: usize,
) -> Result<(Model, SharedPatchEdge)> {
    attach_patch_sheet(
        source,
        target,
        0,
        source_coedge,
        target_coedge,
        max_verification_cells,
    )
}

/// Attach a single-loop face to a boundary edge of an existing open shell.
/// Edges already used twice refuse; no third incident face is admitted.
pub fn attach_patch_sheet(
    source: &Model,
    target: &Model,
    source_loop: usize,
    source_coedge: usize,
    target_coedge: usize,
    max_verification_cells: usize,
) -> Result<(Model, SharedPatchEdge)> {
    attach_impl(
        source,
        target,
        source_loop,
        source_coedge,
        target_coedge,
        max_verification_cells,
        true,
    )
}
/// Attach a face along two boundaries atomically; the intermediate face fan
/// may be disconnected and is never returned or admitted as a Model.
pub fn attach_patch_sheet_two_edges(
    source: &Model,
    target: &Model,
    first: [[usize; 2]; 2],
    second: [[usize; 2]; 2],
    max_verification_cells: usize,
) -> Result<Model> {
    Ok(attach_two_edges_impl(source, target, first, second, max_verification_cells)?.0)
}
fn attach_two_edges_impl(
    source: &Model,
    target: &Model,
    first: [[usize; 2]; 2],
    second: [[usize; 2]; 2],
    max_verification_cells: usize,
) -> Result<(Model, usize)> {
    if first[1][0] != 0 || second[1][0] != 0 {
        return Err(Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Target uses must belong to loop zero",
        ));
    }
    attach_patch_sheet_edges(
        source,
        target,
        &[(first[0], first[1][1]), (second[0], second[1][1])],
        max_verification_cells,
    )
}
/// Attach a complete single-loop face along 1..254 selected boundaries,
/// checking all seams under one shared budget before admitting the result.
pub fn attach_patch_sheet_edges(
    source: &Model,
    target: &Model,
    pairs: &[([usize; 2], usize)],
    max_verification_cells: usize,
) -> Result<(Model, usize)> {
    if pairs.is_empty() || pairs.len() > 254 || max_verification_cells > 100000 {
        return Err(Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Attach needs 1..254 distinct boundary pairs and at most 100000 cells",
        ));
    }
    let mut source_uses = std::collections::BTreeSet::new();
    let mut target_uses = std::collections::BTreeSet::new();
    for &(s, t) in pairs {
        if !source_uses.insert(s) || !target_uses.insert(t) {
            return Err(Error::new(
                "BREP_INVALID_THREAD_PATCH",
                "Repeated boundary correspondence",
            ));
        }
    }
    let (first_source, first_target) = pairs[0];
    let (mut result, proof) = attach_impl(
        source,
        target,
        first_source[0],
        first_source[1],
        first_target,
        max_verification_cells,
        false,
    )?;
    let target_loop = source.loops.len();
    let mut used = proof.verification_cells;
    for &(s, t) in &pairs[1..] {
        let (next, proof) = sew_boundaries_impl(
            &result,
            s,
            [target_loop, t],
            max_verification_cells - used,
            false,
        )?;
        used += proof.verification_cells;
        result = next;
    }
    result.validate()?;
    Ok((result, used))
}

fn attach_impl(
    source: &Model,
    target: &Model,
    source_loop: usize,
    source_coedge: usize,
    target_coedge: usize,
    max_verification_cells: usize,
    validate_result: bool,
) -> Result<(Model, SharedPatchEdge)> {
    source.validate()?;
    target.validate()?;
    if target.faces.len() != 1 || target.loops.len() != 1 {
        return Err(Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Target must be a single-loop face",
        ));
    }
    for model in [source, target] {
        if model.shells.len() != 1
            || model.shells[0].closed
            || !model.bodies.is_empty()
            || model.faces.iter().any(|f| !f.holes.is_empty())
        {
            return Err(Error::new(
                "BREP_INVALID_THREAD_PATCH",
                "Join requires open shells without bodies or holes",
            ));
        }
    }
    let source_wire = source.loops.get(source_loop).ok_or_else(|| {
        Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Source loop index out of range",
        )
    })?;
    let source_face = source
        .faces
        .iter()
        .find(|f| f.outer == source_loop)
        .ok_or_else(|| Error::new("BREP_INVALID_THREAD_PATCH", "Source loop has no face owner"))?;
    let sc = source_wire.coedges.get(source_coedge).ok_or_else(|| {
        Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Source coedge index out of range",
        )
    })?;
    if source
        .loops
        .iter()
        .flat_map(|l| &l.coedges)
        .filter(|c| c.edge == sc.edge)
        .count()
        != 1
    {
        return Err(Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Selected source edge is not a boundary edge",
        ));
    }
    let tc = target.loops[0].coedges.get(target_coedge).ok_or_else(|| {
        Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Target coedge index out of range",
        )
    })?;
    if sc.reversed || tc.reversed {
        return Err(Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Join expects forward source sheet coedges",
        ));
    }
    let se = &source.edges[sc.edge];
    let te = &target.edges[tc.edge];
    if source.vertices[se.vertices[0]].point != target.vertices[te.vertices[1]].point
        || source.vertices[se.vertices[1]].point != target.vertices[te.vertices[0]].point
    {
        return Err(Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Join needs exact opposite boundary endpoints",
        ));
    }
    let tolerance = source.tolerance_mm.min(target.tolerance_mm);
    let provisional = nurbs_core::thread::MappedEdge {
        curve: se.curve.clone(),
        pcurve: sc.pcurve.clone(),
        agreement: Report {
            status: nurbs_core::curve_surface_agreement::Status::Unresolved,
            cells: 0,
            witness: None,
            witness_distance: None,
        },
        requested_world_tolerance: tolerance,
    };
    let shared = share_patch_edge(
        &provisional,
        &source_face.surface,
        &tc.pcurve,
        &target.faces[0].surface,
        true,
        tolerance,
        max_verification_cells,
    )?;
    let mut result = source.clone();
    result.tolerance_mm = tolerance;
    let mut vertices = vec![usize::MAX; target.vertices.len()];
    vertices[te.vertices[0]] = se.vertices[1];
    vertices[te.vertices[1]] = se.vertices[0];
    for (i, vertex) in target.vertices.iter().enumerate() {
        if vertices[i] == usize::MAX {
            let matches: Vec<_> = source
                .vertices
                .iter()
                .enumerate()
                .filter(|(_, v)| v.point == vertex.point)
                .map(|(i, _)| i)
                .collect();
            if matches.len() > 1 {
                return Err(Error::new(
                    "BREP_INVALID_THREAD_PATCH",
                    "Ambiguous exact vertex correspondence",
                ));
            }
            if let Some(&existing) = matches.first() {
                vertices[i] = existing;
            } else {
                vertices[i] = result.vertices.len();
                result.vertices.push(vertex.clone());
            }
        }
    }
    let mut edges = vec![usize::MAX; target.edges.len()];
    edges[tc.edge] = sc.edge;
    for (i, edge) in target.edges.iter().enumerate() {
        if i == tc.edge {
            continue;
        }
        edges[i] = result.edges.len();
        let mut edge = edge.clone();
        edge.vertices = edge.vertices.map(|v| vertices[v]);
        result.edges.push(edge);
    }
    let outer = result.loops.len();
    result.loops.push(Loop {
        coedges: target.loops[0]
            .coedges
            .iter()
            .map(|c| Coedge {
                edge: edges[c.edge],
                reversed: c.edge == tc.edge,
                pcurve: c.pcurve.clone(),
            })
            .collect(),
    });
    let face = result.faces.len();
    result.faces.push(Face {
        surface: target.faces[0].surface.clone(),
        outer,
        holes: vec![],
    });
    result.shells[0].faces.push(FaceUse {
        face,
        reversed: false,
    });
    result.rebuild_topology_ids();
    if validate_result {
        result.inherit_topology_ids(&[source, target]);
    }
    if validate_result {
        result.validate()?;
    }
    Ok((result, shared))
}

/// Verify one immutable world edge on two original patch surfaces. Both uses
/// are checked anew; cached source agreement is not trusted. This constructs
/// geometric boundary evidence, not global sewing or shell orientation.
pub fn share_patch_edge(
    source: &nurbs_core::thread::MappedEdge,
    source_surface: &nurbs_core::surface::Surface,
    target_pcurve: &nurbs_core::curve::Curve,
    target_surface: &nurbs_core::surface::Surface,
    target_reversed: bool,
    tolerance: f64,
    max_verification_cells: usize,
) -> Result<SharedPatchEdge> {
    use nurbs_core::curve_surface_agreement::{self, Status};
    if !(1e-10..=1e-2).contains(&tolerance) || max_verification_cells > 100000 {
        return Err(Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Shared boundary needs valid tolerance and at most 100000 cells",
        ));
    }
    if max_verification_cells == 0 {
        return Err(Error::new(
            "BREP_THREAD_PATCH_UNRESOLVED",
            "Shared boundary verification budget exhausted",
        ));
    }
    let first = curve_surface_agreement::verify(
        &source.curve,
        &source.pcurve,
        source_surface,
        false,
        tolerance,
        max_verification_cells,
    )?;
    if first.status != Status::WithinTolerance || first.cells == max_verification_cells {
        return Err(Error::new(
            "BREP_THREAD_PATCH_UNRESOLVED",
            "Source boundary agreement unproved or budget exhausted",
        ));
    }
    let second = curve_surface_agreement::verify(
        &source.curve,
        target_pcurve,
        target_surface,
        target_reversed,
        tolerance,
        max_verification_cells - first.cells,
    )?;
    if second.status != Status::WithinTolerance {
        return Err(Error::new(
            "BREP_THREAD_PATCH_UNRESOLVED",
            "Target boundary agreement unproved",
        ));
    }
    let cells = first.cells + second.cells;
    Ok(SharedPatchEdge {
        curve: source.curve.clone(),
        source_pcurve: source.pcurve.clone(),
        target_pcurve: target_pcurve.clone(),
        target_reversed,
        agreements: [first, second],
        verification_cells: cells,
    })
}

pub fn trimmed_patch_sheet(
    candidate: &TrimCandidate,
    max_uv_error: f64,
    tolerance: f64,
    max_spans: usize,
    max_verification_cells: usize,
) -> Result<PatchSheet> {
    if !(1e-10..=1e-2).contains(&tolerance) {
        return Err(Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Patch tolerance must be 1e-10..1e-2 mm",
        ));
    }
    let mapped =
        candidate.mapped_edges(max_uv_error, tolerance, max_spans, max_verification_cells)?;
    if !mapped.all_edges_within_tolerance {
        return Err(Error::new(
            "BREP_THREAD_PATCH_UNRESOLVED",
            "Continuous patch boundary agreement is unproved",
        ));
    }
    let uv_wire: Vec<_> = mapped.edges.iter().map(|e| e.pcurve.clone()).collect();
    let components = crate::planar_trim::components(&[uv_wire], 1e-12)?;
    if components.len() != 1 || components[0].0 != 0 || !components[0].1.is_empty() {
        return Err(Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Patch UV boundary must be one simple material-left loop",
        ));
    }
    let mut model = Model::empty(tolerance)?;
    let count = mapped.edges.len();
    for i in 0..count {
        let curve = &mapped.edges[i].curve;
        if curve.control_points.last() != mapped.edges[(i + 1) % count].curve.control_points.first()
        {
            return Err(Error::new(
                "BREP_INVALID_THREAD_PATCH",
                "World boundary endpoints must coincide exactly",
            ));
        }
        let p = &curve.control_points[0];
        model.vertices.push(Vertex {
            point: [p[0], p[1], p[2]],
        });
    }
    let mut agreements = Vec::new();
    let mut coedges = Vec::new();
    for (i, edge) in mapped.edges.into_iter().enumerate() {
        agreements.push(edge.agreement);
        model.edges.push(Edge {
            vertices: [i, (i + 1) % count],
            curve: edge.curve,
            degenerate: false,
        });
        coedges.push(Coedge {
            edge: i,
            reversed: false,
            pcurve: edge.pcurve,
        });
    }
    model.loops.push(Loop { coedges });
    model.faces.push(Face {
        surface: candidate.patch.surface.clone(),
        outer: 0,
        holes: vec![],
    });
    model.shells.push(Shell {
        faces: vec![FaceUse {
            face: 0,
            reversed: false,
        }],
        closed: false,
    });
    model.rebuild_topology_ids();
    model.validate()?;
    Ok(PatchSheet {
        model,
        agreements,
        verification_cells: mapped.verification_cells,
    })
}

/// Independent work budgets for finite external-thread candidate construction.
#[derive(Clone, Copy, Debug)]
pub struct FiniteThreadLimits {
    pub uv_error: f64,
    pub tolerance_mm: f64,
    pub max_spans: usize,
    pub max_curve_change: f64,
    /// Shared by patch mapping, alignment, sewing and both caps.
    pub agreement_cells: usize,
    /// Each cap has these independent region-audit limits.
    pub trim_pairs_per_cap: usize,
    pub trim_cells_per_cap: usize,
    pub domain_cells_per_cap: usize,
    pub mass_relative_tolerance: f64,
    pub mass_evaluations: usize,
}
#[derive(Debug)]
pub struct FiniteThreadCandidate {
    pub body: ClosedBodyCandidate,
    pub agreement_cells: usize,
    pub side_faces: usize,
    /// Maximum per-operation displacement bound, not total ideal-thread error.
    pub max_endpoint_adjustment_upper_bound: f64,
}
/// Build a finite external-thread body candidate with NURBS cap boundaries.
/// The requested slab must support complete connected end contours. Internal
/// threads require an outer wall and are deliberately refused by this API.
/// Independent solid and rounding-inclusive ideal geometry qualification remain.
pub fn finite_external_candidate(
    spec: nurbs_core::thread::Spec,
    z_limits: [f64; 2],
    limits: FiniteThreadLimits,
) -> Result<FiniteThreadCandidate> {
    if spec.kind != nurbs_core::thread::Kind::External
        || !(1..=10_000_000).contains(&limits.agreement_cells)
        || !(1..=100000).contains(&limits.trim_pairs_per_cap)
        || !(1..=100000).contains(&limits.trim_cells_per_cap)
        || !(1..=1000000).contains(&limits.domain_cells_per_cap)
        || limits.mass_evaluations == 0
        || !limits.mass_relative_tolerance.is_finite()
        || limits.mass_relative_tolerance <= 0.
    {
        return Err(Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Finite external candidate needs external profile and positive bounded work limits",
        ));
    }
    let network = clipped_thread_network(
        spec,
        z_limits,
        limits.uv_error,
        limits.tolerance_mm,
        limits.max_spans,
        limits.max_curve_change,
        limits.agreement_cells,
    )?;
    let side_faces = network.model.faces.len();
    let mut used = network.verification_cells;
    let mut model = network.model;
    for z in z_limits {
        let remaining = (limits.agreement_cells - used).min(100000);
        if remaining == 0 {
            return Err(Error::new(
                "BREP_THREAD_PATCH_UNRESOLVED",
                "Finite thread agreement budget exhausted before capping",
            ));
        }
        let (next, cells) = cap_patch_network_plane_general(
            &model,
            z,
            limits.trim_pairs_per_cap,
            limits.trim_cells_per_cap,
            limits.domain_cells_per_cap,
            remaining,
        )?;
        used += cells;
        model = next;
    }
    let body = closed_body_candidate(
        &model,
        limits.mass_relative_tolerance,
        limits.mass_evaluations,
    )?;
    Ok(FiniteThreadCandidate {
        body,
        agreement_cells: used,
        side_faces,
        max_endpoint_adjustment_upper_bound: network.max_endpoint_adjustment_upper_bound,
    })
}
