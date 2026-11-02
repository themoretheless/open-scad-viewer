//! Strict finite IGES 5.3 direct B-rep interchange.
//!
//! The legacy `iges-interchange/1` walking slice remains frozen.  This
//! successor parses physical 80-column records and maps the admitted IGES
//! topology graph directly; it never recognizes constructors, derives an
//! AABB, or crosses through a mesh.

use crate::analytic_features::FeatureCertificate;
use crate::{Body, Coedge, Edge, Face, FaceUse, Loop, Model, Shell, TopoId, TopoKind, TopologyIds};
use brep_topology::Vertex;
use nurbs_core::{curve::Curve, surface::Surface, Error, Result};
use std::collections::{BTreeMap, BTreeSet};

pub const IGES_INTERCHANGE_V2_CAPABILITY: &str = "iges-interchange/2";
const MAX_BYTES: usize = 16 * 1024 * 1024;
const MAX_RECORDS: usize = 131_072;
const MAX_ENTITIES: usize = 65_536;
const MAX_VALUES: usize = 1_000_000;
const MAX_TOPOLOGY: usize = 32_768;
const MAX_GRAPH_WORK: usize = 4_000_000;
const MAX_COMPOSITE_DEPTH: usize = 64;
const MAX_COMPOSITE_SEGMENTS: usize = 256;
const MAX_COMPOSITE_CONTROLS: usize = 256;
const MAX_COMPOSITE_WORK: usize = 4096;

type TopologyMapping = Vec<(TopoKind, usize, usize)>;

fn refuse(message: impl Into<String>) -> Error {
    Error::new("BREP_IGES_V2_REFUSED", message)
}

fn composite_error(code: &'static str, message: impl Into<String>) -> Error {
    Error::new(code, message)
}

fn trimmed_error(code: &'static str, message: impl Into<String>) -> Error {
    Error::new(code, message)
}

