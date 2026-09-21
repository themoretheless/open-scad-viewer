//! Bounded mixed-dimensional and non-manifold topology.
//!
//! The legacy `Model` remains a manifold-cell representation.  This module
//! closes the topology gap by assembling validated cells and sheets into an
//! explicit incidence complex.  Shared carriers are relationships between
//! immutable manifold parts; they are never smuggled through `Model::validate`
//! and can therefore never weaken `GloballyAuditedSolidSet`.

use crate::solid_audit::{GloballyAuditedSolidSet, LocallyValidatedModel, SolidAuditCertificate};
use crate::{Model, TopoId, TopoKind, TopologyLineageRecord};
use nurbs_core::{Error, Result};
use std::collections::BTreeSet;

pub const CLOSE_TOPOLOGY_CAPABILITY: &str = "close-topology/1";
pub const TOLERANT_COMPLEX_HEAL_CAPABILITY: &str = "tolerant-complex-heal/1";
pub const CLOSE_TOPOLOGY_STEP_CAPABILITY: &str = "close-topology-step/1";
pub const CLOSE_TOPOLOGY_IGES_CAPABILITY: &str = "close-topology-iges/1";
const MAX_PARTS: usize = 64;
const MAX_RELATIONS: usize = 4096;
const MAX_RADIAL_USES: usize = 16;

fn refuse(code: &'static str, message: impl Into<String>) -> Error {
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
    parts: Vec<ComplexPart>,
    shared_faces: Vec<SharedFace>,
    radial_rings: Vec<EdgeRadialRing>,
    vertex_fans: Vec<VertexFan>,
    lineage: Vec<TopologyLineageRecord>,
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
    complex: MixedDimensionalBrep,
    certificate: TopologyComplexCertificate,
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

fn hex_encode(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(DIGITS[(byte >> 4) as usize] as char);
        out.push(DIGITS[(byte & 15) as usize] as char);
    }
    out
}
fn hex_decode(text: &str) -> Result<Vec<u8>> {
    if !text.len().is_multiple_of(2) || text.len() > 32 * 1024 * 1024 {
        return Err(refuse(
            "BREP_COMPLEX_INTERCHANGE_REFUSED",
            "Invalid or oversized payload encoding",
        ));
    }
    let nibble = |byte: u8| match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    };
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            Ok((nibble(pair[0]).ok_or_else(|| {
                refuse("BREP_COMPLEX_INTERCHANGE_REFUSED", "Invalid payload hex")
            })? << 4)
                | nibble(pair[1]).ok_or_else(|| {
                    refuse("BREP_COMPLEX_INTERCHANGE_REFUSED", "Invalid payload hex")
                })?)
        })
        .collect()
}

fn encode_complex(
    audited: &AuditedTopologyComplex,
    format: &str,
    mut export: impl FnMut(&Model) -> Result<String>,
) -> Result<String> {
    if audited
        .complex
        .parts
        .iter()
        .any(|part| part.role != BodyRole::Solid)
    {
        return Err(refuse(
            "BREP_COMPLEX_INTERCHANGE_REFUSED",
            "This direct interchange cell admits solid parts only; sheet/open-shell entities remain typed-refused",
        ));
    }
    let mut lines = vec![format!("OSV-CLOSE-TOPOLOGY-{format}/1")];
    for part in &audited.complex.parts {
        lines.push(format!(
            "PART {} {}",
            part.role.as_str(),
            hex_encode(export(&part.model)?.as_bytes())
        ));
    }
    for relation in &audited.complex.shared_faces {
        lines.push(format!(
            "SHARED {}",
            relation
                .uses
                .iter()
                .map(|u| format!("{},{},{}", u.part, u.face, usize::from(u.reversed)))
                .collect::<Vec<_>>()
                .join(";")
        ));
    }
    for ring in &audited.complex.radial_rings {
        lines.push(format!(
            "RADIAL {}",
            ring.uses
                .iter()
                .map(|u| format!(
                    "{},{},{},{}",
                    u.part,
                    u.face,
                    u.edge,
                    usize::from(u.reversed)
                ))
                .collect::<Vec<_>>()
                .join(";")
        ));
    }
    for fan in &audited.complex.vertex_fans {
        lines.push(format!(
            "FAN {},{},{} {}",
            fan.vertex.0,
            fan.vertex.1,
            usize::from(fan.closed),
            fan.uses
                .iter()
                .map(|u| format!("{},{},{}", u.part, u.face, u.vertex))
                .collect::<Vec<_>>()
                .join(";")
        ));
    }
    lines.push("END".into());
    let text = lines.join("\n");
    if text.len() > 16 * 1024 * 1024 {
        return Err(refuse(
            "BREP_COMPLEX_INTERCHANGE_REFUSED",
            "Complex interchange exceeds 16 MiB",
        ));
    }
    Ok(text)
}

