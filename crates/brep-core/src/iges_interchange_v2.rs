//! Strict finite IGES 5.3 direct B-rep interchange.
//!
//! The legacy `iges-interchange/1` walking slice remains frozen.  This
//! successor parses physical 80-column records and maps the admitted IGES
//! topology graph directly; it never recognizes constructors, derives an
//! AABB, or crosses through a mesh.

use crate::analytic_features::FeatureCertificate;
use crate::{Body, Coedge, Edge, Face, FaceUse, Loop, Model, Shell, TopoId, TopoKind, TopologyIds};
use brep_topology::Vertex;
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};
use std::collections::{BTreeMap, BTreeSet};

pub const IGES_INTERCHANGE_V2_CAPABILITY: &str = "iges-interchange/2";
const MAX_BYTES: usize = 16 * 1024 * 1024;
const MAX_RECORDS: usize = 131_072;
const MAX_ENTITIES: usize = 65_536;
const MAX_VALUES: usize = 1_000_000;
const MAX_TOPOLOGY: usize = 32_768;

fn refuse(message: impl Into<String>) -> Error {
    Error::new("BREP_IGES_V2_REFUSED", message)
}

#[derive(Clone, Debug)]
pub struct IgesV2Report {
    pub identity: crate::step_interchange::StepIdentityReport,
    pub ignored_metadata: Vec<String>,
    pub entity_count: usize,
    pub topology_count: usize,
}

#[derive(Clone, Debug)]
struct Directory {
    ty: usize,
    parameter: usize,
    transform: usize,
    color: i32,
    form: usize,
    label: String,
}

#[derive(Clone, Debug)]
struct Entity {
    directory: Directory,
    values: Vec<String>,
    canonical: String,
}

fn field(line: &str, start: usize, end: usize) -> &str {
    &line[start..end]
}

fn integer(text: &str, what: &str) -> Result<i64> {
    text.trim()
        .parse()
        .map_err(|_| refuse(format!("Invalid IGES {what}")))
}

fn positive(text: &str, what: &str) -> Result<usize> {
    let value = integer(text, what)?;
    if value < 0 {
        return Err(refuse(format!("IGES {what} must be nonnegative")));
    }
    Ok(value as usize)
}

fn real(text: &str, what: &str) -> Result<f64> {
    let value: f64 = text
        .trim()
        .replace(['D', 'd'], "E")
        .parse()
        .map_err(|_| refuse(format!("Invalid IGES {what}")))?;
    if !value.is_finite() {
        return Err(refuse(format!("Non-finite IGES {what}")));
    }
    Ok(value)
}

fn tokenize(text: &str, parameter_delimiter: char, record_delimiter: char) -> Result<Vec<String>> {
    let mut out = Vec::new();
    let mut current = String::new();
    let bytes = text.as_bytes();
    let mut at = 0usize;
    while at < bytes.len() {
        let ch = text[at..]
            .chars()
            .next()
            .ok_or_else(|| refuse("Invalid IGES UTF-8"))?;
        if ch.is_ascii_digit() {
            let start = at;
            while at < bytes.len() && bytes[at].is_ascii_digit() {
                at += 1;
            }
            if at < bytes.len() && matches!(bytes[at], b'H' | b'h') {
                let count: usize = text[start..at]
                    .parse()
                    .map_err(|_| refuse("Invalid Hollerith length"))?;
                at += 1;
                if at + count > bytes.len() {
                    return Err(refuse("Truncated Hollerith string"));
                }
                current.push_str(&text[at..at + count]);
                at += count;
                continue;
            }
            current.push_str(&text[start..at]);
            continue;
        }
        at += ch.len_utf8();
        if ch == parameter_delimiter || ch == record_delimiter {
            out.push(current.trim().to_string());
            current.clear();
            if out.len() > MAX_VALUES {
                return Err(refuse("IGES aggregate value budget exceeded"));
            }
            if ch == record_delimiter {
                break;
            }
        } else {
            current.push(ch);
        }
    }
    if !current.trim().is_empty() {
        return Err(refuse("IGES parameter record lacks record delimiter"));
    }
    Ok(out)
}

fn parse_global(text: &str) -> Result<(char, char, f64)> {
    let mut parameter = ',';
    let mut record = ';';
    let provisional = tokenize(text, ',', ';')?;
    if let Some(first) = provisional.first().filter(|v| v.chars().count() == 1) {
        parameter = first.chars().next().unwrap();
    }
    if let Some(second) = provisional.get(1).filter(|v| v.chars().count() == 1) {
        record = second.chars().next().unwrap();
    }
    if parameter == record || parameter.is_ascii_alphanumeric() || record.is_ascii_alphanumeric() {
        return Err(refuse("Invalid IGES global delimiters"));
    }
    let values = tokenize(text, parameter, record)?;
    let unit_flag = values
        .get(13)
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(2);
    let scale = match unit_flag {
        1 => 25.4,
        2 => 1.,
        4 => 304.8,
        5 => 1609344.,
        6 => 1000.,
        7 => 1_000_000.,
        8 => 0.0254,
        9 => 0.001,
        10 => 10.,
        11 => 1e-3,
        _ => return Err(refuse(format!("Unsupported IGES unit flag {unit_flag}"))),
    };
    Ok((parameter, record, scale))
}

