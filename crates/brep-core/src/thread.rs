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

mod caps;
pub use caps::{cap_patch_network_plane, cap_patch_network_plane_general, trimmed_patch_sheet_with_planar_ends, planarize_patch_edges};


mod turns;
pub use turns::{TurnSheet, clipped_thread_network, untrimmed_turn_sheet};


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

mod sewing;
use sewing::attach_two_edges_impl;
pub use sewing::{SharedPatchEdge, sew_patch_boundaries, join_patch_sheets, attach_patch_sheet, attach_patch_sheet_two_edges, attach_patch_sheet_edges, share_patch_edge};


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
