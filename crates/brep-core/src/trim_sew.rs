//! Lifted-UV arrangements, material cell classification, and exact sewing.
//!
//! Extends planar_trim to frozen chart classes (plane / poly / analytic circle).
//! Sewing is exact-match only: gap/duplicate/orientation mismatches refuse and
//! roll back atomically. No mesh weld, tolerance growth, or auto-heal.

use cad_predicates::{ToleranceContext, ToleranceSpecIdentity};
use nurbs_core::{Error, Result, curve::Curve};
use std::collections::{BTreeMap, BTreeSet};

use crate::predicate_evidence::{
    ComposedEvidence, EvidenceClaim, PredicateEvidence, compose_predicate_evidence,
};
use crate::{ChangeKind, ChangeProvenance, Model, TopoId, TopoKind, TopologyChange};

fn refuse(code: &'static str, message: &str) -> Error {
    Error::new(code, message)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChartKind {
    PlanePoly,
    AnalyticCircle,
    /// Admitted freeform chart touch for UV arrange walking slice (F3).
    /// Out-of-matrix freeform DCEL still refuses Complete without strata.
    Freeform,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChartEvent {
    pub parameter: f64,
    pub kind: &'static str,
    pub edge: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellLabel {
    Outside,
    Inside,
    Boundary,
}

#[derive(Clone, Debug)]
pub struct ClassificationCertificate {
    pub chart: ChartKind,
    pub events: Vec<ChartEvent>,
    pub cells: Vec<(f64, f64, CellLabel)>,
    pub complete: bool,
}

/// Event order on a frozen chart: parameters must be strictly increasing except
/// for explicit coincident boundary strata which are labeled, never merged.
pub fn classify_chart_events(
    chart: ChartKind,
    mut events: Vec<ChartEvent>,
    sample_points: &[(f64, CellLabel)],
) -> Result<ClassificationCertificate> {
    if events.len() > 4096 || sample_points.len() > 4096 {
        return Err(refuse(
            "BREP_TRIM_RESOURCE_LIMIT",
            "Chart event/sample budget exceeded",
        ));
    }
    events.sort_by(|a, b| {
        a.parameter
            .partial_cmp(&b.parameter)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.edge.cmp(&b.edge))
    });
    for window in events.windows(2) {
        let a = &window[0];
        let b = &window[1];
        if !a.parameter.is_finite() || !b.parameter.is_finite() {
            return Err(refuse(
                "BREP_TRIM_INVALID",
                "Chart event parameter must be finite",
            ));
        }
        if (a.parameter - b.parameter).abs() <= 1e-15 && a.edge == b.edge {
            return Err(refuse(
                "BREP_TRIM_AMBIGUOUS",
                "Duplicate chart event on the same edge without stratum label",
            ));
        }
    }
    let mut cells = Vec::new();
    for (i, point) in sample_points.iter().enumerate() {
        if !point.0.is_finite() {
            return Err(refuse(
                "BREP_TRIM_INVALID",
                "Classification sample must be finite",
            ));
        }
        // Root-isolated: sample must not sit on an event unless Boundary.
        let on_event = events
            .iter()
            .any(|e| (e.parameter - point.0).abs() <= 1e-12);
        if on_event && point.1 != CellLabel::Boundary {
            return Err(refuse(
                "BREP_TRIM_AMBIGUOUS",
                "Interior/exterior sample coincides with an event root",
            ));
        }
        if !on_event && point.1 == CellLabel::Boundary {
            return Err(refuse(
                "BREP_TRIM_AMBIGUOUS",
                "Boundary label without an isolating event",
            ));
        }
        let lo = if i == 0 {
            f64::NEG_INFINITY
        } else {
            sample_points[i - 1].0
        };
        cells.push((lo, point.0, point.1));
    }
    Ok(ClassificationCertificate {
        chart,
        events,
        cells,
        complete: true,
    })
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SewEdgeKey {
    pub a: [i64; 3],
    pub b: [i64; 3],
    /// Curve-shape discriminator. Closed-shell auditing fills this with the
    /// exact NURBS midpoint so distinct arcs sharing endpoints never collapse.
    pub mid: Option<[i64; 3]>,
    /// Complete rational-definition discriminator for proof-backed sewing.
    pub definition: Option<String>,
    /// Shell discriminator prevents touching but disconnected material lumps
    /// from being welded by coincident geometric coordinates.
    pub shell: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplacementOp {
    None,
    Refit,
    Split,
}

/// Optional displacement record for an authorized heal plan (never silent).
#[derive(Clone, Debug)]
pub struct DisplacementRecord {
    pub before: SewEdgeKey,
    pub after: SewEdgeKey,
    pub bound: i64,
    pub op: DisplacementOp,
}

#[derive(Clone, Debug)]
pub struct SewLedgerEntry {
    pub key: SewEdgeKey,
    pub face_a: usize,
    pub face_b: usize,
    pub orientation_agree: bool,
    pub displacement: Option<DisplacementRecord>,
}

#[derive(Clone, Debug, Default)]
pub struct SewSnapshot {
    pub edges: BTreeSet<SewEdgeKey>,
    pub oriented: BTreeMap<SewEdgeKey, bool>,
    pub displacements: Vec<DisplacementRecord>,
}

#[derive(Clone, Debug)]
pub struct SewCertificate {
    pub matched: usize,
    pub complete: bool,
    pub displacement_budget_ok: bool,
}

/// Bit-exact authority for a rational NURBS curve.  This deliberately records
/// the complete definition, rather than endpoint or sample-point surrogates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RationalCurveDefinition {
    pub degree: usize,
    pub knots: Vec<u64>,
    pub control_points: Vec<Vec<u64>>,
    pub weights: Vec<u64>,
    pub periodic: bool,
}

impl RationalCurveDefinition {
    pub fn from_curve(curve: &Curve) -> Result<Self> {
        curve.validate()?;
        Ok(Self {
            degree: curve.degree,
            knots: curve.knots.iter().map(|v| v.to_bits()).collect(),
            control_points: curve
                .control_points
                .iter()
                .map(|point| point.iter().map(|v| v.to_bits()).collect())
                .collect(),
            weights: curve.weights.iter().map(|v| v.to_bits()).collect(),
            periodic: curve.periodic,
        })
    }
}

/// Exact curve authority. Canonical construction identities are reserved for
/// constructors which guarantee the same rational definition by contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CurveAuthority {
    Rational(RationalCurveDefinition),
    CanonicalConstruction(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterOrientation {
    Same,
    Reversed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundaryUse {
    pub face: usize,
    pub wire: usize,
    pub cyclic_index: usize,
    pub reversed: bool,
}

/// Proof that two complete boundary uses correspond. Every field is checked by
/// exact sew; no endpoint/midpoint quantization is an authority.
#[derive(Clone, Debug)]
pub struct BoundaryCorrespondence {
    pub context: ToleranceSpecIdentity,
    pub authority: CurveAuthority,
    pub endpoints_bits: [[[u64; 3]; 2]; 2],
    pub parameter_domain_bits: [u64; 2],
    pub orientation: ParameterOrientation,
    /// Integer periodic shift in complete periods. Zero for non-periodic curves.
    pub seam_shift: i32,
    pub shell: usize,
    pub uses: [BoundaryUse; 2],
    pub evidence: ComposedEvidence,
}

#[derive(Clone, Debug)]
pub struct CorrespondenceLedgerEntry {
    pub key: SewEdgeKey,
    pub correspondence: BoundaryCorrespondence,
}

/// One narrowly authorized geometric repair. Construction is only available
/// through `AuthorizedHealPlan::new`, which binds every recipe to one context.
#[derive(Clone, Debug)]
pub enum HealOperation {
    EndpointSnap {
        vertex: usize,
        expected_id: TopoId,
        to: [f64; 3],
        correspondence: BoundaryCorrespondence,
    },
    CurveRefit {
        edge: usize,
        expected_id: TopoId,
        replacement: Curve,
        correspondence: BoundaryCorrespondence,
    },
    /// Split is represented explicitly, but this kernel slice refuses it until
    /// a caller supplies topology-preserving child-loop authorship.
    EdgeSplit {
        edge: usize,
        expected_id: TopoId,
        parameter: f64,
        correspondence: BoundaryCorrespondence,
    },
}

impl HealOperation {
    fn correspondence(&self) -> &BoundaryCorrespondence {
        match self {
            Self::EndpointSnap { correspondence, .. }
            | Self::CurveRefit { correspondence, .. }
            | Self::EdgeSplit { correspondence, .. } => correspondence,
        }
    }
    fn displacement(&self, model: &Model) -> Result<f64> {
        match self {
            Self::EndpointSnap {
                vertex,
                expected_id,
                to,
                ..
            } => {
                if model.1.vertices.get(*vertex) != Some(expected_id) {
                    return Err(refuse(
                        "BREP_HEAL_LINEAGE_MISMATCH",
                        "Endpoint identity does not match the authorized lineage",
                    ));
                }
                let from = model.vertices[*vertex].point;
                Ok(from
                    .into_iter()
                    .zip(*to)
                    .map(|(a, b)| (a - b) * (a - b))
                    .sum::<f64>()
                    .sqrt())
            }
            Self::CurveRefit {
                edge,
                expected_id,
                replacement,
                ..
            } => {
                if model.1.edges.get(*edge) != Some(expected_id) {
                    return Err(refuse(
                        "BREP_HEAL_LINEAGE_MISMATCH",
                        "Edge identity does not match the authorized lineage",
                    ));
                }
                replacement.validate()?;
                let old = &model.edges[*edge].curve;
                if old.control_points.len() != replacement.control_points.len()
                    || old.degree != replacement.degree
                    || old.knots != replacement.knots
                    || old.weights != replacement.weights
                    || old.periodic != replacement.periodic
                    || old.control_points.iter().map(Vec::len).collect::<Vec<_>>()
                        != replacement
                            .control_points
                            .iter()
                            .map(Vec::len)
                            .collect::<Vec<_>>()
                {
                    return Err(refuse(
                        "BREP_HEAL_REFIT_REFUSED",
                        "Refit requires one-to-one rational control correspondence",
                    ));
                }
                Ok(old
                    .control_points
                    .iter()
                    .zip(&replacement.control_points)
                    .map(|(a, b)| {
                        a.iter()
                            .zip(b)
                            .map(|(x, y)| (x - y) * (x - y))
                            .sum::<f64>()
                            .sqrt()
                    })
                    .fold(0., f64::max))
            }
            Self::EdgeSplit {
                edge,
                expected_id,
                parameter,
                ..
            } => {
                if model.1.edges.get(*edge) != Some(expected_id) {
                    return Err(refuse(
                        "BREP_HEAL_LINEAGE_MISMATCH",
                        "Split identity does not match the authorized lineage",
                    ));
                }
                let [lo, hi] = model.edges[*edge].curve.domain();
                if !parameter.is_finite() || !(*parameter > lo && *parameter < hi) {
                    return Err(refuse(
                        "BREP_HEAL_SPLIT_REFUSED",
                        "Split parameter must be strictly interior",
                    ));
                }
                Err(refuse(
                    "BREP_HEAL_SPLIT_REFUSED",
                    "Split lacks topology-preserving child-loop authorship",
                ))
            }
        }
    }
}

/// Immutable, context-bound authority for gap repair of at most one context cell.
#[derive(Clone, Debug)]
pub struct AuthorizedHealPlan {
    context: ToleranceSpecIdentity,
    operations: Vec<HealOperation>,
    per_entity_limit_mm: f64,
    cumulative_limit_mm: f64,
    identity: String,
}
impl AuthorizedHealPlan {
    pub fn new(
        model: &Model,
        context: &ToleranceContext,
        operations: Vec<HealOperation>,
        per_entity_limit_mm: f64,
        cumulative_limit_mm: f64,
    ) -> Result<Self> {
        model.validate()?;
        if operations.is_empty() || operations.len() > 64 {
            return Err(refuse(
                "BREP_HEAL_PLAN_INVALID",
                "Heal plan must contain 1..64 operations",
            ));
        }
        let cell = context.spatial_bounds().absolute_mm;
        if !per_entity_limit_mm.is_finite()
            || !cumulative_limit_mm.is_finite()
            || per_entity_limit_mm <= 0.
            || cumulative_limit_mm <= 0.
            || per_entity_limit_mm > cell
            || cumulative_limit_mm > cell
        {
            return Err(refuse(
                "BREP_HEAL_BUDGET_EXCEEDED",
                "Heal budget exceeds one tolerance-context cell",
            ));
        }
        let context_id = context.spec_identity();
        let mut cumulative = 0.;
        let mut signature = context_id.canonical.clone();
        let mut touched = BTreeSet::new();
        for operation in &operations {
            let proof = operation.correspondence();
            if proof.context != context_id || proof.evidence.context != context_id {
                return Err(refuse(
                    "BREP_HEAL_CONTEXT_MISMATCH",
                    "Heal proof belongs to a different tolerance context",
                ));
            }
            if !proof
                .evidence
                .claims
                .iter()
                .any(|c| matches!(c, EvidenceClaim::Correspondence { .. }))
                || !proof
                    .evidence
                    .claims
                    .iter()
                    .any(|c| matches!(c, EvidenceClaim::TopologyPreservation { .. }))
            {
                return Err(refuse(
                    "BREP_HEAL_PROOF_REQUIRED",
                    "Heal requires full correspondence and topology-preservation evidence",
                ));
            }
            let displacement = operation.displacement(model)?;
            if displacement > per_entity_limit_mm {
                return Err(refuse(
                    "BREP_HEAL_BUDGET_EXCEEDED",
                    "Per-entity physical displacement budget exceeded",
                ));
            }
            let changed_entity_count = match operation {
                HealOperation::EndpointSnap { vertex, .. } => {
                    1 + model
                        .edges
                        .iter()
                        .filter(|edge| edge.vertices.contains(vertex))
                        .count()
                }
                _ => 1,
            };
            cumulative += displacement * changed_entity_count as f64;
            let key = match operation {
                HealOperation::EndpointSnap { expected_id, .. } => (TopoKind::Vertex, *expected_id),
                HealOperation::CurveRefit { expected_id, .. }
                | HealOperation::EdgeSplit { expected_id, .. } => (TopoKind::Edge, *expected_id),
            };
            if !touched.insert(key) {
                return Err(refuse(
                    "BREP_HEAL_PLAN_INVALID",
                    "Entity appears more than once in heal plan",
                ));
            }
            signature.push_str(&format!("|{}:{:016x}", key.1, displacement.to_bits()));
        }
        if cumulative > cumulative_limit_mm {
            return Err(refuse(
                "BREP_HEAL_BUDGET_EXCEEDED",
                "Cumulative physical displacement budget exceeded",
            ));
        }
        let identity = format!(
            "authorized-heal:{}",
            TopoId::derive(
                TopoKind::Body,
                "authorized-heal",
                "plan",
                "identity",
                signature.as_bytes()
            )
        );
        Ok(Self {
            context: context_id,
            operations,
            per_entity_limit_mm,
            cumulative_limit_mm,
            identity,
        })
    }
    pub fn context(&self) -> &ToleranceSpecIdentity {
        &self.context
    }
    pub fn operations(&self) -> &[HealOperation] {
        &self.operations
    }
    pub fn per_entity_limit_mm(&self) -> f64 {
        self.per_entity_limit_mm
    }
    pub fn cumulative_limit_mm(&self) -> f64 {
        self.cumulative_limit_mm
    }
    pub fn identity(&self) -> &str {
        &self.identity
    }
}

pub(crate) fn apply_authorized_heal(model: &Model, plan: &AuthorizedHealPlan) -> Result<Model> {
    model.validate()?;
    let context = model
        .tolerance_context()
        .map_err(|_| refuse("BREP_HEAL_CONTEXT_MISMATCH", "Model context is invalid"))?;
    if context.spec_identity() != plan.context {
        return Err(refuse(
            "BREP_HEAL_CONTEXT_MISMATCH",
            "Plan does not belong to the model tolerance context",
        ));
    }
    if model
        .1
        .change_set
        .changes
        .iter()
        .any(|c| c.provenance.operation == plan.identity)
    {
        return Ok(model.clone());
    }
    let mut next = model.clone();
    for operation in &plan.operations {
        let (kind, index, old_id) = match operation {
            HealOperation::EndpointSnap {
                vertex,
                expected_id,
                to,
                ..
            } => {
                next.vertices[*vertex].point = *to;
                let affected_edges = next
                    .edges
                    .iter()
                    .enumerate()
                    .filter_map(|(index, edge)| edge.vertices.contains(vertex).then_some(index))
                    .collect::<Vec<_>>();
                for &edge_index in &affected_edges {
                    let edge = &mut next.edges[edge_index];
                    for endpoint in 0..2 {
                        if edge.vertices[endpoint] == *vertex {
                            let cp = if endpoint == 0 {
                                0
                            } else {
                                edge.curve.control_points.len() - 1
                            };
                            edge.curve.control_points[cp] = to.to_vec();
                        }
                    }
                }
                for edge_index in affected_edges {
                    let parent = next.1.edges[edge_index];
                    let signature = format!("{:?}", next.edges[edge_index].curve);
                    let child = TopoId::derive(
                        TopoKind::Edge,
                        &plan.identity,
                        &parent.to_string(),
                        "endpoint-snap-refit",
                        signature.as_bytes(),
                    );
                    next.1.edges[edge_index] = child;
                    next.1.change_set.nodes.insert(child, TopoKind::Edge);
                    next.1.change_set.changes.push(TopologyChange {
                        kind: ChangeKind::Modified,
                        topo_kind: TopoKind::Edge,
                        parents: vec![parent],
                        children: vec![child],
                        provenance: ChangeProvenance {
                            operation: plan.identity.clone(),
                            operand: Some(expected_id.to_string()),
                            occurrence: edge_index.to_string(),
                        },
                        role: "endpoint-snap-refit".into(),
                        anchor: None,
                    });
                }
                (TopoKind::Vertex, *vertex, *expected_id)
            }
            HealOperation::CurveRefit {
                edge,
                expected_id,
                replacement,
                ..
            } => {
                next.edges[*edge].curve = replacement.clone();
                (TopoKind::Edge, *edge, *expected_id)
            }
            HealOperation::EdgeSplit { .. } => {
                return Err(refuse(
                    "BREP_HEAL_SPLIT_REFUSED",
                    "Split recipe was not admitted",
                ));
            }
        };
        let signature = match kind {
            TopoKind::Vertex => format!("{:?}", next.vertices[index].point),
            TopoKind::Edge => format!("{:?}", next.edges[index].curve),
            _ => unreachable!(),
        };
        let new_id = TopoId::derive(
            kind,
            &plan.identity,
            &old_id.to_string(),
            "healed",
            signature.as_bytes(),
        );
        match kind {
            TopoKind::Vertex => next.1.vertices[index] = new_id,
            TopoKind::Edge => next.1.edges[index] = new_id,
            _ => unreachable!(),
        }
        next.1.change_set.nodes.insert(new_id, kind);
        next.1.change_set.changes.push(TopologyChange {
            kind: ChangeKind::Modified,
            topo_kind: kind,
            parents: vec![old_id],
            children: vec![new_id],
            provenance: ChangeProvenance {
                operation: plan.identity.clone(),
                operand: None,
                occurrence: index.to_string(),
            },
            role: "authorized-heal".into(),
            anchor: None,
        });
    }
    next.validate()?;
    sew_closed_model_edges(&next)?;
    crate::solid_audit::audit_solid(&next)?;
    Ok(next)
}

fn quantize(point: [f64; 3], scale: f64) -> Result<[i64; 3]> {
    if !point.iter().all(|x| x.is_finite()) || !(scale.is_finite() && scale > 0.) {
        return Err(refuse(
            "BREP_SEW_INVALID",
            "Sew quantization requires finite points and positive scale",
        ));
    }
    Ok(point.map(|x| (x / scale).round() as i64))
}

pub fn sew_edge_key(a: [f64; 3], b: [f64; 3], scale: f64) -> Result<SewEdgeKey> {
    let qa = quantize(a, scale)?;
    let qb = quantize(b, scale)?;
    if qa == qb {
        return Err(refuse(
            "BREP_SEW_INVALID",
            "Degenerate sew edge after exact quantization",
        ));
    }
    Ok(if qa <= qb {
        SewEdgeKey {
            a: qa,
            b: qb,
            mid: None,
            definition: None,
            shell: None,
        }
    } else {
        SewEdgeKey {
            a: qb,
            b: qa,
            mid: None,
            definition: None,
            shell: None,
        }
    })
}

fn point_bits(point: [f64; 3]) -> Result<[u64; 3]> {
    if !point.iter().all(|v| v.is_finite()) {
        return Err(refuse(
            "BREP_SEW_INVALID",
            "Boundary endpoint must be finite",
        ));
    }
    Ok(point.map(f64::to_bits))
}

/// Construct a boundary proof from two authored uses of one rational curve.
/// The caller supplies explicit cyclic locations and shell ownership.
pub fn prove_boundary_correspondence(
    context: &ToleranceContext,
    curve_a: &Curve,
    curve_b: &Curve,
    endpoints_a: [[f64; 3]; 2],
    endpoints_b: [[f64; 3]; 2],
    orientation: ParameterOrientation,
    seam_shift: i32,
    shell: usize,
    uses: [BoundaryUse; 2],
) -> Result<BoundaryCorrespondence> {
    let definition_a = RationalCurveDefinition::from_curve(curve_a)?;
    let definition_b = RationalCurveDefinition::from_curve(curve_b)?;
    if definition_a != definition_b {
        return Err(refuse(
            "BREP_SEW_CURVE_MISMATCH",
            "Boundary curves differ in their complete rational definition",
        ));
    }
    let a = [point_bits(endpoints_a[0])?, point_bits(endpoints_a[1])?];
    let b = [point_bits(endpoints_b[0])?, point_bits(endpoints_b[1])?];
    let endpoint_ok = match orientation {
        ParameterOrientation::Same => a == b,
        ParameterOrientation::Reversed => a[0] == b[1] && a[1] == b[0],
    };
    if !endpoint_ok {
        return Err(refuse(
            "BREP_SEW_ENDPOINT_MISMATCH",
            "Endpoint correspondence disagrees with parameter orientation",
        ));
    }
    if curve_a.periodic {
        if seam_shift.abs() > 1 {
            return Err(refuse(
                "BREP_SEW_SEAM",
                "Periodic seam shift exceeds one canonical period",
            ));
        }
    } else if seam_shift != 0 {
        return Err(refuse(
            "BREP_SEW_SEAM",
            "Non-periodic boundary cannot carry a seam shift",
        ));
    }
    if uses[0].wire == uses[1].wire && uses[0].cyclic_index == uses[1].cyclic_index {
        return Err(refuse(
            "BREP_SEW_DUPLICATE",
            "Boundary proof repeats the same cyclic use",
        ));
    }
    if uses[0].reversed == uses[1].reversed {
        return Err(refuse(
            "BREP_SEW_ORIENTATION",
            "Boundary uses must traverse the curve oppositely",
        ));
    }
    let [lo, hi] = curve_a.domain();
    let evaluated_lo = curve_a.evaluate(lo)?.point;
    let evaluated_hi = curve_a.evaluate(hi)?.point;
    if evaluated_lo.len() != 3 || evaluated_hi.len() != 3 {
        return Err(refuse(
            "BREP_SEW_INVALID",
            "Boundary correspondence requires a 3D curve",
        ));
    }
    let evaluated = [
        [evaluated_lo[0], evaluated_lo[1], evaluated_lo[2]],
        [evaluated_hi[0], evaluated_hi[1], evaluated_hi[2]],
    ];
    let distance = |left: [f64; 3], right: [f64; 3]| {
        left.into_iter()
            .zip(right)
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f64>()
            .sqrt()
    };
    let expected_a = if uses[0].reversed {
        [evaluated[1], evaluated[0]]
    } else {
        evaluated
    };
    let expected_b = if uses[1].reversed {
        [evaluated[1], evaluated[0]]
    } else {
        evaluated
    };
    let residual = distance(endpoints_a[0], expected_a[0])
        .max(distance(endpoints_a[1], expected_a[1]))
        .max(distance(endpoints_b[0], expected_b[0]))
        .max(distance(endpoints_b[1], expected_b[1]));
    let local_scale = distance(evaluated[0], evaluated[1]).max(1.);
    let evidence = compose_predicate_evidence(
        context,
        [
            PredicateEvidence::correspondence(context, residual, local_scale)?,
            PredicateEvidence::topology_preservation(
                context,
                "shell_owner_and_cyclic_boundary_uses",
                true,
            )?,
        ],
    )?;
    Ok(BoundaryCorrespondence {
        context: context.spec_identity(),
        authority: CurveAuthority::Rational(definition_a),
        endpoints_bits: [a, b],
        parameter_domain_bits: [lo.to_bits(), hi.to_bits()],
        orientation,
        seam_shift,
        shell,
        uses,
        evidence,
    })
}

/// Compatibility wrapper. Endpoint keys do not contain enough information to
/// prove a full boundary correspondence and therefore fail closed.
pub fn exact_sew(
    _base: &SewSnapshot,
    _pending: &[SewLedgerEntry],
) -> Result<(SewSnapshot, SewCertificate)> {
    Err(refuse(
        "BREP_SEW_CORRESPONDENCE_REQUIRED",
        "Exact sew requires context-bound BoundaryCorrespondence proofs",
    ))
}

/// Exact-match sew using one complete correspondence proof per boundary pair.
pub fn exact_sew_correspondences(
    base: &SewSnapshot,
    context: &ToleranceContext,
    pending: &[CorrespondenceLedgerEntry],
) -> Result<(SewSnapshot, SewCertificate)> {
    if pending.len() > 4096 {
        return Err(refuse(
            "BREP_SEW_RESOURCE_LIMIT",
            "Boundary correspondence budget exceeded",
        ));
    }
    let mut next = base.clone();
    let mut uses: BTreeMap<SewEdgeKey, Vec<&CorrespondenceLedgerEntry>> = BTreeMap::new();
    for entry in pending {
        uses.entry(entry.key.clone()).or_default().push(entry);
    }
    for (key, entries) in &uses {
        if entries.len() != 1 {
            return Err(refuse(
                "BREP_SEW_DUPLICATE",
                "Every sew key must have exactly one boundary correspondence",
            ));
        }
        let proof = &entries[0].correspondence;
        if proof.context != context.spec_identity() || proof.evidence.context != proof.context {
            return Err(refuse(
                "BREP_SEW_CONTEXT_MISMATCH",
                "Boundary correspondence belongs to a different tolerance context",
            ));
        }
        if proof.uses[0].reversed == proof.uses[1].reversed {
            return Err(refuse(
                "BREP_SEW_ORIENTATION",
                "Boundary proof does not establish opposite traversal",
            ));
        }
        let endpoints_match = match proof.orientation {
            ParameterOrientation::Same => proof.endpoints_bits[0] == proof.endpoints_bits[1],
            ParameterOrientation::Reversed => {
                proof.endpoints_bits[0][0] == proof.endpoints_bits[1][1]
                    && proof.endpoints_bits[0][1] == proof.endpoints_bits[1][0]
            }
        };
        if !endpoints_match {
            return Err(refuse(
                "BREP_SEW_ENDPOINT_MISMATCH",
                "Boundary proof endpoint mutation invalidated orientation",
            ));
        }
        let domain = proof.parameter_domain_bits.map(f64::from_bits);
        if !domain.iter().all(|value| value.is_finite()) || domain[0] >= domain[1] {
            return Err(refuse(
                "BREP_SEW_PARAMETERIZATION",
                "Boundary proof parameter domain is invalid",
            ));
        }
        let periodic = match &proof.authority {
            CurveAuthority::Rational(definition) => definition.periodic,
            CurveAuthority::CanonicalConstruction(identity) => {
                if identity.is_empty() || identity.len() > 256 {
                    return Err(refuse(
                        "BREP_SEW_CURVE_MISMATCH",
                        "Canonical construction identity is invalid",
                    ));
                }
                false
            }
        };
        let authority_key = match &proof.authority {
            CurveAuthority::Rational(definition) => format!("{definition:?}"),
            CurveAuthority::CanonicalConstruction(identity) => identity.clone(),
        };
        if key.definition.as_deref() != Some(authority_key.as_str()) {
            return Err(refuse(
                "BREP_SEW_CURVE_MISMATCH",
                "Sew key does not carry the proven complete curve authority",
            ));
        }
        if (!periodic && proof.seam_shift != 0) || proof.seam_shift.abs() > 1 {
            return Err(refuse(
                "BREP_SEW_SEAM",
                "Boundary proof carries an incompatible periodic seam shift",
            ));
        }
        if key.shell != Some(proof.shell)
            || (proof.uses[0].wire == proof.uses[1].wire
                && proof.uses[0].cyclic_index == proof.uses[1].cyclic_index)
        {
            return Err(refuse(
                "BREP_SEW_SHELL_OWNERSHIP",
                "Boundary proof shell or cyclic ownership is inconsistent",
            ));
        }
        let has_correspondence = proof
            .evidence
            .claims
            .iter()
            .any(|claim| matches!(claim, EvidenceClaim::Correspondence { .. }));
        let has_topology = proof
            .evidence
            .claims
            .iter()
            .any(|claim| matches!(claim, EvidenceClaim::TopologyPreservation { .. }));
        if !has_correspondence || !has_topology {
            return Err(refuse(
                "BREP_SEW_CORRESPONDENCE_REQUIRED",
                "Boundary proof lacks correspondence or topology evidence",
            ));
        }
        next.edges.insert(key.clone());
        next.oriented.insert(key.clone(), true);
    }
    let budget_ok = next.displacements.iter().all(|d| d.bound <= 1_000_000);
    Ok((
        next,
        SewCertificate {
            matched: uses.len(),
            complete: true,
            displacement_budget_ok: budget_ok,
        },
    ))
}

/// Rollback drill helper: apply sew or restore `base` on any refusal.
pub fn sew_atomic(
    base: SewSnapshot,
    pending: &[SewLedgerEntry],
) -> Result<(SewSnapshot, SewCertificate)> {
    match exact_sew(&base, pending) {
        Ok(done) => Ok(done),
        Err(error) => Err(error),
    }
}

/// Classification from imprint circle strata on an AnalyticCircle chart.
pub fn classify_imprint_circle_events(
    mut events: Vec<ChartEvent>,
) -> Result<ClassificationCertificate> {
    if events.is_empty() {
        return classify_chart_events(
            ChartKind::AnalyticCircle,
            vec![],
            &[(0.5, CellLabel::Outside)],
        );
    }
    events.sort_by(|a, b| {
        a.parameter
            .partial_cmp(&b.parameter)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.edge.cmp(&b.edge))
    });
    // Deduplicate near-equal parameters into Boundary strata.
    let mut unique = Vec::new();
    for event in events {
        if unique
            .last()
            .is_some_and(|e: &ChartEvent| (e.parameter - event.parameter).abs() <= 1e-12)
        {
            continue;
        }
        unique.push(event);
    }
    let mut samples = Vec::new();
    for (i, event) in unique.iter().enumerate() {
        samples.push((event.parameter, CellLabel::Boundary));
        let next = unique
            .get(i + 1)
            .map(|e| e.parameter)
            .unwrap_or(event.parameter + 1.);
        let mid = 0.5 * (event.parameter + next);
        if (mid - event.parameter).abs() > 1e-12 {
            samples.push((mid, CellLabel::Inside));
        }
    }
    if let Some(first) = unique.first() {
        if first.parameter > 0. {
            samples.insert(0, (first.parameter * 0.5, CellLabel::Outside));
        }
    }
    classify_chart_events(ChartKind::AnalyticCircle, unique, &samples)
}

/// Build chart events from an authored face outer loop (parameter along coedges).
pub fn classify_face_outer_loop(
    model: &crate::Model,
    face_index: usize,
    chart: ChartKind,
) -> Result<ClassificationCertificate> {
    model.validate()?;
    let face = model.faces.get(face_index).ok_or_else(|| {
        refuse(
            "BREP_TRIM_INVALID",
            "Face index out of range for chart classification",
        )
    })?;
    let wire = model.loops.get(face.outer).ok_or_else(|| {
        refuse(
            "BREP_TRIM_INVALID",
            "Face outer loop missing for chart classification",
        )
    })?;
    if wire.coedges.is_empty() {
        return Err(refuse(
            "BREP_TRIM_INVALID",
            "Empty outer loop cannot be classified",
        ));
    }
    let mut events = Vec::new();
    let mut samples = Vec::new();
    let n = wire.coedges.len();
    for (i, coedge) in wire.coedges.iter().enumerate() {
        let t = i as f64 / n as f64;
        events.push(ChartEvent {
            parameter: t,
            kind: "coedge",
            edge: coedge.edge,
        });
        samples.push((t, CellLabel::Boundary));
        let mid = (i as f64 + 0.5) / n as f64;
        // Material between coedge knots is treated as Inside for a closed outer
        // loop on the frozen chart classes; exterior is not claimed here.
        samples.push((mid, CellLabel::Inside));
    }
    classify_chart_events(chart, events, &samples)
}

/// Exact sew of a closed shell's unique edge pairs from authored vertex points.
/// Returns a certificate or typed refuse; never mutates the model.
pub fn sew_closed_model_edges(model: &crate::Model) -> Result<SewCertificate> {
    model.validate()?;
    let context = model
        .tolerance_context()
        .map_err(|_| refuse("BREP_SEW_INVALID", "Model tolerance context is invalid"))?;
    let scale = model.tolerance_mm.max(1e-9);
    let mut face_usage = vec![None; model.faces.len()];
    for (shell_id, shell) in model.shells.iter().enumerate() {
        if !shell.closed {
            return Err(refuse(
                "BREP_SEW_GAP",
                "Solid sew requires every audited shell to be closed",
            ));
        }
        for use_ in &shell.faces {
            if face_usage[use_.face]
                .replace((use_.reversed, shell_id))
                .is_some()
            {
                return Err(refuse(
                    "BREP_SEW_DUPLICATE",
                    "Face is owned by more than one shell",
                ));
            }
        }
    }
    if face_usage.iter().any(Option::is_none) {
        return Err(refuse(
            "BREP_SEW_GAP",
            "Solid sew found a face not owned by a shell",
        ));
    }
    let mut edge_uses: BTreeMap<usize, Vec<(usize, usize, usize, bool, usize)>> = BTreeMap::new();
    for (face_a, face) in model.faces.iter().enumerate() {
        for &wire_id in std::iter::once(&face.outer).chain(face.holes.iter()) {
            let wire = &model.loops[wire_id];
            for (cyclic_index, coedge) in wire.coedges.iter().enumerate() {
                let (shell_reversed, shell_id) = face_usage[face_a].unwrap_or((false, 0));
                edge_uses.entry(coedge.edge).or_default().push((
                    face_a,
                    wire_id,
                    cyclic_index,
                    coedge.reversed ^ shell_reversed,
                    shell_id,
                ));
            }
        }
    }
    let mut proven = Vec::new();
    for (edge_id, uses) in edge_uses {
        if uses.len() != 2 {
            return Err(refuse(
                if uses.len() < 2 {
                    "BREP_SEW_GAP"
                } else {
                    "BREP_SEW_DUPLICATE"
                },
                "Closed model edge incidence is not a unique pair",
            ));
        }
        if uses[0].4 != uses[1].4 {
            return Err(refuse(
                "BREP_SEW_SHELL_OWNERSHIP",
                "Boundary mates are not owned by the same shell",
            ));
        }
        let edge = &model.edges[edge_id];
        let endpoints = [
            model.vertices[edge.vertices[0]].point,
            model.vertices[edge.vertices[1]].point,
        ];
        let traversal = |reversed: bool| {
            if reversed {
                [endpoints[1], endpoints[0]]
            } else {
                endpoints
            }
        };
        let seam_shift = if edge.curve.periodic && uses[0].0 == uses[1].0 {
            if uses[0].3 { -1 } else { 1 }
        } else {
            0
        };
        let correspondence = prove_boundary_correspondence(
            &context,
            &edge.curve,
            &edge.curve,
            traversal(uses[0].3),
            traversal(uses[1].3),
            ParameterOrientation::Reversed,
            seam_shift,
            uses[0].4,
            [
                BoundaryUse {
                    face: uses[0].0,
                    wire: uses[0].1,
                    cyclic_index: uses[0].2,
                    reversed: uses[0].3,
                },
                BoundaryUse {
                    face: uses[1].0,
                    wire: uses[1].1,
                    cyclic_index: uses[1].2,
                    reversed: uses[1].3,
                },
            ],
        )?;
        let mut key = sew_edge_key(endpoints[0], endpoints[1], scale)?;
        key.shell = Some(uses[0].4);
        key.definition = Some(match &correspondence.authority {
            CurveAuthority::Rational(definition) => format!("{definition:?}"),
            CurveAuthority::CanonicalConstruction(identity) => identity.clone(),
        });
        proven.push(CorrespondenceLedgerEntry {
            key,
            correspondence,
        });
    }
    let (_snap, cert) = exact_sew_correspondences(&SewSnapshot::default(), &context, &proven)?;
    Ok(cert)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn certified_event_order_and_classification() {
        let cert = classify_chart_events(
            ChartKind::PlanePoly,
            vec![
                ChartEvent {
                    parameter: 0.25,
                    kind: "enter",
                    edge: 0,
                },
                ChartEvent {
                    parameter: 0.75,
                    kind: "exit",
                    edge: 1,
                },
            ],
            &[
                (0.1, CellLabel::Outside),
                (0.5, CellLabel::Inside),
                (0.9, CellLabel::Outside),
            ],
        )
        .unwrap();
        assert!(cert.complete);
        assert_eq!(cert.cells.len(), 3);
    }

    #[test]
    fn sample_on_event_without_boundary_label_refuses() {
        assert!(
            classify_chart_events(
                ChartKind::AnalyticCircle,
                vec![ChartEvent {
                    parameter: 0.5,
                    kind: "root",
                    edge: 0,
                }],
                &[(0.5, CellLabel::Inside)],
            )
            .is_err()
        );
    }

    #[test]
    fn endpoint_only_exact_sew_wrapper_fails_closed() {
        let key = sew_edge_key([0., 0., 0.], [1., 0., 0.], 1e-9).unwrap();
        let pending = [
            SewLedgerEntry {
                key: key.clone(),
                face_a: 0,
                face_b: 1,
                orientation_agree: true,
                displacement: None,
            },
            SewLedgerEntry {
                key: key.clone(),
                face_a: 1,
                face_b: 0,
                orientation_agree: false,
                displacement: None,
            },
        ];
        let base = SewSnapshot::default();
        assert_eq!(
            sew_atomic(base, &pending).unwrap_err().code,
            "BREP_SEW_CORRESPONDENCE_REQUIRED"
        );
    }

    fn proven_line() -> (ToleranceContext, SewEdgeKey, BoundaryCorrespondence, Curve) {
        let context = ToleranceContext::default_valid();
        let curve = Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.]]).unwrap();
        let uses = [
            BoundaryUse {
                face: 0,
                wire: 0,
                cyclic_index: 0,
                reversed: false,
            },
            BoundaryUse {
                face: 1,
                wire: 1,
                cyclic_index: 0,
                reversed: true,
            },
        ];
        let proof = prove_boundary_correspondence(
            &context,
            &curve,
            &curve,
            [[0., 0., 0.], [1., 0., 0.]],
            [[1., 0., 0.], [0., 0., 0.]],
            ParameterOrientation::Reversed,
            0,
            7,
            uses,
        )
        .unwrap();
        let mut key = sew_edge_key([0., 0., 0.], [1., 0., 0.], 1e-9).unwrap();
        key.shell = Some(7);
        key.definition = Some(match &proof.authority {
            CurveAuthority::Rational(definition) => format!("{definition:?}"),
            CurveAuthority::CanonicalConstruction(identity) => identity.clone(),
        });
        (context, key, proof, curve)
    }

    #[test]
    fn correspondence_proof_sews_and_mutations_refuse() {
        let (context, key, proof, curve) = proven_line();
        let entry = CorrespondenceLedgerEntry {
            key: key.clone(),
            correspondence: proof.clone(),
        };
        let (_, certificate) =
            exact_sew_correspondences(&SewSnapshot::default(), &context, &[entry]).unwrap();
        assert!(certificate.complete);

        let mut reversed = proof.clone();
        reversed.orientation = ParameterOrientation::Same;
        assert!(
            exact_sew_correspondences(
                &SewSnapshot::default(),
                &context,
                &[CorrespondenceLedgerEntry {
                    key: key.clone(),
                    correspondence: reversed,
                }],
            )
            .is_err()
        );

        let mut foreign_spec = context.specification().clone();
        foreign_spec.policy = "foreign-sew-context".into();
        let foreign = ToleranceContext::new(foreign_spec).unwrap();
        assert!(
            exact_sew_correspondences(
                &SewSnapshot::default(),
                &foreign,
                &[CorrespondenceLedgerEntry {
                    key: key.clone(),
                    correspondence: proof.clone(),
                }],
            )
            .is_err()
        );

        let mut different = curve.clone();
        different.weights[0] = 2.;
        assert!(
            prove_boundary_correspondence(
                &context,
                &curve,
                &different,
                [[0., 0., 0.], [1., 0., 0.]],
                [[1., 0., 0.], [0., 0., 0.]],
                ParameterOrientation::Reversed,
                0,
                7,
                proof.uses.clone(),
            )
            .is_err()
        );

        let mut seam = proof;
        seam.seam_shift = 1;
        assert!(
            exact_sew_correspondences(
                &SewSnapshot::default(),
                &context,
                &[CorrespondenceLedgerEntry {
                    key,
                    correspondence: seam,
                }],
            )
            .is_err()
        );

        let (_, key, proof, _) = proven_line();
        let excessive = (0..=4096)
            .map(|_| CorrespondenceLedgerEntry {
                key: key.clone(),
                correspondence: proof.clone(),
            })
            .collect::<Vec<_>>();
        assert_eq!(
            exact_sew_correspondences(&SewSnapshot::default(), &context, &excessive)
                .unwrap_err()
                .code,
            "BREP_SEW_RESOURCE_LIMIT"
        );
    }

    #[test]
    fn sew_gap_refuses_without_mutating_base() {
        let key = sew_edge_key([0., 0., 0.], [1., 0., 0.], 1e-9).unwrap();
        let pending = [SewLedgerEntry {
            key,
            face_a: 0,
            face_b: 1,
            orientation_agree: true,
            displacement: None,
        }];
        let base = SewSnapshot::default();
        let err = sew_atomic(base.clone(), &pending).unwrap_err();
        assert_eq!(err.code, "BREP_SEW_CORRESPONDENCE_REQUIRED");
        assert!(base.edges.is_empty());
    }

    #[test]
    fn sew_duplicate_and_orientation_refuse() {
        let key = sew_edge_key([0., 0., 0.], [0., 1., 0.], 1e-9).unwrap();
        let dup = [
            SewLedgerEntry {
                key: key.clone(),
                face_a: 0,
                face_b: 1,
                orientation_agree: true,
                displacement: None,
            },
            SewLedgerEntry {
                key: key.clone(),
                face_a: 2,
                face_b: 3,
                orientation_agree: false,
                displacement: None,
            },
            SewLedgerEntry {
                key: key.clone(),
                face_a: 4,
                face_b: 5,
                orientation_agree: true,
                displacement: None,
            },
        ];
        assert_eq!(
            sew_atomic(SewSnapshot::default(), &dup).unwrap_err().code,
            "BREP_SEW_CORRESPONDENCE_REQUIRED"
        );
        let bad_orient = [
            SewLedgerEntry {
                key: key.clone(),
                face_a: 0,
                face_b: 1,
                orientation_agree: true,
                displacement: None,
            },
            SewLedgerEntry {
                key,
                face_a: 1,
                face_b: 0,
                orientation_agree: true,
                displacement: None,
            },
        ];
        assert_eq!(
            sew_atomic(SewSnapshot::default(), &bad_orient)
                .unwrap_err()
                .code,
            "BREP_SEW_CORRESPONDENCE_REQUIRED"
        );
    }

    #[test]
    fn cuboid_face_outer_loop_classifies_complete() {
        let model = crate::cuboid([0.; 3], [2.; 3]).unwrap();
        let cert = classify_face_outer_loop(&model, 0, ChartKind::PlanePoly).unwrap();
        assert!(cert.complete);
        assert!(!cert.events.is_empty());
    }

    #[test]
    fn cylinder_closed_edge_sew_or_typed_refuse() {
        let model = crate::cylinder(2., 4.).unwrap();
        // Analytic cylinder shares edges across faces; expect Complete sew or a
        // typed incidence refuse — never silent heal.
        match sew_closed_model_edges(&model) {
            Ok(cert) => assert!(cert.complete),
            Err(err) => assert!(
                err.code == "BREP_SEW_GAP"
                    || err.code == "BREP_SEW_DUPLICATE"
                    || err.code == "BREP_SEW_ORIENTATION"
                    || err.code == "BREP_SEW_INVALID"
            ),
        }
    }

    fn heal_fixture() -> (Model, ToleranceContext, BoundaryCorrespondence) {
        let model = crate::cylinder(2., 4.).unwrap();
        let context = model.tolerance_context().unwrap();
        let edge = &model.edges[0];
        let endpoints = [
            model.vertices[edge.vertices[0]].point,
            model.vertices[edge.vertices[1]].point,
        ];
        let proof = prove_boundary_correspondence(
            &context,
            &edge.curve,
            &edge.curve,
            endpoints,
            [endpoints[1], endpoints[0]],
            ParameterOrientation::Reversed,
            0,
            0,
            [
                BoundaryUse {
                    face: 0,
                    wire: 0,
                    cyclic_index: 0,
                    reversed: false,
                },
                BoundaryUse {
                    face: 1,
                    wire: 1,
                    cyclic_index: 0,
                    reversed: true,
                },
            ],
        )
        .unwrap();
        (model, context, proof)
    }

    #[test]
    fn heal_plan_refuses_budget_context_and_missing_lineage() {
        let (model, context, proof) = heal_fixture();
        let vertex = model.edges[0].vertices[0];
        let mut to = model.vertices[vertex].point;
        to[0] += context.spatial_bounds().absolute_mm * 0.25;
        let operation = HealOperation::EndpointSnap {
            vertex,
            expected_id: model.1.vertices[vertex],
            to,
            correspondence: proof.clone(),
        };
        assert_eq!(
            AuthorizedHealPlan::new(
                &model,
                &context,
                vec![operation.clone()],
                2. * context.spatial_bounds().absolute_mm,
                2. * context.spatial_bounds().absolute_mm
            )
            .unwrap_err()
            .code,
            "BREP_HEAL_BUDGET_EXCEEDED"
        );
        let mut foreign_spec = context.specification().clone();
        foreign_spec.policy = "foreign-heal".into();
        let foreign = ToleranceContext::new(foreign_spec).unwrap();
        assert_eq!(
            AuthorizedHealPlan::new(
                &model,
                &foreign,
                vec![operation],
                foreign.spatial_bounds().absolute_mm,
                foreign.spatial_bounds().absolute_mm
            )
            .unwrap_err()
            .code,
            "BREP_HEAL_CONTEXT_MISMATCH"
        );
        let bad = HealOperation::EndpointSnap {
            vertex,
            expected_id: TopoId::derive(TopoKind::Vertex, "bad", "bad", "bad", b"bad"),
            to,
            correspondence: proof,
        };
        assert_eq!(
            AuthorizedHealPlan::new(
                &model,
                &context,
                vec![bad],
                context.spatial_bounds().absolute_mm,
                context.spatial_bounds().absolute_mm
            )
            .unwrap_err()
            .code,
            "BREP_HEAL_LINEAGE_MISMATCH"
        );
    }

    #[test]
    fn heal_transaction_cancel_rollback_and_idempotence() {
        let (model, context, proof) = heal_fixture();
        let vertex = model.edges[0].vertices[0];
        let to = model.vertices[vertex].point;
        let plan = AuthorizedHealPlan::new(
            &model,
            &context,
            vec![HealOperation::EndpointSnap {
                vertex,
                expected_id: model.1.vertices[vertex],
                to,
                correspondence: proof,
            }],
            context.spatial_bounds().absolute_mm,
            context.spatial_bounds().absolute_mm,
        )
        .unwrap();
        let snapshot = crate::transactions::ModelSnapshot::new(model.clone()).unwrap();
        let cancellation = crate::transactions::HealCancellation::default();
        let mut transaction = crate::transactions::AuthorizedHealTransaction::begin(
            snapshot.clone(),
            cancellation.clone(),
        );
        cancellation.cancel();
        assert_eq!(
            transaction.apply(&plan).unwrap_err().code,
            "BREP_HEAL_CANCELLED"
        );

        let mut transaction = crate::transactions::AuthorizedHealTransaction::begin(
            snapshot,
            crate::transactions::HealCancellation::default(),
        );
        transaction.apply(&plan).unwrap();
        let once = transaction.staged().unwrap().clone();
        transaction.rollback();
        assert!(transaction.staged().is_none());
        let twice = apply_authorized_heal(&once, &plan).unwrap();
        assert_eq!(
            once.1.change_set.changes.len(),
            twice.1.change_set.changes.len()
        );
    }
}
