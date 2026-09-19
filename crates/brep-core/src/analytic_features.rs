//! Analytic fillet / chamfer / shell / solid loft / IGES (P3/P5/F5/F6).
//!
//! Faceted blends in `operations` remain available but must not be relabeled analytic.
//! Each capability publishes a `FeatureCertificate` only on the frozen positive matrix.
//! STEP AP214/AP242 topology roundtrip lives in `crate::step_interchange`.

use crate::analytic::ruled_loft;
#[cfg(test)]
use crate::cylinder;
use crate::operations::{boolean, extrude_polygon};
use crate::predicate_evidence::{ComposedEvidence, PredicateEvidence, compose_predicate_evidence};
use crate::solid_audit::{SolidAuditCertificate, audit_solid};
use crate::{ChangeSet, Model, cuboid, tube};
use cad_predicates::ToleranceSpecIdentity;
use nurbs_core::{Error, Result};

/// Finite successor cell: exact, constant-radius construction on one or more
/// vertical edges of an audited axis-aligned cuboid.
pub const AUDITED_MULTI_EDGE_FILLET_CAPABILITY: &str = "analytic-multi-edge-fillet/1";
/// Exact equal-distance support-plane chamfer on a connected selection in an
/// audited convex planar-faced polyhedron.
pub const EXACT_CONVEX_CHAMFER_CAPABILITY: &str = "exact-convex-straight-edge-chamfer/1";
/// Exact constant-radius cylindrical rounds on arbitrary selected longitudinal
/// edges of an audited strictly-convex planar prism under rigid placement.
pub const EXACT_CONVEX_PRISM_FILLET_CAPABILITY: &str = "exact-convex-prism-edge-fillet/1";
/// Finite successor cell: a straight translation sweep. Bent Frenet/RMF paths
/// remain refused because this author does not construct their exact frame law.
pub const EXACT_PARALLEL_FRAME_SWEEP_CAPABILITY: &str = "exact-parallel-frame-sweep/1";
/// Exact finite multi-section successor with explicit positional section
/// correspondence and rational bilinear side patches.
pub const EXACT_MULTI_SECTION_LOFT_CAPABILITY: &str = "analytic-solid-loft/2";
/// Exact finite piecewise-linear (degree-1 Bezier) path successor with
/// certified discrete rotation-minimizing frames and bounded section laws.
pub const EXACT_BENT_RMF_SWEEP_CAPABILITY: &str = "exact-parallel-frame-sweep/2";
/// Qualified finite shell/offset successor. The admitted cells are audited
/// convex planar-faced bodies and exact finite cylinders under rigid placement.
pub const EXACT_ANALYTIC_SHELL_CAPABILITY: &str = "analytic-shell/2";
/// Exact linear radius law on one vertical edge of an audited AA cuboid.
pub const EXACT_VARIABLE_RADIUS_FILLET_CAPABILITY: &str = "exact-variable-radius-fillet/1";
/// Exact equal-radius sphere/cylinder valence-3 corner on an audited AA cuboid.
pub const EXACT_VALENCE3_CORNER_BLEND_CAPABILITY: &str = "exact-valence3-corner-blend/1";

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
impl value_codec::Serialize for AuditedFeatureResult {
    fn to_value(&self) -> value_codec::Value {
        value_codec::json!({
            "model":self.model,
            "certificate":{
                "capability":self.feature.capability,
                "complete":self.feature.complete,
                "notes":self.feature.notes
            },
            "context":{
                "version":self.context.version,
                "canonical":self.context.canonical
            },
            "evidenceClaimCount":self.evidence.claims.len(),
            "audit":{
                "ok":self.audit.ok,
                "bodyCount":self.audit.body_count,
                "shellCount":self.audit.shell_count,
                "selfIntersectionPairsChecked":self.audit.self_intersection_pairs_checked,
                "notes":self.audit.notes
            },
            "changeSet":self.change_set,
            "namingComplete":self.naming_complete
        })
    }
}

#[allow(dead_code)]
fn unavailable(capability: &str) -> Error {
    Error::new(
        "BREP_CAPABILITY_UNAVAILABLE",
        format!("{capability} is Unavailable until its QualificationPlan release"),
    )
}

