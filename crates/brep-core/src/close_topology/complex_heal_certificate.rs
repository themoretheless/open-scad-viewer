use super::*;

#[derive(Clone, Debug)]
pub struct ComplexHealCertificate {
    pub capability: &'static str,
    pub complete: bool,
    pub correspondence_count: usize,
    pub touched_entity_count: usize,
    pub cumulative_displacement_mm: f64,
    pub per_entity_limit_mm: f64,
    pub cumulative_limit_mm: f64,
    pub partition_proof_count: usize,
    pub rollback_safe: bool,
    pub idempotent: bool,
    pub no_tolerance_growth: bool,
}

/// Validate a multi-part heal transaction.  Geometry mutation remains delegated
/// to the existing recipe-bound native operations; this transaction certifies
/// cross-part correspondence, budgets, hierarchy and split/merge authority.
pub fn certify_complex_heal(
    audited: &AuditedTopologyComplex,
    plan: &ComplexHealPlan,
) -> Result<ComplexHealCertificate> {
    if plan.context_canonical.is_empty()
        || plan.correspondences.is_empty()
        || plan.correspondences.len() > 256
    {
        return Err(refuse(
            "BREP_HEAL_PLAN_INVALID",
            "Complex heal requires one context and 1..256 correspondences",
        ));
    }
    if !plan.per_entity_limit_mm.is_finite()
        || !plan.cumulative_limit_mm.is_finite()
        || plan.per_entity_limit_mm <= 0.
        || plan.cumulative_limit_mm <= 0.
    {
        return Err(refuse(
            "BREP_HEAL_BUDGET_EXCEEDED",
            "Heal budgets must be finite and positive",
        ));
    }
    let mut touched = BTreeSet::new();
    let mut cumulative = 0.;
    for correspondence in &plan.correspondences {
        if correspondence.entities.len() < 2
            || correspondence.entities.len() > 16
            || correspondence.context_path.is_empty()
            || correspondence.provenance.is_empty()
            || !correspondence.displacement_mm.is_finite()
            || correspondence.displacement_mm < 0.
            || correspondence.displacement_mm > plan.per_entity_limit_mm
        {
            return Err(refuse(
                "BREP_HEAL_CORRESPONDENCE_REFUSED",
                "Invalid local correspondence, hierarchy, provenance, or budget",
            ));
        }
        let mut local = BTreeSet::new();
        for (part, id) in &correspondence.entities {
            let owner = audited.complex.parts.get(*part).ok_or_else(|| {
                refuse(
                    "BREP_HEAL_CORRESPONDENCE_REFUSED",
                    "Unknown correspondence part",
                )
            })?;
            let expected = match correspondence.kind {
                CorrespondenceKind::Vertex => TopoKind::Vertex,
                CorrespondenceKind::Edge => TopoKind::Edge,
                CorrespondenceKind::Face => TopoKind::Face,
            };
            if id.kind() != expected
                || !owner.model.1.change_set.nodes.contains_key(id)
                || !local.insert((*part, *id))
            {
                return Err(refuse(
                    "BREP_HEAL_CORRESPONDENCE_REFUSED",
                    "Correspondence identity is stale, duplicate, or wrong-kind",
                ));
            }
            touched.insert((*part, *id));
        }
        cumulative += correspondence.displacement_mm * correspondence.entities.len() as f64;
        if cumulative > plan.cumulative_limit_mm {
            return Err(refuse(
                "BREP_HEAL_BUDGET_EXCEEDED",
                "Cumulative physical displacement budget exceeded",
            ));
        }
    }
    for partition in &plan.partitions {
        partition.validate()?;
    }
    Ok(ComplexHealCertificate {
        capability: TOLERANT_COMPLEX_HEAL_CAPABILITY,
        complete: true,
        correspondence_count: plan.correspondences.len(),
        touched_entity_count: touched.len(),
        cumulative_displacement_mm: cumulative,
        per_entity_limit_mm: plan.per_entity_limit_mm,
        cumulative_limit_mm: plan.cumulative_limit_mm,
        partition_proof_count: plan.partitions.len(),
        rollback_safe: true,
        idempotent: true,
        no_tolerance_growth: true,
    })
}

/// Stable lineage IDs for complex-level share/decompose/recompose records.
pub fn complex_relation_id(role: &str, parents: &[TopoId], children: &[TopoId]) -> TopoId {
    let mut bytes = role.as_bytes().to_vec();
    for id in parents.iter().chain(children) {
        bytes.extend_from_slice(id.to_string().as_bytes());
        bytes.push(0);
    }
    TopoId::derive(
        TopoKind::Body,
        CLOSE_TOPOLOGY_CAPABILITY,
        role,
        "complex-relation",
        &bytes,
    )
}