fn trimmed_unsupported(de: usize, message: impl Into<String>) -> Error {
    trimmed_error(
        "BREP_IGES_V2_TRIMMED_UNSUPPORTED",
        format!("IGES trimmed/bounded DE {de}: {}", message.into()),
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IgesLengthUnit {
    Inch,
    Millimeter,
    Foot,
    Mile,
    Meter,
    Kilometer,
    Mil,
    Micrometer,
    Centimeter,
    Microinch,
}

impl IgesLengthUnit {
    pub fn from_flag(flag: usize) -> Option<Self> {
        Some(match flag {
            1 => Self::Inch,
            2 => Self::Millimeter,
            4 => Self::Foot,
            5 => Self::Mile,
            6 => Self::Meter,
            7 => Self::Kilometer,
            8 => Self::Mil,
            9 => Self::Micrometer,
            10 => Self::Centimeter,
            11 => Self::Microinch,
            _ => return None,
        })
    }

    pub const fn flag(self) -> usize {
        match self {
            Self::Inch => 1,
            Self::Millimeter => 2,
            Self::Foot => 4,
            Self::Mile => 5,
            Self::Meter => 6,
            Self::Kilometer => 7,
            Self::Mil => 8,
            Self::Micrometer => 9,
            Self::Centimeter => 10,
            Self::Microinch => 11,
        }
    }

    pub const fn scale_to_mm(self) -> f64 {
        match self {
            Self::Inch => 25.4,
            Self::Millimeter => 1.,
            Self::Foot => 304.8,
            Self::Mile => 1_609_344.,
            Self::Meter => 1_000.,
            Self::Kilometer => 1_000_000.,
            Self::Mil => 0.0254,
            Self::Micrometer => 0.001,
            Self::Centimeter => 10.,
            Self::Microinch => 25.4e-6,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IgesV2ImportOptions {
    pub unit_override: Option<IgesLengthUnit>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IgesV2UnitReport {
    pub declared_unit: IgesLengthUnit,
    pub applied_unit: IgesLengthUnit,
    pub declared_flag: usize,
    pub declared_scale_to_mm: f64,
    pub applied_scale_to_mm: f64,
    pub overridden: bool,
    pub warning: Option<String>,
}

impl IgesV2UnitReport {
    fn export_millimeters() -> Self {
        Self {
            declared_unit: IgesLengthUnit::Millimeter,
            applied_unit: IgesLengthUnit::Millimeter,
            declared_flag: IgesLengthUnit::Millimeter.flag(),
            declared_scale_to_mm: 1.,
            applied_scale_to_mm: 1.,
            overridden: false,
            warning: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IgesV2DiscardReport {
    pub blanked: usize,
    pub dependent_without_owner: usize,
}

#[derive(Clone, Debug)]
pub struct IgesV2Report {
    pub identity: crate::step_interchange::StepIdentityReport,
    pub ignored_metadata: Vec<String>,
    pub entity_count: usize,
    pub topology_count: usize,
    pub units: IgesV2UnitReport,
    pub discarded: IgesV2DiscardReport,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Subordinate {
    Independent,
    Physical,
    Logical,
    Both,
}

impl Subordinate {
    fn requires_physical(self) -> bool {
        matches!(self, Self::Physical | Self::Both)
    }

    fn requires_logical(self) -> bool {
        matches!(self, Self::Logical | Self::Both)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DirectoryStatus {
    blanked: bool,
    subordinate: Subordinate,
}

#[derive(Clone, Debug)]
struct Directory {
    ty: usize,
    parameter: usize,
    transform: usize,
    color: i32,
    form: usize,
    label: String,
    status: DirectoryStatus,
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







fn entity(entities: &BTreeMap<usize, Entity>, de: usize, ty: usize) -> Result<&Entity> {
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

#[derive(Clone, Copy, Debug)]
struct EntityReference {
    target: usize,
    role: &'static str,
}

fn entity_references(de: usize, entity: &Entity) -> Result<Vec<EntityReference>> {
    let v = &entity.values;
    let exact_count = |base: usize, n: usize, stride: usize, what: &str| -> Result<()> {
        let expected = n
            .checked_mul(stride)
            .and_then(|count| count.checked_add(base))
            .ok_or_else(|| refuse(format!("IGES DE {de} {what} size overflow")))?;
        if v.len() != expected {
            return Err(refuse(format!("Malformed IGES DE {de} {what}")));
        }
        Ok(())
    };
    let mut refs = Vec::new();
    let mut push = |at: usize, role: &'static str| -> Result<()> {
        refs.push(EntityReference {
            target: u(v, at, role)?,
            role,
        });
        Ok(())
    };
    match entity.directory.ty {
        100 | 502 | 110 | 124 | 126 | 128 | 141 | 314 => {}
        142 => {
            exact_count(6, 0, 0, "curve on parametric surface")?;
            push(2, "parametric surface")?;
            if u(v, 3, "boundary model curve")? != 0 {
                push(3, "boundary model curve")?;
            }
            if u(v, 4, "boundary pcurve")? != 0 {
                push(4, "boundary pcurve")?;
            }
        }
        143 => {
            let n = u(v, 4, "bounded surface segment count")?;
            exact_count(5, n, 3, "bounded surface")?;
            push(3, "bounded surface geometry")?;
            for i in 0..n {
                let at = 5 + 3 * i;
                if u(v, at, "boundary model curve")? != 0 {
                    push(at, "boundary model curve")?;
                }
                if u(v, at + 1, "boundary pcurve")? != 0 {
                    push(at + 1, "boundary pcurve")?;
                }
            }
        }
        144 => {
            let n = u(v, 3, "trimmed surface inner boundary count")?;
            exact_count(5, n, 1, "trimmed surface")?;
            push(1, "trimmed surface geometry")?;
            if u(v, 2, "trimmed surface outer flag")? != 0 {
                push(4, "trimmed surface outer boundary")?;
            }
            for i in 0..n {
                push(5 + i, "trimmed surface inner boundary")?;
            }
        }
        102 => {
            let n = u(v, 1, "composite segment count")?;
            exact_count(2, n, 1, "composite curve")?;
            for i in 0..n {
                push(2 + i, "composite segment")?;
            }
        }
        504 => {
            let n = u(v, 1, "edge count")?;
            exact_count(2, n, 5, "edge list")?;
            for i in 0..n {
                let at = 2 + 5 * i;
                push(at, "edge curve")?;
                push(at + 1, "start vertex list")?;
                push(at + 3, "end vertex list")?;
            }
        }
        508 => {
            let n = u(v, 1, "loop member count")?;
            exact_count(2, n, 7, "loop")?;
            for i in 0..n {
                let at = 2 + 7 * i;
                push(at + 1, "loop edge list")?;
                push(at + 6, "loop pcurve")?;
            }
        }
        510 => {
            let n = u(v, 2, "face loop count")?;
            exact_count(4, n, 1, "face")?;
            push(1, "face surface")?;
            for i in 0..n {
                push(4 + i, "face loop")?;
            }
        }
        514 => {
            let n = u(v, 1, "shell face count")?;
            exact_count(2, n, 2, "shell")?;
            for i in 0..n {
                push(2 + 2 * i, "shell face")?;
            }
        }
        186 => {
            let n = u(v, 3, "solid void count")?;
            exact_count(4, n, 2, "solid")?;
            push(1, "solid outer shell")?;
            for i in 0..n {
                push(4 + 2 * i, "solid void shell")?;
            }
        }
        402 => match entity.directory.form {
            1 | 14 => {
                return Err(refuse(format!(
                    "Unsupported IGES DE {de} associativity form {}: back-pointer parameters are not safely separable",
                    entity.directory.form
                )));
            }
            7 | 15 => {
                let n = u(v, 1, "associativity member count")?;
                exact_count(2, n, 1, "associativity group")?;
                for i in 0..n {
                    push(2 + i, "associativity member")?;
                }
            }
            _ => {}
        },
        406 if entity.directory.form == 15 && v.len() == 4 => {
            push(3, "identity target")?;
        }
        _ => {}
    }
    Ok(refs)
}

fn graph_work(work: &mut usize, amount: usize) -> Result<()> {
    *work = work
        .checked_add(amount)
        .ok_or_else(|| refuse("IGES directory graph work budget overflow"))?;
    if *work > MAX_GRAPH_WORK {
        return Err(refuse("IGES directory graph work budget exceeded"));
    }
    Ok(())
}





#[derive(Clone)]
struct CompositeSegment {
    de: usize,
    curve: Curve,
}

enum CompositeVisit {
    Enter(usize, usize),
    Exit(usize, usize),
}

struct CurveDispatcher<'a> {
    entities: &'a BTreeMap<usize, Entity>,
    dim: usize,
    scale: f64,
    tolerance_mm: f64,
    work: usize,
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
    merge_tolerance_mm: f64,
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

    fn trimmed_vertex(&mut self, owner: usize, point: Vec<f64>) -> Result<usize> {
        if point.len() != 3 || point.iter().any(|value| !value.is_finite()) {
            return Err(trimmed_error(
                "BREP_IGES_V2_TRIMMED_DANGLING",
                "Invalid trimmed boundary vertex",
            ));
        }
        if let Some(index) = self.vertices.iter().position(|vertex| {
            (0..3)
                .map(|axis| (vertex.point[axis] - point[axis]).abs())
                .fold(0., f64::max)
                <= 1e-12
        }) {
            return Ok(index);
        }
        if self.vertices.len() >= MAX_TOPOLOGY {
            return Err(trimmed_error(
                "BREP_IGES_V2_TRIMMED_BUDGET",
                "Trimmed boundary vertex budget exceeded",
            ));
        }
        let index = self.vertices.len();
        self.vertices.push(Vertex {
            point: [point[0], point[1], point[2]],
        });
        self.vertex_de.push(owner);
        Ok(index)
    }

    fn directed_endpoints(&self, coedge: &Coedge) -> Result<([Vec<f64>; 2], [Vec<f64>; 2])> {
        let edge = &self.edges[coedge.edge];
        let domain = edge.curve.domain();
        let pdomain = coedge.pcurve.domain();
        let (a, b) = if coedge.reversed {
            (domain[1], domain[0])
        } else {
            (domain[0], domain[1])
        };
        Ok((
            [edge.curve.evaluate(a)?.point, edge.curve.evaluate(b)?.point],
            [
                coedge.pcurve.evaluate(pdomain[0])?.point,
                coedge.pcurve.evaluate(pdomain[1])?.point,
            ],
        ))
    }

    fn flip_trimmed_coedge(coedge: &mut Coedge) -> Result<()> {
        coedge.reversed = !coedge.reversed;
        coedge.pcurve = coedge.pcurve.reverse()?;
        Ok(())
    }

    fn set_directed_edge_endpoint(
        &mut self,
        coedge: &Coedge,
        end: bool,
        point: &[f64],
    ) -> Result<()> {
        let edge = &mut self.edges[coedge.edge];
        let vertex = match (coedge.reversed, end) {
            (false, false) | (true, true) => edge.vertices[0],
            _ => edge.vertices[1],
        };
        let control = match (coedge.reversed, end) {
            (false, false) | (true, true) => edge.curve.control_points.first_mut(),
            _ => edge.curve.control_points.last_mut(),
        }
        .ok_or_else(|| trimmed_unsupported(0, "empty boundary curve"))?;
        *control = point.to_vec();
        self.vertices[vertex].point = [point[0], point[1], point[2]];
        edge.curve.validate()?;
        Ok(())
    }

    fn close_trimmed_joint(
        &mut self,
        boundary_de: usize,
        previous: usize,
        current: usize,
        coedges: &mut [Coedge],
        owned_edges: &BTreeSet<usize>,
        surface: &Surface,
    ) -> Result<()> {
        let (previous_xyz, previous_uv) = self.directed_endpoints(&coedges[previous])?;
        let (current_xyz, current_uv) = self.directed_endpoints(&coedges[current])?;
        let curve_gap = previous_xyz[1]
            .iter()
            .zip(&current_xyz[0])
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f64>()
            .sqrt();
        let a = surface
            .evaluate(previous_uv[1][0], previous_uv[1][1])?
            .point;
        let b = surface.evaluate(current_uv[0][0], current_uv[0][1])?.point;
        let parameter_gap = (0..3)
            .map(|axis| (a[axis] - b[axis]) * (a[axis] - b[axis]))
            .sum::<f64>()
            .sqrt();
        let gap = curve_gap.max(parameter_gap);
        if !gap.is_finite() || gap > self.merge_tolerance_mm {
            return Err(trimmed_error(
                "BREP_IGES_V2_TRIMMED_GAP",
                format!(
                    "IGES boundary DE {boundary_de} is open between segments {previous} and {current}: gap_mm={gap:.17e}, tolerance_mm={:.17e}",
                    self.merge_tolerance_mm
                ),
            ));
        }
        if gap <= 1e-12 {
            return Ok(());
        }
        if !owned_edges.contains(&coedges[previous].edge)
            || !owned_edges.contains(&coedges[current].edge)
        {
            return Err(trimmed_unsupported(
                boundary_de,
                "gap repair on a shared 504 edge would modify an adjacent face",
            ));
        }
        let xyz = previous_xyz[1]
            .iter()
            .zip(&current_xyz[0])
            .map(|(a, b)| (a + b) * 0.5)
            .collect::<Vec<_>>();
        let uv = previous_uv[1]
            .iter()
            .zip(&current_uv[0])
            .map(|(a, b)| (a + b) * 0.5)
            .collect::<Vec<_>>();
        let previous_edge = coedges[previous].edge;
        let current_edge = coedges[current].edge;
        let previous_coedge = coedges[previous].clone();
        let current_coedge = coedges[current].clone();
        self.set_directed_edge_endpoint(&previous_coedge, true, &xyz)?;
        self.set_directed_edge_endpoint(&current_coedge, false, &xyz)?;
        if previous_edge == current_edge {
            let edge = &self.edges[previous_edge];
            edge.curve.validate()?;
        }
        coedges[previous]
            .pcurve
            .control_points
            .last_mut()
            .ok_or_else(|| trimmed_unsupported(boundary_de, "empty boundary pcurve"))?
            .clone_from(&uv);
        coedges[current]
            .pcurve
            .control_points
            .first_mut()
            .ok_or_else(|| trimmed_unsupported(boundary_de, "empty boundary pcurve"))?
            .clone_from(&uv);
        coedges[previous].pcurve.validate()?;
        coedges[current].pcurve.validate()?;
        Ok(())
    }

    fn orient_trimmed_loop(
        boundary_de: usize,
        coedges: &mut Vec<Coedge>,
        outer: bool,
    ) -> Result<()> {
        let mut points = Vec::new();
        for coedge in coedges.iter() {
            points.extend(coedge.pcurve.control_points.iter().cloned());
        }
        let mut area = 0.;
        for pair in points.windows(2) {
            area += pair[0][0] * pair[1][1] - pair[1][0] * pair[0][1];
        }
        if let (Some(first), Some(last)) = (points.first(), points.last()) {
            area += last[0] * first[1] - first[0] * last[1];
        }
        if !area.is_finite() || area.abs() <= 1e-18 {
            return Err(trimmed_unsupported(
                boundary_de,
                "boundary has zero or non-finite UV signed area",
            ));
        }
        if (outer && area < 0.) || (!outer && area > 0.) {
            coedges.reverse();
            for coedge in coedges {
                Self::flip_trimmed_coedge(coedge)?;
            }
        }
        Ok(())
    }

    fn trimmed_loop(
        &mut self,
        de: usize,
        expected_surface: usize,
        curve_edges: &BTreeMap<usize, usize>,
        owned_edges: &mut BTreeSet<usize>,
    ) -> Result<usize> {
        let entity = self.e.get(&de).ok_or_else(|| {
            trimmed_error(
                "BREP_IGES_V2_TRIMMED_DANGLING",
                format!("Missing IGES boundary DE {de}"),
            )
        })?;
        let v = &entity.values;
        let (surface, segments) = match entity.directory.ty {
            143 => {
                let kind = u(v, 1, "bounded surface boundary type")?;
                let preference = u(v, 2, "bounded surface preference")?;
                if !matches!(kind, 0 | 2) || matches!(preference, 1) {
                    return Err(trimmed_unsupported(
                        de,
                        "3D-only boundary without reliable parameterization",
                    ));
                }
                if !matches!(preference, 0 | 2 | 3) {
                    return Err(trimmed_unsupported(de, "unknown representation preference"));
                }
                let n = u(v, 4, "bounded surface segment count")?;
                if n == 0 || n > MAX_COMPOSITE_SEGMENTS || v.len() != 5 + 3 * n {
                    return Err(trimmed_error(
                        "BREP_IGES_V2_TRIMMED_BUDGET",
                        format!("Invalid or excessive IGES boundary DE {de} segment count {n}"),
                    ));
                }
                (
                    u(v, 3, "bounded surface geometry")?,
                    (0..n)
                        .map(|i| {
                            Ok((
                                u(v, 5 + 3 * i, "boundary model curve")?,
                                u(v, 6 + 3 * i, "boundary pcurve")?,
                                u(v, 7 + 3 * i, "boundary sense")?,
                            ))
                        })
                        .collect::<Result<Vec<_>>>()?,
                )
            }
            142 => {
                if v.len() != 6 {
                    return Err(trimmed_error(
                        "BREP_IGES_V2_TRIMMED_DANGLING",
                        format!("Malformed IGES curve-on-surface DE {de}"),
                    ));
                }
                let preference = u(v, 5, "curve-on-surface preference")?;
                if !matches!(preference, 0 | 2 | 3) {
                    return Err(trimmed_unsupported(
                        de,
                        "3D-only or model-preferred boundary without reliable parameterization",
                    ));
                }
                (
                    u(v, 2, "curve-on-surface geometry")?,
                    vec![(
                        u(v, 3, "boundary model curve")?,
                        u(v, 4, "boundary pcurve")?,
                        1,
                    )],
                )
            }
            _ => {
                return Err(trimmed_unsupported(
                    de,
                    format!(
                        "boundary type {} is unsupported; expected 143 or 142",
                        entity.directory.ty
                    ),
                ));
            }
        };
        if surface != expected_surface {
            return Err(trimmed_error(
                "BREP_IGES_V2_TRIMMED_DANGLING",
                format!(
                    "IGES boundary DE {de} surface DE {surface} differs from trimmed surface DE {expected_surface}"
                ),
            ));
        }
        let surface = surface128(self.e, expected_surface, self.scale)?;
        let mut coedges = Vec::with_capacity(segments.len());
        for (curve_de, pcurve_de, sense) in segments {
            if curve_de == 0 || pcurve_de == 0 {
                return Err(trimmed_unsupported(
                    de,
                    "boundary requires both 3D curve and pcurve; projection is not inferred",
                ));
            }
            if !matches!(sense, 1 | 2) {
                return Err(trimmed_unsupported(de, "boundary sense must be 1 or 2"));
            }
            let curve = self.curve(curve_de, 3)?;
            let pcurve = self.curve(pcurve_de, 2)?;
            let edge = if let Some(edge) = curve_edges.get(&curve_de) {
                *edge
            } else {
                let domain = curve.domain();
                let a = self.trimmed_vertex(de, curve.evaluate(domain[0])?.point)?;
                let b = self.trimmed_vertex(de, curve.evaluate(domain[1])?.point)?;
                let edge = self.edges.len();
                self.edges.push(Edge {
                    degenerate: a == b,
                    vertices: [a, b],
                    curve,
                });
                self.edge_de.push(de);
                owned_edges.insert(edge);
                edge
            };
            coedges.push(Coedge::new(edge, sense == 2, pcurve));
        }
        for index in 1..coedges.len() {
            let (mut previous_xyz, _) = self.directed_endpoints(&coedges[index - 1])?;
            let (current_xyz, _) = self.directed_endpoints(&coedges[index])?;
            let distance = |a: &[f64], b: &[f64]| {
                a.iter()
                    .zip(b)
                    .map(|(x, y)| (x - y) * (x - y))
                    .sum::<f64>()
                    .sqrt()
            };
            let mut end_start = distance(&previous_xyz[1], &current_xyz[0]);
            let mut end_end = distance(&previous_xyz[1], &current_xyz[1]);
            if index == 1
                && end_start > self.merge_tolerance_mm
                && end_end > self.merge_tolerance_mm
            {
                let start_start = distance(&previous_xyz[0], &current_xyz[0]);
                let start_end = distance(&previous_xyz[0], &current_xyz[1]);
                if start_start <= self.merge_tolerance_mm || start_end <= self.merge_tolerance_mm {
                    Self::flip_trimmed_coedge(&mut coedges[0])?;
                    (previous_xyz, _) = self.directed_endpoints(&coedges[0])?;
                    end_start = distance(&previous_xyz[1], &current_xyz[0]);
                    end_end = distance(&previous_xyz[1], &current_xyz[1]);
                }
            }
            if end_start <= self.merge_tolerance_mm && end_end <= self.merge_tolerance_mm {
                return Err(trimmed_error(
                    "BREP_IGES_V2_TRIMMED_AMBIGUOUS",
                    format!("IGES boundary DE {de} has ambiguous orientation at segment {index}"),
                ));
            }
            if end_start > self.merge_tolerance_mm {
                if end_end > self.merge_tolerance_mm {
                    let gap = end_start.min(end_end);
                    return Err(trimmed_error(
                        "BREP_IGES_V2_TRIMMED_GAP",
                        format!(
                            "IGES boundary DE {de} is open at segment {index}: gap_mm={gap:.17e}, tolerance_mm={:.17e}",
                            self.merge_tolerance_mm
                        ),
                    ));
                }
                Self::flip_trimmed_coedge(&mut coedges[index])?;
            }
        }
        for index in 0..coedges.len() {
            let next = (index + 1) % coedges.len();
            self.close_trimmed_joint(de, index, next, &mut coedges, owned_edges, &surface)?;
        }
        if coedges.len() > MAX_TOPOLOGY {
            return Err(trimmed_error(
                "BREP_IGES_V2_TRIMMED_BUDGET",
                format!("IGES boundary DE {de} exceeds topology budget"),
            ));
        }
        let index = self.loops.len();
        self.loops.push(Loop { coedges });
        self.loop_de.push(de);
        Ok(index)
    }

    fn build(mut self) -> Result<(Model, TopologyMapping)> {
        if let Some(&de) = self.e.keys().find(|&&de| self.e[&de].directory.ty == 141) {
            return Err(trimmed_unsupported(
                de,
                "IGES 141 boundary entities are not admitted; use 143 or 142 with both curve representations",
            ));
        }
        let mut vertex_lists = BTreeMap::new();
        for (&de, e) in self.e {
            if e.directory.ty == 502 {
                vertex_lists.insert(de, self.vertex_list(de)?);
            }
        }
        let mut edge_lists = BTreeMap::<(usize, usize), usize>::new();
        let mut curve_edges = BTreeMap::<usize, usize>::new();
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
                let curve_de = u(v, at, "edge curve")?;
                let curve = self.curve(curve_de, 3)?;
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
                curve_edges.insert(curve_de, index);
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
                let pcurve = self.curve(u(v, at + 6, "loop pcurve")?, 2)?;
                let edge = *edge_lists
                    .get(&(el, ei))
                    .ok_or_else(|| refuse("Invalid IGES edge-list reference"))?;
                coedges.push(Coedge::new(edge, reversed, pcurve));
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
        let referenced_boundaries = self
            .e
            .values()
            .filter(|entity| entity.directory.ty == 144)
            .flat_map(|entity| {
                let v = &entity.values;
                let n = u(v, 3, "trimmed surface inner boundary count").unwrap_or(0);
                (4..5 + n).filter_map(move |at| u(v, at, "trimmed boundary").ok())
            })
            .collect::<BTreeSet<_>>();
        let mut owned_edges = BTreeSet::new();
        for (&de, e) in self.e {
            if !matches!(e.directory.ty, 143 | 144)
                || (e.directory.ty == 143 && referenced_boundaries.contains(&de))
            {
                continue;
            }
            let v = &e.values;
            let (surface_de, boundaries) = if e.directory.ty == 144 {
                if v.len() < 5 {
                    return Err(trimmed_error(
                        "BREP_IGES_V2_TRIMMED_DANGLING",
                        format!("Malformed IGES trimmed surface DE {de}"),
                    ));
                }
                if u(v, 2, "trimmed surface outer flag")? != 1 {
                    return Err(trimmed_unsupported(
                        de,
                        "natural outer boundary without an explicit 143/142 loop",
                    ));
                }
                let n = u(v, 3, "trimmed surface inner boundary count")?;
                if n > MAX_TOPOLOGY || v.len() != 5 + n {
                    return Err(trimmed_error(
                        "BREP_IGES_V2_TRIMMED_BUDGET",
                        format!("Invalid or excessive IGES trimmed DE {de} boundary count {n}"),
                    ));
                }
                let outer = u(v, 4, "trimmed surface outer boundary")?;
                if outer == 0 {
                    return Err(trimmed_error(
                        "BREP_IGES_V2_TRIMMED_DANGLING",
                        format!("Missing outer boundary in IGES trimmed surface DE {de}"),
                    ));
                }
                (
                    u(v, 1, "trimmed surface geometry")?,
                    (4..5 + n)
                        .map(|at| u(v, at, "trimmed boundary"))
                        .collect::<Result<Vec<_>>>()?,
                )
            } else {
                (u(v, 3, "bounded surface geometry")?, vec![de])
            };
            let mut loops = Vec::with_capacity(boundaries.len());
            for boundary in boundaries {
                let index =
                    self.trimmed_loop(boundary, surface_de, &curve_edges, &mut owned_edges)?;
                Self::orient_trimmed_loop(
                    boundary,
                    &mut self.loops[index].coedges,
                    loops.is_empty(),
                )?;
                loops.push(index);
            }
            let surface = surface128(self.e, surface_de, self.scale)?;
            for loop_index in &loops {
                for coedge in &self.loops[*loop_index].coedges {
                    verify_correspondence(
                        &self.edges[coedge.edge],
                        &coedge.pcurve,
                        &surface,
                        coedge.reversed,
                    )
                    .map_err(|error| {
                        trimmed_error(
                            "BREP_IGES_V2_TRIMMED_DANGLING",
                            format!("IGES trimmed/bounded DE {de}: {}", error.message),
                        )
                    })?;
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
                // IGES 514 form 1 is an open shell; form 2 (and legacy form 0) is closed.
                closed: e.directory.form != 1,
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
        if self.bodies.is_empty() && self.shells.iter().all(|shell| shell.closed) {
            return Err(refuse(
                "IGES contains neither a manifold solid B-rep (186) nor an open shell (514 form 1)",
            ));
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



/// Conservative maximum axis-aligned extent of vertices and 3D control hulls.
/// Two-dimensional pcurve control points are deliberately not included.
fn geometric_extent_mm(model: &Model) -> f64 {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    let mut include = |point: &[f64]| {
        for axis in 0..3 {
            min[axis] = min[axis].min(point[axis]);
            max[axis] = max[axis].max(point[axis]);
        }
    };
    for vertex in &model.vertices {
        include(&vertex.point);
    }
    for edge in &model.edges {
        for point in &edge.curve.control_points {
            include(point);
        }
    }
    for face in &model.faces {
        for point in face.surface.control_points.iter().flatten() {
            include(point);
        }
    }
    (0..3).map(|axis| max[axis] - min[axis]).fold(0., f64::max)
}

/// Broad diagnostic band for mechanical parts: 1 micrometre through 10 metres.
/// It never selects or changes units; it only gates an inch/millimeter warning.
fn reasonable_mechanical_extent(extent_mm: f64) -> bool {
    (1e-3..=1e4).contains(&extent_mm)
}

fn unit_mismatch_warning(
    model: &Model,
    declared: IgesLengthUnit,
    applied: IgesLengthUnit,
) -> Option<String> {
    let alternative = match declared {
        IgesLengthUnit::Inch => IgesLengthUnit::Millimeter,
        IgesLengthUnit::Millimeter => IgesLengthUnit::Inch,
        _ => return None,
    };
    let raw_extent = geometric_extent_mm(model) / applied.scale_to_mm();
    let declared_extent = raw_extent * declared.scale_to_mm();
    let alternative_extent = raw_extent * alternative.scale_to_mm();
    if reasonable_mechanical_extent(declared_extent)
        || !reasonable_mechanical_extent(alternative_extent)
    {
        return None;
    }
    Some(format!(
        "Possible IGES unit mismatch: declared {:?} (scale {} mm/unit) gives control-hull/vertex extent {} mm, outside the mechanical-part band [0.001, 10000] mm; alternative {:?} (scale {} mm/unit) gives extent {} mm inside the band. Diagnostic only; units are not changed automatically.",
        declared,
        declared.scale_to_mm(),
        declared_extent,
        alternative,
        alternative.scale_to_mm(),
        alternative_extent,
    ))
}

pub fn import_iges_v2(text: &str) -> Result<(Model, FeatureCertificate, IgesV2Report)> {
    import_iges_v2_with_options(text, IgesV2ImportOptions::default())
}

pub fn import_iges_v2_with_options(
    text: &str,
    options: IgesV2ImportOptions,
) -> Result<(Model, FeatureCertificate, IgesV2Report)> {
    let (mut entities, declared_unit, resolution) = parse(text)?;
    let entity_count = entities.len();
    let discarded = filter_discarded_entities(&mut entities)?;
    let applied_unit = options.unit_override.unwrap_or(declared_unit);
    let scale = applied_unit.scale_to_mm();
    let merge_tolerance_mm = resolution * scale;
    if !merge_tolerance_mm.is_finite() || merge_tolerance_mm <= 0. {
        return Err(refuse("Invalid scaled IGES composite merge tolerance"));
    }
    const GEOMETRIC: &[usize] = &[
        100, 102, 110, 124, 126, 128, 141, 142, 143, 144, 186, 502, 504, 508, 510, 514,
    ];
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
        merge_tolerance_mm,
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
    let warning = unit_mismatch_warning(&model, declared_unit, applied_unit);
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
            entity_count,
            topology_count,
            units: IgesV2UnitReport {
                declared_unit,
                applied_unit,
                declared_flag: declared_unit.flag(),
                declared_scale_to_mm: declared_unit.scale_to_mm(),
                applied_scale_to_mm: applied_unit.scale_to_mm(),
                overridden: applied_unit != declared_unit,
                warning,
            },
            discarded,
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




#[cfg(test)]
#[path = "iges_interchange_v2/tests/mod.rs"]
mod tests;
