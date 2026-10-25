//! Finite internal thread: original inner walls, cylindrical outer wall and
//! directly owned annular caps with identity XY parameterization.
use super::*;
use nurbs_core::{
    interval_eval::Interval as I,
    surface::Surface,
    thread::{Hand, Kind, Spec},
};
fn fail(s: &str) -> Error {
    Error::new("BREP_THREAD_PATCH_UNRESOLVED", s)
}
fn outer_sheet(radius: f64, z: [f64; 2]) -> Result<Model> {
    let height = z[1] - z[0];
    let mut model = crate::cylinder(radius, height)?;
    if model.faces.len() != 6
        || model.loops.len() != 6
        || model.faces[..4]
            .iter()
            .any(|f| f.surface.degree_u != 2 || f.surface.degree_v != 1)
    {
        return Err(fail("Unexpected authored cylinder wall layout"));
    }
    model.faces.truncate(4);
    model.loops.truncate(4);
    model.shells[0].faces.truncate(4);
    model.shells[0].closed = false;
    model.bodies.clear();
    let map = |value: f64| -> Result<f64> {
        if value == 0. {
            Ok(z[0])
        } else if value == height {
            Ok(z[1])
        } else {
            Err(fail("Outer wall has an unexpected axial control"))
        }
    };
    for v in &mut model.vertices {
        v.point[2] = map(v.point[2])?;
    }
    for e in &mut model.edges {
        for p in &mut e.curve.control_points {
            p[2] = map(p[2])?;
        }
    }
    for f in &mut model.faces {
        for p in f.surface.control_points.iter_mut().flatten() {
            p[2] = map(p[2])?;
        }
    }
    model.rebuild_topology_ids();
    model.validate()?;
    Ok(model)
}
fn append(mut inner: Model, outer: Model, tolerance: f64) -> Model {
    let vo = inner.vertices.len();
    let eo = inner.edges.len();
    let lo = inner.loops.len();
    let fo = inner.faces.len();
    inner.vertices.extend(outer.vertices.iter().cloned());
    inner.edges.extend(outer.edges.iter().cloned().map(|mut e| {
        e.vertices = e.vertices.map(|v| v + vo);
        e
    }));
    inner.loops.extend(outer.loops.iter().cloned().map(|mut l| {
        for c in &mut l.coedges {
            c.edge += eo;
        }
        l
    }));
    inner.faces.extend(outer.faces.iter().cloned().map(|mut f| {
        f.outer += lo;
        for h in &mut f.holes {
            *h += lo;
        }
        f
    }));
    inner.shells[0]
        .faces
        .extend(outer.shells[0].faces.iter().cloned().map(|mut u| {
            u.face += fo;
            u
        }));
    inner.tolerance_mm = tolerance;
    inner.rebuild_topology_ids();
    inner
}
/// Negative parameter-domain reflection preserves the original rational image
/// exactly: knot negation requires no rounded a+b-k subtraction.
fn oriented_xy(c: &nurbs_core::curve::Curve, reverse: bool) -> nurbs_core::curve::Curve {
    let mut p = c.clone();
    for q in &mut p.control_points {
        q.truncate(2);
    }
    if reverse {
        p.control_points.reverse();
        p.weights.reverse();
        p.knots = p.knots.iter().rev().map(|k| -k).collect();
    }
    p
}
#[derive(Clone, Debug)]
pub struct CapProfileProof {
    wires: Vec<Vec<nurbs_core::curve::Curve>>,
    certificate: crate::profile_certificate::ProfileCertificate,
    verified: crate::profile_certificate::VerifiedProfile,
}
impl CapProfileProof {
    pub fn wires(&self) -> &[Vec<nurbs_core::curve::Curve>] {
        &self.wires
    }
    pub fn certificate(&self) -> &crate::profile_certificate::ProfileCertificate {
        &self.certificate
    }
    pub fn verified(&self) -> &crate::profile_certificate::VerifiedProfile {
        &self.verified
    }
}
struct CapEdge {
    edge: usize,
    reverse: bool,
    start: usize,
    end: usize,
    outer: bool,
}
fn cap(
    model: &mut Model,
    z: f64,
    bottom: bool,
    inner_faces: usize,
    limits: FiniteThreadLimits,
) -> Result<CapProfileProof> {
    let mut incidence = vec![0usize; model.edges.len()];
    for f in &model.faces {
        for &li in std::iter::once(&f.outer).chain(f.holes.iter()) {
            for c in &model.loops[li].coedges {
                incidence[c.edge] += 1;
            }
        }
    }
    let mut edges = Vec::new();
    for usage in &model.shells[0].faces {
        let f = &model.faces[usage.face];
        for &li in std::iter::once(&f.outer).chain(f.holes.iter()) {
            for c in &model.loops[li].coedges {
                let e = &model.edges[c.edge];
                if incidence[c.edge] != 1 || !e.curve.control_points.iter().all(|p| p[2] == z) {
                    continue;
                }
                let reverse = (!(c.reversed ^ usage.reversed)) ^ bottom;
                let [start, end] = if reverse {
                    [e.vertices[1], e.vertices[0]]
                } else {
                    e.vertices
                };
                edges.push(CapEdge {
                    edge: c.edge,
                    reverse,
                    start,
                    end,
                    outer: usage.face >= inner_faces,
                });
            }
        }
    }
    let mut outgoing = std::collections::BTreeMap::new();
    let mut incoming = std::collections::BTreeSet::new();
    for (i, e) in edges.iter().enumerate() {
        if outgoing.insert(e.start, i).is_some() || !incoming.insert(e.end) {
            return Err(fail("Annular cap has nonunique endpoint ownership"));
        }
    }
    if outgoing.len() != incoming.len() || outgoing.keys().any(|v| !incoming.contains(v)) {
        return Err(fail("Annular cap has an open directed boundary"));
    }
    let mut used = vec![false; edges.len()];
    let mut cycles = Vec::new();
    for first in 0..edges.len() {
        if used[first] {
            continue;
        }
        let mut cycle = Vec::new();
        let mut current = first;
        loop {
            if used[current] {
                return Err(fail("Annular boundary repeats an edge before closure"));
            }
            used[current] = true;
            cycle.push(current);
            let end = edges[current].end;
            if end == edges[first].start {
                break;
            }
            current = *outgoing
                .get(&end)
                .ok_or_else(|| fail("Annular boundary lacks an outgoing owner"))?;
        }
        if cycle.len() < 3 || cycle.iter().any(|&i| edges[i].outer != edges[first].outer) {
            return Err(fail("Invalid mixed annular boundary cycle"));
        }
        cycles.push(cycle);
    }
    if cycles.len() != if inner_faces == 0 { 1 } else { 2 } {
        return Err(fail(
            "Internal thread cap requires exactly two closed cycles",
        ));
    }
    cycles.sort_by_key(|c| !edges[c[0]].outer);
    if !edges[cycles[0][0]].outer || (inner_faces > 0 && edges[cycles[1][0]].outer) {
        return Err(fail("Annular material roles are unresolved"));
    }
    let wires: Vec<Vec<_>> = cycles
        .iter()
        .map(|c| {
            c.iter()
                .map(|&i| oriented_xy(&model.edges[edges[i].edge].curve, edges[i].reverse))
                .collect()
        })
        .collect();
    let span_count = wires
        .iter()
        .flatten()
        .map(|c| {
            (c.degree..c.control_points.len())
                .filter(|&i| c.knots[i] < c.knots[i + 1])
                .count()
        })
        .sum::<usize>();
    if span_count > limits.trim_cells_per_cap
        || span_count.saturating_mul(span_count.saturating_sub(1)) / 2 > limits.trim_pairs_per_cap
    {
        return Err(fail("Annular source-profile budget exhausted"));
    }
    let witness = crate::profile_certificate::certify(&wires, limits.tolerance_mm)?;
    let verified = crate::profile_certificate::verify(&wires, limits.tolerance_mm, &witness)?;
    if verified.pieces() > limits.trim_cells_per_cap
        || witness.separating_normals.len() > limits.trim_pairs_per_cap
    {
        return Err(fail(
            "Annular exact profile witness exceeds its admitted budget",
        ));
    }
    let extent = wires
        .iter()
        .flatten()
        .flat_map(|c| c.control_points.iter().flatten())
        .map(|x| x.abs())
        .fold(1_f64, f64::max)
        * 2.;
    let surface = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-extent, -extent, extent, extent],
        knots_v: vec![-extent, -extent, extent, extent],
        control_points: vec![
            vec![vec![-extent, -extent, z], vec![-extent, extent, z]],
            vec![vec![extent, -extent, z], vec![extent, extent, z]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let mut loops = Vec::new();
    let proof = CapProfileProof {
        wires: wires.clone(),
        certificate: witness,
        verified,
    };
    for (cycle, wire) in cycles.iter().zip(wires) {
        let coedges = cycle
            .iter()
            .zip(wire)
            .map(|(&i, pcurve)| Coedge::new(
                edges[i].edge,
                edges[i].reverse,
                pcurve,
            ))
            .collect();
        loops.push(model.loops.len());
        model.loops.push(Loop { coedges });
    }
    let face = model.faces.len();
    model.faces.push(Face {
        surface,
        outer: loops[0],
        holes: loops[1..].to_vec(),
    });
    model.shells[0].faces.push(FaceUse {
        face,
        reversed: bottom,
    });
    model.rebuild_topology_ids();
    Ok(proof)
}
/// Closed internal-thread ring candidate with two exact owned annular caps.
/// Original patch/clipping/boundary certificates remain separate from global
/// solid admission and a combined ideal finite-body Hausdorff certificate.
pub fn finite_internal_with_certificates(
    spec: Spec,
    z_limits: [f64; 2],
    outer_radius: f64,
    limits: FiniteThreadLimits,
) -> Result<CertifiedFiniteThread> {
    if spec.kind != Kind::Internal
        || !outer_radius.is_finite()
        || outer_radius <= 0.
        || outer_radius > 100000.
        || !(1..=10000000).contains(&limits.agreement_cells)
        || !(1..=100000).contains(&limits.trim_pairs_per_cap)
        || !(1..=100000).contains(&limits.trim_cells_per_cap)
        || !(1..=1000000).contains(&limits.domain_cells_per_cap)
    {
        return Err(fail(
            "Internal thread needs internal kind, bounded outer radius and positive proof limits",
        ));
    }
    let (_, patch_certificate) = nurbs_core::thread::patches_certified(spec, 2560)?;
    let trims = nurbs_core::thread::trim_candidates(spec, z_limits, limits.tolerance_mm)?;
    let trim_certificate = nurbs_core::thread::certify_trim_candidates(
        spec,
        &trims,
        limits.uv_error,
        limits.tolerance_mm,
        2560,
    )?;
    let outer = outer_sheet(outer_radius, z_limits)?;
    let mut outer_range = [f64::INFINITY, 0_f64];
    for f in &outer.faces {
        // The shared radial owner measures 3D distance. Exact projection to
        // Z=0 makes this an XY-cylinder bound, independent of axial position.
        let mut radial = f.surface.clone();
        for p in radial.control_points.iter_mut().flatten() {
            p[2] = 0.;
        }
        let range = nurbs_core::radial_bounds::radius_bounds(&radial, [0.; 3])?
            .ok_or_else(|| fail("Outer wall XY radius remains unproved"))?;
        outer_range[0] = outer_range[0].min(range[0]);
        outer_range[1] = outer_range[1].max(range[1]);
    }
    let inner_max = I::point(patch_certificate.crest_radius[1])
        .add(I::point(patch_certificate.error_upper))?
        .add(I::point(limits.tolerance_mm))?
        .hi;
    if outer_range[0] <= inner_max {
        return Err(fail(
            "Outer wall is not separated from the complete inner thread",
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
    let mut inner = network.model;
    if spec.hand == Hand::Left {
        for u in &mut inner.shells[0].faces {
            u.reversed = !u.reversed;
        }
    }
    let mut model = append(inner, outer, limits.tolerance_mm);
    let cap_profiles = vec![
        cap(&mut model, z_limits[0], true, side_faces, limits)?,
        cap(&mut model, z_limits[1], false, side_faces, limits)?,
    ];
    let body = closed_body_candidate(
        &model,
        limits.mass_relative_tolerance,
        limits.mass_evaluations,
    )?;
    let candidate = FiniteThreadCandidate {
        body,
        agreement_cells: network.verification_cells,
        side_faces: side_faces + 4,
        outer_wall_faces: 4,
        outer_wall_radius_bounds: Some(outer_range),
        max_endpoint_adjustment_upper_bound: network.max_endpoint_adjustment_upper_bound,
    };
    verified::admit(
        candidate,
        spec,
        trims,
        patch_certificate,
        trim_certificate,
        cap_profiles,
        limits,
    )
}

pub(super) fn external_candidate(
    spec: Spec,
    z_limits: [f64; 2],
    limits: FiniteThreadLimits,
) -> Result<(FiniteThreadCandidate, Vec<CapProfileProof>)> {
    if spec.kind != Kind::External
        || !(1..=10000000).contains(&limits.agreement_cells)
        || !(1..=100000).contains(&limits.trim_pairs_per_cap)
        || !(1..=100000).contains(&limits.trim_cells_per_cap)
    {
        return Err(fail(
            "External exact-cap constructor needs valid kind and bounded proof limits",
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
    external_network_candidate(network, spec.hand, z_limits, limits)
}
pub(super) fn external_network_candidate(
    network: TurnSheet,
    hand: Hand,
    z_limits: [f64; 2],
    limits: FiniteThreadLimits,
) -> Result<(FiniteThreadCandidate, Vec<CapProfileProof>)> {
    let side_faces = network.model.faces.len();
    let mut model = network.model;
    if hand == Hand::Right {
        for u in &mut model.shells[0].faces {
            u.reversed = !u.reversed;
        }
    }
    let cap_profiles = vec![
        cap(&mut model, z_limits[0], true, 0, limits)?,
        cap(&mut model, z_limits[1], false, 0, limits)?,
    ];
    let body = closed_body_candidate(
        &model,
        limits.mass_relative_tolerance,
        limits.mass_evaluations,
    )?;
    Ok((
        FiniteThreadCandidate {
            body,
            agreement_cells: network.verification_cells,
            side_faces,
            outer_wall_faces: 0,
            outer_wall_radius_bounds: None,
            max_endpoint_adjustment_upper_bound: network.max_endpoint_adjustment_upper_bound,
        },
        cap_profiles,
    ))
}

/// Exact-profile fallback for the legacy single-cycle cap entry point. The
/// existing source edge and the new cap lift are both rechecked continuously.
pub(super) fn cap_single_exact(source: &Model, z: f64, max_cells: usize) -> Result<(Model, usize)> {
    use num_rational::BigRational as Q;
    use num_traits::Zero;
    if max_cells == 0
        || max_cells > 100000
        || source.shells.len() != 1
        || source.shells[0].closed
        || !source.bodies.is_empty()
    {
        return Err(fail(
            "Exact single-cycle cap needs one open shell and a positive bounded agreement budget",
        ));
    }
    let mut incidence = vec![0usize; source.edges.len()];
    for f in &source.faces {
        for &li in std::iter::once(&f.outer).chain(&f.holes) {
            for c in &source.loops[li].coedges {
                incidence[c.edge] += 1;
            }
        }
    }
    let mut area = Q::zero();
    for u in &source.shells[0].faces {
        let f = &source.faces[u.face];
        for &li in std::iter::once(&f.outer).chain(&f.holes) {
            for c in &source.loops[li].coedges {
                let e = &source.edges[c.edge];
                if incidence[c.edge] != 1 || !e.curve.control_points.iter().all(|p| p[2] == z) {
                    continue;
                }
                let a = &e.curve.control_points[0];
                let b = e.curve.control_points.last().unwrap();
                let term = Q::from_float(a[0]).unwrap() * Q::from_float(b[1]).unwrap()
                    - Q::from_float(a[1]).unwrap() * Q::from_float(b[0]).unwrap();
                area += if c.reversed ^ u.reversed { -term } else { term };
            }
        }
    }
    let choice = area > Q::zero();
    let mut model = source.clone();
    let limits = FiniteThreadLimits {
        tolerance_mm: source.tolerance_mm,
        ..FiniteThreadLimits::default()
    };
    if cap(&mut model, z, choice, 0, limits).is_err() {
        model = source.clone();
        cap(&mut model, z, !choice, 0, limits)?;
    }
    model.validate()?;
    let last = model.faces.last().unwrap();
    let selected: std::collections::BTreeSet<_> = std::iter::once(&last.outer)
        .chain(&last.holes)
        .flat_map(|&li| model.loops[li].coedges.iter().map(|c| c.edge))
        .collect();
    let mut used = 0;
    for f in &model.faces {
        for &li in std::iter::once(&f.outer).chain(&f.holes) {
            for c in &model.loops[li].coedges {
                if !selected.contains(&c.edge) {
                    continue;
                }
                if used == max_cells {
                    return Err(fail("Single-cycle cap shared edge/lift budget exhausted"));
                }
                let r = nurbs_core::curve_surface_agreement::verify(
                    &model.edges[c.edge].curve,
                    &c.pcurve,
                    &f.surface,
                    c.reversed,
                    source.tolerance_mm,
                    max_cells - used,
                )?;
                used += r.cells;
                if r.status != nurbs_core::curve_surface_agreement::Status::WithinTolerance {
                    return Err(fail(
                        "Single-cycle cap source/target agreement remains unresolved",
                    ));
                }
            }
        }
    }
    Ok((model, used))
}
