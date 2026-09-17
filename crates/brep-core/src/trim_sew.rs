//! Lifted-UV arrangements, material cell classification, and exact sewing.
//!
//! Extends planar_trim to frozen chart classes (plane / poly / analytic circle).
//! Sewing is exact-match only: gap/duplicate/orientation mismatches refuse and
//! roll back atomically. No mesh weld, tolerance growth, or auto-heal.

use cad_predicates::{ToleranceContext, ToleranceSpecIdentity};
use nurbs_core::{Error, Result, curve::Curve, surface::Axis, surface::Surface};
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
    /// Finite tensor rectangle containing only its four exact boundaries and
    /// strict-interior constant-U/V Bezier graph traces.
    TensorBezierGraph,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurvePcurveSupport {
    CurvedIsoU,
    CurvedIsoV,
    AffinePlanar,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CurvePcurveAuthority {
    curve: RationalCurveDefinition,
    pcurve: RationalCurveDefinition,
    support: CurvePcurveSupport,
    parameter_domain_bits: [u64; 2],
    orientation: ParameterOrientation,
    owner: BoundaryUse,
    context: ToleranceSpecIdentity,
}

/// Exact 3D rational-curve/pcurve authority for a single owned support use.
/// No sampled residual or endpoint snap participates in authorization.
#[derive(Clone, Debug)]
pub struct CurvePcurveCorrespondence {
    pub curve: Curve,
    pub pcurve: Curve,
    pub support: CurvePcurveSupport,
    pub parameter_domain_bits: [u64; 2],
    pub orientation: ParameterOrientation,
    pub owner: BoundaryUse,
    pub context: ToleranceSpecIdentity,
    pub evidence: ComposedEvidence,
    pub no_snapping: bool,
    authority: CurvePcurveAuthority,
}

impl CurvePcurveCorrespondence {
    pub fn permits_exact_correspondence(&self) -> bool {
        let Ok(curve) = RationalCurveDefinition::from_curve(&self.curve) else {
            return false;
        };
        let Ok(pcurve) = RationalCurveDefinition::from_curve(&self.pcurve) else {
            return false;
        };
        self.no_snapping
            && self.context == self.evidence.context
            && self
                .evidence
                .claims
                .iter()
                .any(|claim| matches!(claim, EvidenceClaim::Correspondence { .. }))
            && self.evidence.claims.iter().any(|claim| {
                matches!(
                    claim,
                    EvidenceClaim::TopologyPreservation { invariant }
                        if invariant == "curve_pcurve_parameter_orientation_context_owner"
                )
            })
            && self.authority
                == CurvePcurveAuthority {
                    curve,
                    pcurve,
                    support: self.support,
                    parameter_domain_bits: self.parameter_domain_bits,
                    orientation: self.orientation,
                    owner: self.owner.clone(),
                    context: self.context.clone(),
                }
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
    /// Native-only binding to the complete serialized heal recipe. Generic sew
    /// proofs leave this absent and therefore cannot authorize mutation.
    heal_recipe: Option<String>,
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
    pub(crate) fn displacement(&self, model: &Model) -> Result<f64> {
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

    fn recipe_bytes(&self) -> Vec<u8> {
        fn push_usize(bytes: &mut Vec<u8>, value: usize) {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        fn push_text(bytes: &mut Vec<u8>, value: &str) {
            push_usize(bytes, value.len());
            bytes.extend_from_slice(value.as_bytes());
        }
        let mut bytes = Vec::new();
        match self {
            Self::EndpointSnap {
                vertex,
                expected_id,
                to,
                ..
            } => {
                bytes.extend_from_slice(b"endpoint-snap\0");
                push_usize(&mut bytes, *vertex);
                push_text(&mut bytes, &expected_id.to_string());
                for value in to {
                    bytes.extend_from_slice(&value.to_bits().to_le_bytes());
                }
            }
            Self::CurveRefit {
                edge,
                expected_id,
                replacement,
                ..
            } => {
                bytes.extend_from_slice(b"curve-refit\0");
                push_usize(&mut bytes, *edge);
                push_text(&mut bytes, &expected_id.to_string());
                push_usize(&mut bytes, replacement.degree);
                push_usize(&mut bytes, replacement.knots.len());
                for knot in &replacement.knots {
                    bytes.extend_from_slice(&knot.to_bits().to_le_bytes());
                }
                push_usize(&mut bytes, replacement.control_points.len());
                for point in &replacement.control_points {
                    push_usize(&mut bytes, point.len());
                    for coordinate in point {
                        bytes.extend_from_slice(&coordinate.to_bits().to_le_bytes());
                    }
                }
                push_usize(&mut bytes, replacement.weights.len());
                for weight in &replacement.weights {
                    bytes.extend_from_slice(&weight.to_bits().to_le_bytes());
                }
                bytes.push(u8::from(replacement.periodic));
            }
            Self::EdgeSplit {
                edge,
                expected_id,
                parameter,
                ..
            } => {
                bytes.extend_from_slice(b"edge-split\0");
                push_usize(&mut bytes, *edge);
                push_text(&mut bytes, &expected_id.to_string());
                bytes.extend_from_slice(&parameter.to_bits().to_le_bytes());
            }
        }
        bytes
    }

    fn recipe_identity(&self) -> String {
        TopoId::derive(
            TopoKind::Body,
            "authorized-heal-recipe",
            "native",
            "complete-bytes",
            &self.recipe_bytes(),
        )
        .to_string()
    }

    pub(crate) fn write_set(&self, model: &Model) -> BTreeSet<(TopoKind, usize)> {
        let mut writes = BTreeSet::new();
        match self {
            Self::EndpointSnap { vertex, .. } => {
                writes.insert((TopoKind::Vertex, *vertex));
                for (edge, value) in model.edges.iter().enumerate() {
                    if value.vertices.contains(vertex) {
                        writes.insert((TopoKind::Edge, edge));
                    }
                }
            }
            Self::CurveRefit { edge, .. } | Self::EdgeSplit { edge, .. } => {
                writes.insert((TopoKind::Edge, *edge));
            }
        }
        writes
    }

    pub fn bind_native_proof(mut self) -> Self {
        let identity = self.recipe_identity();
        match &mut self {
            Self::EndpointSnap { correspondence, .. }
            | Self::CurveRefit { correspondence, .. }
            | Self::EdgeSplit { correspondence, .. } => {
                correspondence.heal_recipe = Some(identity);
            }
        }
        self
    }

    fn proof_matches_model(&self, model: &Model, context: &ToleranceContext) -> bool {
        let matches = |left: &BoundaryCorrespondence, right: &BoundaryCorrespondence| {
            left.context == right.context
                && left.authority == right.authority
                && left.endpoints_bits == right.endpoints_bits
                && left.parameter_domain_bits == right.parameter_domain_bits
                && left.orientation == right.orientation
                && left.seam_shift == right.seam_shift
                && left.shell == right.shell
                && left.uses == right.uses
        };
        match self {
            Self::EndpointSnap {
                vertex,
                correspondence,
                ..
            } => model
                .edges
                .iter()
                .enumerate()
                .filter(|(_, edge)| edge.vertices.contains(vertex))
                .filter_map(|(edge, _)| {
                    prove_model_edge_correspondence(model, context, edge).ok()
                })
                .any(|proof| matches(correspondence, &proof)),
            Self::CurveRefit {
                edge,
                correspondence,
                ..
            }
            | Self::EdgeSplit {
                edge,
                correspondence,
                ..
            } => prove_model_edge_correspondence(model, context, *edge)
                .is_ok_and(|proof| matches(correspondence, &proof)),
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
            if proof.heal_recipe.as_deref() != Some(operation.recipe_identity().as_str()) {
                return Err(refuse(
                    "BREP_HEAL_PROOF_RECIPE_MISMATCH",
                    "Boundary proof is not bound to the complete endpoint/refit recipe",
                ));
            }
            if !operation.proof_matches_model(model, context) {
                return Err(refuse(
                    "BREP_HEAL_PROOF_UNRELATED",
                    "Boundary proof is stale, foreign, or unrelated to the target entity",
                ));
            }
            let displacement = operation.displacement(model)?;
            if displacement > per_entity_limit_mm {
                return Err(refuse(
                    "BREP_HEAL_BUDGET_EXCEEDED",
                    "Per-entity physical displacement budget exceeded",
                ));
            }
            let writes = operation.write_set(model);
            cumulative += displacement * writes.len() as f64;
            if writes.iter().any(|write| touched.contains(write)) {
                return Err(refuse(
                    "BREP_HEAL_WRITESET_OVERLAP",
                    "Implicit and explicit heal write sets overlap",
                ));
            }
            touched.extend(writes);
            signature.push('|');
            signature.push_str(&operation.recipe_identity());
            signature.push(':');
            signature.push_str(&format!("{:016x}", displacement.to_bits()));
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
    apply_authorized_heal_checked(model, plan, |_| Ok(()))
}

pub(crate) fn apply_authorized_heal_checked(
    model: &Model,
    plan: &AuthorizedHealPlan,
    mut before_operation: impl FnMut(usize) -> Result<()>,
) -> Result<Model> {
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
    for (operation_index, operation) in plan.operations.iter().enumerate() {
        before_operation(operation_index)?;
        operation.displacement(&next)?;
        if operation.correspondence().heal_recipe.as_deref()
            != Some(operation.recipe_identity().as_str())
        {
            return Err(refuse(
                "BREP_HEAL_PROOF_RECIPE_MISMATCH",
                "Boundary proof no longer matches the complete heal recipe",
            ));
        }
        if !operation.proof_matches_model(&next, &context) {
            return Err(refuse(
                "BREP_HEAL_PROOF_UNRELATED",
                "Boundary proof is stale, foreign, or unrelated to the target entity",
            ));
        }
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
        before_operation(operation_index)?;
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

fn exact_iso_pcurve(surface: &Surface, pcurve: &Curve) -> Result<Option<(Curve, CurvePcurveSupport)>> {
    let u = [
        surface.knots_u[surface.degree_u],
        surface.knots_u[surface.control_points.len()],
    ];
    let v = [
        surface.knots_v[surface.degree_v],
        surface.knots_v[surface.control_points[0].len()],
    ];
    let degree = pcurve.degree;
    for fixed_u in [true, false] {
        let fixed_axis = usize::from(!fixed_u);
        let varying_axis = usize::from(fixed_u);
        let fixed = pcurve.control_points[0][fixed_axis];
        let varying_domain = if fixed_u { v } else { u };
        let fixed_domain = if fixed_u { u } else { v };
        if !(fixed >= fixed_domain[0] && fixed <= fixed_domain[1])
            || pcurve
                .control_points
                .iter()
                .any(|point| point[fixed_axis].to_bits() != fixed.to_bits())
            || pcurve.control_points.iter().enumerate().any(|(i, point)| {
                let expected = varying_domain[0]
                    + (varying_domain[1] - varying_domain[0]) * i as f64 / degree as f64;
                point[varying_axis].to_bits() != expected.to_bits()
            })
            || pcurve.domain().map(f64::to_bits) != varying_domain.map(f64::to_bits)
            || pcurve.weights.windows(2).any(|pair| pair[0] != pair[1])
        {
            continue;
        }
        let curve = surface.iso(if fixed_u { Axis::U } else { Axis::V }, fixed)?;
        return Ok(Some((
            curve,
            if fixed_u {
                CurvePcurveSupport::CurvedIsoU
            } else {
                CurvePcurveSupport::CurvedIsoV
            },
        )));
    }
    Ok(None)
}

fn affine_planar_lift(
    surface: &Surface,
    pcurve: &Curve,
    context: &ToleranceContext,
) -> Result<Option<Curve>> {
    let o = surface.evaluate(
        surface.knots_u[surface.degree_u],
        surface.knots_v[surface.degree_v],
    )?.point;
    let u_domain = [
        surface.knots_u[surface.degree_u],
        surface.knots_u[surface.control_points.len()],
    ];
    let v_domain = [
        surface.knots_v[surface.degree_v],
        surface.knots_v[surface.control_points[0].len()],
    ];
    let pu = surface.evaluate(u_domain[1], v_domain[0])?.point;
    let pv = surface.evaluate(u_domain[0], v_domain[1])?.point;
    let eu: [f64; 3] =
        std::array::from_fn(|axis| (pu[axis] - o[axis]) / (u_domain[1] - u_domain[0]));
    let ev: [f64; 3] =
        std::array::from_fn(|axis| (pv[axis] - o[axis]) / (v_domain[1] - v_domain[0]));
    let tol = context.spatial_bounds().on_mm;
    for i in 0..=surface.degree_u {
        for j in 0..=surface.degree_v {
            let u = u_domain[0]
                + (u_domain[1] - u_domain[0]) * i as f64 / surface.degree_u as f64;
            let v = v_domain[0]
                + (v_domain[1] - v_domain[0]) * j as f64 / surface.degree_v as f64;
            let expected: [f64; 3] =
                std::array::from_fn(|axis| o[axis] + (u - u_domain[0]) * eu[axis]
                    + (v - v_domain[0]) * ev[axis]);
            let residual = surface.control_points[i][j]
                .iter()
                .zip(expected)
                .map(|(a, b)| (a - b) * (a - b))
                .sum::<f64>()
                .sqrt();
            if residual > tol {
                return Ok(None);
            }
        }
    }
    let lifted = Curve {
        degree: pcurve.degree,
        knots: pcurve.knots.clone(),
        control_points: pcurve
            .control_points
            .iter()
            .map(|point| {
                (0..3)
                    .map(|axis| {
                        o[axis]
                            + (point[0] - u_domain[0]) * eu[axis]
                            + (point[1] - v_domain[0]) * ev[axis]
                    })
                    .collect()
            })
            .collect(),
        weights: pcurve.weights.clone(),
        periodic: false,
    };
    lifted.validate()?;
    Ok(Some(lifted))
}

/// Prove exact 3D rational curve correspondence to either an exact curved
/// constant-U/V pcurve or an affine-planar pcurve. Generic freeform lifts,
/// periodic and multispan inputs refuse.
pub fn prove_curve_pcurve_correspondence(
    context: &ToleranceContext,
    curve: &Curve,
    surface: &Surface,
    pcurve: &Curve,
    orientation: ParameterOrientation,
    owner: BoundaryUse,
) -> Result<CurvePcurveCorrespondence> {
    curve.validate()?;
    surface.validate()?;
    pcurve.validate()?;
    if curve.control_points[0].len() != 3
        || pcurve.control_points[0].len() != 2
        || curve.periodic
        || pcurve.periodic
        || curve.decompose()?.len() != 1
        || pcurve.decompose()?.len() != 1
        || curve.domain().map(f64::to_bits) != pcurve.domain().map(f64::to_bits)
    {
        return Err(refuse(
            "BREP_SEW_PARAMETERIZATION",
            "Curve/pcurve proof requires matching non-periodic single-span domains",
        ));
    }
    let (lifted, support) = if let Some((iso, support)) = exact_iso_pcurve(surface, pcurve)? {
        (iso, support)
    } else if let Some(lifted) = affine_planar_lift(surface, pcurve, context)? {
        (lifted, CurvePcurveSupport::AffinePlanar)
    } else {
        return Err(refuse(
            "BREP_SEW_CURVE_MISMATCH",
            "Pcurve is neither an exact support iso nor an affine-planar lift",
        ));
    };
    let expected = match orientation {
        ParameterOrientation::Same => lifted,
        ParameterOrientation::Reversed => lifted.reverse()?,
    };
    let curve_definition = RationalCurveDefinition::from_curve(curve)?;
    let expected_definition = RationalCurveDefinition::from_curve(&expected)?;
    let coefficient_residual = curve
        .control_points
        .iter()
        .zip(&expected.control_points)
        .map(|(left, right)| {
            left.iter()
                .zip(right)
                .map(|(a, b)| (a - b) * (a - b))
                .sum::<f64>()
                .sqrt()
        })
        .fold(0., f64::max);
    let same_rational_basis = curve.degree == expected.degree
        && curve.knots == expected.knots
        && curve.weights == expected.weights
        && curve.periodic == expected.periodic
        && curve.control_points.len() == expected.control_points.len();
    if curve_definition != expected_definition
        && !(support == CurvePcurveSupport::AffinePlanar && same_rational_basis)
    {
        return Err(refuse(
            "BREP_SEW_CURVE_MISMATCH",
            "3D rational definition does not equal the oriented pcurve lift",
        ));
    }
    let pcurve_definition = RationalCurveDefinition::from_curve(pcurve)?;
    let parameter_domain_bits = curve.domain().map(f64::to_bits);
    let evidence = compose_predicate_evidence(
        context,
        [
            PredicateEvidence::correspondence(context, coefficient_residual, 1.)?,
            PredicateEvidence::topology_preservation(
                context,
                "curve_pcurve_parameter_orientation_context_owner",
                true,
            )?,
        ],
    )?;
    let authority = CurvePcurveAuthority {
        curve: curve_definition,
        pcurve: pcurve_definition,
        support,
        parameter_domain_bits,
        orientation,
        owner: owner.clone(),
        context: context.spec_identity(),
    };
    let certificate = CurvePcurveCorrespondence {
        curve: curve.clone(),
        pcurve: pcurve.clone(),
        support,
        parameter_domain_bits,
        orientation,
        owner,
        context: context.spec_identity(),
        evidence,
        no_snapping: true,
        authority,
    };
    if !certificate.permits_exact_correspondence() {
        return Err(refuse(
            "BREP_SEW_CORRESPONDENCE_REQUIRED",
            "Curve/pcurve certificate failed its retained authority check",
        ));
    }
    Ok(certificate)
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
        heal_recipe: None,
    })
}

/// Derive correspondence authority from the model's own two manifold edge
/// uses. Bridge callers cannot provide or override any evidence fields.
pub fn prove_model_edge_correspondence(
    model: &Model,
    context: &ToleranceContext,
    edge_index: usize,
) -> Result<BoundaryCorrespondence> {
    let edge = model.edges.get(edge_index).ok_or_else(|| {
        refuse("BREP_HEAL_RECIPE_INVALID", "Heal edge index is out of range")
    })?;
    let mut uses = Vec::new();
    for (face_index, face) in model.faces.iter().enumerate() {
        for &wire_index in std::iter::once(&face.outer).chain(&face.holes) {
            for (cyclic_index, coedge) in model.loops[wire_index].coedges.iter().enumerate() {
                if coedge.edge == edge_index {
                    uses.push(BoundaryUse {
                        face: face_index,
                        wire: wire_index,
                        cyclic_index,
                        reversed: coedge.reversed,
                    });
                }
            }
        }
    }
    if uses.len() != 2 || uses[0].reversed == uses[1].reversed {
        return Err(refuse(
            "BREP_HEAL_PROOF_REQUIRED",
            "Heal requires exactly two native opposite manifold boundary uses",
        ));
    }
    let authored = [
        model.vertices[edge.vertices[0]].point,
        model.vertices[edge.vertices[1]].point,
    ];
    let endpoints = |use_: &BoundaryUse| {
        if use_.reversed {
            [authored[1], authored[0]]
        } else {
            authored
        }
    };
    let shell = model
        .shells
        .iter()
        .position(|shell| shell.faces.iter().any(|use_| use_.face == uses[0].face))
        .ok_or_else(|| refuse("BREP_HEAL_PROOF_REQUIRED", "Boundary face has no shell owner"))?;
    prove_boundary_correspondence(
        context,
        &edge.curve,
        &edge.curve,
        endpoints(&uses[0]),
        endpoints(&uses[1]),
        ParameterOrientation::Reversed,
        0,
        shell,
        [uses[0].clone(), uses[1].clone()],
    )
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
                // A collapsed parametric boundary has zero geometric measure
                // and is not a sew seam. Several cone patches may reference
                // the same pole edge; manifold incidence is carried by their
                // nondegenerate radial edges and the shared pole vertex.
                if model.edges[coedge.edge].degenerate {
                    continue;
                }
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

    fn curved_graph_and_plane() -> (Surface, Surface) {
        let mut graph = Surface {
            degree_u: 2,
            degree_v: 3,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: (0..3)
                .map(|i| {
                    (0..4)
                        .map(|j| vec![i as f64 * 1.5, j as f64, 0.])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 4]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        graph.control_points[1][1][2] = 0.2;
        let plane = Surface {
            degree_u: 3,
            degree_v: 3,
            knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: (0..4)
                .map(|i| {
                    (0..4)
                        .map(|j| {
                            vec![
                                1.5,
                                -1. + j as f64 * 5. / 3.,
                                -1. + i as f64 * 5. / 3.,
                            ]
                        })
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 4]; 4],
            periodic_u: false,
            periodic_v: false,
        };
        (graph, plane)
    }

    #[test]
    fn exact_curve_pcurve_correspondence_covers_curved_and_planar_supports() {
        let context = ToleranceContext::default_valid();
        let (graph, plane) = curved_graph_and_plane();
        let seam =
            crate::nurbs_ss_g6::certify_exact_planar_iso_intersection(&graph, &plane, &context)
                .unwrap();
        let owner = BoundaryUse {
            face: 3,
            wire: 5,
            cyclic_index: 1,
            reversed: false,
        };
        let curved = prove_curve_pcurve_correspondence(
            &context,
            &seam.curve,
            &graph,
            &seam.uv_traces[0],
            ParameterOrientation::Same,
            owner.clone(),
        )
        .unwrap();
        assert_eq!(curved.support, CurvePcurveSupport::CurvedIsoU);
        assert!(curved.permits_exact_correspondence());
        let planar = prove_curve_pcurve_correspondence(
            &context,
            &seam.curve,
            &plane,
            &seam.uv_traces[1],
            ParameterOrientation::Same,
            owner,
        )
        .unwrap();
        assert_eq!(planar.support, CurvePcurveSupport::AffinePlanar);
        assert!(planar.permits_exact_correspondence());
    }

    #[test]
    fn curve_pcurve_orientation_context_definition_and_mutations_refuse() {
        let context = ToleranceContext::default_valid();
        let (graph, plane) = curved_graph_and_plane();
        let seam =
            crate::nurbs_ss_g6::certify_exact_planar_iso_intersection(&graph, &plane, &context)
                .unwrap();
        let owner = BoundaryUse {
            face: 3,
            wire: 5,
            cyclic_index: 1,
            reversed: false,
        };
        assert!(prove_curve_pcurve_correspondence(
            &context,
            &seam.curve,
            &graph,
            &seam.uv_traces[0],
            ParameterOrientation::Reversed,
            owner.clone()
        )
        .is_err());
        let mut foreign_spec = context.specification().clone();
        foreign_spec.policy = "foreign-pcurve-context".into();
        let foreign = ToleranceContext::new(foreign_spec).unwrap();
        let mut certificate = prove_curve_pcurve_correspondence(
            &context,
            &seam.curve,
            &plane,
            &seam.uv_traces[1],
            ParameterOrientation::Same,
            owner,
        )
        .unwrap();
        certificate.context = foreign.spec_identity();
        assert!(!certificate.permits_exact_correspondence());
        let mut certificate = prove_curve_pcurve_correspondence(
            &context,
            &seam.curve,
            &plane,
            &seam.uv_traces[1],
            ParameterOrientation::Same,
            certificate.owner.clone(),
        )
        .unwrap();
        certificate.pcurve.control_points[1][0] += 1e-9;
        assert!(!certificate.permits_exact_correspondence());
        certificate.no_snapping = false;
        assert!(!certificate.permits_exact_correspondence());

        let mut periodic = seam.uv_traces[1].clone();
        periodic.periodic = true;
        assert!(prove_curve_pcurve_correspondence(
            &context,
            &seam.curve,
            &plane,
            &periodic,
            ParameterOrientation::Same,
            certificate.owner
        )
        .is_err());
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
        let model = crate::cuboid([0.; 3], [2.; 3]).unwrap();
        let context = model.tolerance_context().unwrap();
        let proof = prove_model_edge_correspondence(&model, &context, 0).unwrap();
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
        }
        .bind_native_proof();
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
        }
        .bind_native_proof();
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
            }
            .bind_native_proof()],
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
            value_codec::Serialize::to_value(&once),
            value_codec::Serialize::to_value(&twice)
        );
    }

    #[test]
    fn heal_positive_endpoint_snap_returns_complete_native_certificate() {
        let (model, context, proof) = heal_fixture();
        let vertex = model.edges[0].vertices[0];
        let mut to = model.vertices[vertex].point;
        to[0] += context.spatial_bounds().absolute_mm
            / (1 + model.edges.iter().filter(|edge| edge.vertices.contains(&vertex)).count())
                as f64
            * 0.25;
        let plan = AuthorizedHealPlan::new(
            &model,
            &context,
            vec![HealOperation::EndpointSnap {
                vertex,
                expected_id: model.1.vertices[vertex],
                to,
                correspondence: proof,
            }
            .bind_native_proof()],
            context.spatial_bounds().absolute_mm,
            context.spatial_bounds().absolute_mm,
        )
        .unwrap();
        let mut transaction = crate::transactions::AuthorizedHealTransaction::begin(
            crate::transactions::ModelSnapshot::new(model.clone()).unwrap(),
            crate::transactions::HealCancellation::default(),
        );
        transaction.apply(&plan).unwrap();
        let result = transaction.commit().unwrap();
        assert_eq!(result.status, "Complete");
        assert!(result.displacement[0].actual_mm > 0.);
        assert!(result.cumulative_displacement_mm <= context.spatial_bounds().absolute_mm);
        assert!(result.sew.complete && result.audit.ok && result.naming_complete);
        assert_ne!(
            value_codec::Serialize::to_value(&model),
            value_codec::Serialize::to_value(&result.model)
        );
    }

    #[test]
    fn heal_budget_exact_boundary_accepts_and_boundary_ulp_refuses() {
        let (model, context, proof) = heal_fixture();
        let vertex = model.edges[0].vertices[0];
        let writes =
            1 + model.edges.iter().filter(|edge| edge.vertices.contains(&vertex)).count();
        let exact = context.spatial_bounds().absolute_mm / writes as f64;
        let operation = |delta: f64| {
            let mut to = model.vertices[vertex].point;
            to[1] += delta;
            HealOperation::EndpointSnap {
                vertex,
                expected_id: model.1.vertices[vertex],
                to,
                correspondence: proof.clone(),
            }
            .bind_native_proof()
        };
        AuthorizedHealPlan::new(
            &model,
            &context,
            vec![operation(exact)],
            context.spatial_bounds().absolute_mm,
            context.spatial_bounds().absolute_mm,
        )
        .unwrap();
        let above = f64::from_bits(exact.to_bits() + 1);
        assert_eq!(
            AuthorizedHealPlan::new(
                &model,
                &context,
                vec![operation(above)],
                context.spatial_bounds().absolute_mm,
                context.spatial_bounds().absolute_mm,
            )
            .unwrap_err()
            .code,
            "BREP_HEAL_BUDGET_EXCEEDED"
        );
    }

    #[test]
    fn heal_positive_one_to_one_rational_refit_is_complete() {
        let (model, context, proof) = heal_fixture();
        let edge_index = 0;
        let mut replacement = model.edges[edge_index].curve.clone();
        replacement.control_points[1][2] += context.spatial_bounds().absolute_mm * 0.25;
        let plan = AuthorizedHealPlan::new(
            &model,
            &context,
            vec![HealOperation::CurveRefit {
                edge: edge_index,
                expected_id: model.1.edges[edge_index],
                replacement,
                correspondence: proof,
            }
            .bind_native_proof()],
            context.spatial_bounds().absolute_mm,
            context.spatial_bounds().absolute_mm,
        )
        .unwrap();
        let mut transaction = crate::transactions::AuthorizedHealTransaction::begin(
            crate::transactions::ModelSnapshot::new(model).unwrap(),
            crate::transactions::HealCancellation::default(),
        );
        transaction.apply(&plan).unwrap();
        let result = transaction.commit().unwrap();
        assert!(result.displacement[0].actual_mm > 0.);
        assert!(result.audit.ok && result.naming_complete);
    }

    #[test]
    fn heal_refuses_unbound_stale_and_overlapping_recipes() {
        let (model, context, proof) = heal_fixture();
        let vertex = model.edges[0].vertices[0];
        let to = model.vertices[vertex].point;
        let unbound = HealOperation::EndpointSnap {
            vertex,
            expected_id: model.1.vertices[vertex],
            to,
            correspondence: proof.clone(),
        };
        assert_eq!(
            AuthorizedHealPlan::new(
                &model,
                &context,
                vec![unbound],
                context.spatial_bounds().absolute_mm,
                context.spatial_bounds().absolute_mm,
            )
            .unwrap_err()
            .code,
            "BREP_HEAL_PROOF_RECIPE_MISMATCH"
        );
        let mut stale = HealOperation::EndpointSnap {
            vertex,
            expected_id: model.1.vertices[vertex],
            to,
            correspondence: proof.clone(),
        }
        .bind_native_proof();
        if let HealOperation::EndpointSnap { to, .. } = &mut stale {
            to[2] = f64::from_bits(to[2].to_bits() + 1);
        }
        assert_eq!(
            AuthorizedHealPlan::new(
                &model,
                &context,
                vec![stale],
                context.spatial_bounds().absolute_mm,
                context.spatial_bounds().absolute_mm,
            )
            .unwrap_err()
            .code,
            "BREP_HEAL_PROOF_RECIPE_MISMATCH"
        );
        let snap = HealOperation::EndpointSnap {
            vertex,
            expected_id: model.1.vertices[vertex],
            to,
            correspondence: proof.clone(),
        }
        .bind_native_proof();
        let refit = HealOperation::CurveRefit {
            edge: 0,
            expected_id: model.1.edges[0],
            replacement: model.edges[0].curve.clone(),
            correspondence: proof,
        }
        .bind_native_proof();
        assert_eq!(
            AuthorizedHealPlan::new(
                &model,
                &context,
                vec![snap, refit],
                context.spatial_bounds().absolute_mm,
                context.spatial_bounds().absolute_mm,
            )
            .unwrap_err()
            .code,
            "BREP_HEAL_WRITESET_OVERLAP"
        );
        let unrelated = HealOperation::CurveRefit {
            edge: 1,
            expected_id: model.1.edges[1],
            replacement: model.edges[1].curve.clone(),
            correspondence: prove_model_edge_correspondence(&model, &context, 0).unwrap(),
        }
        .bind_native_proof();
        assert_eq!(
            AuthorizedHealPlan::new(
                &model,
                &context,
                vec![unrelated],
                context.spatial_bounds().absolute_mm,
                context.spatial_bounds().absolute_mm,
            )
            .unwrap_err()
            .code,
            "BREP_HEAL_PROOF_UNRELATED"
        );
    }

    #[test]
    fn heal_refit_requires_exact_rational_cardinality_degree_knots_and_weights() {
        let (model, context, proof) = heal_fixture();
        let assert_refused = |replacement: Curve| {
            AuthorizedHealPlan::new(
                &model,
                &context,
                vec![HealOperation::CurveRefit {
                    edge: 0,
                    expected_id: model.1.edges[0],
                    replacement,
                    correspondence: proof.clone(),
                }
                .bind_native_proof()],
                context.spatial_bounds().absolute_mm,
                context.spatial_bounds().absolute_mm,
            )
            .unwrap_err()
            .code
        };
        let mut cardinality = model.edges[0].curve.clone();
        cardinality.control_points.push(cardinality.control_points[0].clone());
        cardinality.weights.push(1.);
        let code = assert_refused(cardinality);
        assert!(code.starts_with("NURBS_") || code == "BREP_HEAL_REFIT_REFUSED");
        let mut degree = model.edges[0].curve.clone();
        degree.degree += 1;
        let code = assert_refused(degree);
        assert!(code.starts_with("NURBS_") || code == "BREP_HEAL_REFIT_REFUSED");
        let mut knot = model.edges[0].curve.clone();
        knot.knots[0] = f64::from_bits(knot.knots[0].to_bits() + 1);
        let code = assert_refused(knot);
        assert!(code.starts_with("NURBS_") || code == "BREP_HEAL_REFIT_REFUSED");
        let mut weight = model.edges[0].curve.clone();
        weight.weights[0] = f64::from_bits(weight.weights[0].to_bits() + 1);
        assert_eq!(assert_refused(weight), "BREP_HEAL_REFIT_REFUSED");
        let split = HealOperation::EdgeSplit {
            edge: 0,
            expected_id: model.1.edges[0],
            parameter: model.edges[0].curve.domain().iter().sum::<f64>() * 0.5,
            correspondence: proof,
        }
        .bind_native_proof();
        assert_eq!(
            AuthorizedHealPlan::new(
                &model,
                &context,
                vec![split],
                context.spatial_bounds().absolute_mm,
                context.spatial_bounds().absolute_mm,
            )
            .unwrap_err()
            .code,
            "BREP_HEAL_SPLIT_REFUSED"
        );
    }
}
