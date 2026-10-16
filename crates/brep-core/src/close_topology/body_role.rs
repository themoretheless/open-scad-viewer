use super::*;

pub(super) fn refuse(code: &'static str, message: impl Into<String>) -> Error {
    Error::new(code, message)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BodyRole {
    Wire,
    Face,
    SheetShell,
    OpenShell,
    Solid,
    Compound,
}
impl BodyRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Wire => "wire",
            Self::Face => "face",
            Self::SheetShell => "sheet-shell",
            Self::OpenShell => "open-shell",
            Self::Solid => "solid",
            Self::Compound => "compound",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FaceRef {
    pub part: usize,
    pub face: usize,
    /// Orientation in the material cell or sheet shell.
    pub reversed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EdgeUseRef {
    pub part: usize,
    pub face: usize,
    pub edge: usize,
    pub reversed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct VertexUseRef {
    pub part: usize,
    pub face: usize,
    pub vertex: usize,
}

/// Exact shared face carrier with two or more incident material cells.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharedFace {
    pub uses: Vec<FaceRef>,
}

/// Cyclic order of all uses around one non-manifold edge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdgeRadialRing {
    pub uses: Vec<EdgeUseRef>,
}

/// One connected link component at a vertex.  Multiple records for the same
/// vertex explicitly model disconnected fans.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VertexFan {
    pub vertex: (usize, usize),
    pub uses: Vec<VertexUseRef>,
    pub closed: bool,
}

#[derive(Clone, Debug)]
pub struct ComplexPart {
    pub role: BodyRole,
    pub model: Model,
}

#[derive(Clone, Debug)]
pub struct MixedDimensionalBrep {
    pub(super) parts: Vec<ComplexPart>,
    pub(super) shared_faces: Vec<SharedFace>,
    pub(super) radial_rings: Vec<EdgeRadialRing>,
    pub(super) vertex_fans: Vec<VertexFan>,
    pub(super) lineage: Vec<TopologyLineageRecord>,
}

impl MixedDimensionalBrep {
    pub fn new(
        parts: Vec<ComplexPart>,
        shared_faces: Vec<SharedFace>,
        radial_rings: Vec<EdgeRadialRing>,
        vertex_fans: Vec<VertexFan>,
        lineage: Vec<TopologyLineageRecord>,
    ) -> Result<Self> {
        if parts.is_empty() || parts.len() > MAX_PARTS {
            return Err(refuse(
                "BREP_COMPLEX_RESOURCE_LIMIT",
                "Complex requires 1..64 parts",
            ));
        }
        let relations = shared_faces.len() + radial_rings.len() + vertex_fans.len();
        if relations > MAX_RELATIONS {
            return Err(refuse(
                "BREP_COMPLEX_RESOURCE_LIMIT",
                "Complex incidence exceeds 4096 relations",
            ));
        }
        Ok(Self {
            parts,
            shared_faces,
            radial_rings,
            vertex_fans,
            lineage,
        })
    }
    pub fn parts(&self) -> &[ComplexPart] {
        &self.parts
    }
    pub fn shared_faces(&self) -> &[SharedFace] {
        &self.shared_faces
    }
    pub fn radial_rings(&self) -> &[EdgeRadialRing] {
        &self.radial_rings
    }
    pub fn vertex_fans(&self) -> &[VertexFan] {
        &self.vertex_fans
    }
    pub fn lineage(&self) -> &[TopologyLineageRecord] {
        &self.lineage
    }

    pub fn audit(self) -> Result<AuditedTopologyComplex> {
        audit_complex(self)
    }
}

#[derive(Clone, Debug)]
pub struct TopologyComplexCertificate {
    pub capability: &'static str,
    pub complete: bool,
    pub part_count: usize,
    pub solid_cell_count: usize,
    pub sheet_count: usize,
    pub open_shell_count: usize,
    pub shared_face_count: usize,
    pub non_manifold_edge_count: usize,
    pub vertex_fan_count: usize,
    pub boundary_face_count: usize,
    pub max_radial_valence: usize,
    pub manifold_cell_audits: Vec<SolidAuditCertificate>,
    pub naming_complete: bool,
    pub notes: Vec<&'static str>,
}

/// Typestate proving bounded incidence and independent manifold-cell audits.
#[derive(Clone, Debug)]
pub struct AuditedTopologyComplex {
    pub(super) complex: MixedDimensionalBrep,
    pub(super) certificate: TopologyComplexCertificate,
}

#[derive(Clone, Debug)]
pub struct ComplexInterchangeCertificate {
    pub capability: &'static str,
    pub complete: bool,
    pub part_count: usize,
    pub shared_face_count: usize,
    pub radial_ring_count: usize,
    pub vertex_fan_count: usize,
    pub direct_manifold_payloads: bool,
    pub supplemental_incidence_preserved: bool,
}
impl AuditedTopologyComplex {
    pub fn complex(&self) -> &MixedDimensionalBrep {
        &self.complex
    }
    pub fn certificate(&self) -> &TopologyComplexCertificate {
        &self.certificate
    }
    pub fn into_complex(self) -> MixedDimensionalBrep {
        self.complex
    }

    /// The legacy solid hand-off remains deliberately narrow.
    pub fn try_into_globally_audited_solid_set(self) -> Result<GloballyAuditedSolidSet> {
        if self.complex.parts.len() != 1
            || self.complex.parts[0].role != BodyRole::Solid
            || !self.complex.shared_faces.is_empty()
            || !self.complex.radial_rings.is_empty()
            || !self.complex.vertex_fans.is_empty()
        {
            return Err(refuse(
                "BREP_SOLID_AUDIT_REFUSED",
                "Mixed-dimensional or non-manifold complex cannot enter solid typestate",
            ));
        }
        LocallyValidatedModel::new(self.complex.parts.into_iter().next().unwrap().model)?.audit()
    }

    pub fn boundary_faces(&self) -> Vec<FaceRef> {
        let internal: BTreeSet<_> = self
            .complex
            .shared_faces
            .iter()
            .flat_map(|relation| relation.uses.iter().map(|u| (u.part, u.face)))
            .collect();
        let mut boundary = Vec::new();
        for (part_index, part) in self.complex.parts.iter().enumerate() {
            if !matches!(
                part.role,
                BodyRole::Solid | BodyRole::SheetShell | BodyRole::OpenShell | BodyRole::Face
            ) {
                continue;
            }
            for shell in &part.model.shells {
                for use_ in &shell.faces {
                    if !internal.contains(&(part_index, use_.face)) {
                        boundary.push(FaceRef {
                            part: part_index,
                            face: use_.face,
                            reversed: use_.reversed,
                        });
                    }
                }
            }
        }
        boundary
    }

    /// Exact manifold decomposition: no geometry is copied between cells and
    /// no tolerance participates in the partition.
    pub fn manifold_decomposition(&self) -> Vec<ComplexPart> {
        self.complex.parts.clone()
    }
}

pub(super) fn face_surface_key(model: &Model, face: usize) -> Result<String> {
    let face = model
        .faces
        .get(face)
        .ok_or_else(|| refuse("BREP_COMPLEX_INCIDENCE_REFUSED", "Unknown shared face"))?;
    Ok(crate::rational_identity::surface_signature(&face.surface))
}

pub(super) fn edge_curve_key(model: &Model, edge: usize) -> Result<String> {
    let edge = model
        .edges
        .get(edge)
        .ok_or_else(|| refuse("BREP_COMPLEX_INCIDENCE_REFUSED", "Unknown radial edge"))?;
    Ok(crate::rational_identity::curve_signature(&edge.curve))
}

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