fn parse(text: &str) -> Result<(BTreeMap<usize, Entity>, f64)> {
    if text.len() > MAX_BYTES {
        return Err(refuse("IGES payload exceeds 16 MiB"));
    }
    let mut sections: BTreeMap<char, Vec<(usize, String)>> = BTreeMap::new();
    let mut last = BTreeMap::<char, usize>::new();
    for raw in text.lines() {
        let raw = raw.trim_end_matches('\r');
        if raw.len() != 80 || !raw.is_ascii() {
            return Err(refuse(
                "Every IGES physical record must be 80 ASCII columns",
            ));
        }
        let section = raw.as_bytes()[72] as char;
        if !matches!(section, 'S' | 'G' | 'D' | 'P' | 'T') {
            return Err(refuse("Unknown IGES section marker"));
        }
        let sequence = positive(field(raw, 73, 80), "record sequence")?;
        if sequence != last.get(&section).copied().unwrap_or(0) + 1 {
            return Err(refuse(format!("Non-contiguous IGES {section} sequence")));
        }
        last.insert(section, sequence);
        sections
            .entry(section)
            .or_default()
            .push((sequence, raw.to_string()));
        if last.values().sum::<usize>() > MAX_RECORDS {
            return Err(refuse("IGES physical record budget exceeded"));
        }
    }
    for required in ['S', 'G', 'D', 'P', 'T'] {
        if !sections.contains_key(&required) {
            return Err(refuse(format!("IGES missing {required} section")));
        }
    }
    if sections[&'T'].len() != 1 {
        return Err(refuse("IGES must have one terminate record"));
    }
    let order = text
        .lines()
        .map(|l| l.as_bytes()[72] as char)
        .collect::<Vec<_>>();
    if !order
        .windows(2)
        .all(|w| "SGDPT".find(w[0]).unwrap() <= "SGDPT".find(w[1]).unwrap())
    {
        return Err(refuse("IGES sections are out of order"));
    }
    let global = sections[&'G']
        .iter()
        .map(|(_, l)| field(l, 0, 72))
        .collect::<String>();
    let (pd, rd, scale) = parse_global(&global)?;
    let directory = &sections[&'D'];
    if directory.len() % 2 != 0 {
        return Err(refuse("IGES directory section must contain record pairs"));
    }
    let mut directories = BTreeMap::new();
    for pair in directory.chunks_exact(2) {
        let a = &pair[0].1;
        let b = &pair[1].1;
        let de = pair[0].0;
        if de % 2 == 0 || pair[1].0 != de + 1 {
            return Err(refuse("Malformed IGES directory pair"));
        }
        let ty = positive(field(a, 0, 8), "entity type")?;
        if positive(field(b, 0, 8), "repeated entity type")? != ty {
            return Err(refuse("IGES directory type mismatch"));
        }
        directories.insert(
            de,
            Directory {
                ty,
                parameter: positive(field(a, 8, 16), "parameter pointer")?,
                transform: positive(field(a, 48, 56), "transformation pointer")?,
                color: integer(field(b, 16, 24), "color")? as i32,
                form: positive(field(b, 32, 40), "form")?,
                label: field(b, 56, 64).trim().to_string(),
            },
        );
        if directories.len() > MAX_ENTITIES {
            return Err(refuse("IGES entity count exceeds 65536"));
        }
    }
    let mut parameter_rows = BTreeMap::new();
    for (sequence, row) in &sections[&'P'] {
        let owner = positive(field(row, 64, 72), "parameter owner")?;
        parameter_rows.insert(*sequence, (owner, field(row, 0, 64).to_string()));
    }
    let mut entities = BTreeMap::new();
    for (de, dir) in directories {
        let mut sequence = dir.parameter;
        let mut parameter_text = String::new();
        loop {
            let (owner, payload) = parameter_rows
                .get(&sequence)
                .ok_or_else(|| refuse(format!("Missing parameter row {sequence}")))?;
            if *owner != de {
                return Err(refuse(
                    "IGES parameter owner does not match directory entry",
                ));
            }
            parameter_text.push_str(payload);
            if payload.contains(rd) {
                break;
            }
            sequence += 1;
            if sequence - dir.parameter > 4096 {
                return Err(refuse("IGES entity parameter span exceeds 4096 records"));
            }
        }
        let values = tokenize(&parameter_text, pd, rd)?;
        if values.first().and_then(|v| v.parse::<usize>().ok()) != Some(dir.ty) {
            return Err(refuse("IGES parameter entity type mismatch"));
        }
        let canonical = values
            .iter()
            .map(|v| v.trim())
            .collect::<Vec<_>>()
            .join(",");
        entities.insert(
            de,
            Entity {
                directory: dir,
                values,
                canonical,
            },
        );
    }
    Ok((entities, scale))
}

fn entity<'a>(entities: &'a BTreeMap<usize, Entity>, de: usize, ty: usize) -> Result<&'a Entity> {
    let e = entities
        .get(&de)
        .ok_or_else(|| refuse(format!("Missing IGES directory entry {de}")))?;
    if e.directory.ty != ty {
        return Err(refuse(format!(
            "Expected IGES {ty}, found {}",
            e.directory.ty
        )));
    }
    Ok(e)
}
fn u(v: &[String], at: usize, what: &str) -> Result<usize> {
    positive(
        v.get(at).ok_or_else(|| refuse(format!("Missing {what}")))?,
        what,
    )
}
fn f(v: &[String], at: usize, what: &str) -> Result<f64> {
    real(
        v.get(at).ok_or_else(|| refuse(format!("Missing {what}")))?,
        what,
    )
}

fn curve126(
    entities: &BTreeMap<usize, Entity>,
    de: usize,
    dim: usize,
    scale: f64,
) -> Result<Curve> {
    let v = &entity(entities, de, 126)?.values;
    let k = u(v, 1, "curve upper index")?;
    let degree = u(v, 2, "curve degree")?;
    let n = k + 1;
    let knot_count = n + degree + 1;
    if n == 0 || n > 4096 || v.len() < 8 + knot_count + n + 3 * n {
        return Err(refuse("Malformed IGES rational B-spline curve"));
    }
    let mut knots = (0..knot_count)
        .map(|i| f(v, 7 + i, "curve knot"))
        .collect::<Result<Vec<_>>>()?;
    let weights = (0..n)
        .map(|i| f(v, 7 + knot_count + i, "curve weight"))
        .collect::<Result<Vec<_>>>()?;
    let cp_at = 7 + knot_count + n;
    let mut cps = Vec::with_capacity(n);
    for i in 0..n {
        let mut p = vec![
            f(v, cp_at + 3 * i, "curve X")?,
            f(v, cp_at + 3 * i + 1, "curve Y")?,
        ];
        if dim == 3 {
            p.push(f(v, cp_at + 3 * i + 2, "curve Z")?);
            for x in &mut p {
                *x *= scale;
            }
        }
        cps.push(p);
    }
    let domain_at = cp_at + 3 * n;
    let a = f(v, domain_at, "curve domain start")?;
    let b = f(v, domain_at + 1, "curve domain end")?;
    if !(a < b) {
        return Err(refuse("IGES curve domain is not increasing"));
    }
    let old = [knots[degree], knots[knots.len() - degree - 1]];
    if (old[0] - a).abs() > 1e-10 || (old[1] - b).abs() > 1e-10 {
        let factor = (b - a) / (old[1] - old[0]);
        for x in &mut knots {
            *x = a + (*x - old[0]) * factor;
        }
    }
    let c = Curve {
        degree,
        knots,
        control_points: cps,
        weights,
        periodic: u(v, 5, "curve periodic flag")? != 0,
    };
    c.validate()?;
    Ok(c)
}

