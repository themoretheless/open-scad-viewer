use super::*;

#[derive(Clone, Debug)]
pub struct AuditedFeatureResult {
    pub model: Model,
    pub feature: FeatureCertificate,
    pub context: ToleranceSpecIdentity,
    pub evidence: ComposedEvidence,
    pub audit: SolidAuditCertificate,
    pub change_set: ChangeSet,
    pub naming_complete: bool,
}

#[allow(dead_code)]
pub(super) fn unavailable(capability: &str) -> Error {
    Error::new(
        "BREP_CAPABILITY_UNAVAILABLE",
        format!("{capability} is Unavailable until its QualificationPlan release"),
    )
}

pub(super) fn refuse(code: &'static str, message: &str) -> Error {
    Error::new(code, message)
}

#[derive(Clone, Debug)]
pub struct FeatureCertificate {
    pub capability: &'static str,
    pub complete: bool,
    pub notes: Vec<&'static str>,
}

impl FeatureCertificate {
    pub fn permits_topology_change(&self) -> bool {
        self.complete
    }
}

pub(super) fn is_axis_aligned_cuboid(model: &Model) -> bool {
    if model.validate().is_err()
        || model.vertices.len() != 8
        || model.edges.len() != 12
        || model.faces.len() != 6
        || model.shells.len() != 1
        || model.bodies.len() != 1
        || model
            .faces
            .iter()
            .any(|f| f.surface.degree_u != 1 || f.surface.degree_v != 1)
        || model.edges.iter().any(|e| e.curve.degree != 1)
    {
        return false;
    }
    let (min, max) = model_bounds(model);
    if (0..3).any(|axis| !min[axis].is_finite() || max[axis] - min[axis] <= 1e-12) {
        return false;
    }
    let mut corners = std::collections::BTreeSet::new();
    for vertex in &model.vertices {
        let mut bits = 0u8;
        for axis in 0..3 {
            if (vertex.point[axis] - min[axis]).abs() <= 1e-9 {
                continue;
            }
            if (vertex.point[axis] - max[axis]).abs() <= 1e-9 {
                bits |= 1 << axis;
            } else {
                return false;
            }
        }
        if !corners.insert(bits) {
            return false;
        }
    }
    model.edges.iter().all(|edge| {
        let a = model.vertices[edge.vertices[0]].point;
        let b = model.vertices[edge.vertices[1]].point;
        (0..3)
            .filter(|axis| (a[*axis] - b[*axis]).abs() > 1e-9)
            .count()
            == 1
    })
}

pub(super) fn model_bounds(model: &Model) -> ([f64; 3], [f64; 3]) {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for v in &model.vertices {
        for i in 0..3 {
            min[i] = min[i].min(v.point[i]);
            max[i] = max[i].max(v.point[i]);
        }
    }
    (min, max)
}

pub(super) fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub(super) fn sub3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub(super) fn cross3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub(super) fn unit3(v: [f64; 3]) -> Option<[f64; 3]> {
    let n = dot3(v, v).sqrt();
    (n > 1e-12 && n.is_finite()).then_some([v[0] / n, v[1] / n, v[2] / n])
}

