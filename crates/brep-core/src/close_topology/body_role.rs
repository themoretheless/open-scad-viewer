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

#[doc(hidden)]
pub fn encode_complex(
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

#[doc(hidden)]
pub fn decode_complex(
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

#[doc(hidden)]
pub fn interchange_certificate(
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

pub(super) fn face_surface_key(model: &Model, face: usize) -> Result<String> {
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

pub(super) fn edge_curve_key(model: &Model, edge: usize) -> Result<String> {
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