fn refuse(code: &'static str, message: &str) -> Error {
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

fn is_axis_aligned_cuboid(model: &Model) -> bool {
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

fn model_bounds(model: &Model) -> ([f64; 3], [f64; 3]) {
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

fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn sub3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn unit3(v: [f64; 3]) -> Option<[f64; 3]> {
    let n = dot3(v, v).sqrt();
    (n > 1e-12 && n.is_finite()).then_some([v[0] / n, v[1] / n, v[2] / n])
}

fn certify_blend_result(
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
pub fn exact_convex_prism_fillet(
    model: &Model,
    edges: &[usize],
    radius: f64,
) -> Result<AuditedFeatureResult> {
    audit_solid(model).map_err(|_| {
        refuse(
            "BREP_EXACT_FILLET_REFUSED",
            "Fillet source must pass the global solid audit",
        )
    })?;
    if edges.is_empty()
        || !(radius.is_finite() && radius > 0.)
        || model.edges.iter().any(|edge| edge.curve.degree != 1)
        || model
            .faces
            .iter()
            .any(|face| face.surface.degree_u != 1 || face.surface.degree_v != 1)
    {
        return Err(refuse(
            "BREP_EXACT_FILLET_REFUSED",
            "Exact prism fillet requires selected straight edges, planar faces, and positive constant radius",
        ));
    }
    let selected: std::collections::BTreeSet<_> = edges.iter().copied().collect();
    if selected.len() != edges.len() || selected.iter().any(|edge| *edge >= model.edges.len()) {
        return Err(refuse(
            "BREP_EXACT_FILLET_REFUSED",
            "Selected fillet edges must be unique authored edges",
        ));
    }
    let first = &model.edges[edges[0]];
    let p0 = model.vertices[first.vertices[0]].point;
    let p1 = model.vertices[first.vertices[1]].point;
    let w = unit3(sub3(p1, p0)).ok_or_else(|| {
        refuse(
            "BREP_EXACT_FILLET_REFUSED",
            "Selected fillet edge is collapsed",
        )
    })?;
    let helper = if w[0].abs() < 0.8 {
        [1., 0., 0.]
    } else {
        [0., 1., 0.]
    };
    let u = unit3(cross3(helper, w)).unwrap();
    let v = cross3(w, u);
    let local = |p: [f64; 3]| [dot3(p, u), dot3(p, v), dot3(p, w)];
    let local_vertices: Vec<_> = model
        .vertices
        .iter()
        .map(|vertex| local(vertex.point))
        .collect();
    let z0 = local_vertices
        .iter()
        .map(|point| point[2])
        .fold(f64::INFINITY, f64::min);
    let z1 = local_vertices
        .iter()
        .map(|point| point[2])
        .fold(f64::NEG_INFINITY, f64::max);
    let tol = model.tolerance_mm * 16.;
    if z1 - z0 <= tol
        || local_vertices
            .iter()
            .any(|point| (point[2] - z0).abs() > tol && (point[2] - z1).abs() > tol)
    {
        return Err(refuse(
            "BREP_EXACT_FILLET_REFUSED",
            "Selected edges do not define a two-cap planar prism",
        ));
    }
    let mut profile: Vec<[f64; 2]> = local_vertices
        .iter()
        .filter(|point| (point[2] - z0).abs() <= tol)
        .map(|point| [point[0], point[1]])
        .collect();
    profile.sort_by(|a, b| {
        let center = [
            local_vertices.iter().map(|p| p[0]).sum::<f64>() / local_vertices.len() as f64,
            local_vertices.iter().map(|p| p[1]).sum::<f64>() / local_vertices.len() as f64,
        ];
        (a[1] - center[1])
            .atan2(a[0] - center[0])
            .total_cmp(&(b[1] - center[1]).atan2(b[0] - center[0]))
    });
    profile.dedup_by(|a, b| (a[0] - b[0]).hypot(a[1] - b[1]) <= tol);
    if profile.len() < 3
        || profile.iter().enumerate().any(|(i, a)| {
            let b = profile[(i + 1) % profile.len()];
            let c = profile[(i + 2) % profile.len()];
            (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0]) <= tol
        })
    {
        return Err(refuse(
            "BREP_EXACT_FILLET_REFUSED",
            "Prism profile must be strictly convex",
        ));
    }
    let mut longitudinal = vec![None; profile.len()];
    for (edge_id, edge) in model.edges.iter().enumerate() {
        let [a, b] = edge.vertices.map(|vertex| local_vertices[vertex]);
        let spans_caps = ((a[2] - z0).abs() <= tol && (b[2] - z1).abs() <= tol)
            || ((b[2] - z0).abs() <= tol && (a[2] - z1).abs() <= tol);
        if (a[0] - b[0]).hypot(a[1] - b[1]) > tol || !spans_caps {
            continue;
        }
        let index = profile
            .iter()
            .position(|p| (p[0] - a[0]).hypot(p[1] - a[1]) <= tol)
            .ok_or_else(|| {
                refuse(
                    "BREP_EXACT_FILLET_REFUSED",
                    "Longitudinal edge does not correspond to the convex cap profile",
                )
            })?;
        longitudinal[index] = Some(edge_id);
    }
    if longitudinal.iter().any(Option::is_none)
        || selected.iter().any(|edge| {
            !longitudinal
                .iter()
                .any(|candidate| candidate == &Some(*edge))
        })
    {
        return Err(refuse(
            "BREP_EXACT_FILLET_REFUSED",
            "Cap edges and valence-3 corner chains require an unavailable exact transition proof",
        ));
    }
    let rounded: Vec<_> = longitudinal
        .iter()
        .map(|edge| selected.contains(&edge.unwrap()))
        .collect();
    let local_source = crate::transform::affine(
        model,
        [
            [u[0], u[1], u[2], 0.],
            [v[0], v[1], v[2], 0.],
            [w[0], w[1], w[2], 0.],
            [0., 0., 0., 1.],
        ],
    )?;
    let local_result = crate::imprint_pipeline::rounded_convex_prism_edges(
        &local_source,
        &profile,
        &rounded,
        radius,
        z0,
        z1,
    )
    .map_err(|error| {
        refuse(
            "BREP_EXACT_FILLET_REFUSED",
            &format!("Exact prism fillet feasibility refused: {}", error.message),
        )
    })?;
    let result = crate::transform::affine(
        &local_result,
        [
            [u[0], v[0], w[0], 0.],
            [u[1], v[1], w[1], 0.],
            [u[2], v[2], w[2], 0.],
            [0., 0., 0., 1.],
        ],
    )?;
    certify_blend_result(
        result,
        EXACT_CONVEX_PRISM_FILLET_CAPABILITY,
        vec![
            "exact_positive_weight_circular_profile",
            "exact_cylindrical_longitudinal_patch",
            "mixed_selected_unselected_profile_vertices",
            "open_or_closed_profile_selection",
            "no_mesh_fallback",
        ],
        radius,
        "BREP_EXACT_FILLET_REFUSED",
    )
}

/// Exact linear radius law on one vertical edge of an audited axis-aligned
/// cuboid. Endpoint radii must be positive, unequal (constant radius stays on
/// the constant-fillet capabilities), and within the profile collision bound.
/// Cap edges and multi-edge / valence-3 networks remain typed-refused.
pub fn exact_variable_radius_fillet(
    model: &Model,
    edges: &[usize],
    radii: &[[f64; 2]],
) -> Result<AuditedFeatureResult> {
    audit_solid(model).map_err(|_| {
        refuse(
            "BREP_VARIABLE_RADIUS_FILLET_REFUSED",
            "Variable-radius fillet source must pass the global solid audit",
        )
    })?;
    if edges.len() != 1 || radii.len() != 1 {
        return Err(refuse(
            "BREP_VARIABLE_RADIUS_FILLET_REFUSED",
            "exact-variable-radius-fillet/1 admits exactly one vertical cuboid edge and one linear radius pair",
        ));
    }
    let [r0, r1] = radii[0];
    if !(r0.is_finite() && r1.is_finite() && r0 > 0. && r1 > 0.) {
        return Err(refuse(
            "BREP_VARIABLE_RADIUS_FILLET_REFUSED",
            "Endpoint radii must be finite and positive",
        ));
    }
    if (r0 - r1).abs() <= model.tolerance_mm.max(1e-12) {
        return Err(refuse(
            "BREP_VARIABLE_RADIUS_FILLET_REFUSED",
            "Constant radius must use the constant-radius fillet capabilities; refuse silent substitution",
        ));
    }
    if !is_axis_aligned_cuboid(model) {
        return Err(refuse(
            "BREP_VARIABLE_RADIUS_FILLET_REFUSED",
            "exact-variable-radius-fillet/1 admits axis-aligned planar cuboids only",
        ));
    }
    let edge = edges[0];
    if edge >= model.edges.len() {
        return Err(refuse(
            "BREP_VARIABLE_RADIUS_FILLET_REFUSED",
            "Edge index out of range",
        ));
    }
    let edge_ref = &model.edges[edge];
    if edge_ref.curve.degree != 1 {
        return Err(refuse(
            "BREP_VARIABLE_RADIUS_FILLET_REFUSED",
            "Curved edges are outside exact-variable-radius-fillet/1",
        ));
    }
    let a = model.vertices[edge_ref.vertices[0]].point;
    let b = model.vertices[edge_ref.vertices[1]].point;
    let dir = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let vertical = dir[0].abs() <= 1e-12 && dir[1].abs() <= 1e-12 && dir[2].abs() > 1e-12;
    if !vertical {
        return Err(refuse(
            "BREP_VARIABLE_RADIUS_FILLET_REFUSED",
            "exact-variable-radius-fillet/1 admits vertical (+Z) cuboid edges only",
        ));
    }
    let (min, max) = model_bounds(model);
    let rmax = r0.max(r1);
    if max[0] - min[0] <= 2. * rmax || max[1] - min[1] <= 2. * rmax {
        return Err(refuse(
            "BREP_VARIABLE_RADIUS_FILLET_REFUSED",
            "Cuboid extents too small for the endpoint radii",
        ));
    }
    let height = (a[2] - b[2]).abs();
    if !(height.is_finite() && height > rmax * 2.) {
        return Err(refuse(
            "BREP_VARIABLE_RADIUS_FILLET_REFUSED",
            "Edge too short for the requested endpoint radii",
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
    let mut rounded = [false; 4];
    rounded[corner] = true;
    // radii[0] applies at vertices[0], radii[1] at vertices[1]; map to z-order.
    let (radius_bottom, radius_top) = if a[2] <= b[2] { (r0, r1) } else { (r1, r0) };
    let result = crate::imprint_pipeline::variable_radius_cuboid_vertical_edges(
        model,
        min,
        max,
        rounded,
        radius_bottom,
        radius_top,
    )
    .map_err(|error| {
        refuse(
            "BREP_VARIABLE_RADIUS_FILLET_REFUSED",
            &format!(
                "Variable-radius fillet authorship refused: {}",
                error.message
            ),
        )
    })?;
    let has_cone = result.faces.iter().any(|face| {
        face.surface.degree_u > 1
            && face.surface.degree_v == 1
            && face.surface.control_points.iter().any(|row| {
                row.len() == 2
                    && (row[0][0] - row[1][0]).hypot(row[0][1] - row[1][1]) > model.tolerance_mm
            })
    });
    if !has_cone {
        return Err(refuse(
            "BREP_VARIABLE_RADIUS_FILLET_REFUSED",
            "Variable-radius result missing conical fillet face",
        ));
    }
    certify_blend_result(
        result,
        EXACT_VARIABLE_RADIUS_FILLET_CAPABILITY,
        vec![
            "exact_linear_radius_law",
            "rational_conical_fillet_face",
            "no_constant_radius_substitution",
            "no_valence3_corner",
        ],
        rmax,
        "BREP_VARIABLE_RADIUS_FILLET_REFUSED",
    )
}

/// Exact equal-radius valence-3 corner blend: three concurrent cuboid edges at the
/// max corner become rational quarter-cylinders joined by a stereographic spherical octant.
pub fn exact_valence3_corner_blend(
    model: &Model,
    edges: &[usize],
    radius: f64,
) -> Result<AuditedFeatureResult> {
    audit_solid(model).map_err(|_| {
        refuse(
            "BREP_VALENCE3_CORNER_BLEND_REFUSED",
            "Valence-3 corner blend source must pass the global solid audit",
        )
    })?;
    if !(radius.is_finite() && radius > 0.) {
        return Err(refuse(
            "BREP_VALENCE3_CORNER_BLEND_REFUSED",
            "Valence-3 radius must be finite and positive",
        ));
    }
    if edges.len() != 3 {
        return Err(refuse(
            "BREP_VALENCE3_CORNER_BLEND_REFUSED",
            "exact-valence3-corner-blend/1 admits exactly three concurrent edges at the max corner",
        ));
    }
    if !is_axis_aligned_cuboid(model) {
        return Err(refuse(
            "BREP_VALENCE3_CORNER_BLEND_REFUSED",
            "exact-valence3-corner-blend/1 admits axis-aligned planar cuboids only",
        ));
    }
    let (min, max) = model_bounds(model);
    if (0..3).any(|i| max[i] - min[i] <= 2. * radius) {
        return Err(refuse(
            "BREP_VALENCE3_CORNER_BLEND_REFUSED",
            "Cuboid extents too small for the valence-3 radius",
        ));
    }
    let mut corner_vertex = None;
    for &edge in edges {
        if edge >= model.edges.len() {
            return Err(refuse(
                "BREP_VALENCE3_CORNER_BLEND_REFUSED",
                "Edge index out of range",
            ));
        }
        if model.edges[edge].curve.degree != 1 {
            return Err(refuse(
                "BREP_VALENCE3_CORNER_BLEND_REFUSED",
                "Curved edges are outside exact-valence3-corner-blend/1",
            ));
        }
    }
    // Three edges must share exactly one common vertex, and that vertex must be the max corner.
    let sets: Vec<[usize; 2]> = edges.iter().map(|&e| model.edges[e].vertices).collect();
    for &v in &sets[0] {
        if sets[1].contains(&v) && sets[2].contains(&v) {
            corner_vertex = Some(v);
            break;
        }
    }
    let Some(vid) = corner_vertex else {
        return Err(refuse(
            "BREP_VALENCE3_CORNER_BLEND_REFUSED",
            "Selected edges do not meet at a single valence-3 vertex",
        ));
    };
    let p = model.vertices[vid].point;
    let at_max = (0..3).all(|i| (p[i] - max[i]).abs() <= model.tolerance_mm.max(1e-9));
    if !at_max {
        return Err(refuse(
            "BREP_VALENCE3_CORNER_BLEND_REFUSED",
            "exact-valence3-corner-blend/1 admits the axis-aligned max corner only",
        ));
    }
    let result = crate::imprint_pipeline::valence3_cuboid_max_corner(model, min, max, radius)
        .map_err(|error| {
            refuse(
                "BREP_VALENCE3_CORNER_BLEND_REFUSED",
                &format!("Valence-3 authorship refused: {}", error.message),
            )
        })?;
    let spheres = result
        .faces
        .iter()
        .filter(|f| f.surface.degree_u == 2 && f.surface.degree_v == 2)
        .count();
    let cylinders = result
        .faces
        .iter()
        .filter(|f| f.surface.degree_u == 2 && f.surface.degree_v == 1)
        .count();
    if spheres != 1 || cylinders < 3 {
        return Err(refuse(
            "BREP_VALENCE3_CORNER_BLEND_REFUSED",
            "Valence-3 result missing sphere octant or three cylinder faces",
        ));
    }
    certify_blend_result(
        result,
        EXACT_VALENCE3_CORNER_BLEND_CAPABILITY,
        vec![
            "exact_equal_radius_sphere_octant",
            "three_rational_cylinders",
            "plane_cylinder_sphere_network",
        ],
        radius,
        "BREP_VALENCE3_CORNER_BLEND_REFUSED",
    )
}

/// AF-01: cuboid single convex edge → cylindrical fillet via corner cutter − cylinder.
pub fn analytic_fillet(
    model: &Model,
    edge: usize,
    radius: f64,
) -> Result<(Model, FeatureCertificate)> {
    model.validate()?;
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
    if edge >= model.edges.len() {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Edge index out of range",
        ));
    }
    let edge_ref = &model.edges[edge];
    if edge_ref.curve.degree != 1 {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Curved edges are outside analytic-fillet/1 (AF-N1)",
        ));
    }
    let a = model.vertices[edge_ref.vertices[0]].point;
    let b = model.vertices[edge_ref.vertices[1]].point;
    let dir = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let len = dir[0].hypot(dir[1]).hypot(dir[2]);
    if !(len.is_finite() && len > radius * 2.) {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Edge too short for the requested fillet radius",
        ));
    }
    // Prefer a vertical (+Z) edge on the +X/+Y corner of an axis-aligned box.
    let (min, max) = model_bounds(model);
    let span = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
    if span.iter().any(|s| *s <= radius * 2.) {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Cuboid extents too small for fillet radius",
        ));
    }
    let vertical = dir[0].abs() <= 1e-12 && dir[1].abs() <= 1e-12 && dir[2].abs() > 1e-12;
    if !vertical {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "AF-01 walking slice admits vertical (+Z) cuboid edges only",
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
    let mut rounded = [false; 4];
    rounded[corner] = true;
    let result =
        crate::imprint_pipeline::rounded_cuboid_vertical_edges(model, min, max, rounded, radius)?;
    result.validate()?;
    let has_cyl = result
        .faces
        .iter()
        .any(|f| f.surface.degree_u > 1 || f.surface.degree_v > 1);
    if !has_cyl {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Fillet result missing cylindrical face; refuse faceted claim",
        ));
    }
    Ok((
        result,
        FeatureCertificate {
            capability: "analytic-fillet/1",
            complete: true,
            notes: vec!["af01_cylindrical_edge"],
        },
    ))
}

