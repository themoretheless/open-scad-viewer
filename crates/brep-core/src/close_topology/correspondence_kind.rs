use super::*;

pub(super) fn audit_complex(complex: MixedDimensionalBrep) -> Result<AuditedTopologyComplex> {
    let mut solid_audits = Vec::new();
    let mut sheet_count = 0;
    let mut open_shell_count = 0;
    for part in &complex.parts {
        part.model.validate()?;
        match part.role {
            BodyRole::Solid => {
                if part.model.bodies.is_empty() || part.model.shells.iter().any(|s| !s.closed) {
                    return Err(refuse(
                        "BREP_COMPLEX_ROLE_REFUSED",
                        "Solid part requires closed body shells",
                    ));
                }
                solid_audits.push(
                    LocallyValidatedModel::new(part.model.clone())?
                        .audit()?
                        .certificate()
                        .clone(),
                );
            }
            BodyRole::SheetShell => {
                sheet_count += 1;
                if part.model.shells.is_empty()
                    || part.model.shells.iter().any(|s| !s.closed)
                    || !part.model.bodies.is_empty()
                {
                    return Err(refuse(
                        "BREP_COMPLEX_ROLE_REFUSED",
                        "Sheet shell requires closed shell topology without a solid body",
                    ));
                }
            }
            BodyRole::OpenShell => {
                open_shell_count += 1;
                if part.model.shells.is_empty()
                    || part.model.shells.iter().all(|s| s.closed)
                    || !part.model.bodies.is_empty()
                {
                    return Err(refuse(
                        "BREP_COMPLEX_ROLE_REFUSED",
                        "Open shell requires an open shell and no solid body",
                    ));
                }
            }
            BodyRole::Face if part.model.faces.len() != 1 || !part.model.bodies.is_empty() => {
                return Err(refuse(
                    "BREP_COMPLEX_ROLE_REFUSED",
                    "Face part requires exactly one face and no body",
                ));
            }
            BodyRole::Wire
                if !part.model.faces.is_empty()
                    || !part.model.shells.is_empty()
                    || !part.model.bodies.is_empty() =>
            {
                return Err(refuse(
                    "BREP_COMPLEX_ROLE_REFUSED",
                    "Wire role cannot own faces, shells, or bodies",
                ));
            }
            BodyRole::Compound => {
                return Err(refuse(
                    "BREP_COMPLEX_ROLE_REFUSED",
                    "Nested compound parts are not admitted",
                ));
            }
            _ => {}
        }
    }

    let mut shared_uses = BTreeSet::new();
    for relation in &complex.shared_faces {
        if !(2..=8).contains(&relation.uses.len()) {
            return Err(refuse(
                "BREP_COMPLEX_BRANCH_REFUSED",
                "Shared face valence must be 2..8",
            ));
        }
        let first = relation.uses[0];
        let first_part = complex
            .parts
            .get(first.part)
            .ok_or_else(|| refuse("BREP_COMPLEX_INCIDENCE_REFUSED", "Unknown shared-face part"))?;
        if first_part.role != BodyRole::Solid {
            return Err(refuse(
                "BREP_COMPLEX_ROLE_REFUSED",
                "Shared material faces require solid cell owners",
            ));
        }
        let carrier = face_surface_key(&first_part.model, first.face)?;
        let mut orientations = BTreeSet::new();
        for use_ in &relation.uses {
            let part = complex.parts.get(use_.part).ok_or_else(|| {
                refuse("BREP_COMPLEX_INCIDENCE_REFUSED", "Unknown shared-face part")
            })?;
            if part.role != BodyRole::Solid || face_surface_key(&part.model, use_.face)? != carrier
            {
                return Err(refuse(
                    "BREP_COMPLEX_CARRIER_REFUSED",
                    "Shared faces require one exact surface carrier",
                ));
            }
            if !shared_uses.insert((use_.part, use_.face)) {
                return Err(refuse(
                    "BREP_COMPLEX_INCIDENCE_REFUSED",
                    "Face appears in multiple shared-face relations",
                ));
            }
            orientations.insert(use_.reversed);
        }
        if relation.uses.len() == 2 && orientations.len() != 2 {
            return Err(refuse(
                "BREP_COMPLEX_ORIENTATION_REFUSED",
                "Two-cell shared face orientations must oppose",
            ));
        }
    }

    let mut max_radial_valence = 0;
    let mut radial_keys = BTreeSet::new();
    for ring in &complex.radial_rings {
        if !(3..=MAX_RADIAL_USES).contains(&ring.uses.len()) {
            return Err(refuse(
                "BREP_COMPLEX_BRANCH_REFUSED",
                "Non-manifold radial ring valence must be 3..16",
            ));
        }
        max_radial_valence = max_radial_valence.max(ring.uses.len());
        let first = ring.uses[0];
        let first_part = complex
            .parts
            .get(first.part)
            .ok_or_else(|| refuse("BREP_COMPLEX_INCIDENCE_REFUSED", "Unknown radial part"))?;
        let carrier = edge_curve_key(&first_part.model, first.edge)?;
        let mut ring_seen = BTreeSet::new();
        for use_ in &ring.uses {
            let part = complex
                .parts
                .get(use_.part)
                .ok_or_else(|| refuse("BREP_COMPLEX_INCIDENCE_REFUSED", "Unknown radial part"))?;
            if part.model.faces.get(use_.face).is_none()
                || edge_curve_key(&part.model, use_.edge)? != carrier
            {
                return Err(refuse(
                    "BREP_COMPLEX_CARRIER_REFUSED",
                    "Radial uses require one exact curve carrier",
                ));
            }
            if !ring_seen.insert((use_.part, use_.face, use_.edge)) {
                return Err(refuse(
                    "BREP_COMPLEX_INCIDENCE_REFUSED",
                    "Duplicate use in radial ring",
                ));
            }
        }
        let canonical = ring
            .uses
            .iter()
            .map(|u| (u.part, u.face, u.edge))
            .min()
            .unwrap();
        if !radial_keys.insert(canonical) {
            return Err(refuse(
                "BREP_COMPLEX_INCIDENCE_REFUSED",
                "Duplicate radial ring",
            ));
        }
    }

    let mut fan_membership = BTreeSet::new();
    for fan in &complex.vertex_fans {
        if fan.uses.is_empty() || fan.uses.len() > MAX_RADIAL_USES {
            return Err(refuse(
                "BREP_COMPLEX_BRANCH_REFUSED",
                "Vertex fan requires 1..16 uses",
            ));
        }
        let part = complex
            .parts
            .get(fan.vertex.0)
            .ok_or_else(|| refuse("BREP_COMPLEX_INCIDENCE_REFUSED", "Unknown fan part"))?;
        if part.model.vertices.get(fan.vertex.1).is_none() {
            return Err(refuse(
                "BREP_COMPLEX_INCIDENCE_REFUSED",
                "Unknown fan vertex",
            ));
        }
        for use_ in &fan.uses {
            let owner = complex
                .parts
                .get(use_.part)
                .ok_or_else(|| refuse("BREP_COMPLEX_INCIDENCE_REFUSED", "Unknown fan use part"))?;
            if owner.model.faces.get(use_.face).is_none()
                || owner.model.vertices.get(use_.vertex).is_none()
            {
                return Err(refuse(
                    "BREP_COMPLEX_INCIDENCE_REFUSED",
                    "Unknown vertex fan incidence",
                ));
            }
            if !fan_membership.insert((fan.vertex, *use_)) {
                return Err(refuse(
                    "BREP_COMPLEX_INCIDENCE_REFUSED",
                    "Vertex use repeated in disconnected fans",
                ));
            }
        }
    }

    for relation in &complex.lineage {
        if relation.parents.is_empty() && relation.children.is_empty() {
            return Err(refuse(
                "BREP_COMPLEX_NAMING_REFUSED",
                "Empty complex lineage relation",
            ));
        }
        if !matches!(
            relation.operation.as_str(),
            "persist" | "split" | "merge" | "share" | "decompose" | "recompose"
        ) {
            return Err(refuse(
                "BREP_COMPLEX_NAMING_REFUSED",
                "Unsupported complex lineage operation",
            ));
        }
    }
    let boundary_face_count = complex
        .parts
        .iter()
        .enumerate()
        .flat_map(|(p, part)| {
            part.model
                .shells
                .iter()
                .flat_map(move |s| s.faces.iter().map(move |u| (p, u.face)))
        })
        .filter(|face| !shared_uses.contains(face))
        .count();
    let naming_complete = complex
        .parts
        .iter()
        .all(|part| part.model.persistent_naming_complete());
    if !naming_complete {
        return Err(refuse(
            "BREP_COMPLEX_NAMING_REFUSED",
            "Every part requires complete persistent naming",
        ));
    }
    let certificate = TopologyComplexCertificate {
        capability: CLOSE_TOPOLOGY_CAPABILITY,
        complete: true,
        part_count: complex.parts.len(),
        solid_cell_count: solid_audits.len(),
        sheet_count,
        open_shell_count,
        shared_face_count: complex.shared_faces.len(),
        non_manifold_edge_count: complex.radial_rings.len(),
        vertex_fan_count: complex.vertex_fans.len(),
        boundary_face_count,
        max_radial_valence,
        manifold_cell_audits: solid_audits,
        naming_complete,
        notes: vec![
            "manifold_cells_independently_globally_audited",
            "non_manifold_incidence_is_explicit",
            "mixed_dimensional_roles_are_explicit",
            "solid_typestate_not_weakened",
            "no_tolerance_growth",
        ],
    };
    Ok(AuditedTopologyComplex {
        complex,
        certificate,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CorrespondenceKind {
    Vertex,
    Edge,
    Face,
}

#[derive(Clone, Debug)]
pub struct LocalCorrespondence {
    pub kind: CorrespondenceKind,
    pub entities: Vec<(usize, TopoId)>,
    pub displacement_mm: f64,
    pub context_path: Vec<String>,
    pub provenance: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactParameterPartition {
    pub carrier_definition: String,
    pub parent_domain_bits: [u64; 2],
    pub child_domain_bits: Vec<[u64; 2]>,
}
impl ExactParameterPartition {
    pub fn validate(&self) -> Result<()> {
        if self.child_domain_bits.len() < 2 || self.child_domain_bits.len() > 16 {
            return Err(refuse(
                "BREP_HEAL_PARTITION_REFUSED",
                "Split/merge requires 2..16 child intervals",
            ));
        }
        let decode = |bits: u64| f64::from_bits(bits);
        let [parent_lo, parent_hi] = self.parent_domain_bits.map(decode);
        if self.carrier_definition.is_empty()
            || !(parent_lo.is_finite() && parent_hi.is_finite() && parent_lo < parent_hi)
        {
            return Err(refuse(
                "BREP_HEAL_PARTITION_REFUSED",
                "Invalid exact carrier or parent interval",
            ));
        }
        let mut cursor = parent_lo;
        for interval in &self.child_domain_bits {
            let [lo, hi] = interval.map(decode);
            if lo.to_bits() != cursor.to_bits() || !lo.is_finite() || !hi.is_finite() || lo >= hi {
                return Err(refuse(
                    "BREP_HEAL_PARTITION_REFUSED",
                    "Child intervals must exactly and contiguously partition parent",
                ));
            }
            cursor = hi;
        }
        if cursor.to_bits() != parent_hi.to_bits() {
            return Err(refuse(
                "BREP_HEAL_PARTITION_REFUSED",
                "Child intervals do not exactly cover parent",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct ComplexHealPlan {
    pub context_canonical: String,
    pub correspondences: Vec<LocalCorrespondence>,
    pub per_entity_limit_mm: f64,
    pub cumulative_limit_mm: f64,
    pub partitions: Vec<ExactParameterPartition>,
}

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
