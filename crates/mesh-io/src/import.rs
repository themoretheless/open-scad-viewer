//! Bounded native mesh decoders and import welding.
use crate::{Error, Result};
use std::collections::BTreeMap;
pub const MAX_TRIANGLES: usize = 250_000;
pub const MAX_VERTICES: usize = 750_000;
#[derive(Clone, Debug)]
pub struct RawMesh {
    pub positions: Vec<f64>,
    pub indices: Vec<usize>,
}
#[derive(Clone, Debug)]
pub struct ImportedMesh {
    pub mesh: RawMesh,
    pub source_vertex_count: usize,
    pub degenerate_triangles: usize,
}
#[derive(Clone, Copy, Debug)]
pub enum Weld {
    None,
    DisplayExact,
    Grid(f64),
}
fn err(kind: &str, message: impl Into<String>) -> Error {
    Error::new(
        match kind {
            "EMPTY" => "MESH_IMPORT_EMPTY",
            "LIMIT" => "MESH_IMPORT_LIMIT",
            "ENCODING" => "MESH_IMPORT_ENCODING",
            _ => "MESH_IMPORT_INVALID_DATA",
        },
        message,
    )
}
fn bad(message: impl Into<String>) -> Error {
    err("INVALID_DATA", message)
}
fn limit(value: usize, max: usize, label: &str) -> Result<()> {
    if value > max {
        Err(err("LIMIT", format!("{label} exceeds the {max} limit.")))
    } else {
        Ok(())
    }
}
fn text(bytes: &[u8]) -> Result<&str> {
    std::str::from_utf8(bytes)
        .map(|s| s.trim_start_matches('\u{feff}'))
        .map_err(|_| err("ENCODING", "Input must be valid UTF-8 text."))
}
fn number(token: Option<&str>) -> Result<f64> {
    let x = token
        .ok_or_else(|| bad("Missing number."))?
        .parse::<f64>()
        .map_err(|_| bad("Invalid number."))?;
    if x.is_finite() {
        Ok(x)
    } else {
        Err(bad("Coordinate must be finite."))
    }
}
fn integer(token: Option<&str>) -> Result<i64> {
    token
        .ok_or_else(|| bad("Missing integer."))?
        .parse()
        .map_err(|_| bad("Invalid integer."))
}
fn fan(indices: &mut Vec<usize>, face: &[usize]) -> Result<()> {
    if face.len() < 3 {
        return Err(bad("Face needs at least three vertices."));
    }
    limit(
        indices.len() / 3 + face.len() - 2,
        MAX_TRIANGLES,
        "Triangle count",
    )?;
    for i in 1..face.len() - 1 {
        indices.extend([face[0], face[i], face[i + 1]]);
    }
    Ok(())
}
pub fn obj(bytes: &[u8]) -> Result<RawMesh> {
    let mut mesh = RawMesh {
        positions: Vec::new(),
        indices: Vec::new(),
    };
    for line in text(bytes)?.split(['\r', '\n']) {
        let line = line.split('#').next().unwrap();
        let parts: Vec<_> = line.split_whitespace().collect();
        match parts.first().copied() {
            Some("v") => {
                limit(mesh.positions.len() / 3 + 1, MAX_VERTICES, "Vertex count")?;
                let w = if parts.len() >= 5 && parts.len() != 7 {
                    number(parts.get(4).copied())?
                } else {
                    1.
                };
                if w == 0. {
                    return Err(bad("Vertex w must be non-zero."));
                }
                for k in 1..4 {
                    mesh.positions.push(number(parts.get(k).copied())? / w);
                }
            }
            Some("f") => {
                let count = mesh.positions.len() / 3;
                let mut face = Vec::new();
                for part in &parts[1..] {
                    let index = integer(part.split('/').next())?;
                    let resolved = if index > 0 {
                        index - 1
                    } else {
                        count as i64 + index
                    };
                    if index == 0 || resolved < 0 || resolved >= count as i64 {
                        return Err(bad(
                            "Face vertex index is out of range or before it is defined.",
                        ));
                    }
                    face.push(resolved as usize);
                }
                fan(&mut mesh.indices, &face)?;
            }
            _ => (),
        }
    }
    if mesh.indices.is_empty() {
        return Err(err("EMPTY", "OBJ contains no faces."));
    }
    Ok(mesh)
}
pub fn finalize(mesh: RawMesh, weld: Weld) -> Result<ImportedMesh> {
    if !mesh.positions.len().is_multiple_of(3) || !mesh.indices.len().is_multiple_of(3) {
        return Err(bad("Mesh storage is inconsistent."));
    }
    let count = mesh.positions.len() / 3;
    limit(count, MAX_VERTICES, "Vertex count")?;
    limit(mesh.indices.len() / 3, MAX_TRIANGLES, "Triangle count")?;
    if mesh.positions.iter().any(|x| !x.is_finite()) || mesh.indices.iter().any(|i| *i >= count) {
        return Err(bad(
            "Non-finite coordinate or triangle index out of bounds.",
        ));
    }
    let mut positions = Vec::new();
    let mut remap = Vec::new();
    let mut seen = BTreeMap::new();
    if let Weld::Grid(distance) = weld
        && (!distance.is_finite() || distance <= 0.)
    {
        return Err(bad("Weld distance must be positive."));
    }
    for p in mesh.positions.as_chunks::<3>().0 {
        let key = p.map(|x| {
            let v = match weld {
                Weld::DisplayExact => (x as f32) as f64,
                Weld::Grid(d) => {
                    let x = x * (1. / d);
                    let low = x.floor();
                    if x - low >= 0.5 { low + 1. } else { low }
                }
                Weld::None => x,
            };
            if v == 0. { 0 } else { v.to_bits() }
        });
        let id = if matches!(weld, Weld::None) {
            let id = positions.len() / 3;
            positions.extend(p);
            id
        } else {
            *seen.entry(key).or_insert_with(|| {
                let id = positions.len() / 3;
                positions.extend(p);
                id
            })
        };
        remap.push(id);
    }
    let mut indices = Vec::new();
    let mut degenerate = 0;
    for t in mesh.indices.as_chunks::<3>().0 {
        let [a, b, c] = t.map(|i| remap[i]);
        if a == b || b == c || c == a {
            degenerate += 1;
        } else {
            indices.extend([a, b, c]);
        }
    }
    if indices.is_empty() {
        return Err(err("EMPTY", "Input contains no usable triangles."));
    }
    Ok(ImportedMesh {
        mesh: RawMesh { positions, indices },
        source_vertex_count: count,
        degenerate_triangles: degenerate,
    })
}
#[derive(Clone, Copy)]
enum Scalar {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    F32,
    F64,
}
fn scalar(s: &str) -> Result<Scalar> {
    Ok(match s {
        "char" | "int8" => Scalar::I8,
        "uchar" | "uint8" => Scalar::U8,
        "short" | "int16" => Scalar::I16,
        "ushort" | "uint16" => Scalar::U16,
        "int" | "int32" => Scalar::I32,
        "uint" | "uint32" => Scalar::U32,
        "float" | "float32" => Scalar::F32,
        "double" | "float64" => Scalar::F64,
        _ => return Err(bad("Unknown PLY property type.")),
    })
}
struct Property {
    name: String,
    item: Scalar,
    list: Option<Scalar>,
}
struct Element {
    name: String,
    count: usize,
    properties: Vec<Property>,
}
struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
    tokens: Option<std::str::SplitWhitespace<'a>>,
    little: bool,
}
impl Reader<'_> {
    fn read(&mut self, kind: Scalar) -> Result<f64> {
        if let Some(tokens) = self.tokens.as_mut() {
            return match kind {
                Scalar::F32 | Scalar::F64 => number(tokens.next()),
                _ => integer(tokens.next()).map(|x| x as f64),
            };
        }
        let size = match kind {
            Scalar::I8 | Scalar::U8 => 1,
            Scalar::I16 | Scalar::U16 => 2,
            Scalar::I32 | Scalar::U32 | Scalar::F32 => 4,
            Scalar::F64 => 8,
        };
        let end = self
            .offset
            .checked_add(size)
            .ok_or_else(|| bad("Truncated PLY body."))?;
        let bytes = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| bad("Truncated PLY body."))?;
        self.offset = end;
        let mut v = [0u8; 8];
        if self.little {
            v[..size].copy_from_slice(bytes);
        } else {
            for (i, b) in bytes.iter().rev().enumerate() {
                v[i] = *b;
            }
        }
        Ok(match kind {
            Scalar::I8 => v[0] as i8 as f64,
            Scalar::U8 => v[0] as f64,
            Scalar::I16 => i16::from_le_bytes(v[..2].try_into().unwrap()) as f64,
            Scalar::U16 => u16::from_le_bytes(v[..2].try_into().unwrap()) as f64,
            Scalar::I32 => i32::from_le_bytes(v[..4].try_into().unwrap()) as f64,
            Scalar::U32 => u32::from_le_bytes(v[..4].try_into().unwrap()) as f64,
            Scalar::F32 => f32::from_le_bytes(v[..4].try_into().unwrap()) as f64,
            Scalar::F64 => f64::from_le_bytes(v),
        })
    }
}
pub fn ply(bytes: &[u8]) -> Result<RawMesh> {
    let mut offset = 0;
    let mut elements: Vec<Element> = Vec::new();
    let mut format = None;
    let mut ended = false;
    while offset < bytes.len().min(1 << 20) {
        let end = bytes[offset..]
            .iter()
            .position(|b| *b == b'\n')
            .map(|n| offset + n)
            .ok_or_else(|| bad("PLY header needs end_header and newline."))?;
        let line = text(&bytes[offset..end])?.trim();
        offset = end + 1;
        let p: Vec<_> = line.split_whitespace().collect();
        match p.first().copied() {
            Some("ply") => (),
            Some("format") => {
                if p.get(2) != Some(&"1.0") {
                    return Err(bad("Unsupported PLY version."));
                }
                let f = *p.get(1).ok_or_else(|| bad("Missing PLY format."))?;
                if !["ascii", "binary_little_endian", "binary_big_endian"].contains(&f) {
                    return Err(bad("Unsupported PLY format."));
                }
                format = Some(f.to_owned());
            }
            Some("comment" | "obj_info") | None => (),
            Some("element") => {
                let count = integer(p.get(2).copied())?;
                if count < 0 {
                    return Err(bad("Negative element count."));
                }
                limit(count as usize, MAX_VERTICES, "Element count")?;
                elements.push(Element {
                    name: p
                        .get(1)
                        .ok_or_else(|| bad("Missing element name."))?
                        .to_string(),
                    count: count as usize,
                    properties: Vec::new(),
                });
            }
            Some("property") => {
                let element = elements
                    .last_mut()
                    .ok_or_else(|| bad("Property before element."))?;
                let property = if p.get(1) == Some(&"list") {
                    Property {
                        name: p
                            .get(4)
                            .ok_or_else(|| bad("Missing list name."))?
                            .to_string(),
                        list: Some(scalar(p.get(2).copied().unwrap_or(""))?),
                        item: scalar(p.get(3).copied().unwrap_or(""))?,
                    }
                } else {
                    Property {
                        name: p
                            .get(2)
                            .ok_or_else(|| bad("Missing property name."))?
                            .to_string(),
                        list: None,
                        item: scalar(p.get(1).copied().unwrap_or(""))?,
                    }
                };
                element.properties.push(property);
            }
            Some("end_header") => {
                ended = true;
                break;
            }
            _ => return Err(bad("Unknown PLY header statement.")),
        }
    }
    if !ended || !bytes.starts_with(b"ply") {
        return Err(bad("Missing PLY header."));
    }
    let format = format.ok_or_else(|| bad("Missing PLY format."))?;
    let vertex = elements
        .iter()
        .position(|e| e.name == "vertex")
        .ok_or_else(|| bad("PLY has no vertex element."))?;
    let face = elements
        .iter()
        .position(|e| e.name == "face")
        .ok_or_else(|| bad("PLY has no face element."))?;
    let count = elements[vertex].count;
    limit(count, MAX_VERTICES, "Vertex count")?;
    limit(elements[face].count, MAX_TRIANGLES, "Face count")?;
    let axis = ["x", "y", "z"]
        .map(|name| {
            elements[vertex]
                .properties
                .iter()
                .position(|p| p.name == name && p.list.is_none())
                .ok_or_else(|| bad("PLY lacks scalar coordinate."))
        })
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
    let face_list = elements[face]
        .properties
        .iter()
        .position(|p| {
            p.list.is_some() && ["vertex_indices", "vertex_index"].contains(&p.name.as_str())
        })
        .ok_or_else(|| bad("PLY lacks vertex_indices list."))?;
    let mut reader = Reader {
        bytes,
        offset,
        tokens: if format == "ascii" {
            Some(text(&bytes[offset..])?.split_whitespace())
        } else {
            None
        },
        little: format == "binary_little_endian",
    };
    let mut mesh = RawMesh {
        positions: Vec::new(),
        indices: Vec::new(),
    };
    for (eid, e) in elements.iter().enumerate() {
        for _ in 0..e.count {
            let mut values = Vec::new();
            let mut face_ids = Vec::new();
            for (pid, p) in e.properties.iter().enumerate() {
                if let Some(kind) = p.list {
                    let length = reader.read(kind)?;
                    if !length.is_finite()
                        || length < 0.
                        || length.fract() != 0.
                        || length > MAX_VERTICES as f64
                    {
                        return Err(bad("Invalid PLY list length."));
                    }
                    for _ in 0..length as usize {
                        let id = reader.read(p.item)?;
                        if eid == face && pid == face_list {
                            if !id.is_finite() || id < 0. || id.fract() != 0. || id >= count as f64
                            {
                                return Err(bad("PLY face index out of range."));
                            }
                            face_ids.push(id as usize);
                        }
                    }
                    values.push(f64::NAN);
                } else {
                    values.push(reader.read(p.item)?);
                }
            }
            if eid == vertex {
                for k in &axis {
                    let v = values[*k];
                    if !v.is_finite() {
                        return Err(bad("Non-finite PLY vertex."));
                    }
                    mesh.positions.push(v);
                }
            } else if eid == face {
                fan(&mut mesh.indices, &face_ids)?;
            }
        }
    }
    if mesh.indices.is_empty() {
        return Err(err("EMPTY", "PLY contains no faces."));
    }
    Ok(mesh)
}