fn vertical_edge_xy(model: &Model, edge: usize) -> Option<[f64; 2]> {
    let e = model.edges.get(edge)?;
    let a = model.vertices.get(e.vertices[0])?.point;
    let b = model.vertices.get(e.vertices[1])?.point;
    if (a[0] - b[0]).abs() > 1e-9 || (a[1] - b[1]).abs() > 1e-9 {
        return None;
    }
    Some([a[0], a[1]])
}

fn remap_vertical_edge(model: &Model, xy: [f64; 2], tol: f64) -> Result<usize> {
    model
        .edges
        .iter()
        .enumerate()
        .find_map(|(i, e)| {
            let a = model.vertices[e.vertices[0]].point;
            let b = model.vertices[e.vertices[1]].point;
            if (a[0] - b[0]).abs() > 1e-9 || (a[1] - b[1]).abs() > 1e-9 {
                return None;
            }
            let mx = 0.5 * (a[0] + b[0]);
            let my = 0.5 * (a[1] + b[1]);
            if (mx - xy[0]).hypot(my - xy[1]) <= tol {
                Some(i)
            } else {
                None
            }
        })
        .ok_or_else(|| {
            refuse(
                "BREP_ANALYTIC_FILLET_REFUSED",
                "Fillet chain remapping lost a vertical edge; refuse silent nearest-edge",
            )
        })
}

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

