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

#[path="iges_interchange_v2/methods_budget.rs"]
mod methods_budget;
#[path="iges_interchange_v2/methods_curve.rs"]
mod methods_curve;
#[path="iges_interchange_v2/algorithm_parse_directory_status.rs"]
mod algorithm_parse_directory_status;
use algorithm_parse_directory_status::parse_directory_status;
#[path="iges_interchange_v2/algorithm_tokenize.rs"]
mod algorithm_tokenize;
use algorithm_tokenize::tokenize;
#[path="iges_interchange_v2/algorithm_parse_global.rs"]
mod algorithm_parse_global;
use algorithm_parse_global::parse_global;
#[path="iges_interchange_v2/algorithm_parse.rs"]
mod algorithm_parse;
use algorithm_parse::parse;
#[path="iges_interchange_v2/algorithm_entity_references.rs"]
mod algorithm_entity_references;
use algorithm_entity_references::entity_references;
#[path="iges_interchange_v2/algorithm_filter_discarded_entities.rs"]
mod algorithm_filter_discarded_entities;
use algorithm_filter_discarded_entities::filter_discarded_entities;
#[path="iges_interchange_v2/algorithm_curve126.rs"]
mod algorithm_curve126;
use algorithm_curve126::curve126;
#[path="iges_interchange_v2/algorithm_surface128.rs"]
mod algorithm_surface128;
use algorithm_surface128::surface128;
#[path="iges_interchange_v2/algorithm_restore_identity.rs"]
mod algorithm_restore_identity;
use algorithm_restore_identity::restore_identity;
#[path="iges_interchange_v2/algorithm_import_iges_v2_with_options.rs"]
mod algorithm_import_iges_v2_with_options;
pub use algorithm_import_iges_v2_with_options::import_iges_v2_with_options;
#[path="iges_interchange_v2/algorithm_render.rs"]
mod algorithm_render;
use algorithm_render::render;

#[path="iges_interchange_v2/algorithm_export_iges_v2.rs"]
mod algorithm_export_iges_v2;
pub use algorithm_export_iges_v2::export_iges_v2;