/// Strict binary-only STL decoder used by the interactive scene importer.
pub fn binary_stl(bytes: &[u8]) -> Result<Vec<f32>> {
    let ascii = || {
        let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
        let bytes = bytes
            .iter()
            .copied()
            .skip_while(|b| b.is_ascii_whitespace())
            .take(5)
            .map(|b| b.to_ascii_lowercase())
            .collect::<Vec<_>>();
        bytes == b"solid"
    };
    if bytes.len() < 84 {
        return Err(Error::new(
            if ascii() {
                "STL_UNSUPPORTED_ASCII"
            } else {
                "STL_INVALID_HEADER"
            },
            if ascii() {
                "ASCII STL is not supported; choose a binary STL"
            } else {
                "Binary STL is shorter than its 84-byte header"
            },
        ));
    }
    let count = u32::from_le_bytes(bytes[80..84].try_into().unwrap()) as usize;
    if count > MAX_TRIANGLES {
        return Err(Error::new(
            "STL_TOO_MANY_TRIANGLES",
            "STL exceeds the 250,000 triangle import limit",
        ));
    }
    if bytes.len() != 84 + count * 50 {
        return Err(Error::new(
            if ascii() {
                "STL_UNSUPPORTED_ASCII"
            } else {
                "STL_INVALID_SIZE"
            },
            if ascii() {
                "ASCII STL is not supported; choose a binary STL"
            } else {
                "Binary STL size does not match its declared triangle count"
            },
        ));
    }
    let mut positions = Vec::with_capacity(count * 9);
    for record in bytes[84..].as_chunks::<50>().0 {
        for b in record[12..48].as_chunks::<4>().0 {
            let x = f32::from_le_bytes(*b);
            if !x.is_finite() {
                return Err(Error::new(
                    "STL_NON_FINITE_COORDINATE",
                    "STL contains a non-finite coordinate",
                ));
            }
            positions.push(x);
        }
    }
    Ok(positions)
}