fn surface128(entities: &BTreeMap<usize, Entity>, de: usize, scale: f64) -> Result<Surface> {
    let v = &entity(entities, de, 128)?.values;
    let ku = u(v, 1, "surface U upper index")?;
    let kv = u(v, 2, "surface V upper index")?;
    let du = u(v, 3, "surface U degree")?;
    let dv = u(v, 4, "surface V degree")?;
    let nu = ku + 1;
    let nv = kv + 1;
    let n = nu
        .checked_mul(nv)
        .ok_or_else(|| refuse("Surface size overflow"))?;
    if n == 0 || n > 16384 {
        return Err(refuse("IGES surface control net exceeds 16384"));
    }
    let ku_count = nu + du + 1;
    let kv_count = nv + dv + 1;
    let mut at = 10;
    let knots_u = (0..ku_count)
        .map(|i| f(v, at + i, "surface U knot"))
        .collect::<Result<Vec<_>>>()?;
    at += ku_count;
    let knots_v = (0..kv_count)
        .map(|i| f(v, at + i, "surface V knot"))
        .collect::<Result<Vec<_>>>()?;
    at += kv_count;
    let flat_w = (0..n)
        .map(|i| f(v, at + i, "surface weight"))
        .collect::<Result<Vec<_>>>()?;
    at += n;
    let mut flat_p = Vec::with_capacity(n);
    for i in 0..n {
        flat_p.push(vec![
            f(v, at + 3 * i, "surface X")? * scale,
            f(v, at + 3 * i + 1, "surface Y")? * scale,
            f(v, at + 3 * i + 2, "surface Z")? * scale,
        ]);
    }
    let control_points = flat_p.chunks(nv).map(|r| r.to_vec()).collect();
    let weights = flat_w.chunks(nv).map(|r| r.to_vec()).collect();
    let s = Surface {
        degree_u: du,
        degree_v: dv,
        knots_u,
        knots_v,
        control_points,
        weights,
        periodic_u: u(v, 7, "surface U periodic flag")? != 0,
        periodic_v: u(v, 8, "surface V periodic flag")? != 0,
    };
    s.validate()?;
    Ok(s)
}

fn topology_digest(entity: &Entity) -> String {
    let mut h = 0xcbf29ce484222325u64;
    for b in entity.canonical.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}
fn verify_correspondence(
    edge: &Edge,
    pcurve: &Curve,
    surface: &Surface,
    reversed: bool,
) -> Result<()> {
    let d3 = edge.curve.domain();
    let d2 = pcurve.domain();
    for sample in 0..=8 {
        let q = sample as f64 / 8.;
        let t3 = if reversed {
            d3[1] - q * (d3[1] - d3[0])
        } else {
            d3[0] + q * (d3[1] - d3[0])
        };
        let uv = pcurve.evaluate(d2[0] + q * (d2[1] - d2[0]))?.point;
        if uv.len() != 2 {
            return Err(refuse("IGES pcurve is not two-dimensional"));
        }
        let a = edge.curve.evaluate(t3)?.point;
        let b = surface.evaluate(uv[0], uv[1])?.point;
        let error = (a[0] - b[0]).hypot(a[1] - b[1]).hypot(a[2] - b[2]);
        if !error.is_finite() || error > 1e-5 {
            return Err(refuse(format!(
                "Independent IGES curve-pcurve correspondence failed ({error:.3e})"
            )));
        }
    }
    Ok(())
}