pub(super) fn certify_blend_result(
    result: Model,
    capability: &'static str,
    notes: Vec<&'static str>,
    scale: f64,
    refusal_code: &'static str,
) -> Result<AuditedFeatureResult> {
    result.validate()?;
    let context = result.tolerance_context()?;
    let evidence = compose_predicate_evidence(
        &context,
        [
            PredicateEvidence::correspondence(&context, 0., scale.max(1e-12))?,
            PredicateEvidence::topology_preservation(
                &context,
                "exact blend authored shared-edge incidence",
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
            refusal_code,
            "Exact blend lacks complete ChangeSet/persistent naming evidence",
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

/// Exact finite successor for connected open/closed selections of arbitrary
/// straight convex edges on an audited convex planar-faced polyhedron.
pub fn exact_convex_chamfer(
    model: &Model,
    edges: &[usize],
    distance: f64,
) -> Result<AuditedFeatureResult> {
    audit_solid(model).map_err(|_| {
        refuse(
            "BREP_EXACT_CHAMFER_REFUSED",
            "Chamfer source must pass the global solid audit",
        )
    })?;
    if model.edges.iter().any(|edge| edge.curve.degree != 1)
        || model
            .faces
            .iter()
            .any(|face| face.surface.degree_u != 1 || face.surface.degree_v != 1)
    {
        return Err(refuse(
            "BREP_EXACT_CHAMFER_REFUSED",
            "Exact convex chamfer admits straight edges and planar faces only",
        ));
    }
    let result = crate::operations::chamfer_edges(model, edges, distance).map_err(|error| {
        refuse(
            "BREP_EXACT_CHAMFER_REFUSED",
            &format!(
                "Exact convex chamfer feasibility refused: {}",
                error.message
            ),
        )
    })?;
    certify_blend_result(
        result,
        EXACT_CONVEX_CHAMFER_CAPABILITY,
        vec![
            "exact_equal_distance_support_plane",
            "connected_open_or_closed_selection",
            "deterministic_planar_corner_intersection",
            "no_mesh_fallback",
        ],
        distance,
        "BREP_EXACT_CHAMFER_REFUSED",
    )
}

/// Exact finite fillet successor for a strictly-convex prism. Selected authored
/// edges may be any subset of its longitudinal straight edges; mixed rounded
/// and sharp profile vertices and the all-selected closed profile are admitted.
/// Cap-edge chains and valence-3 rolling-ball corners remain typed-refused.
pub(super) fn ordered_prism_cap_profile(
    model: &Model,
    vertices: &[[f64; 3]],
    z: f64,
    tol: f64,
) -> Result<Vec<[f64; 2]>> {
    // Consolidate coplanar cap faces by removing their shared internal edges.
    let mut counts = std::collections::BTreeMap::<usize, usize>::new();
    for face in &model.faces {
        let loops: Vec<_> = std::iter::once(face.outer)
            .chain(face.holes.iter().copied())
            .collect();
        if !loops
            .iter()
            .flat_map(|&l| &model.loops[l].coedges)
            .all(|coedge| {
                model.edges[coedge.edge]
                    .vertices
                    .iter()
                    .all(|&vertex| (vertices[vertex][2] - z).abs() <= tol)
            })
        {
            continue;
        }
        for coedge in loops.iter().flat_map(|&l| &model.loops[l].coedges) {
            *counts.entry(coedge.edge).or_default() += 1;
        }
    }
    let mut neighbors = std::collections::BTreeMap::<usize, Vec<usize>>::new();
    for (edge, count) in counts {
        if count == 2 {
            continue;
        }
        if count != 1 {
            return Err(refuse(
                "BREP_EXACT_FILLET_REFUSED",
                "Cap partition has nonmanifold edge incidence",
            ));
        }
        let [a, b] = model.edges[edge].vertices;
        neighbors.entry(a).or_default().push(b);
        neighbors.entry(b).or_default().push(a);
    }
    if neighbors.len() < 3 || neighbors.values().any(|n| n.len() != 2) {
        return Err(refuse(
            "BREP_EXACT_FILLET_REFUSED",
            "Prism cap requires a closed simple boundary",
        ));
    }
    let start = *neighbors.keys().next().unwrap();
    let mut current = start;
    let mut previous = usize::MAX;
    let mut profile = Vec::with_capacity(neighbors.len());
    for step in 0..neighbors.len() {
        if step > 0 && current == start {
            return Err(refuse(
                "BREP_EXACT_FILLET_REFUSED",
                "Prism cap holes or disconnected boundaries are unsupported",
            ));
        }
        let p = vertices[current];
        profile.push([p[0], p[1]]);
        let next = *neighbors[&current]
            .iter()
            .find(|&&v| v != previous)
            .unwrap();
        previous = current;
        current = next;
    }
    if current != start {
        return Err(refuse(
            "BREP_EXACT_FILLET_REFUSED",
            "Prism cap boundary does not close",
        ));
    }
    let area: f64 = profile
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let b = profile[(i + 1) % profile.len()];
            a[0] * b[1] - a[1] * b[0]
        })
        .sum();
    if area < 0. {
        profile.reverse();
    }
    Ok(profile)
}

pub fn exact_convex_prism_fillet(
    model: &Model,
    edges: &[usize],
    radius: f64,
) -> Result<AuditedFeatureResult> {
    exact_prism_fillet(model, edges, radius, true)
}

/// Constant-radius convex longitudinal rounds; holes, concave selected corners,
/// curved input spans and unresolved boundary separation are refused.
pub fn exact_simple_prism_fillet(
    model: &Model,
    edges: &[usize],
    radius: f64,
) -> Result<AuditedFeatureResult> {
    exact_prism_fillet(model, edges, radius, false)
}

/// Constant outer rounds of complete longitudinal chains in a layered prism.
/// Cavity intersections and unproven layer arrangements are refused.
pub fn exact_layered_prism_fillet(
    model: &Model,
    edges: &[usize],
    radius: f64,
) -> Result<AuditedFeatureResult> {
    let denied = |message: &str| refuse("BREP_LAYERED_FILLET_REFUSED", message);
    model.validate()?;
    if edges.is_empty()
        || edges.iter().any(|i| *i >= model.edges.len())
        || !radius.is_finite()
        || radius <= 0.
    {
        return Err(denied("Select source edges and a positive finite radius"));
    }
    let first = &model.edges[edges[0]];
    let direction = sub3(
        model.vertices[first.vertices[1]].point,
        model.vertices[first.vertices[0]].point,
    );
    let (local, _, frame) = crate::prism_frame::localize_along(model, model, Some(direction))?
        .ok_or_else(|| denied("Source has no proven common prism frame"))?;
    if local.edges.iter().any(|e| e.curve.degree != 1) {
        return Err(denied(
            "Layered envelope requires straight source boundaries",
        ));
    }
    let layers = crate::stepped_prism::recognize(&local)?
        .ok_or_else(|| denied("Source layers are unresolved"))?;
    let low = layers.iter().map(|l| l.low).fold(f64::INFINITY, f64::min);
    let high = layers
        .iter()
        .map(|l| l.high)
        .fold(f64::NEG_INFINITY, f64::max);
    // Monotone hull supplies a cutting envelope, never substitute source material.
    // The construction below proves containment independently for every layer.
    let mut points: Vec<[f64; 2]> = local
        .vertices
        .iter()
        .map(|v| [v.point[0], v.point[1]])
        .collect();
    points.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    points.dedup();
    let turn = |a: [f64; 2], b: [f64; 2], c: [f64; 2]| {
        (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    };
    let hull_scale = points.iter().flatten().fold(1_f64, |a, b| a.max(b.abs()));
    let hull_roundoff = 128. * f64::EPSILON * hull_scale * hull_scale;
    let mut hull = Vec::new();
    let mut lower: Vec<[f64; 2]> = Vec::new();
    let mut upper: Vec<[f64; 2]> = Vec::new();
    for (chain, sequence) in [
        (&mut lower, points.clone()),
        (&mut upper, points.iter().rev().copied().collect()),
    ] {
        for p in sequence {
            while chain.len() >= 2
                && turn(chain[chain.len() - 2], chain[chain.len() - 1], p) <= hull_roundoff
            {
                chain.pop();
            }
            chain.push(p);
        }
        chain.pop();
    }
    hull.extend(lower);
    hull.extend(upper);
    if hull.len() < 3 {
        return Err(denied("Source envelope is degenerate"));
    }
    let ring: Vec<_> = (0..hull.len())
        .map(|i| crate::line(hull[i].to_vec(), hull[(i + 1) % hull.len()].to_vec()))
        .collect();
    let envelope = crate::prism::extrude(&[ring], low, high)?;
    let result = round_layered_prism_in_envelope(&local, &envelope, edges, radius)?;
    let mut result = frame.restore(&result)?;
    result.inherit_topology_ids(&[model]);
    certify_blend_result(
        result,
        "exact-layered-prism-edge-fillet/1",
        vec![
            "complete_longitudinal_chains",
            "removed_region_inside_every_layer",
            "source_preserving_envelope_intersection",
            "bounded_rigid_frame_normalization",
        ],
        radius,
        "BREP_LAYERED_FILLET_REFUSED",
    )
}

/// Internal construction in a common extrusion frame. The envelope is only a
/// cutting tool: every removed region must belong to every source layer.
/// Caller-selected source edges must match complete envelope edges.
pub(super) fn round_layered_prism_in_envelope(
    source: &Model,
    envelope: &Model,
    edges: &[usize],
    radius: f64,
) -> Result<Model> {
    let denied = |message: &str| refuse("BREP_LAYERED_FILLET_REFUSED", message);
    source.validate()?;
    audit_solid(source)?;
    if edges.is_empty()
        || edges.iter().any(|i| *i >= source.edges.len())
        || edges
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != edges.len()
    {
        return Err(denied("Select unique source edges"));
    }
    let layers = crate::stepped_prism::recognize(source)?
        .ok_or_else(|| denied("Source must have proven prismatic layers"))?;
    let profile = crate::prism::recognize(envelope)?
        .ok_or_else(|| denied("Fillet envelope must be a proven prism"))?;
    let tol = source.tolerance_mm;
    if layers.is_empty()
        || layers
            .iter()
            .any(|l| l.low < profile.z_min || l.high > profile.z_max)
    {
        return Err(denied("Envelope must span every source layer"));
    }
    let same = |a: [f64; 3], b: [f64; 3]| (0..3).all(|i| (a[i] - b[i]).abs() <= tol);
    let mut coverage = std::collections::BTreeMap::<usize, Vec<(f64, f64)>>::new();
    for index in edges {
        let edge = &source.edges[*index];
        if edge.curve.degree != 1 {
            return Err(denied("Selected source edges must be straight"));
        }
        let [a, b] = edge.vertices.map(|i| source.vertices[i].point);
        let matches: Vec<_> = envelope
            .edges
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                let [c, d] = e.vertices.map(|j| envelope.vertices[j].point);
                let vertical = same([a[0], a[1], 0.], [c[0], c[1], 0.])
                    && same([b[0], b[1], 0.], [d[0], d[1], 0.])
                    && same([c[0], c[1], 0.], [d[0], d[1], 0.]);
                (vertical
                    && a[2].min(b[2]) >= c[2].min(d[2]) - tol
                    && a[2].max(b[2]) <= c[2].max(d[2]) + tol
                    && (a[2] - b[2]).abs() > tol)
                    .then_some(i)
            })
            .collect();
        if matches.len() != 1 {
            return Err(denied("Selected edge must span a complete envelope corner"));
        }
        coverage
            .entry(matches[0])
            .or_default()
            .push((a[2].min(b[2]), a[2].max(b[2])));
    }
    for spans in coverage.values_mut() {
        spans.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut end = profile.z_min;
        for &(low, high) in spans.iter() {
            if (low - end).abs() > tol {
                return Err(denied(
                    "Select the complete corner chain without gaps or overlaps",
                ));
            }
            end = high;
        }
        if (end - profile.z_max).abs() > tol {
            return Err(denied("Select the complete corner chain"));
        }
    }
    let envelope_edges: Vec<_> = coverage.keys().copied().collect();
    let rounded = exact_convex_prism_fillet(envelope, &envelope_edges, radius)?.model;
    let rounded_profile = crate::prism::recognize(&rounded)?
        .ok_or_else(|| denied("Rounded envelope lost its prismatic proof"))?;
    let removed =
        crate::planar_trim::boolean(&profile.loops, &rounded_profile.loops, "difference", tol)?;
    for layer in &layers {
        if !crate::planar_trim::boolean(&layer.profile, &profile.loops, "difference", tol)?
            .is_empty()
        {
            return Err(denied("Envelope does not contain the complete source"));
        }
        if !crate::planar_trim::boolean(&removed, &layer.profile, "difference", tol)?.is_empty() {
            return Err(denied(
                "Fillet would intersect a cavity or missing material",
            ));
        }
    }
    let mut result = boolean(source, &rounded, "intersection")?;
    result.inherit_topology_ids(&[source]);
    audit_solid(&result)?;
    Ok(result)
}