pub fn stl(bytes: &[u8]) -> Result<RawMesh> {
    if bytes.len() >= 84 {
        let count = u32::from_le_bytes(bytes[80..84].try_into().unwrap()) as usize;
        if 84 + count * 50 == bytes.len() {
            let positions = binary_stl(bytes)
                .map_err(|e| bad(e.message))?
                .into_iter()
                .map(f64::from)
                .collect::<Vec<_>>();
            let indices = (0..positions.len() / 3).collect();
            return Ok(RawMesh { positions, indices });
        }
    }
    let mut mesh = RawMesh {
        positions: Vec::new(),
        indices: Vec::new(),
    };
    let mut state = 0;
    let mut facet = Vec::new();
    for line in text(bytes)?.split(['\r', '\n']) {
        let p: Vec<_> = line.split_whitespace().collect();
        let Some(keyword) = p.first() else {
            continue;
        };
        match keyword.to_ascii_lowercase().as_str() {
            "solid" if state == 0 => state = 1,
            "endsolid" if state == 1 => state = 6,
            "facet" if state == 1 && p.len() == 5 && p[1].eq_ignore_ascii_case("normal") => {
                for n in &p[2..] {
                    number(Some(n))?;
                }
                state = 2;
            }
            "outer" if state == 2 && p.len() == 2 && p[1].eq_ignore_ascii_case("loop") => state = 3,
            "vertex" if state == 3 && p.len() == 4 && facet.len() < 9 => {
                for n in &p[1..] {
                    facet.push(number(Some(n))?);
                }
            }
            "endloop" if state == 3 && facet.len() == 9 && p.len() == 1 => state = 4,
            "endfacet" if state == 4 && p.len() == 1 => {
                limit(
                    mesh.indices.len() / 3 + 1,
                    MAX_TRIANGLES,
                    "STL triangle count",
                )?;
                let base = mesh.positions.len() / 3;
                mesh.positions.append(&mut facet);
                mesh.indices.extend([base, base + 1, base + 2]);
                state = 1;
            }
            _ => return Err(bad("Invalid or misplaced ASCII STL statement.")),
        }
    }
    if state != 6 {
        return Err(bad("ASCII STL is incomplete."));
    }
    if mesh.indices.is_empty() {
        return Err(err("EMPTY", "STL contains no faces."));
    }
    Ok(mesh)
}
fn triangulate_polygon(positions: &[f64], face: &[usize]) -> Result<Vec<usize>> {
    if face.len() == 3 {
        return Ok(face.to_vec());
    }
    if face.len() < 3 {
        return Err(bad("Face needs at least three vertices."));
    }
    let mut normal = [0.; 3];
    for i in 0..face.len() {
        let p = &positions[face[i] * 3..face[i] * 3 + 3];
        let q = &positions[face[(i + 1) % face.len()] * 3..face[(i + 1) % face.len()] * 3 + 3];
        normal[0] += (p[1] - q[1]) * (p[2] + q[2]);
        normal[1] += (p[2] - q[2]) * (p[0] + q[0]);
        normal[2] += (p[0] - q[0]) * (p[1] + q[1]);
    }
    let axis = if normal[0].abs() >= normal[1].abs() && normal[0].abs() >= normal[2].abs() {
        0
    } else if normal[1].abs() >= normal[2].abs() {
        1
    } else {
        2
    };
    let points: Vec<[f64; 2]> = face
        .iter()
        .map(|i| {
            let p = &positions[i * 3..i * 3 + 3];
            match axis {
                0 => [p[1], p[2]],
                1 => [p[0], p[2]],
                _ => [p[0], p[1]],
            }
        })
        .collect();
    let mut area = 0.;
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        area += a[0] * b[1] - b[0] * a[1];
    }
    if area == 0. {
        return Err(bad("Degenerate polygon face."));
    }
    let orientation = area.signum();
    let cross = |a: [f64; 2], b: [f64; 2], c: [f64; 2]| {
        (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    };
    let mut remaining: Vec<usize> = (0..face.len()).collect();
    let mut out = Vec::new();
    let mut work = 0usize;
    while remaining.len() > 3 {
        let mut clipped = false;
        for i in 0..remaining.len() {
            work += remaining.len();
            if work > 8_000_000 {
                return Err(err("LIMIT", "Polygon triangulation work budget exceeded."));
            }
            let before = remaining[(i + remaining.len() - 1) % remaining.len()];
            let current = remaining[i];
            let after = remaining[(i + 1) % remaining.len()];
            let (a, b, c) = (points[before], points[current], points[after]);
            if cross(a, b, c) * orientation <= 0. {
                continue;
            }
            if remaining.iter().any(|j| {
                *j != before
                    && *j != current
                    && *j != after
                    && cross(a, b, points[*j]) * orientation >= 0.
                    && cross(b, c, points[*j]) * orientation >= 0.
                    && cross(c, a, points[*j]) * orientation >= 0.
            }) {
                continue;
            }
            out.extend([face[before], face[current], face[after]]);
            remaining.remove(i);
            clipped = true;
            break;
        }
        if !clipped {
            return Err(bad("Self-intersecting polygon face."));
        }
    }
    out.extend(remaining.iter().map(|i| face[*i]));
    Ok(out)
}
pub fn off(bytes: &[u8]) -> Result<RawMesh> {
    let lines: Vec<Vec<_>> = text(bytes)?
        .split(['\r', '\n'])
        .map(|line| line.split('#').next().unwrap().split_whitespace().collect())
        .filter(|p: &Vec<_>| !p.is_empty())
        .collect();
    let header = lines.first().ok_or_else(|| bad("Missing OFF header."))?;
    if !["OFF", "COFF"].contains(&header[0].to_ascii_uppercase().as_str()) {
        return Err(bad("OFF input must begin with OFF or COFF."));
    }
    let mut line = 1;
    let counts = if header.len() > 1 {
        &header[1..]
    } else {
        line = 2;
        lines.get(1).ok_or_else(|| bad("Missing OFF counts."))?
    };
    let count = integer(counts.first().copied())?;
    let faces = integer(counts.get(1).copied())?;
    let edge_count = integer(counts.get(2).copied())?;
    if count < 0 || faces < 0 || edge_count < 0 {
        return Err(bad("Negative OFF counts."));
    }
    limit(count as usize, MAX_VERTICES, "OFF vertex count")?;
    limit(faces as usize, MAX_TRIANGLES, "OFF face count")?;
    let mut mesh = RawMesh {
        positions: Vec::new(),
        indices: Vec::new(),
    };
    for _ in 0..count {
        let p = lines
            .get(line)
            .ok_or_else(|| bad("Incomplete OFF vertices."))?;
        for k in 0..3 {
            mesh.positions.push(number(p.get(k).copied())?);
        }
        for attr in &p[3..] {
            number(Some(attr))?;
        }
        line += 1;
    }
    for _ in 0..faces {
        let p = lines
            .get(line)
            .ok_or_else(|| bad("Incomplete OFF faces."))?;
        let n = integer(p.first().copied())?;
        if n < 3 || n as usize + 1 > p.len() {
            return Err(bad("Incomplete OFF polygon."));
        }
        let mut face = Vec::new();
        for id in &p[1..=n as usize] {
            let id = integer(Some(id))?;
            if id < 0 || id >= count {
                return Err(bad("OFF face index out of bounds."));
            }
            face.push(id as usize);
        }
        for attr in &p[n as usize + 1..] {
            number(Some(attr))?;
        }
        limit(
            mesh.indices.len() / 3 + face.len() - 2,
            MAX_TRIANGLES,
            "OFF triangle count",
        )?;
        mesh.indices
            .extend(triangulate_polygon(&mesh.positions, &face)?);
        line += 1;
    }
    if mesh.indices.is_empty() {
        return Err(err("EMPTY", "OFF contains no faces."));
    }
    Ok(mesh)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn obj_negative_indices_and_homogeneous_coordinates() {
        let m = obj(b"v 0 0 0\nv 2 0 0 2\nv 0 1 0\nf -3/1 -2/1 -1/1\n").unwrap();
        assert_eq!(m.positions, [0., 0., 0., 1., 0., 0., 0., 1., 0.]);
        assert_eq!(m.indices, [0, 1, 2]);
    }
    #[test]
    fn truncated_and_invalid_inputs_have_typed_errors() {
        assert_eq!(obj(b"v 0 0 0\n").unwrap_err().code, "MESH_IMPORT_EMPTY");
        assert_eq!(obj(&[255]).unwrap_err().code, "MESH_IMPORT_ENCODING");
        assert!(ply(b"ply\nformat binary_little_endian 1.0\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n").is_err());
    }
    #[test]
    fn weld_preserves_first_coordinate_and_rounds_negative_ties_like_js() {
        let m = RawMesh {
            positions: vec![-0.5, 0., 0., 0., 0., 0., 1., 0., 0., 0., 1., 0.],
            indices: vec![0, 2, 3, 1, 2, 3],
        };
        let out = finalize(m, Weld::Grid(1.)).unwrap();
        assert_eq!(out.mesh.positions.len(), 9);
        assert_eq!(out.mesh.positions[0], -0.5);
        assert_eq!(out.mesh.indices, [0, 1, 2, 0, 1, 2]);
    }
}