struct Builder<'a> {
    e: &'a BTreeMap<usize, Entity>,
    scale: f64,
    vertices: Vec<Vertex>,
    edges: Vec<Edge>,
    loops: Vec<Loop>,
    faces: Vec<Face>,
    shells: Vec<Shell>,
    bodies: Vec<Body>,
    vertex_de: Vec<usize>,
    edge_de: Vec<usize>,
    loop_de: Vec<usize>,
    face_de: Vec<usize>,
    shell_de: Vec<usize>,
    body_de: Vec<usize>,
}
impl<'a> Builder<'a> {
    fn vertex_list(&mut self, de: usize) -> Result<Vec<usize>> {
        let v = &entity(self.e, de, 502)?.values;
        let n = u(v, 1, "vertex count")?;
        if n > MAX_TOPOLOGY || v.len() != 2 + 3 * n {
            return Err(refuse("Malformed IGES vertex list"));
        }
        let mut out = Vec::new();
        for i in 0..n {
            let index = self.vertices.len();
            self.vertices.push(Vertex {
                point: [
                    f(v, 2 + 3 * i, "vertex X")? * self.scale,
                    f(v, 3 + 3 * i, "vertex Y")? * self.scale,
                    f(v, 4 + 3 * i, "vertex Z")? * self.scale,
                ],
            });
            self.vertex_de.push(de);
            out.push(index);
        }
        Ok(out)
    }
    fn build(mut self) -> Result<(Model, Vec<(TopoKind, usize, usize)>)> {
        let mut vertex_lists = BTreeMap::new();
        for (&de, e) in self.e {
            if e.directory.ty == 502 {
                vertex_lists.insert(de, self.vertex_list(de)?);
            }
        }
        let mut edge_lists = BTreeMap::<(usize, usize), usize>::new();
        for (&de, e) in self.e {
            if e.directory.ty != 504 {
                continue;
            }
            let v = &e.values;
            let n = u(v, 1, "edge count")?;
            if v.len() != 2 + 5 * n {
                return Err(refuse("Malformed IGES edge list"));
            }
            for i in 0..n {
                let at = 2 + 5 * i;
                let curve = curve126(self.e, u(v, at, "edge curve")?, 3, self.scale)?;
                let vl0 = u(v, at + 1, "start vertex list")?;
                let vi0 = u(v, at + 2, "start vertex index")?;
                let vl1 = u(v, at + 3, "end vertex list")?;
                let vi1 = u(v, at + 4, "end vertex index")?;
                let a = *vertex_lists
                    .get(&vl0)
                    .and_then(|x| x.get(vi0.checked_sub(1)?))
                    .ok_or_else(|| refuse("Invalid IGES start vertex index"))?;
                let b = *vertex_lists
                    .get(&vl1)
                    .and_then(|x| x.get(vi1.checked_sub(1)?))
                    .ok_or_else(|| refuse("Invalid IGES end vertex index"))?;
                let index = self.edges.len();
                self.edges.push(Edge {
                    degenerate: a == b,
                    vertices: [a, b],
                    curve,
                });
                self.edge_de.push(de);
                edge_lists.insert((de, i + 1), index);
            }
        }
        let mut loop_map = BTreeMap::new();
        for (&de, e) in self.e {
            if e.directory.ty != 508 {
                continue;
            }
            let v = &e.values;
            let n = u(v, 1, "loop member count")?;
            let mut at = 2;
            let mut coedges = Vec::new();
            for _ in 0..n {
                if u(v, at, "loop member type")? != 0 {
                    return Err(refuse("Vertex-only IGES loop member is unsupported"));
                }
                let el = u(v, at + 1, "loop edge list")?;
                let ei = u(v, at + 2, "loop edge index")?;
                let reversed = u(v, at + 3, "loop orientation")? == 0;
                let pcount = u(v, at + 4, "loop pcurve count")?;
                if pcount != 1 {
                    return Err(refuse("Each IGES loop edge requires exactly one pcurve"));
                }
                let iso = u(v, at + 5, "loop pcurve isoparametric")?;
                if iso > 1 {
                    return Err(refuse("Invalid IGES isoparametric flag"));
                }
                let pcurve = curve126(self.e, u(v, at + 6, "loop pcurve")?, 2, 1.)?;
                let edge = *edge_lists
                    .get(&(el, ei))
                    .ok_or_else(|| refuse("Invalid IGES edge-list reference"))?;
                coedges.push(Coedge {
                    edge,
                    reversed,
                    pcurve,
                });
                at += 7;
            }
            if at != v.len() {
                return Err(refuse("Trailing IGES loop parameters"));
            }
            let index = self.loops.len();
            self.loops.push(Loop { coedges });
            self.loop_de.push(de);
            loop_map.insert(de, index);
        }
        let mut face_map = BTreeMap::new();
        for (&de, e) in self.e {
            if e.directory.ty != 510 {
                continue;
            }
            let v = &e.values;
            let surface = surface128(self.e, u(v, 1, "face surface")?, self.scale)?;
            let n = u(v, 2, "face loop count")?;
            if n == 0 || v.len() != 4 + n {
                return Err(refuse("Malformed IGES face"));
            }
            let loops = (0..n)
                .map(|i| {
                    loop_map
                        .get(&u(v, 4 + i, "face loop")?)
                        .copied()
                        .ok_or_else(|| refuse("Missing IGES face loop"))
                })
                .collect::<Result<Vec<_>>>()?;
            for loop_index in &loops {
                for coedge in &self.loops[*loop_index].coedges {
                    verify_correspondence(
                        &self.edges[coedge.edge],
                        &coedge.pcurve,
                        &surface,
                        coedge.reversed,
                    )?;
                }
            }
            let index = self.faces.len();
            self.faces.push(Face {
                surface,
                outer: loops[0],
                holes: loops[1..].to_vec(),
            });
            self.face_de.push(de);
            face_map.insert(de, index);
        }
        let mut shell_map = BTreeMap::new();
        for (&de, e) in self.e {
            if e.directory.ty != 514 {
                continue;
            }
            let v = &e.values;
            let n = u(v, 1, "shell face count")?;
            if n == 0 || v.len() != 2 + 2 * n {
                return Err(refuse("Malformed IGES shell"));
            }
            let faces = (0..n)
                .map(|i| {
                    Ok(FaceUse {
                        face: *face_map
                            .get(&u(v, 2 + 2 * i, "shell face")?)
                            .ok_or_else(|| refuse("Missing IGES shell face"))?,
                        reversed: u(v, 3 + 2 * i, "shell orientation")? == 0,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let index = self.shells.len();
            self.shells.push(Shell {
                faces,
                closed: true,
            });
            self.shell_de.push(de);
            shell_map.insert(de, index);
        }
        for (&de, e) in self.e {
            if e.directory.ty != 186 {
                continue;
            }
            let v = &e.values;
            let outer = *shell_map
                .get(&u(v, 1, "solid outer shell")?)
                .ok_or_else(|| refuse("Missing IGES outer shell"))?;
            let n = u(v, 3, "solid void count")?;
            if v.len() != 4 + 2 * n {
                return Err(refuse("Malformed IGES manifold solid B-rep"));
            }
            let inners = (0..n)
                .map(|i| {
                    shell_map
                        .get(&u(v, 4 + 2 * i, "void shell")?)
                        .copied()
                        .ok_or_else(|| refuse("Missing IGES void shell"))
                })
                .collect::<Result<Vec<_>>>()?;
            self.bodies.push(Body {
                outer_shell: outer,
                inner_shells: inners,
            });
            self.body_de.push(de);
        }
        if self.bodies.is_empty() {
            return Err(refuse("IGES contains no manifold solid B-rep (186)"));
        }
        let count = self.vertices.len()
            + self.edges.len()
            + self.loops.len()
            + self.faces.len()
            + self.shells.len()
            + self.bodies.len();
        if count > MAX_TOPOLOGY {
            return Err(refuse("IGES topology budget exceeds 32768 entities"));
        }
        let mut model = Model(
            brep_topology::Model {
                vertices: self.vertices,
                edges: self.edges,
                loops: self.loops,
                faces: self.faces,
                shells: self.shells,
                bodies: self.bodies,
                tolerance_mm: 1e-7,
            },
            TopologyIds::default(),
        );
        model.rebuild_topology_ids();
        model.validate()?;
        let mut map = Vec::new();
        for (kind, des) in [
            (TopoKind::Vertex, &self.vertex_de),
            (TopoKind::Edge, &self.edge_de),
            (TopoKind::Loop, &self.loop_de),
            (TopoKind::Face, &self.face_de),
            (TopoKind::Shell, &self.shell_de),
            (TopoKind::Body, &self.body_de),
        ] {
            map.extend(des.iter().enumerate().map(|(i, de)| (kind, i, *de)));
        }
        Ok((model, map))
    }
}

fn restore_identity(
    entities: &BTreeMap<usize, Entity>,
    map: &[(TopoKind, usize, usize)],
    model: &mut Model,
) -> Result<crate::step_interchange::StepIdentityReport> {
    let mut metadata = BTreeMap::new();
    for e in entities
        .values()
        .filter(|e| e.directory.ty == 406 && e.directory.form == 15)
    {
        if e.values.len() != 4 {
            continue;
        }
        let Some(payload) = e.values[2].strip_prefix("OSCAD_TOPO/IGES2|") else {
            continue;
        };
        let p = payload.split('|').collect::<Vec<_>>();
        if p.len() != 4 {
            return Err(refuse("Malformed IGES identity property"));
        }
        let kind = TopoKind::parse(p[0]).ok_or_else(|| refuse("Unknown IGES identity kind"))?;
        let id = TopoId::parse(p[1]).map_err(refuse)?;
        let index = p[3]
            .parse::<usize>()
            .map_err(|_| refuse("Invalid IGES identity index"))?;
        let target = u(&e.values, 3, "identity target")?;
        let target_entity = entities
            .get(&target)
            .ok_or_else(|| refuse("Missing IGES identity target"))?;
        if id.kind() != kind || p[2] != topology_digest(target_entity) {
            return Err(refuse("Transplanted IGES identity metadata"));
        }
        if metadata.insert((kind, target, index), id).is_some() {
            return Err(refuse("Duplicate IGES identity property"));
        }
    }
    let count = map.len();
    if metadata.is_empty() {
        return Ok(crate::step_interchange::StepIdentityReport {
            preserved: false,
            source: "external-iges",
            preserved_count: 0,
            created_count: count,
            lost_count: count,
        });
    }
    if metadata.len() != count {
        return Err(refuse("Partial IGES identity metadata"));
    }
    let mut seen = BTreeSet::new();
    let mut remap = BTreeMap::new();
    let mut ordinal = BTreeMap::<(TopoKind, usize), usize>::new();
    for &(kind, index, de) in map {
        let occurrence = ordinal.entry((kind, de)).or_insert(0);
        let id = *metadata
            .get(&(kind, de, *occurrence))
            .ok_or_else(|| refuse("IGES identity map is not topology-bijective"))?;
        *occurrence += 1;
        if !seen.insert(id) {
            return Err(refuse("Duplicate IGES TopoId"));
        }
        let old = match kind {
            TopoKind::Vertex => model.1.vertices[index],
            TopoKind::Edge => model.1.edges[index],
            TopoKind::Loop => model.1.loops[index],
            TopoKind::Face => model.1.faces[index],
            TopoKind::Shell => model.1.shells[index],
            TopoKind::Body => model.1.bodies[index],
            TopoKind::ControlPoint => unreachable!(),
        };
        remap.insert(old, id);
        match kind {
            TopoKind::Vertex => model.1.vertices[index] = id,
            TopoKind::Edge => model.1.edges[index] = id,
            TopoKind::Loop => model.1.loops[index] = id,
            TopoKind::Face => model.1.faces[index] = id,
            TopoKind::Shell => model.1.shells[index] = id,
            TopoKind::Body => model.1.bodies[index] = id,
            TopoKind::ControlPoint => unreachable!(),
        }
    }
    model.1.change_set.nodes = model
        .1
        .change_set
        .nodes
        .iter()
        .map(|(id, kind)| (remap.get(id).copied().unwrap_or(*id), *kind))
        .collect();
    for change in &mut model.1.change_set.changes {
        for id in change.parents.iter_mut().chain(&mut change.children) {
            if let Some(next) = remap.get(id) {
                *id = *next
            }
        }
    }
    for relation in &mut model.1.lineage {
        for id in relation.parents.iter_mut().chain(&mut relation.children) {
            if let Some(next) = remap.get(id) {
                *id = *next
            }
        }
    }
    Ok(crate::step_interchange::StepIdentityReport {
        preserved: true,
        source: "internal-metadata",
        preserved_count: count,
        created_count: 0,
        lost_count: 0,
    })
}

pub fn import_iges_v2(text: &str) -> Result<(Model, FeatureCertificate, IgesV2Report)> {
    let (entities, scale) = parse(text)?;
    const GEOMETRIC: &[usize] = &[124, 126, 128, 141, 142, 143, 186, 502, 504, 508, 510, 514];
    const METADATA: &[usize] = &[314, 402, 406];
    let mut ignored = BTreeSet::new();
    for e in entities.values() {
        if !GEOMETRIC.contains(&e.directory.ty) && !METADATA.contains(&e.directory.ty) {
            return Err(refuse(format!(
                "IGES entity {} is outside the finite direct subset",
                e.directory.ty
            )));
        }
        if METADATA.contains(&e.directory.ty) {
            ignored.insert(format!("entity-{}", e.directory.ty));
        }
        if e.directory.color != 0 {
            ignored.insert("directory-color".into());
        }
        if !e.directory.label.is_empty() {
            ignored.insert("directory-label".into());
        }
        if e.directory.transform != 0 {
            return Err(refuse(
                "Per-entity IGES transformations are typed-refused; bake one rigid placement into geometry",
            ));
        }
    }
    let builder = Builder {
        e: &entities,
        scale,
        vertices: vec![],
        edges: vec![],
        loops: vec![],
        faces: vec![],
        shells: vec![],
        bodies: vec![],
        vertex_de: vec![],
        edge_de: vec![],
        loop_de: vec![],
        face_de: vec![],
        shell_de: vec![],
        body_de: vec![],
    };
    let (mut model, map) = builder.build()?;
    let identity = restore_identity(&entities, &map, &mut model)?;
    let topology_count = map.len();
    Ok((
        model,
        FeatureCertificate {
            capability: IGES_INTERCHANGE_V2_CAPABILITY,
            complete: true,
            notes: vec![
                "strict_80_column_sgdp_t",
                "direct_126_128_502_504_508_510_514_186",
                "shared_topology_and_pcurves",
                "multi_body_and_cavity",
                "no_aabb_constructor_or_mesh",
            ],
        },
        IgesV2Report {
            identity,
            ignored_metadata: ignored.into_iter().collect(),
            entity_count: entities.len(),
            topology_count,
        },
    ))
}

#[derive(Clone)]
struct OutEntity {
    ty: usize,
    form: usize,
    label: String,
    color: i32,
    values: Vec<String>,
}
struct Writer {
    entities: Vec<OutEntity>,
}
impl Writer {
    fn new() -> Self {
        Self { entities: vec![] }
    }
    fn emit(&mut self, ty: usize, values: Vec<String>) -> usize {
        self.entities.push(OutEntity {
            ty,
            form: 0,
            label: String::new(),
            color: 0,
            values,
        });
        2 * self.entities.len() - 1
    }
}
fn number(x: f64) -> String {
    format!("{x:.17e}")
}
fn curve_values(c: &Curve, dim: usize) -> Vec<String> {
    let n = c.control_points.len();
    let mut v = vec![
        "126".into(),
        (n - 1).to_string(),
        c.degree.to_string(),
        "0".into(),
        ((c.degree + 1 == n) as usize).to_string(),
        (c.periodic as usize).to_string(),
        "0".into(),
    ];
    v.extend(c.knots.iter().map(|x| number(*x)));
    v.extend(c.weights.iter().map(|x| number(*x)));
    for p in &c.control_points {
        v.push(number(p[0]));
        v.push(number(p[1]));
        v.push(number(if dim == 3 { p[2] } else { 0. }));
    }
    let d = c.domain();
    v.extend([
        number(d[0]),
        number(d[1]),
        "0".into(),
        "0".into(),
        "1".into(),
    ]);
    v
}
fn surface_values(s: &Surface) -> Vec<String> {
    let nu = s.control_points.len();
    let nv = s.control_points[0].len();
    let mut v = vec![
        "128".into(),
        (nu - 1).to_string(),
        (nv - 1).to_string(),
        s.degree_u.to_string(),
        s.degree_v.to_string(),
        "0".into(),
        "0".into(),
        (s.periodic_u as usize).to_string(),
        (s.periodic_v as usize).to_string(),
        "0".into(),
    ];
    v.extend(s.knots_u.iter().map(|x| number(*x)));
    v.extend(s.knots_v.iter().map(|x| number(*x)));
    v.extend(s.weights.iter().flatten().map(|x| number(*x)));
    for p in s.control_points.iter().flatten() {
        v.extend(p.iter().map(|x| number(*x)))
    }
    let u = [
        s.knots_u[s.degree_u],
        s.knots_u[s.knots_u.len() - s.degree_u - 1],
    ];
    let q = [
        s.knots_v[s.degree_v],
        s.knots_v[s.knots_v.len() - s.degree_v - 1],
    ];
    v.extend([number(u[0]), number(u[1]), number(q[0]), number(q[1])]);
    v
}
fn canonical(values: &[String]) -> String {
    values
        .iter()
        .map(|v| v.trim())
        .collect::<Vec<_>>()
        .join(",")
}
fn digest_values(values: &[String]) -> String {
    let mut h = 0xcbf29ce484222325u64;
    for b in canonical(values).bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

fn pad_record(payload: &str, section: char, sequence: usize, width: usize) -> Result<String> {
    if payload.len() > width {
        return Err(refuse("IGES physical payload overflow"));
    }
    Ok(format!(
        "{payload:<width$}{section}{sequence:>7}",
        width = width
    ))
}
fn hollerith(s: &str) -> String {
    format!("{}H{s}", s.len())
}
fn render(mut w: Writer) -> Result<String> {
    let mut p_lines = Vec::new();
    let mut p_start = Vec::new();
    for (index, e) in w.entities.iter().enumerate() {
        let de = 2 * index + 1;
        let text = format!("{};", e.values.join(","));
        p_start.push(p_lines.len() + 1);
        for chunk in text.as_bytes().chunks(64) {
            let payload =
                std::str::from_utf8(chunk).map_err(|_| refuse("Non-ASCII IGES parameter"))?;
            p_lines.push(
                pad_record(payload, 'P', p_lines.len() + 1, 64)?[..64].to_string()
                    + &format!("{de:>8}P{:>7}", p_lines.len() + 1),
            );
        }
    }
    let mut lines = vec![pad_record(
        "OpenSCAD Viewer direct finite B-rep",
        'S',
        1,
        72,
    )?];
    let global = format!(
        "1H,,1H;,{} ,{} ,{} ,{} ,{} ,{},32,38,6,308,15,{},1.0,2,2HMM,1,0.001,{},{} ,11,0,{} ,{};",
        hollerith("OpenSCAD Viewer"),
        hollerith("open-scad-viewer"),
        hollerith("OpenSCAD Viewer"),
        hollerith("open-scad-viewer"),
        hollerith(""),
        hollerith(""),
        hollerith("IGES2"),
        number(1e-7),
        hollerith("20260917.000000"),
        number(1e-7),
        hollerith("20260917.000000")
    );
    for (i, chunk) in global.as_bytes().chunks(72).enumerate() {
        lines.push(pad_record(
            std::str::from_utf8(chunk).unwrap(),
            'G',
            i + 1,
            72,
        )?);
    }
    for (index, e) in w.entities.iter_mut().enumerate() {
        let de = 2 * index + 1;
        let a = format!(
            "{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}",
            e.ty, p_start[index], 0, 0, 0, 0, 0, 0, "00000000"
        );
        let b = format!(
            "{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:<8}{:>8}",
            e.ty, 0, e.color, 0, e.form, 0, 0, e.label, 0
        );
        lines.push(pad_record(&a, 'D', de, 72)?);
        lines.push(pad_record(&b, 'D', de + 1, 72)?);
    }
    for p in &p_lines {
        lines.push(p.clone());
    }
    let terminate = format!(
        "S{:>7}G{:>7}D{:>7}P{:>7}",
        1,
        lines.iter().filter(|l| l.as_bytes()[72] == b'G').count(),
        2 * w.entities.len(),
        p_lines.len()
    );
    lines.push(pad_record(&terminate, 'T', 1, 72)?);
    let text = lines.join("\n") + "\n";
    if text.len() > MAX_BYTES {
        return Err(refuse("IGES output exceeds 16 MiB"));
    }
    Ok(text)
}

pub fn export_iges_v2(model: &Model) -> Result<(String, FeatureCertificate, IgesV2Report)> {
    model.validate()?;
    if model.bodies.is_empty() || model.shells.iter().any(|s| !s.closed) {
        return Err(refuse("IGES /2 exports closed manifold bodies only"));
    }
    let mut w = Writer::new();
    let surfaces = model
        .faces
        .iter()
        .map(|face| w.emit(128, surface_values(&face.surface)))
        .collect::<Vec<_>>();
    let curves = model
        .edges
        .iter()
        .map(|edge| w.emit(126, curve_values(&edge.curve, 3)))
        .collect::<Vec<_>>();
    let mut vertex_values = vec!["502".into(), model.vertices.len().to_string()];
    for vertex in &model.vertices {
        vertex_values.extend(vertex.point.iter().map(|x| number(*x)))
    }
    let vertex_list = w.emit(502, vertex_values);
    let mut edge_values = vec!["504".into(), model.edges.len().to_string()];
    for (i, e) in model.edges.iter().enumerate() {
        edge_values.extend([
            curves[i].to_string(),
            vertex_list.to_string(),
            (e.vertices[0] + 1).to_string(),
            vertex_list.to_string(),
            (e.vertices[1] + 1).to_string(),
        ])
    }
    let edge_list = w.emit(504, edge_values);
    let mut loop_des = Vec::new();
    for l in &model.loops {
        let mut values = vec!["508".into(), l.coedges.len().to_string()];
        for c in &l.coedges {
            let pc = w.emit(126, curve_values(&c.pcurve, 2));
            values.extend([
                "0".into(),
                edge_list.to_string(),
                (c.edge + 1).to_string(),
                ((!c.reversed) as usize).to_string(),
                "1".into(),
                "0".into(),
                pc.to_string(),
            ]);
        }
        loop_des.push(w.emit(508, values));
    }
    let mut face_des = Vec::new();
    for (i, face) in model.faces.iter().enumerate() {
        let loops = std::iter::once(face.outer)
            .chain(face.holes.iter().copied())
            .collect::<Vec<_>>();
        let mut values = vec![
            "510".into(),
            surfaces[i].to_string(),
            loops.len().to_string(),
            "1".into(),
        ];
        values.extend(loops.iter().map(|i| loop_des[*i].to_string()));
        face_des.push(w.emit(510, values));
    }
    let mut shell_des = Vec::new();
    for shell in &model.shells {
        let mut values = vec!["514".into(), shell.faces.len().to_string()];
        for f in &shell.faces {
            values.extend([
                face_des[f.face].to_string(),
                ((!f.reversed) as usize).to_string(),
            ])
        }
        shell_des.push(w.emit(514, values));
    }
    let mut body_des = Vec::new();
    for body in &model.bodies {
        let mut values = vec![
            "186".into(),
            shell_des[body.outer_shell].to_string(),
            "1".into(),
            body.inner_shells.len().to_string(),
        ];
        for s in &body.inner_shells {
            values.extend([shell_des[*s].to_string(), "0".into()])
        }
        body_des.push(w.emit(186, values));
    }
    let groups = [
        (
            TopoKind::Vertex,
            model.1.vertices.as_slice(),
            vec![vertex_list; model.vertices.len()],
        ),
        (
            TopoKind::Edge,
            model.1.edges.as_slice(),
            vec![edge_list; model.edges.len()],
        ),
        (TopoKind::Loop, model.1.loops.as_slice(), loop_des.clone()),
        (TopoKind::Face, model.1.faces.as_slice(), face_des.clone()),
        (
            TopoKind::Shell,
            model.1.shells.as_slice(),
            shell_des.clone(),
        ),
        (TopoKind::Body, model.1.bodies.as_slice(), body_des.clone()),
    ];
    for (kind, ids, targets) in groups {
        let mut ordinal = BTreeMap::<usize, usize>::new();
        for (&id, target) in ids.iter().zip(targets) {
            let occurrence = ordinal.entry(target).or_insert(0);
            let values = &w.entities[(target - 1) / 2].values;
            let payload = format!(
                "OSCAD_TOPO/IGES2|{}|{}|{}|{}",
                kind.as_str(),
                id,
                digest_values(values),
                *occurrence
            );
            *occurrence += 1;
            let de = w.emit(
                406,
                vec![
                    "406".into(),
                    "15".into(),
                    hollerith(&payload),
                    target.to_string(),
                ],
            );
            w.entities[(de - 1) / 2].form = 15;
        }
    }
    let text = render(w)?;
    let entity_count = parse(&text)?.0.len();
    let topology_count = model.vertices.len()
        + model.edges.len()
        + model.loops.len()
        + model.faces.len()
        + model.shells.len()
        + model.bodies.len();
    Ok((
        text,
        FeatureCertificate {
            capability: IGES_INTERCHANGE_V2_CAPABILITY,
            complete: true,
            notes: vec![
                "direct_model_export",
                "rational_126_128",
                "manifold_186_topology",
                "graph_bound_topoid_properties",
                "no_fallback",
            ],
        },
        IgesV2Report {
            identity: crate::step_interchange::StepIdentityReport {
                preserved: true,
                source: "internal-metadata",
                preserved_count: topology_count,
                created_count: 0,
                lost_count: 0,
            },
            ignored_metadata: vec![],
            entity_count,
            topology_count,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nurbs_step_solid::freeform_cuboid_solid;

    fn append(target: &mut Model, source: Model) {
        let Model(source, _) = source;
        let (vo, eo, lo, fo, so) = (
            target.vertices.len(),
            target.edges.len(),
            target.loops.len(),
            target.faces.len(),
            target.shells.len(),
        );
        target.0.vertices.extend(source.vertices);
        target.0.edges.extend(source.edges.into_iter().map(|mut e| {
            e.vertices = [e.vertices[0] + vo, e.vertices[1] + vo];
            e
        }));
        target.0.loops.extend(source.loops.into_iter().map(|mut l| {
            for c in &mut l.coedges {
                c.edge += eo
            }
            l
        }));
        target.0.faces.extend(source.faces.into_iter().map(|mut f| {
            f.outer += lo;
            for h in &mut f.holes {
                *h += lo
            }
            f
        }));
        target
            .0
            .shells
            .extend(source.shells.into_iter().map(|mut s| {
                for f in &mut s.faces {
                    f.face += fo
                }
                s
            }));
        target
            .0
            .bodies
            .extend(source.bodies.into_iter().map(|mut b| {
                b.outer_shell += so;
                for s in &mut b.inner_shells {
                    *s += so
                }
                b
            }));
    }

    #[test]
    fn direct_roundtrip_is_80_column_and_preserves_graph_identity() {
        let model = freeform_cuboid_solid([0., 0., 0.], [2., 3., 4.]).unwrap();
        let (text, _, report) = export_iges_v2(&model).unwrap();
        assert!(text.lines().all(|line| line.len() == 80));
        assert!(
            text.contains("     126") && text.contains("     128") && text.contains("     186")
        );
        let (back, cert, identity) = import_iges_v2(&text).unwrap();
        assert_eq!(cert.capability, IGES_INTERCHANGE_V2_CAPABILITY);
        assert!(identity.identity.preserved);
        assert_eq!(
            (
                back.vertices.len(),
                back.edges.len(),
                back.faces.len(),
                back.bodies.len()
            ),
            (8, 12, 6, 1)
        );
        assert_eq!(back.1.faces, model.1.faces);
        assert_eq!(report.topology_count, 34);
    }

    #[test]
    fn metadata_free_reports_loss_and_mutation_refuses() {
        let model = freeform_cuboid_solid([0., 0., 0.], [1., 1., 1.]).unwrap();
        let (text, _, _) = export_iges_v2(&model).unwrap();
        let mut lines = text.lines().map(str::to_string).collect::<Vec<_>>();
        for line in &mut lines {
            if line.as_bytes()[72] == b'D' && line[..8].trim() == "406" {
                line.replace_range(0..8, "     314");
            }
        }
        let stripped = lines.join("\n") + "\n";
        assert!(import_iges_v2(&stripped).is_err());
        let mutated = text.replacen("0.00000000000000000e0", "1.00000000000000000e-1", 1);
        assert!(import_iges_v2(&mutated).is_err());
    }

    #[test]
    fn malformed_records_units_unknown_and_resources_refuse() {
        assert!(parse("not iges").is_err());
        let model = freeform_cuboid_solid([0., 0., 0.], [1., 1., 1.]).unwrap();
        let (text, _, _) = export_iges_v2(&model).unwrap();
        assert!(import_iges_v2(&text.replacen("     128", "     999", 1)).is_err());
        assert!(parse(&" ".repeat(MAX_BYTES + 1)).is_err());
    }

    #[test]
    fn multiple_bodies_and_cavities_roundtrip_without_topology_reconstruction() {
        let mut model = Model::empty(1e-7).unwrap();
        append(
            &mut model,
            freeform_cuboid_solid([0., 0., 0.], [10., 10., 10.]).unwrap(),
        );
        append(
            &mut model,
            freeform_cuboid_solid([2., 2., 2.], [4., 4., 4.]).unwrap(),
        );
        append(
            &mut model,
            freeform_cuboid_solid([20., 0., 0.], [21., 1., 1.]).unwrap(),
        );
        model.0.bodies = vec![
            Body {
                outer_shell: 0,
                inner_shells: vec![1],
            },
            Body {
                outer_shell: 2,
                inner_shells: vec![],
            },
        ];
        model.rebuild_topology_ids();
        model.validate().unwrap();
        let (text, _, _) = export_iges_v2(&model).unwrap();
        let (back, _, report) = import_iges_v2(&text).unwrap();
        assert_eq!(back.bodies.len(), 2);
        assert_eq!(back.bodies[0].inner_shells.len(), 1);
        assert_eq!(
            (
                back.vertices.len(),
                back.edges.len(),
                back.faces.len(),
                back.shells.len()
            ),
            (24, 36, 18, 3)
        );
        assert!(report.identity.preserved);
    }
}