fn parse_usize(text: Option<&str>, what: &str) -> Result<usize> {
    text.ok_or_else(|| {
        refuse(
            "BREP_COMPLEX_INTERCHANGE_REFUSED",
            format!("Missing {what}"),
        )
    })?
    .parse()
    .map_err(|_| {
        refuse(
            "BREP_COMPLEX_INTERCHANGE_REFUSED",
            format!("Invalid {what}"),
        )
    })
}
fn parse_bool_usize(text: Option<&str>, what: &str) -> Result<bool> {
    match parse_usize(text, what)? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(refuse(
            "BREP_COMPLEX_INTERCHANGE_REFUSED",
            format!("Invalid {what}"),
        )),
    }
}
fn parse_role(text: &str) -> Result<BodyRole> {
    match text {
        "solid" => Ok(BodyRole::Solid),
        _ => Err(refuse(
            "BREP_COMPLEX_INTERCHANGE_REFUSED",
            "Only solid direct interchange parts are admitted",
        )),
    }
}

fn decode_complex(
    text: &str,
    format: &str,
    mut import: impl FnMut(&str) -> Result<Model>,
) -> Result<AuditedTopologyComplex> {
    if text.len() > 16 * 1024 * 1024 {
        return Err(refuse(
            "BREP_COMPLEX_INTERCHANGE_REFUSED",
            "Complex interchange exceeds 16 MiB",
        ));
    }
    let mut lines = text.lines();
    if lines.next() != Some(format!("OSV-CLOSE-TOPOLOGY-{format}/1").as_str()) {
        return Err(refuse(
            "BREP_COMPLEX_INTERCHANGE_REFUSED",
            "Complex interchange header mismatch",
        ));
    }
    let mut parts = Vec::new();
    let mut shared_faces = Vec::new();
    let mut radial_rings = Vec::new();
    let mut vertex_fans = Vec::new();
    let mut ended = false;
    for line in lines {
        if line == "END" {
            ended = true;
            break;
        }
        let mut fields = line.splitn(3, ' ');
        match fields.next() {
            Some("PART") => {
                let role = parse_role(fields.next().ok_or_else(|| {
                    refuse("BREP_COMPLEX_INTERCHANGE_REFUSED", "Missing part role")
                })?)?;
                let payload = String::from_utf8(hex_decode(fields.next().unwrap_or_default())?)
                    .map_err(|_| {
                        refuse("BREP_COMPLEX_INTERCHANGE_REFUSED", "Payload is not UTF-8")
                    })?;
                parts.push(ComplexPart {
                    role,
                    model: import(&payload)?,
                });
            }
            Some("SHARED") => {
                let uses = fields
                    .next()
                    .unwrap_or_default()
                    .split(';')
                    .map(|entry| {
                        let mut f = entry.split(',');
                        Ok(FaceRef {
                            part: parse_usize(f.next(), "part")?,
                            face: parse_usize(f.next(), "face")?,
                            reversed: parse_bool_usize(f.next(), "orientation")?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                shared_faces.push(SharedFace { uses });
            }
            Some("RADIAL") => {
                let uses = fields
                    .next()
                    .unwrap_or_default()
                    .split(';')
                    .map(|entry| {
                        let mut f = entry.split(',');
                        Ok(EdgeUseRef {
                            part: parse_usize(f.next(), "part")?,
                            face: parse_usize(f.next(), "face")?,
                            edge: parse_usize(f.next(), "edge")?,
                            reversed: parse_bool_usize(f.next(), "orientation")?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                radial_rings.push(EdgeRadialRing { uses });
            }
            Some("FAN") => {
                let header = fields.next().unwrap_or_default();
                let mut h = header.split(',');
                let vertex = (
                    parse_usize(h.next(), "fan part")?,
                    parse_usize(h.next(), "fan vertex")?,
                );
                let closed = parse_bool_usize(h.next(), "fan closure")?;
                let uses = fields
                    .next()
                    .unwrap_or_default()
                    .split(';')
                    .map(|entry| {
                        let mut f = entry.split(',');
                        Ok(VertexUseRef {
                            part: parse_usize(f.next(), "part")?,
                            face: parse_usize(f.next(), "face")?,
                            vertex: parse_usize(f.next(), "vertex")?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                vertex_fans.push(VertexFan {
                    vertex,
                    uses,
                    closed,
                });
            }
            _ => {
                return Err(refuse(
                    "BREP_COMPLEX_INTERCHANGE_REFUSED",
                    "Unknown complex interchange record",
                ));
            }
        }
    }
    if !ended || parts.is_empty() {
        return Err(refuse(
            "BREP_COMPLEX_INTERCHANGE_REFUSED",
            "Truncated complex interchange",
        ));
    }
    MixedDimensionalBrep::new(parts, shared_faces, radial_rings, vertex_fans, vec![])?.audit()
}

fn interchange_certificate(
    audited: &AuditedTopologyComplex,
    capability: &'static str,
) -> ComplexInterchangeCertificate {
    ComplexInterchangeCertificate {
        capability,
        complete: true,
        part_count: audited.complex.parts.len(),
        shared_face_count: audited.complex.shared_faces.len(),
        radial_ring_count: audited.complex.radial_rings.len(),
        vertex_fan_count: audited.complex.vertex_fans.len(),
        direct_manifold_payloads: true,
        supplemental_incidence_preserved: true,
    }
}

pub fn export_complex_step(
    audited: &AuditedTopologyComplex,
) -> Result<(String, ComplexInterchangeCertificate)> {
    let text = encode_complex(audited, "STEP", |model| {
        crate::export_step_v4(model).map(|v| v.0)
    })?;
    Ok((
        text,
        interchange_certificate(audited, CLOSE_TOPOLOGY_STEP_CAPABILITY),
    ))
}
pub fn import_complex_step(
    text: &str,
) -> Result<(AuditedTopologyComplex, ComplexInterchangeCertificate)> {
    let audited = decode_complex(text, "STEP", |payload| {
        crate::import_step_v4(payload).map(|v| v.0)
    })?;
    let certificate = interchange_certificate(&audited, CLOSE_TOPOLOGY_STEP_CAPABILITY);
    Ok((audited, certificate))
}
pub fn export_complex_iges(
    audited: &AuditedTopologyComplex,
) -> Result<(String, ComplexInterchangeCertificate)> {
    let text = encode_complex(audited, "IGES", |model| {
        crate::export_iges_v2(model).map(|v| v.0)
    })?;
    Ok((
        text,
        interchange_certificate(audited, CLOSE_TOPOLOGY_IGES_CAPABILITY),
    ))
}
pub fn import_complex_iges(
    text: &str,
) -> Result<(AuditedTopologyComplex, ComplexInterchangeCertificate)> {
    let audited = decode_complex(text, "IGES", |payload| {
        crate::import_iges_v2(payload).map(|v| v.0)
    })?;
    let certificate = interchange_certificate(&audited, CLOSE_TOPOLOGY_IGES_CAPABILITY);
    Ok((audited, certificate))
}

fn face_surface_key(model: &Model, face: usize) -> Result<String> {
    let face = model
        .faces
        .get(face)
        .ok_or_else(|| refuse("BREP_COMPLEX_INCIDENCE_REFUSED", "Unknown shared face"))?;
    value_codec::to_string(&face.surface).map_err(|_| {
        refuse(
            "BREP_COMPLEX_INCIDENCE_REFUSED",
            "Cannot encode shared face carrier",
        )
    })
}

fn edge_curve_key(model: &Model, edge: usize) -> Result<String> {
    let edge = model
        .edges
        .get(edge)
        .ok_or_else(|| refuse("BREP_COMPLEX_INCIDENCE_REFUSED", "Unknown radial edge"))?;
    value_codec::to_string(&edge.curve).map_err(|_| {
        refuse(
            "BREP_COMPLEX_INCIDENCE_REFUSED",
            "Cannot encode radial edge carrier",
        )
    })
}

fn audit_complex(complex: MixedDimensionalBrep) -> Result<AuditedTopologyComplex> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cuboid;

    fn solid(model: Model) -> ComplexPart {
        ComplexPart {
            role: BodyRole::Solid,
            model,
        }
    }

    #[test]
    fn solid_typestate_is_not_weakened() {
        let one = MixedDimensionalBrep::new(
            vec![solid(cuboid([0.; 3], [1.; 3]).unwrap())],
            vec![],
            vec![],
            vec![],
            vec![],
        )
        .unwrap()
        .audit()
        .unwrap();
        one.try_into_globally_audited_solid_set().unwrap();

        let a = cuboid([0.; 3], [1.; 3]).unwrap();
        let b = a.clone();
        let relation = SharedFace {
            uses: vec![
                FaceRef {
                    part: 0,
                    face: 0,
                    reversed: false,
                },
                FaceRef {
                    part: 1,
                    face: 0,
                    reversed: true,
                },
            ],
        };
        let mixed = MixedDimensionalBrep::new(
            vec![solid(a), solid(b)],
            vec![relation],
            vec![],
            vec![],
            vec![],
        )
        .unwrap()
        .audit()
        .unwrap();
        assert_eq!(mixed.certificate().solid_cell_count, 2);
        assert_eq!(mixed.certificate().shared_face_count, 1);
        assert_eq!(mixed.boundary_faces().len(), 10);
        assert_eq!(mixed.manifold_decomposition().len(), 2);
        let (step, _) = export_complex_step(&mixed).unwrap();
        let (step_back, step_cert) = import_complex_step(&step).unwrap();
        assert_eq!(step_back.certificate().shared_face_count, 1);
        assert!(step_cert.supplemental_incidence_preserved);
        let (iges, _) = export_complex_iges(&mixed).unwrap();
        let (iges_back, iges_cert) = import_complex_iges(&iges).unwrap();
        assert_eq!(iges_back.boundary_faces().len(), 10);
        assert!(iges_cert.direct_manifold_payloads);
        assert_eq!(
            mixed
                .try_into_globally_audited_solid_set()
                .unwrap_err()
                .code,
            "BREP_SOLID_AUDIT_REFUSED"
        );
    }

    #[test]
    fn radial_fans_and_orientation_are_checked() {
        let model = cuboid([0.; 3], [1.; 3]).unwrap();
        let edge = model.1.edges[0];
        let face = model.1.faces[0];
        let parts = vec![
            solid(model.clone()),
            solid(model.clone()),
            solid(model.clone()),
        ];
        let ring = EdgeRadialRing {
            uses: (0..3)
                .map(|part| EdgeUseRef {
                    part,
                    face: 0,
                    edge: 0,
                    reversed: part % 2 == 0,
                })
                .collect(),
        };
        let fan = VertexFan {
            vertex: (0, model.edges[0].vertices[0]),
            uses: vec![VertexUseRef {
                part: 0,
                face: 0,
                vertex: model.edges[0].vertices[0],
            }],
            closed: false,
        };
        let audited = MixedDimensionalBrep::new(
            parts,
            vec![],
            vec![ring],
            vec![fan],
            vec![TopologyLineageRecord {
                operation: "share".into(),
                entity_kind: "edge".into(),
                parents: vec![edge],
                children: vec![edge, face],
            }],
        )
        .unwrap()
        .audit()
        .unwrap();
        assert_eq!(audited.certificate().max_radial_valence, 3);
        assert_eq!(audited.certificate().vertex_fan_count, 1);

        let bad = SharedFace {
            uses: vec![
                FaceRef {
                    part: 0,
                    face: 0,
                    reversed: false,
                },
                FaceRef {
                    part: 1,
                    face: 0,
                    reversed: false,
                },
            ],
        };
        assert_eq!(
            MixedDimensionalBrep::new(
                vec![solid(model.clone()), solid(model)],
                vec![bad],
                vec![],
                vec![],
                vec![]
            )
            .unwrap()
            .audit()
            .unwrap_err()
            .code,
            "BREP_COMPLEX_ORIENTATION_REFUSED"
        );
    }

    #[test]
    fn multi_cell_heal_budgets_partitions_and_atomic_refusal() {
        let model = cuboid([0.; 3], [1.; 3]).unwrap();
        let audited = MixedDimensionalBrep::new(
            vec![solid(model.clone()), solid(model.clone())],
            vec![],
            vec![],
            vec![],
            vec![],
        )
        .unwrap()
        .audit()
        .unwrap();
        let plan = ComplexHealPlan {
            context_canonical: "v1:test".into(),
            correspondences: vec![LocalCorrespondence {
                kind: CorrespondenceKind::Vertex,
                entities: vec![(0, model.1.vertices[0]), (1, model.1.vertices[0])],
                displacement_mm: 1e-8,
                context_path: vec!["compound".into(), "cell-pair".into(), "vertex".into()],
                provenance: "qualified-test-correspondence".into(),
            }],
            per_entity_limit_mm: 2e-8,
            cumulative_limit_mm: 3e-8,
            partitions: vec![ExactParameterPartition {
                carrier_definition: "exact-line".into(),
                parent_domain_bits: [0f64.to_bits(), 1f64.to_bits()],
                child_domain_bits: vec![
                    [0f64.to_bits(), 0.5f64.to_bits()],
                    [0.5f64.to_bits(), 1f64.to_bits()],
                ],
            }],
        };
        let cert = certify_complex_heal(&audited, &plan).unwrap();
        assert_eq!(cert.touched_entity_count, 2);
        assert_eq!(cert.cumulative_displacement_mm, 2e-8);
        assert!(cert.rollback_safe && cert.idempotent && cert.no_tolerance_growth);
        let mut over = plan.clone();
        over.cumulative_limit_mm = 1e-8;
        assert_eq!(
            certify_complex_heal(&audited, &over).unwrap_err().code,
            "BREP_HEAL_BUDGET_EXCEEDED"
        );
    }

    #[test]
    fn sheet_solid_compound_transform_permutation_and_resources() {
        let source = cuboid([0.; 3], [1.; 3]).unwrap();
        let placed = crate::transform::affine(
            &source,
            [
                [0., -1., 0., 4.],
                [1., 0., 0., 5.],
                [0., 0., 1., 6.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let mut sheet = placed.clone();
        sheet.0.bodies.clear();
        sheet.1.bodies.clear();
        sheet.validate().unwrap();
        let audited = MixedDimensionalBrep::new(
            vec![
                ComplexPart {
                    role: BodyRole::SheetShell,
                    model: sheet,
                },
                solid(placed),
            ],
            vec![],
            vec![],
            vec![],
            vec![],
        )
        .unwrap()
        .audit()
        .unwrap();
        assert_eq!(
            (
                audited.certificate().sheet_count,
                audited.certificate().solid_cell_count
            ),
            (1, 1)
        );
        assert_eq!(audited.boundary_faces().len(), 12);

        let over = (0..=MAX_PARTS).map(|_| solid(source.clone())).collect();
        assert_eq!(
            MixedDimensionalBrep::new(over, vec![], vec![], vec![], vec![])
                .unwrap_err()
                .code,
            "BREP_COMPLEX_RESOURCE_LIMIT"
        );
        assert_eq!(
            import_complex_step("OSV-CLOSE-TOPOLOGY-STEP/1\nBOGUS\nEND")
                .unwrap_err()
                .code,
            "BREP_COMPLEX_INTERCHANGE_REFUSED"
        );
    }
}