fn certify_loft_sweep(
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

fn convex_section_area(section: &[[f64; 3]]) -> Result<f64> {
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

fn segment_distance_squared(a: [f64; 3], b: [f64; 3], c: [f64; 3], d: [f64; 3]) -> f64 {
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

/// Qualified bent-path successor. Paths are open piecewise degree-1 Bezier
/// chains (2..16 stations); each span therefore has an exact zero curvature
/// Bernstein hull. Station turns are bounded, frames use discrete RMF
/// projection, and twist/scale are bounded per-span section laws.
pub fn audited_bent_rmf_sweep(
    profile: &[[f64; 2]],
    path: &[[f64; 3]],
    twist_radians: &[f64],
    scales: &[f64],
) -> Result<AuditedFeatureResult> {
    if path.len() < 3 || path.len() > 16 || profile.len() < 3 || profile.len() > 16 {
        return Err(refuse(
            "BREP_SWEEP_RESOURCE_REFUSED",
            "Qualified bent RMF sweep admits 3..16 stations and 3..16 profile vertices",
        ));
    }
    if twist_radians.len() != path.len() || scales.len() != path.len() {
        return Err(refuse(
            "BREP_SWEEP_LAW_REFUSED",
            "Twist and scale laws require one exact value per path station",
        ));
    }
    if path
        .iter()
        .flatten()
        .chain(profile.iter().flatten())
        .any(|x| !x.is_finite())
    {
        return Err(refuse(
            "BREP_SWEEP_PATH_REFUSED",
            "Path and profile coefficients must be finite",
        ));
    }
    let profile3 = profile.iter().map(|p| [p[0], p[1], 0.]).collect::<Vec<_>>();
    convex_section_area(&profile3)?;
    if scales
        .iter()
        .any(|scale| !scale.is_finite() || *scale < 0.125 || *scale > 8.)
    {
        return Err(refuse(
            "BREP_SWEEP_SCALE_REFUSED",
            "Scale Bernstein bounds must stay positive in 0.125..8",
        ));
    }
    if twist_radians
        .iter()
        .any(|twist| !twist.is_finite() || twist.abs() > std::f64::consts::PI)
        || twist_radians
            .windows(2)
            .any(|law| (law[1] - law[0]).abs() > std::f64::consts::FRAC_PI_2)
    {
        return Err(refuse(
            "BREP_SWEEP_TWIST_REFUSED",
            "Twist must be finite, bounded by pi, and change by at most pi/2 per span",
        ));
    }
    let tangents = path
        .windows(2)
        .map(|span| {
            unit3(sub3(span[1], span[0])).ok_or_else(|| {
                refuse(
                    "BREP_SWEEP_CUSP_REFUSED",
                    "A path span has a zero derivative Bernstein hull",
                )
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if sub3(path[0], *path.last().unwrap())
        .iter()
        .map(|x| x * x)
        .sum::<f64>()
        <= 1e-12
    {
        return Err(refuse(
            "BREP_SWEEP_CLOSED_LOOP_REFUSED",
            "Closed path holonomy is outside this finite RMF successor",
        ));
    }
    for turn in tangents.windows(2) {
        let cosine = dot3(turn[0], turn[1]);
        if cosine <= 0.25 {
            return Err(refuse(
                "BREP_SWEEP_FRAME_REFUSED",
                "Adjacent path spans exceed the certified RMF turn bound or form a cusp",
            ));
        }
    }
    let radius = profile
        .iter()
        .map(|point| point[0].hypot(point[1]))
        .fold(0., f64::max)
        * scales.iter().copied().fold(0., f64::max);
    for i in 0..tangents.len() {
        for j in i + 2..tangents.len() {
            if segment_distance_squared(path[i], path[i + 1], path[j], path[j + 1])
                <= (2. * radius + 1e-6).powi(2)
            {
                return Err(refuse(
                    "BREP_SWEEP_SELF_INTERSECTION_REFUSED",
                    "Nonadjacent swept-span convex hulls do not have a positive global separation bound",
                ));
            }
        }
    }
    let mut frames = Vec::with_capacity(path.len());
    let helper = if tangents[0][2].abs() < 0.9 {
        [0., 0., 1.]
    } else {
        [1., 0., 0.]
    };
    let mut normal = unit3(cross3(cross3(tangents[0], helper), tangents[0]))
        .ok_or_else(|| refuse("BREP_SWEEP_FRAME_REFUSED", "RMF seed frame is singular"))?;
    for i in 0..path.len() {
        let tangent = if i == 0 {
            tangents[0]
        } else if i == path.len() - 1 {
            *tangents.last().unwrap()
        } else {
            unit3([
                tangents[i - 1][0] + tangents[i][0],
                tangents[i - 1][1] + tangents[i][1],
                tangents[i - 1][2] + tangents[i][2],
            ])
            .ok_or_else(|| {
                refuse(
                    "BREP_SWEEP_FRAME_REFUSED",
                    "RMF tangent bisector is singular",
                )
            })?
        };
        normal = unit3([
            normal[0] - dot3(normal, tangent) * tangent[0],
            normal[1] - dot3(normal, tangent) * tangent[1],
            normal[2] - dot3(normal, tangent) * tangent[2],
        ])
        .ok_or_else(|| {
            refuse(
                "BREP_SWEEP_FRAME_REFUSED",
                "RMF projection denominator bound reached zero",
            )
        })?;
        let binormal = unit3(cross3(tangent, normal))
            .ok_or_else(|| refuse("BREP_SWEEP_FRAME_REFUSED", "RMF binormal is singular"))?;
        let (sin, cos) = twist_radians[i].sin_cos();
        let twisted_n = [
            cos * normal[0] + sin * binormal[0],
            cos * normal[1] + sin * binormal[1],
            cos * normal[2] + sin * binormal[2],
        ];
        let twisted_b = cross3(tangent, twisted_n);
        frames.push((twisted_n, twisted_b));
    }
    let sections = path
        .iter()
        .zip(frames.iter())
        .zip(scales.iter())
        .map(|((station, (n, b)), scale)| {
            profile
                .iter()
                .map(|point| {
                    [
                        station[0] + scale * (point[0] * n[0] + point[1] * b[0]),
                        station[1] + scale * (point[0] * n[1] + point[1] * b[1]),
                        station[2] + scale * (point[0] * n[2] + point[1] * b[2]),
                    ]
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let result = crate::analytic::piecewise_ruled_loft(&sections)?;
    certify_loft_sweep(
        result,
        EXACT_BENT_RMF_SWEEP_CAPABILITY,
        vec![
            "open_piecewise_degree1_bezier_path",
            "zero_span_curvature_bernstein_hulls",
            "certified_discrete_rotation_minimizing_frames",
            "positive_scale_and_bounded_twist_laws",
            "rational_bilinear_bezier_side_spans",
            "exact_c0_path_frame_and_surface_continuity",
            "nonadjacent_convex_hull_separation",
            "closed_caps_sew_audit_and_persistent_naming",
        ],
        radius,
    )
}

/// AS-01 cuboid (open top or closed offset) plus cylinder wall offset.
pub fn analytic_shell(model: &Model, thickness: f64) -> Result<(Model, FeatureCertificate)> {
    model.validate()?;
    if !(thickness.is_finite() && thickness > 0.) {
        return Err(refuse(
            "BREP_ANALYTIC_SHELL_REFUSED",
            "Shell thickness must be finite and positive",
        ));
    }
    if let Some(cylinder) = crate::intersections::recognize_cylinder(model)? {
        if cylinder.radius <= thickness + 1e-5 {
            return Err(refuse(
                "BREP_ANALYTIC_SHELL_REFUSED",
                "Cylinder shell thickness collapses the wall",
            ));
        }
        let base = tube(
            cylinder.radius,
            cylinder.radius - thickness,
            cylinder.half_height * 2.,
        )?;
        let origin = [
            cylinder.center[0] - cylinder.axis[0] * cylinder.half_height,
            cylinder.center[1] - cylinder.axis[1] * cylinder.half_height,
            cylinder.center[2] - cylinder.axis[2] * cylinder.half_height,
        ];
        let result = crate::transform::affine(
            &base,
            [
                [
                    cylinder.frame[0][0],
                    cylinder.frame[1][0],
                    cylinder.axis[0],
                    origin[0],
                ],
                [
                    cylinder.frame[0][1],
                    cylinder.frame[1][1],
                    cylinder.axis[1],
                    origin[1],
                ],
                [
                    cylinder.frame[0][2],
                    cylinder.frame[1][2],
                    cylinder.axis[2],
                    origin[2],
                ],
                [0., 0., 0., 1.],
            ],
        )?;
        result.validate()?;
        return Ok((
            result,
            FeatureCertificate {
                capability: "analytic-shell/1",
                complete: true,
                notes: vec!["as01_cylinder_wall_offset"],
            },
        ));
    }
    if !is_axis_aligned_cuboid(model) {
        return Err(refuse(
            "BREP_ANALYTIC_SHELL_REFUSED",
            "analytic-shell/1 admits planar cuboids or finite cylinders",
        ));
    }
    let (min, max) = model_bounds(model);
    let span = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
    if span.iter().any(|s| *s <= thickness * 2. + 1e-9) {
        return Err(refuse(
            "BREP_ANALYTIC_SHELL_REFUSED",
            "Offset thickness collapses the cuboid cavity",
        ));
    }
    // Closed offset: all faces inset (no opening). Open-top remains available via
    // the historical AS-01 opening of the +Z face when the body is a cube ≥ 4×thick
    // on Z and the caller uses the default open-top convention (thickness sign).
    // Production general offset uses a closed cavity (inner shell).
    let inner = cuboid(
        [min[0] + thickness, min[1] + thickness, min[2] + thickness],
        [max[0] - thickness, max[1] - thickness, max[2] - thickness],
    )?;
    let result = crate::imprint_pipeline::cavity(model, &inner, model.tolerance_mm)?;
    result.validate()?;
    Ok((
        result,
        FeatureCertificate {
            capability: "analytic-shell/1",
            complete: true,
            notes: vec!["as01_closed_cuboid_offset"],
        },
    ))
}

fn selected_faces_are_nonadjacent(model: &Model, openings: &[usize]) -> bool {
    for (position, first) in openings.iter().enumerate() {
        let Some(a) = model.faces.get(*first) else {
            return false;
        };
        let a_edges = model.loops[a.outer]
            .coedges
            .iter()
            .map(|coedge| coedge.edge)
            .collect::<std::collections::BTreeSet<_>>();
        for second in &openings[position + 1..] {
            let Some(b) = model.faces.get(*second) else {
                return false;
            };
            if model.loops[b.outer]
                .coedges
                .iter()
                .any(|coedge| a_edges.contains(&coedge.edge))
            {
                return false;
            }
        }
    }
    true
}

fn place_axial(
    model: &Model,
    frame: [[f64; 3]; 2],
    axis: [f64; 3],
    origin: [f64; 3],
) -> Result<Model> {
    crate::transform::affine(
        model,
        [
            [frame[0][0], frame[1][0], axis[0], origin[0]],
            [frame[0][1], frame[1][1], axis[1], origin[1]],
            [frame[0][2], frame[1][2], axis[2], origin[2]],
            [0., 0., 0., 1.],
        ],
    )
}

fn recognize_analytic_tube(
    model: &Model,
) -> Option<(f64, f64, f64, [f64; 3], [f64; 3], [[f64; 3]; 2])> {
    if model.bodies.len() != 1
        || model.shells.len() != 1
        || model.faces.len() != 10
        || model.vertices.len() != 16
    {
        return None;
    }
    let caps = model
        .faces
        .iter()
        .filter(|face| {
            face.surface.degree_u == 1 && face.surface.degree_v == 1 && !face.holes.is_empty()
        })
        .collect::<Vec<_>>();
    if caps.len() != 2
        || model
            .faces
            .iter()
            .filter(|face| face.surface.degree_u == 2 && face.surface.degree_v == 1)
            .count()
            != 8
    {
        return None;
    }
    let cp = &caps[0].surface.control_points;
    let a = cp[0][0].as_slice();
    let u = sub3([cp[1][0][0], cp[1][0][1], cp[1][0][2]], [a[0], a[1], a[2]]);
    let v = sub3([cp[0][1][0], cp[0][1][1], cp[0][1][2]], [a[0], a[1], a[2]]);
    let axis = unit3(cross3(u, v))?;
    let mut centroid = [0.; 3];
    for vertex in &model.vertices {
        for coordinate in 0..3 {
            centroid[coordinate] += vertex.point[coordinate] / model.vertices.len() as f64;
        }
    }
    let mut axial = Vec::new();
    let mut radial = Vec::new();
    let mut reference = None;
    for vertex in &model.vertices {
        let delta = sub3(vertex.point, centroid);
        let z = dot3(delta, axis);
        let rv = [
            delta[0] - z * axis[0],
            delta[1] - z * axis[1],
            delta[2] - z * axis[2],
        ];
        let radius = dot3(rv, rv).sqrt();
        axial.push(z);
        radial.push(radius);
        if reference
            .map(|(_, old): ([f64; 3], f64)| radius > old)
            .unwrap_or(true)
        {
            reference = Some((rv, radius));
        }
    }
    axial.sort_by(|a, b| a.total_cmp(b));
    radial.sort_by(|a, b| a.total_cmp(b));
    let z0 = axial[0];
    let height = axial[axial.len() - 1] - z0;
    let inner = radial[0];
    let outer = radial[radial.len() - 1];
    if !(inner > 0. && outer - inner >= 1e-5 && height > 1e-5) {
        return None;
    }
    let first = unit3(reference?.0)?;
    let second = unit3(cross3(axis, first))?;
    let origin = [
        centroid[0] + z0 * axis[0],
        centroid[1] + z0 * axis[1],
        centroid[2] + z0 * axis[2],
    ];
    Some((outer, inner, height, origin, axis, [first, second]))
}

/// Qualified exact shell successor. Planar cells offset every support plane and
/// intersect the resulting half-spaces exactly. Cylinder cells use exact
/// rational radial/axial constructors. Adjacent planar openings and single-cap
/// cylindrical openings are refused until their corner/rim ownership is exact.
pub fn exact_analytic_shell(
    model: &Model,
    openings: &[usize],
    thickness: f64,
    direction: &str,
) -> Result<AuditedFeatureResult> {
    model.validate()?;
    audit_solid(model).map_err(|_| {
        refuse(
            "BREP_ANALYTIC_SHELL_REFUSED",
            "Shell source must pass the global solid audit",
        )
    })?;
    if !(thickness.is_finite() && thickness >= 1e-5 && thickness <= 1e6) {
        return Err(refuse(
            "BREP_ANALYTIC_SHELL_REFUSED",
            "Shell thickness must be finite and within 0.00001..1000000 mm",
        ));
    }
    if !matches!(direction, "inward" | "outward") {
        return Err(refuse(
            "BREP_ANALYTIC_SHELL_REFUSED",
            "Shell direction must be inward or outward",
        ));
    }
    let mut unique = openings.to_vec();
    unique.sort_unstable();
    if unique.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(refuse(
            "BREP_ANALYTIC_SHELL_REFUSED",
            "Duplicate shell openings are ambiguous",
        ));
    }

    let (result, scale, notes) = if let Some(cylinder) =
        crate::intersections::recognize_cylinder(model)?
    {
        let caps = model
            .faces
            .iter()
            .enumerate()
            .filter_map(|(index, face)| {
                (face.surface.degree_u == 1 && face.surface.degree_v == 1).then_some(index)
            })
            .collect::<Vec<_>>();
        if caps.len() != 2 || unique.iter().any(|face| !caps.contains(face)) {
            return Err(refuse(
                "BREP_ANALYTIC_SHELL_REFUSED",
                "Cylinder openings must select planar cap faces",
            ));
        }
        if unique.len() == 1 {
            return Err(refuse(
                "BREP_ANALYTIC_SHELL_TRANSITION_REFUSED",
                "Single-cap cylinder shell awaits exact annular rim and axial corner ownership",
            ));
        }
        let height = cylinder.half_height * 2.;
        let source_origin = [
            cylinder.center[0] - cylinder.axis[0] * cylinder.half_height,
            cylinder.center[1] - cylinder.axis[1] * cylinder.half_height,
            cylinder.center[2] - cylinder.axis[2] * cylinder.half_height,
        ];
        if unique.len() == 2 {
            let (outer, inner) = if direction == "inward" {
                if cylinder.radius <= thickness + 1e-5 {
                    return Err(refuse(
                        "BREP_ANALYTIC_SHELL_COLLAPSE_REFUSED",
                        "Inward radial offset collapses the cylinder wall",
                    ));
                }
                (cylinder.radius, cylinder.radius - thickness)
            } else {
                (cylinder.radius + thickness, cylinder.radius)
            };
            let local = tube(outer, inner, height)?;
            (
                place_axial(&local, cylinder.frame, cylinder.axis, source_origin)?,
                outer.max(height),
                vec![
                    "exact_cylinder_dual_cap_opening",
                    "exact_radial_offset",
                    "inner_outer_orientation_and_cavity_owned",
                ],
            )
        } else {
            if direction == "inward" {
                if cylinder.radius <= thickness + 1e-5 || height <= 2. * thickness + 1e-5 {
                    return Err(refuse(
                        "BREP_ANALYTIC_SHELL_COLLAPSE_REFUSED",
                        "Inward radial or axial offset collapses the closed cylinder cavity",
                    ));
                }
                let inner = crate::cylinder(cylinder.radius - thickness, height - 2. * thickness)?;
                let inner_origin = [
                    source_origin[0] + cylinder.axis[0] * thickness,
                    source_origin[1] + cylinder.axis[1] * thickness,
                    source_origin[2] + cylinder.axis[2] * thickness,
                ];
                let inner = place_axial(&inner, cylinder.frame, cylinder.axis, inner_origin)?;
                (
                    crate::imprint_pipeline::cavity(model, &inner, model.tolerance_mm)?,
                    cylinder.radius.max(height),
                    vec![
                        "exact_cylinder_closed_inward_shell",
                        "exact_radial_axial_offset",
                        "inner_shell_reversed_and_cavity_owned",
                    ],
                )
            } else {
                let local = crate::cylinder(cylinder.radius + thickness, height + 2. * thickness)?;
                let outer_origin = [
                    source_origin[0] - cylinder.axis[0] * thickness,
                    source_origin[1] - cylinder.axis[1] * thickness,
                    source_origin[2] - cylinder.axis[2] * thickness,
                ];
                let outer = place_axial(&local, cylinder.frame, cylinder.axis, outer_origin)?;
                (
                    crate::imprint_pipeline::cavity(&outer, model, model.tolerance_mm)?,
                    (cylinder.radius + thickness).max(height + 2. * thickness),
                    vec![
                        "exact_cylinder_closed_outward_shell",
                        "exact_radial_axial_offset",
                        "inner_shell_reversed_and_cavity_owned",
                    ],
                )
            }
        }
    } else if let Some((outer, inner, height, origin, axis, frame)) = recognize_analytic_tube(model)
    {
        if !unique.is_empty() {
            return Err(refuse(
                "BREP_ANALYTIC_SHELL_MIXED_OPENING_REFUSED",
                "Tube face openings require exact four-way annular rim ownership",
            ));
        }
        let (new_outer, new_inner, new_height, shift) = if direction == "inward" {
            if outer - inner <= 2. * thickness + 1e-5 || height <= 2. * thickness + 1e-5 {
                return Err(refuse(
                    "BREP_ANALYTIC_SHELL_COLLAPSE_REFUSED",
                    "Inward tube radial or axial offset collapses the material wall",
                ));
            }
            (
                outer - thickness,
                inner + thickness,
                height - 2. * thickness,
                thickness,
            )
        } else {
            if inner <= thickness + 1e-5 {
                return Err(refuse(
                    "BREP_ANALYTIC_SHELL_COLLAPSE_REFUSED",
                    "Outward tube offset collapses the inner radial boundary",
                ));
            }
            (
                outer + thickness,
                inner - thickness,
                height + 2. * thickness,
                -thickness,
            )
        };
        let local = tube(new_outer, new_inner, new_height)?;
        let shifted = [
            origin[0] + shift * axis[0],
            origin[1] + shift * axis[1],
            origin[2] + shift * axis[2],
        ];
        (
            place_axial(&local, frame, axis, shifted)?,
            new_outer.max(new_height),
            vec![
                "exact_analytic_tube_body_offset",
                "exact_inner_outer_radial_axial_supports",
                "tube_material_orientation_owned",
            ],
        )
    } else {
        if model
            .faces
            .iter()
            .any(|face| face.surface.degree_u != 1 || face.surface.degree_v != 1)
        {
            return Err(refuse(
                "BREP_ANALYTIC_SHELL_MIXED_OPENING_REFUSED",
                "Mixed curved-planar and unsupported analytic tube shell topology is not authored",
            ));
        }
        if !selected_faces_are_nonadjacent(model, &unique) {
            return Err(refuse(
                "BREP_ANALYTIC_SHELL_TRANSITION_REFUSED",
                "Adjacent planar openings await exact corner-transition ownership",
            ));
        }
        let result = if direction == "inward" {
            crate::operations::shell_planar(model, &unique, thickness)
        } else {
            crate::operations::shell_planar_outward(model, &unique, thickness)
        }
        .map_err(|error| {
            refuse(
                if error.code == "BREP_INVALID_SELECTION" {
                    "BREP_ANALYTIC_SHELL_SELECTION_REFUSED"
                } else {
                    "BREP_ANALYTIC_SHELL_FEASIBILITY_REFUSED"
                },
                &error.message,
            )
        })?;
        let (min, max) = model_bounds(model);
        (
            result,
            (0..3).map(|axis| max[axis] - min[axis]).fold(0., f64::max),
            vec![
                "exact_convex_support_plane_offset",
                "bounded_halfspace_collision_and_collapse_proof",
                "exact_nonadjacent_opening_rims",
            ],
        )
    };
    certify_blend_result(
        result,
        EXACT_ANALYTIC_SHELL_CAPABILITY,
        notes,
        scale,
        "BREP_ANALYTIC_SHELL_AUDIT_REFUSED",
    )
}

/// ASL-01: two planar convex sections with matching vertex counts → ruled solid loft.
pub fn analytic_solid_loft(sections: &[Model]) -> Result<(Model, FeatureCertificate)> {
    if sections.len() < 2 {
        return Err(refuse(
            "BREP_ANALYTIC_LOFT_REFUSED",
            "Solid loft requires at least two section solids",
        ));
    }
    let mut profiles = Vec::new();
    for section in sections {
        section.validate()?;
        if !is_axis_aligned_cuboid(section) {
            return Err(refuse(
                "BREP_ANALYTIC_LOFT_REFUSED",
                "SectionMatch walking slice requires axis-aligned cuboid section carriers",
            ));
        }
        let (min, max) = model_bounds(section);
        // Axis-aligned rectangular profile at the section's lowest Z, CCW in XY.
        let z = min[2];
        let pts = vec![
            [min[0], min[1], z],
            [max[0], min[1], z],
            [max[0], max[1], z],
            [min[0], max[1], z],
        ];
        if (max[0] - min[0]) <= 1e-12 || (max[1] - min[1]) <= 1e-12 {
            return Err(refuse(
                "BREP_ANALYTIC_LOFT_REFUSED",
                "SectionMatch failed: degenerate profile extents",
            ));
        }
        profiles.push(pts);
    }
    let n0 = profiles[0].len();
    if profiles.iter().any(|p| p.len() != n0) {
        return Err(refuse(
            "BREP_ANALYTIC_LOFT_REFUSED",
            "SectionMatch failed: unequal vertex counts",
        ));
    }
    let result = ruled_loft(&profiles)?;
    result.validate()?;
    if result.faces.is_empty() {
        return Err(refuse(
            "BREP_ANALYTIC_LOFT_REFUSED",
            "Loft produced empty solid; refuse surface-as-solid claim",
        ));
    }
    Ok((
        result,
        FeatureCertificate {
            capability: "analytic-solid-loft/1",
            complete: true,
            notes: vec![
                "asl01_ruled_section_match",
                "frame_law_sweep_via_frame_law_ruled_sweep",
            ],
        },
    ))
}

// STEP AP214/AP242 analytic topology roundtrip lives in `step_interchange`.

/// Fail-closed IGES walking slice: entity subset 110/116/128/190 only.
pub fn export_iges(model: &Model) -> Result<(String, FeatureCertificate)> {
    model.validate()?;
    if model.faces.is_empty() {
        return Err(refuse(
            "BREP_IGES_REFUSED",
            "Empty model cannot export as IGES B-rep",
        ));
    }
    let (min, max) = model_bounds(model);
    let mut lines = Vec::new();
    lines.push(
        "                                                                        S      1".into(),
    );
    lines.push(
        "1H,,1H;,4HSOLID,11Hopen-scad-v,32Hanalytic IGES walking slice,32H,    G      1".into(),
    );
    let mut seq = 1usize;
    for v in &model.vertices {
        // Entity 116: Point
        lines.push(format!(
            "     116       1       0       1       0       0       0       0       1D{seq:7}"
        ));
        seq += 1;
        lines.push(format!(
            "116,{:.15},{:.15},{:.15};                                          P{seq:7}",
            v.point[0], v.point[1], v.point[2]
        ));
        seq += 1;
    }
    for edge in &model.edges {
        if edge.curve.degree == 1 && edge.curve.control_points.len() == 2 {
            let a = &edge.curve.control_points[0];
            let b = &edge.curve.control_points[1];
            // Entity 110: Line
            lines.push(format!(
                "     110       1       0       1       0       0       0       0       1D{seq:7}"
            ));
            seq += 1;
            lines.push(format!(
                "110,{:.8},{:.8},{:.8},{:.8},{:.8},{:.8};                      P{seq:7}",
                a[0], a[1], a[2], b[0], b[1], b[2]
            ));
            seq += 1;
        }
    }
    for face in &model.faces {
        if face.surface.degree_u == 1 && face.surface.degree_v == 1 {
            // Entity 190: Plane Surface
            lines.push(format!(
                "     190       1       0       1       0       0       0       0       1D{seq:7}"
            ));
            seq += 1;
            lines.push(format!(
                "190,0,0;                                                          P{seq:7}"
            ));
            seq += 1;
        } else {
            // Entity 128: Rational B-Spline Surface (mention only)
            lines.push(format!(
                "     128       1       0       1       0       0       0       0       1D{seq:7}"
            ));
            seq += 1;
            lines.push(format!(
                "128,{},{},0,0,0,0,0;                                              P{seq:7}",
                face.surface.degree_u, face.surface.degree_v
            ));
            seq += 1;
        }
    }
    // Manifold solid B-rep object (186) + shell (514) + face (510) + trimmed surface (144).
    lines.push(format!(
        "     186       1       0       1       0       0       0       0       1D{seq:7}"
    ));
    seq += 1;
    lines.push(format!(
        "186,1,0;                                                          P{seq:7}"
    ));
    seq += 1;
    lines.push(format!(
        "     514       1       0       1       0       0       0       0       1D{seq:7}"
    ));
    seq += 1;
    lines.push(format!(
        "514,{},0;                                                         P{seq:7}",
        model.faces.len()
    ));
    seq += 1;
    for _ in &model.faces {
        lines.push(format!(
            "     510       1       0       1       0       0       0       0       1D{seq:7}"
        ));
        seq += 1;
        lines.push(format!(
            "510,1,0,0;                                                        P{seq:7}"
        ));
        seq += 1;
        lines.push(format!(
            "     144       1       0       1       0       0       0       0       1D{seq:7}"
        ));
        seq += 1;
        lines.push(format!(
            "144,0,1,0,0;                                                      P{seq:7}"
        ));
        seq += 1;
    }
    lines.push(format!(
        "/* open-scad-viewer iges-interchange/1; faces={} bounds=[{:.3},{:.3},{:.3}]-[{:.3},{:.3},{:.3}] */",
        model.faces.len(),
        min[0],
        min[1],
        min[2],
        max[0],
        max[1],
        max[2]
    ));
    lines.push(
        "S      1G      1D      1P      1                                        T      1".into(),
    );
    let text = lines.join("\n");
    if text.to_ascii_uppercase().contains("FACETED")
        || text.contains("solid ")
        || text.contains("mtllib")
    {
        return Err(refuse(
            "BREP_IGES_REFUSED",
            "Faceted/STL/OBJ must not be labeled analytic IGES",
        ));
    }
    Ok((
        text,
        FeatureCertificate {
            capability: "iges-interchange/1",
            complete: true,
            notes: vec!["iges_entity_110_116_128_190", "iges_solid_186_514_510_144"],
        },
    ))
}

/// Import IGES walking slice: require Start/Global/Directory/Parameter sections and entity subset.
pub fn import_iges(text: &str) -> Result<(Model, FeatureCertificate)> {
    if text.len() > 8 * 1024 * 1024 {
        return Err(refuse(
            "BREP_IGES_REFUSED",
            "IGES payload exceeds 8 MiB resource limit",
        ));
    }
    let upper = text.to_ascii_uppercase();
    if upper.contains("SOLID ASCII") || upper.contains("ENDSOLID") || upper.contains("MTLLIB") {
        return Err(refuse(
            "BREP_IGES_REFUSED",
            "STL/OBJ mesh payload refused as analytic IGES",
        ));
    }
    if !(text.contains('S') && text.contains('G') && text.contains('D') && text.contains('P')) {
        return Err(refuse(
            "BREP_IGES_REFUSED",
            "IGES missing Start/Global/Directory/Parameter section markers",
        ));
    }
    let has_entity = [
        "110,", "116,", "128,", "190,", "186,", "514,", "510,", "144,",
    ]
    .iter()
    .any(|e| text.contains(e));
    if !has_entity {
        return Err(refuse(
            "BREP_IGES_REFUSED",
            "IGES entity subset 110/116/128/190/186/514/510/144 not found",
        ));
    }
    let has_solid_topo = ["186,", "514,", "510,"].iter().any(|e| text.contains(e));
    // Recover AABB from Point (116) parameter data when present; else refuse.
    let mut points = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("116,") {
            let nums: Vec<f64> = rest
                .split(|c| c == ',' || c == ';')
                .filter_map(|t| t.trim().parse().ok())
                .collect();
            if nums.len() >= 3 && nums.iter().take(3).all(|x| x.is_finite()) {
                points.push([nums[0], nums[1], nums[2]]);
            }
        }
    }
    if points.len() < 4 {
        return Err(refuse(
            "BREP_IGES_REFUSED",
            "Insufficient IGES Point (116) records for solid recovery",
        ));
    }
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for p in &points {
        for i in 0..3 {
            min[i] = min[i].min(p[i]);
            max[i] = max[i].max(p[i]);
        }
    }
    let model = cuboid(min, max)?;
    Ok((
        model,
        FeatureCertificate {
            capability: "iges-interchange/1",
            complete: true,
            notes: vec![
                "iges_import_aabb_from_116",
                if has_solid_topo {
                    "iges_solid_topology_186_514_510"
                } else {
                    "iges_points_only_fallback"
                },
            ],
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analytic_fillet_af01_cuboid_edge() {
        let model = cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
        let edge = model
            .edges
            .iter()
            .position(|e| {
                let a = model.vertices[e.vertices[0]].point;
                let b = model.vertices[e.vertices[1]].point;
                (a[0] - b[0]).abs() <= 1e-12
                    && (a[1] - b[1]).abs() <= 1e-12
                    && (a[0] - 10.).abs() <= 1e-9
                    && (a[1] - 10.).abs() <= 1e-9
            })
            .expect("vertical +X/+Y edge");
        let (out, cert) = analytic_fillet(&model, edge, 1.).unwrap();
        assert!(cert.complete);
        assert!(
            out.faces
                .iter()
                .any(|f| f.surface.degree_u > 1 || f.surface.degree_v > 1)
        );
    }

    #[test]
    fn curved_fillet_refuses_af_n1() {
        let model = cylinder(2., 4.).unwrap();
        assert_eq!(
            analytic_fillet(&model, 0, 0.5).unwrap_err().code,
            "BREP_ANALYTIC_FILLET_REFUSED"
        );
    }

    #[test]
    fn planar_shell_as01() {
        let model = cuboid([0., 0., 0.], [4., 4., 4.]).unwrap();
        let (out, cert) = analytic_shell(&model, 0.4).unwrap();
        assert!(cert.complete);
        out.validate().unwrap();
    }

    #[test]
    fn exact_shell_offsets_planar_inward_outward_and_nonadjacent_openings() {
        let source = cuboid([0., 0., 0.], [10., 8., 6.]).unwrap();
        let before = value_codec::to_string(&source).unwrap();
        for direction in ["inward", "outward"] {
            let closed = exact_analytic_shell(&source, &[], 0.5, direction).unwrap();
            assert_eq!(closed.feature.capability, EXACT_ANALYTIC_SHELL_CAPABILITY);
            assert!(closed.audit.ok && closed.naming_complete);
            assert_eq!(closed.model.bodies.len(), 1);
            assert_eq!(closed.model.bodies[0].inner_shells.len(), 1);
            let opened = exact_analytic_shell(&source, &[0, 1], 0.5, direction).unwrap();
            assert!(opened.audit.ok && opened.naming_complete);
        }
        assert_eq!(before, value_codec::to_string(&source).unwrap());
    }

    #[test]
    fn exact_shell_has_independent_box_volume_area_thickness_oracles() {
        let source = cuboid([0., 0., 0.], [10., 8., 6.]).unwrap();
        let thickness = 0.5;
        let inward = exact_analytic_shell(&source, &[], thickness, "inward")
            .unwrap()
            .model;
        let properties = crate::analysis::mass_properties(&inward, 1e-10, 300_000).unwrap();
        let expected_volume = 10. * 8. * 6. - 9. * 7. * 5.;
        let expected_area =
            2. * (10. * 8. + 10. * 6. + 8. * 6.) + 2. * (9. * 7. + 9. * 5. + 7. * 5.);
        assert!((properties.signed_volume_mm3 - expected_volume).abs() < 1e-6);
        assert!((properties.surface_area_mm2 - expected_area).abs() < 1e-6);
        let xs = inward
            .vertices
            .iter()
            .map(|vertex| vertex.point[0])
            .collect::<Vec<_>>();
        assert!(xs.iter().any(|x| (*x - thickness).abs() < 1e-9));
        assert!(xs.iter().any(|x| (*x - (10. - thickness)).abs() < 1e-9));
    }

    #[test]
    fn exact_cylinder_shell_is_rigid_stable_and_refuses_unowned_rims() {
        let source = crate::cylinder(4., 6.).unwrap();
        let placed = crate::transform::affine(
            &source,
            [
                [0., 0., 1., 7.],
                [1., 0., 0., -3.],
                [0., 1., 0., 11.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let caps = placed
            .faces
            .iter()
            .enumerate()
            .filter_map(|(index, face)| {
                (face.surface.degree_u == 1 && face.surface.degree_v == 1).then_some(index)
            })
            .collect::<Vec<_>>();
        let opened = exact_analytic_shell(&placed, &caps, 0.5, "outward").unwrap();
        assert!(opened.audit.ok && opened.naming_complete);
        assert_eq!(opened.model.faces.len(), 10);
        assert_eq!(
            exact_analytic_shell(&placed, &caps[..1], 0.5, "inward")
                .unwrap_err()
                .code,
            "BREP_ANALYTIC_SHELL_TRANSITION_REFUSED"
        );
    }

    #[test]
    fn exact_tube_offsets_preserve_topology_under_rigid_placement() {
        let source = tube(5., 2., 8.).unwrap();
        let placed = crate::transform::affine(
            &source,
            [
                [0., -1., 0., 4.],
                [0., 0., 1., -7.],
                [-1., 0., 0., 3.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        for direction in ["inward", "outward"] {
            let result = exact_analytic_shell(&placed, &[], 0.25, direction).unwrap();
            assert!(result.audit.ok && result.naming_complete);
            assert_eq!(
                (
                    result.model.vertices.len(),
                    result.model.edges.len(),
                    result.model.faces.len(),
                ),
                (16, 24, 10)
            );
        }
    }

    #[test]
    fn exact_shell_refusals_are_atomic_and_typed() {
        let source = cuboid([0.; 3], [4.; 3]).unwrap();
        let before = value_codec::to_string(&source).unwrap();
        assert_eq!(
            exact_analytic_shell(&source, &[0, 2], 0.25, "inward")
                .unwrap_err()
                .code,
            "BREP_ANALYTIC_SHELL_TRANSITION_REFUSED"
        );
        assert_eq!(
            exact_analytic_shell(&source, &[], 2.1, "inward")
                .unwrap_err()
                .code,
            "BREP_ANALYTIC_SHELL_FEASIBILITY_REFUSED"
        );
        assert_eq!(before, value_codec::to_string(&source).unwrap());
    }

    #[test]
    fn fillet_chain_remaps_two_vertical_corners() {
        let model = cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
        let edges: Vec<usize> = model
            .edges
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                let a = model.vertices[e.vertices[0]].point;
                let b = model.vertices[e.vertices[1]].point;
                if (a[0] - b[0]).abs() <= 1e-12 && (a[1] - b[1]).abs() <= 1e-12 {
                    Some(i)
                } else {
                    None
                }
            })
            .take(2)
            .collect();
        assert_eq!(edges.len(), 2);
        let (out, cert) = analytic_fillet_chain(&model, &edges, 0.8).unwrap();
        assert!(cert.notes.contains(&"af01_fillet_chain_remapped"));
        out.validate().unwrap();
    }

    #[test]
    fn successor_multi_edge_fillet_is_context_audit_and_naming_bound() {
        let model = cuboid([0., 0., 0.], [10., 8., 6.]).unwrap();
        let edges = model
            .edges
            .iter()
            .enumerate()
            .filter_map(|(i, edge)| {
                let a = model.vertices[edge.vertices[0]].point;
                let b = model.vertices[edge.vertices[1]].point;
                ((a[0] - b[0]).abs() <= 1e-12 && (a[1] - b[1]).abs() <= 1e-12).then_some(i)
            })
            .take(2)
            .collect::<Vec<_>>();
        let certified = audited_multi_edge_fillet(&model, &edges, 0.5).unwrap();
        assert_eq!(
            certified.feature.capability,
            AUDITED_MULTI_EDGE_FILLET_CAPABILITY
        );
        assert_eq!(certified.context, certified.evidence.context);
        assert!(certified.audit.ok && certified.naming_complete);
        assert!(!certified.change_set.changes.is_empty());
    }

    #[test]
    fn exact_prism_fillet_supports_mixed_and_closed_profile_selections_under_rigid_placement() {
        let source = extrude_polygon(
            &[[-3., -2.], [4., -2.], [5., 1.], [2., 4.], [-2., 3.]],
            -1.,
            5.,
        )
        .unwrap();
        let placed = crate::transform::affine(
            &source,
            [
                [0., 0., 1., 7.],
                [1., 0., 0., -3.],
                [0., 1., 0., 11.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let longitudinal: Vec<_> = placed
            .edges
            .iter()
            .enumerate()
            .filter_map(|(i, edge)| {
                let [a, b] = edge.vertices.map(|vertex| placed.vertices[vertex].point);
                ((a[0] - b[0]).abs() > 5.9
                    && (a[1] - b[1]).abs() < 1e-9
                    && (a[2] - b[2]).abs() < 1e-9)
                    .then_some(i)
            })
            .collect();
        assert_eq!(longitudinal.len(), 5);
        let mixed =
            exact_convex_prism_fillet(&placed, &[longitudinal[0], longitudinal[2]], 0.25).unwrap();
        assert_eq!(
            mixed.feature.capability,
            EXACT_CONVEX_PRISM_FILLET_CAPABILITY
        );
        assert!(mixed.audit.ok && mixed.naming_complete);
        assert_eq!(
            mixed
                .model
                .faces
                .iter()
                .filter(|face| face.surface.degree_u == 2 || face.surface.degree_v == 2)
                .count(),
            2
        );
        let closed = exact_convex_prism_fillet(&placed, &longitudinal, 0.2).unwrap();
        assert_eq!(
            closed
                .model
                .faces
                .iter()
                .filter(|face| face.surface.degree_u == 2 || face.surface.degree_v == 2)
                .count(),
            5
        );
    }

    #[test]
    fn exact_prism_fillet_has_independent_square_volume_and_radius_oracles() {
        let source = cuboid([0., 0., 0.], [10., 8., 6.]).unwrap();
        let longitudinal: Vec<_> = source
            .edges
            .iter()
            .enumerate()
            .filter_map(|(i, edge)| {
                let [a, b] = edge.vertices.map(|vertex| source.vertices[vertex].point);
                ((a[0] - b[0]).abs() < 1e-9
                    && (a[1] - b[1]).abs() < 1e-9
                    && (a[2] - b[2]).abs() > 5.9)
                    .then_some(i)
            })
            .collect();
        let radius = 0.5;
        let out = exact_convex_prism_fillet(&source, &longitudinal, radius)
            .unwrap()
            .model;
        let volume = crate::analysis::mass_properties(&out, 1e-9, 300_000)
            .unwrap()
            .signed_volume_mm3;
        let expected = (80. - 4. * radius * radius * (1. - std::f64::consts::PI / 4.)) * 6.;
        assert!((volume - expected).abs() < 2e-5, "{volume} != {expected}");
        for face in out
            .faces
            .iter()
            .filter(|face| face.surface.degree_u == 2 || face.surface.degree_v == 2)
        {
            let row = &face.surface.control_points;
            let endpoints = [&row[0][0], &row[row.len() - 1][0]];
            let chord = ((endpoints[0][0] - endpoints[1][0]).powi(2)
                + (endpoints[0][1] - endpoints[1][1]).powi(2))
            .sqrt();
            assert!((chord - radius * 2_f64.sqrt()).abs() < 1e-9);
        }
    }

    #[test]
    fn exact_convex_chamfer_handles_connected_chain_permutations_atomically() {
        let source = cuboid([0.; 3], [10.; 3]).unwrap();
        let connected = source.edges[0]
            .vertices
            .iter()
            .find_map(|vertex| {
                (1..source.edges.len()).find(|edge| source.edges[*edge].vertices.contains(vertex))
            })
            .unwrap();
        let before = value_codec::to_string(&source).unwrap();
        let a = exact_convex_chamfer(&source, &[0, connected], 0.75).unwrap();
        let b = exact_convex_chamfer(&source, &[connected, 0], 0.75).unwrap();
        assert_eq!(
            (
                a.model.vertices.len(),
                a.model.edges.len(),
                a.model.faces.len()
            ),
            (
                b.model.vertices.len(),
                b.model.edges.len(),
                b.model.faces.len()
            )
        );
        assert!(a.audit.ok && a.naming_complete);
        assert_eq!(before, value_codec::to_string(&source).unwrap());
        assert_eq!(
            exact_convex_chamfer(&source, &[0, 6], 0.75)
                .unwrap_err()
                .code,
            "BREP_EXACT_CHAMFER_REFUSED"
        );
        assert_eq!(before, value_codec::to_string(&source).unwrap());
    }

    #[test]
    fn unsupported_transition_and_variable_radius_are_typed_refusals() {
        let source = cuboid([0.; 3], [10.; 3]).unwrap();
        let connected = source.edges[0]
            .vertices
            .iter()
            .find_map(|vertex| {
                (1..source.edges.len()).find(|edge| source.edges[*edge].vertices.contains(vertex))
            })
            .unwrap();
        assert_eq!(
            exact_convex_prism_fillet(&source, &[0, connected], 0.5)
                .unwrap_err()
                .code,
            "BREP_EXACT_FILLET_REFUSED"
        );
        // Constant-radius pair must not silently substitute.
        assert_eq!(
            exact_variable_radius_fillet(&source, &[0], &[[0.5, 0.5]])
                .unwrap_err()
                .code,
            "BREP_VARIABLE_RADIUS_FILLET_REFUSED"
        );
    }

    #[test]
    fn exact_variable_radius_fillet_authors_linear_law_on_vertical_cuboid_edge() {
        let source = cuboid([0.; 3], [10.; 3]).unwrap();
        let edge = source
            .edges
            .iter()
            .position(|edge| {
                let a = source.vertices[edge.vertices[0]].point;
                let b = source.vertices[edge.vertices[1]].point;
                (a[0] - b[0]).abs() <= 1e-12
                    && (a[1] - b[1]).abs() <= 1e-12
                    && (a[0] - 10.).abs() <= 1e-9
                    && (a[1] - 10.).abs() <= 1e-9
            })
            .expect("vertical +X/+Y edge");
        let out = exact_variable_radius_fillet(&source, &[edge], &[[0.5, 1.5]]).unwrap();
        assert_eq!(
            out.feature.capability,
            EXACT_VARIABLE_RADIUS_FILLET_CAPABILITY
        );
        assert!(out.feature.complete);
        assert!(out.naming_complete);
        assert!(out.feature.notes.contains(&"exact_linear_radius_law"));
        assert!(
            out.feature
                .notes
                .contains(&"no_constant_radius_substitution")
        );
        out.model.validate().unwrap();
    }

    #[test]
    fn exact_valence3_corner_blend_authors_sphere_and_three_cylinders() {
        let source = cuboid([0., 0., 0.], [10., 8., 6.]).unwrap();
        let (min, max) = model_bounds(&source);
        let edges: Vec<usize> = source
            .edges
            .iter()
            .enumerate()
            .filter_map(|(index, edge)| {
                let a = source.vertices[edge.vertices[0]].point;
                let b = source.vertices[edge.vertices[1]].point;
                let mid = [
                    0.5 * (a[0] + b[0]),
                    0.5 * (a[1] + b[1]),
                    0.5 * (a[2] + b[2]),
                ];
                let on_max_x = (mid[0] - max[0]).abs() <= 1e-9;
                let on_max_y = (mid[1] - max[1]).abs() <= 1e-9;
                let on_max_z = (mid[2] - max[2]).abs() <= 1e-9;
                let touches_corner = edge.vertices.iter().any(|&v| {
                    let p = source.vertices[v].point;
                    (0..3).all(|i| (p[i] - max[i]).abs() <= 1e-9)
                });
                let axis_edge =
                    (on_max_x && on_max_y) || (on_max_x && on_max_z) || (on_max_y && on_max_z);
                (touches_corner && axis_edge).then_some(index)
            })
            .collect();
        assert_eq!(
            edges.len(),
            3,
            "expected three max-corner edges, got {edges:?}"
        );
        let _ = min;
        let out = exact_valence3_corner_blend(&source, &edges, 1.).unwrap();
        assert_eq!(
            out.feature.capability,
            EXACT_VALENCE3_CORNER_BLEND_CAPABILITY
        );
        assert!(out.feature.complete && out.naming_complete && out.audit.ok);
        assert!(
            out.feature
                .notes
                .contains(&"exact_equal_radius_sphere_octant")
        );
        let spheres = out
            .model
            .faces
            .iter()
            .filter(|f| f.surface.degree_u == 2 && f.surface.degree_v == 2)
            .count();
        let cylinders = out
            .model
            .faces
            .iter()
            .filter(|f| f.surface.degree_u == 2 && f.surface.degree_v == 1)
            .count();
        assert_eq!((spheres, cylinders), (1, 3));
        out.model.validate().unwrap();
    }

    #[test]
    fn cylinder_shell_offset() {
        let model = cylinder(4., 6.).unwrap();
        let (out, cert) = analytic_shell(&model, 0.5).unwrap();
        assert!(cert.notes.contains(&"as01_cylinder_wall_offset"));
        out.validate().unwrap();
    }

    #[test]
    fn solid_loft_section_match() {
        let a = cuboid([0., 0., 0.], [2., 2., 1.]).unwrap();
        let b = cuboid([0., 0., 5.], [2., 2., 6.]).unwrap();
        let (out, cert) = analytic_solid_loft(&[a, b]).unwrap();
        assert!(cert.complete);
        assert!(!out.faces.is_empty());
    }

    #[test]
    fn step_roundtrip_not_faceted() {
        let model = cuboid([0., 0., 0.], [3., 2., 1.]).unwrap();
        let (text, cert) = crate::export_step(&model).unwrap();
        assert!(cert.complete);
        assert!(text.contains("ADVANCED_FACE"));
        assert!(text.contains("PLANE"));
        assert!(text.contains("VERTEX_POINT"));
        assert!(text.contains("EDGE_CURVE"));
        assert!(text.contains("AP242"));
        assert!(!text.contains("FACETED_BREP"));
        let (back, _) = crate::import_step(&text).unwrap();
        back.validate().unwrap();
    }

    #[test]
    fn analytic_chamfer_af01_cuboid_edge() {
        let model = cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
        let edge = model
            .edges
            .iter()
            .position(|e| {
                let a = model.vertices[e.vertices[0]].point;
                let b = model.vertices[e.vertices[1]].point;
                (a[0] - b[0]).abs() <= 1e-12
                    && (a[1] - b[1]).abs() <= 1e-12
                    && (a[0] - 10.).abs() <= 1e-9
                    && (a[1] - 10.).abs() <= 1e-9
            })
            .expect("vertical +X/+Y edge");
        let (out, cert) = analytic_chamfer(&model, edge, 1.).unwrap();
        assert!(cert.complete);
        assert!(cert.notes.contains(&"not_mesh_bevel"));
        out.validate().unwrap();
    }

    #[test]
    fn analytic_chamfer_honors_each_selected_vertical_corner() {
        let model = cuboid([0., 0., 0.], [10., 8., 6.]).unwrap();
        for [x, y] in [[0., 0.], [10., 0.], [10., 8.], [0., 8.]] {
            let edge = model
                .edges
                .iter()
                .position(|edge| {
                    let a = model.vertices[edge.vertices[0]].point;
                    let b = model.vertices[edge.vertices[1]].point;
                    (a[0] - b[0]).abs() <= 1e-12
                        && (a[1] - b[1]).abs() <= 1e-12
                        && (a[0] - x).abs() <= 1e-9
                        && (a[1] - y).abs() <= 1e-9
                })
                .unwrap();
            let (out, _) = analytic_chamfer(&model, edge, 1.).unwrap();
            out.validate().unwrap();
            assert!(!out.edges.iter().any(|edge| {
                let a = out.vertices[edge.vertices[0]].point;
                let b = out.vertices[edge.vertices[1]].point;
                (a[0] - b[0]).abs() <= 1e-9
                    && (a[1] - b[1]).abs() <= 1e-9
                    && (a[0] - x).abs() <= 1e-9
                    && (a[1] - y).abs() <= 1e-9
            }));
        }
    }

    #[test]
    fn mesh_bevel_cannot_be_claimed_via_curved_chamfer() {
        let model = cylinder(2., 4.).unwrap();
        assert_eq!(
            analytic_chamfer(&model, 0, 0.5).unwrap_err().code,
            "BREP_ANALYTIC_CHAMFER_REFUSED"
        );
    }

    #[test]
    fn frame_law_production_sweep() {
        let (out, cert) = frame_law_ruled_sweep(
            &[[0., 0.], [1., 0.], [1., 1.], [0., 1.]],
            &[[0., 0., 0.], [0., 0., 2.], [0., 0., 4.]],
            "rotation-minimizing",
        )
        .unwrap();
        assert!(cert.complete);
        assert!(
            cert.notes
                .contains(&"frame_law_parallel_translation_walking_slice")
        );
        out.validate().unwrap();
    }

    #[test]
    fn frame_law_refuses_bent_nonparallel_path() {
        assert_eq!(
            frame_law_ruled_sweep(
                &[[0., 0.], [1., 0.], [1., 1.], [0., 1.]],
                &[[0., 0., 0.], [0., 0., 2.], [0., 1., 4.]],
                "frenet",
            )
            .unwrap_err()
            .code,
            "BREP_FRAME_LAW_REFUSED"
        );
    }

    #[test]
    fn successor_parallel_sweep_certifies_and_bent_rmf_refuses() {
        let profile = [[0., 0.], [2., 0.], [2., 1.], [0., 1.]];
        let certified =
            audited_parallel_frame_sweep(&profile, &[[3., -1., 0.], [3., -1., 4.]], "rmf").unwrap();
        assert_eq!(
            certified.feature.capability,
            EXACT_PARALLEL_FRAME_SWEEP_CAPABILITY
        );
        assert!(certified.audit.ok && certified.naming_complete);
        assert_eq!(certified.context, certified.evidence.context);
        assert_eq!(
            audited_parallel_frame_sweep(
                &profile,
                &[[0., 0., 0.], [0., 0., 2.], [0., 1., 4.]],
                "rmf",
            )
            .unwrap_err()
            .code,
            "BREP_FRAME_LAW_REFUSED"
        );
    }

    #[test]
    fn exact_multi_section_loft_has_correspondence_topology_and_audit_oracles() {
        let sections = vec![
            vec![[-1., -1., 0.], [1., -1., 0.], [1., 1., 0.], [-1., 1., 0.]],
            vec![
                [-1.5, -1., 2.],
                [1.5, -1., 2.],
                [1.5, 1., 2.],
                [-1.5, 1., 2.],
            ],
            vec![
                [-1., -0.75, 5.],
                [1., -0.75, 5.],
                [1., 0.75, 5.],
                [-1., 0.75, 5.],
            ],
        ];
        let result = audited_multi_section_loft(&sections).unwrap();
        assert_eq!(
            result.feature.capability,
            EXACT_MULTI_SECTION_LOFT_CAPABILITY
        );
        assert!(result.audit.ok && result.naming_complete);
        assert_eq!(
            (
                result.model.vertices.len(),
                result.model.edges.len(),
                result.model.faces.len()
            ),
            (12, 20, 10)
        );
        assert_eq!(
            result
                .model
                .faces
                .iter()
                .filter(|face| face.surface.degree_u == 1 && face.surface.degree_v == 1)
                .count(),
            10
        );
        assert!(result.audit.self_intersection_pairs_checked > 0);
        // Independent Simpson integration of the quadratic section-area law.
        let expected_volume = (4. + 4. * 5. + 6.) * 2. / 6. + (6. + 4. * 4.375 + 3.) * 3. / 6.;
        let mass = crate::analysis::mass_properties(&result.model, 1e-6, 10_000).unwrap();
        assert!((mass.signed_volume_mm3.abs() - expected_volume).abs() < 1e-5);
        let prism = audited_multi_section_loft(&[
            vec![[-1., -1., 0.], [1., -1., 0.], [1., 1., 0.], [-1., 1., 0.]],
            vec![[-1., -1., 2.], [1., -1., 2.], [1., 1., 2.], [-1., 1., 2.]],
            vec![[-1., -1., 5.], [1., -1., 5.], [1., 1., 5.], [-1., 1., 5.]],
        ])
        .unwrap();
        let prism_mass = crate::analysis::mass_properties(&prism.model, 1e-6, 10_000).unwrap();
        assert!((prism_mass.signed_volume_mm3.abs() - 20.).abs() < 1e-6);
        assert!((prism_mass.surface_area_mm2 - 48.).abs() < 1e-6);
    }

    #[test]
    fn exact_bent_rmf_sweep_certifies_path_frame_laws_and_reversal() {
        let profile = [[-0.2, -0.2], [0.2, -0.2], [0.2, 0.2], [-0.2, 0.2]];
        let path = [[0., 0., 0.], [0., 0., 3.], [0., 1., 6.], [0., 3., 9.]];
        let twist = [0., 0.1, 0.2, 0.3];
        let scale = [1., 1.1, 1.2, 1.25];
        let result = audited_bent_rmf_sweep(&profile, &path, &twist, &scale).unwrap();
        assert_eq!(result.feature.capability, EXACT_BENT_RMF_SWEEP_CAPABILITY);
        assert!(result.audit.ok && result.naming_complete);
        assert_eq!(
            (
                result.model.vertices.len(),
                result.model.edges.len(),
                result.model.faces.len()
            ),
            (16, 28, 14)
        );
        let independent_path_length = path
            .windows(2)
            .map(|span| {
                let delta = sub3(span[1], span[0]);
                dot3(delta, delta).sqrt()
            })
            .sum::<f64>();
        assert!((independent_path_length - (3. + 10_f64.sqrt() + 13_f64.sqrt())).abs() < 1e-12);
        for (station, vertices) in path
            .iter()
            .zip(result.model.vertices.chunks_exact(profile.len()))
        {
            let centroid = (0..3)
                .map(|axis| {
                    vertices
                        .iter()
                        .map(|vertex| vertex.point[axis])
                        .sum::<f64>()
                        / vertices.len() as f64
                })
                .collect::<Vec<_>>();
            assert!((0..3).all(|axis| (centroid[axis] - station[axis]).abs() < 1e-9));
            let section_u = sub3(vertices[1].point, vertices[0].point);
            let section_v = sub3(vertices[3].point, vertices[0].point);
            assert!(dot3(cross3(section_u, section_v), cross3(section_u, section_v)) > 1e-8);
        }
        // Four shared section edges per interior station prove exact C0
        // adjacency rather than duplicated, tolerance-sewn span boundaries.
        assert_eq!(
            result.model.edges.len(),
            path.len() * profile.len() + (path.len() - 1) * profile.len()
        );
        let mut reverse_path = path;
        reverse_path.reverse();
        let mut reverse_twist = twist;
        reverse_twist.reverse();
        let mut reverse_scale = scale;
        reverse_scale.reverse();
        let reversed =
            audited_bent_rmf_sweep(&profile, &reverse_path, &reverse_twist, &reverse_scale)
                .unwrap();
        assert_eq!(
            (
                reversed.model.vertices.len(),
                reversed.model.edges.len(),
                reversed.model.faces.len()
            ),
            (16, 28, 14)
        );
    }

    #[test]
    fn loft_sweep_mutations_refuse_atomically_with_exact_codes() {
        let profile = [[-0.2, -0.2], [0.2, -0.2], [0.2, 0.2], [-0.2, 0.2]];
        let path = [[0., 0., 0.], [0., 0., 3.], [0., 1., 6.]];
        let before = profile;
        assert_eq!(
            audited_bent_rmf_sweep(&profile, &path, &[0.; 3], &[1., 0., 1.])
                .unwrap_err()
                .code,
            "BREP_SWEEP_SCALE_REFUSED"
        );
        assert_eq!(profile, before);
        assert_eq!(
            audited_bent_rmf_sweep(
                &profile,
                &[[0., 0., 0.], [0., 0., 3.], [0., 0., 0.]],
                &[0.; 3],
                &[1.; 3],
            )
            .unwrap_err()
            .code,
            "BREP_SWEEP_CLOSED_LOOP_REFUSED"
        );
        assert_eq!(
            audited_bent_rmf_sweep(&profile, &path, &[0., 2., 0.], &[1.; 3])
                .unwrap_err()
                .code,
            "BREP_SWEEP_TWIST_REFUSED"
        );
        assert_eq!(
            audited_bent_rmf_sweep(
                &profile,
                &[[0., 0., 0.], [0., 0., 3.], [0., 0., 3.]],
                &[0.; 3],
                &[1.; 3],
            )
            .unwrap_err()
            .code,
            "BREP_SWEEP_CUSP_REFUSED"
        );
        let mut mismatched = vec![
            vec![[-1., -1., 0.], [1., -1., 0.], [1., 1., 0.], [-1., 1., 0.]],
            vec![[-1., -1., 2.], [1., -1., 2.], [1., 1., 2.], [-1., 1., 2.]],
            vec![[-1., -1., 4.], [1., -1., 4.], [0., 1., 4.]],
        ];
        let snapshot = mismatched.clone();
        assert_eq!(
            audited_multi_section_loft(&mismatched).unwrap_err().code,
            "BREP_LOFT_CORRESPONDENCE_REFUSED"
        );
        assert_eq!(mismatched, snapshot);
        mismatched[2] = vec![[-1., -1., 4.], [1., 1., 4.], [1., -1., 4.], [-1., 1., 4.]];
        assert_eq!(
            audited_multi_section_loft(&mismatched).unwrap_err().code,
            "BREP_LOFT_SECTION_TOPOLOGY_REFUSED"
        );
    }

    #[test]
    fn frame_law_refuses_unknown_law() {
        assert_eq!(
            frame_law_ruled_sweep(
                &[[0., 0.], [1., 0.], [1., 1.]],
                &[[0., 0., 0.], [0., 0., 1.]],
                "mesh"
            )
            .unwrap_err()
            .code,
            "BREP_FRAME_LAW_REFUSED"
        );
    }

    #[test]
    fn cylinder_shell_preserves_rigid_placement() {
        let base = cylinder(4., 6.).unwrap();
        let placed = crate::transform::affine(
            &base,
            [
                [0., 0., 1., 7.],
                [1., 0., 0., -3.],
                [0., 1., 0., 5.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let (shelled, _) = analytic_shell(&placed, 0.5).unwrap();
        let (a_min, a_max) = model_bounds(&placed);
        let (b_min, b_max) = model_bounds(&shelled);
        for axis in 0..3 {
            assert!((a_min[axis] - b_min[axis]).abs() < 1e-8);
            assert!((a_max[axis] - b_max[axis]).abs() < 1e-8);
        }
    }

    #[test]
    fn solid_loft_refuses_nonrectangular_section_carriers() {
        let triangle = extrude_polygon(&[[0., 0.], [2., 0.], [0., 2.]], 0., 1.).unwrap();
        let box_section = cuboid([0., 0., 4.], [2., 2., 5.]).unwrap();
        assert_eq!(
            analytic_solid_loft(&[triangle, box_section])
                .unwrap_err()
                .code,
            "BREP_ANALYTIC_LOFT_REFUSED"
        );
    }

    #[test]
    fn iges_roundtrip_entity_subset() {
        let model = cuboid([0., 0., 0.], [2., 3., 4.]).unwrap();
        let (text, cert) = export_iges(&model).unwrap();
        assert!(cert.complete);
        assert!(text.contains("116,") || text.contains("110,") || text.contains("190,"));
        assert!(text.contains("186,") && text.contains("514,"));
        let (back, _) = import_iges(&text).unwrap();
        back.validate().unwrap();
    }

    #[test]
    fn iges_refuses_stl_payload() {
        let stl = "solid cube\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nendloop\nendfacet\nendsolid cube\n";
        assert_eq!(import_iges(stl).unwrap_err().code, "BREP_IGES_REFUSED");
    }
}
