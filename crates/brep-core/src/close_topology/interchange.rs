//! Incidence-preserving interchange for topology complexes.
use super::*;

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
