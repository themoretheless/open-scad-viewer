use super::*;

impl HealOperation {
    pub(super) fn correspondence(&self) -> &BoundaryCorrespondence {
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

    pub(super) fn recipe_bytes(&self) -> Vec<u8> {
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
                bytes.extend_from_slice(&curve_signature_bytes(replacement));
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

    pub(super) fn recipe_identity(&self) -> String {
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
        self.write_set_with(&vertex_edge_incidence(model))
    }

    pub(super) fn write_set_with(&self, incidence: &[Vec<usize>]) -> BTreeSet<(TopoKind, usize)> {
        let mut writes = BTreeSet::new();
        match self {
            Self::EndpointSnap { vertex, .. } => {
                writes.insert((TopoKind::Vertex, *vertex));
                for &edge in incidence.get(*vertex).into_iter().flatten() {
                    writes.insert((TopoKind::Edge, edge));
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

    pub(super) fn proof_matches_model_with(
        &self,
        model: &Model,
        context: &ToleranceContext,
        incidence: &[Vec<usize>],
    ) -> bool {
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
            } => incidence
                .get(*vertex)
                .into_iter()
                .flatten()
                .copied()
                .filter_map(|edge| prove_model_edge_correspondence(model, context, edge).ok())
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

/// Immutable, context-bound authority for gap repair. Ordinary plans retain
/// the one-cell limit; explicitly tolerant endpoint plans carry a user cap.
#[derive(Clone, Debug)]
pub struct AuthorizedHealPlan {
    pub(super) context: ToleranceSpecIdentity,
    pub(super) operations: Vec<HealOperation>,
    pub(super) per_entity_limit_mm: f64,
    pub(super) cumulative_limit_mm: f64,
    pub(super) identity: String,
    pub(super) tolerant_endpoints: bool,
}

fn apply_tolerant_endpoint(admission: &mut Model, vertex: usize, to: [f64; 3]) -> Result<()> {
    if vertex >= admission.vertices.len() || to.iter().any(|value| !value.is_finite()) {
        return Err(refuse(
            "BREP_HEAL_RECIPE_INVALID",
            "Tolerant endpoint target is invalid",
        ));
    }
    admission.vertices[vertex].point = to;
    for edge in &mut admission.edges {
        for endpoint in 0..2 {
            if edge.vertices[endpoint] == vertex {
                let control = if endpoint == 0 {
                    0
                } else {
                    edge.curve.control_points.len() - 1
                };
                edge.curve.control_points[control] = to.to_vec();
            }
        }
    }
    Ok(())
}

pub(crate) fn tolerant_endpoint_snapshot(
    model: &Model,
    vertex: usize,
    to: [f64; 3],
) -> Result<Model> {
    let mut admission = model.clone();
    apply_tolerant_endpoint(&mut admission, vertex, to)?;
    admission.validate()?;
    Ok(admission)
}

fn tolerant_endpoint_admission(model: &Model, operations: &[HealOperation]) -> Result<Model> {
    let mut admission = model.clone();
    for operation in operations {
        let HealOperation::EndpointSnap { vertex, to, .. } = operation else {
            return Err(refuse(
                "BREP_HEAL_PLAN_INVALID",
                "Tolerant sewing admits only endpoint snap operations",
            ));
        };
        apply_tolerant_endpoint(&mut admission, *vertex, *to)?;
    }
    admission.validate()?;
    Ok(admission)
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
        let incidence = vertex_edge_incidence(model);
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
            if !operation.proof_matches_model_with(model, context, &incidence) {
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
            let writes = operation.write_set_with(&incidence);
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
            tolerant_endpoints: false,
        })
    }

    /// Build a user-capped endpoint plan after proving that the requested
    /// snaps are exactly sufficient to make the private admission copy valid.
    pub fn new_tolerant_endpoints(
        model: &Model,
        context: &ToleranceContext,
        operations: Vec<HealOperation>,
        per_entity_limit_mm: f64,
        cumulative_limit_mm: f64,
    ) -> Result<Self> {
        if !per_entity_limit_mm.is_finite()
            || !cumulative_limit_mm.is_finite()
            || per_entity_limit_mm <= 0.
            || cumulative_limit_mm <= 0.
        {
            return Err(refuse(
                "BREP_HEAL_BUDGET_EXCEEDED",
                "Tolerant sewing caps must be finite and positive",
            ));
        }
        let admission = tolerant_endpoint_admission(model, &operations)?;
        let cell = context.spatial_bounds().absolute_mm;
        let mut plan = Self::new(&admission, context, operations, cell, cell)?;
        let source_identity = plan.identity.clone();
        plan.per_entity_limit_mm = per_entity_limit_mm;
        plan.cumulative_limit_mm = cumulative_limit_mm;
        plan.tolerant_endpoints = true;
        let signature = format!(
            "{source_identity}:{:016x}:{:016x}",
            per_entity_limit_mm.to_bits(),
            cumulative_limit_mm.to_bits()
        );
        plan.identity = format!(
            "authorized-tolerant-sew:{}",
            TopoId::derive(
                TopoKind::Body,
                "authorized-tolerant-sew",
                "plan",
                "identity",
                signature.as_bytes()
            )
        );
        Ok(plan)
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
    let admission = if plan.tolerant_endpoints {
        Some(tolerant_endpoint_admission(model, plan.operations())?)
    } else {
        model.validate()?;
        None
    };
    let proof_model = admission.as_ref().unwrap_or(model);
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
    // edge.vertices never mutate during plan application (EndpointSnap moves
    // points and endpoint control points, CurveRefit swaps the curve,
    // EdgeSplit is refused), so one incidence build covers the whole loop.
    let incidence = vertex_edge_incidence(&next);
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
        let operation_proof_model = if plan.tolerant_endpoints {
            proof_model
        } else {
            &next
        };
        if !operation.proof_matches_model_with(operation_proof_model, &context, &incidence) {
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
                let affected_edges = incidence
                    .get(*vertex)
                    .into_iter()
                    .flatten()
                    .copied()
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
                    let signature = curve_signature_bytes(&next.edges[edge_index].curve);
                    let child = TopoId::derive(
                        TopoKind::Edge,
                        &plan.identity,
                        &parent.to_string(),
                        "endpoint-snap-refit",
                        &signature,
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
            TopoKind::Vertex => point_signature_bytes(&next.vertices[index].point),
            TopoKind::Edge => curve_signature_bytes(&next.edges[index].curve),
            _ => unreachable!(),
        };
        let new_id = TopoId::derive(
            kind,
            &plan.identity,
            &old_id.to_string(),
            "healed",
            &signature,
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

pub(super) fn quantize(point: [f64; 3], scale: f64) -> Result<[i64; 3]> {
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

pub(super) fn point_bits(point: [f64; 3]) -> Result<[u64; 3]> {
    if !point.iter().all(|v| v.is_finite()) {
        return Err(refuse(
            "BREP_SEW_INVALID",
            "Boundary endpoint must be finite",
        ));
    }
    Ok(point.map(f64::to_bits))
}
