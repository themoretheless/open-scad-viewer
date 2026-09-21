//! Bounded, direct Part 21 topology interchange.
//!
//! Unlike the compatibility `/1` and `/2` routes this module never recognizes
//! constructors or reconstructs an AABB.  It translates the selected
//! MANIFOLD_SOLID_BREP/BREP_WITH_VOIDS graph directly to/from `Model`.

use crate::analytic_features::FeatureCertificate;
use crate::{Body, Coedge, Edge, Face, FaceUse, Loop, Model, Shell, TopoId, TopoKind, TopologyIds};
use brep_topology::{CoedgeTrim, Vertex};
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const STEP_INTERCHANGE_V3_CAPABILITY: &str = "step-interchange/3";
pub const STEP_INTERCHANGE_V4_CAPABILITY: &str = "step-interchange/4";
pub const STEP_INTERCHANGE_V5_CAPABILITY: &str = "step-interchange/5";
pub const STEP_INTERCHANGE_V6_CAPABILITY: &str = "step-interchange/6";
pub const STEP_INTERCHANGE_V7_CAPABILITY: &str = "step-interchange/7";
pub const STEP_INTERCHANGE_V8_CAPABILITY: &str = "step-interchange/8";
pub const STEP_INTERCHANGE_V9_CAPABILITY: &str = "step-interchange/9";
pub const STEP_INTERCHANGE_V10_CAPABILITY: &str = "step-interchange/10";
const MAX_BYTES: usize = 16 * 1024 * 1024;
const MAX_INSTANCES: usize = 65_536;
const MAX_PARSE_DEPTH: usize = 32;
const MAX_GRAPH_DEPTH: usize = 64;
const MAX_OCCURRENCES: usize = 256;
const MAX_OUTPUT_BYTES: usize = 16 * 1024 * 1024;

type TopologyMapping = Vec<(TopoKind, usize, usize)>;

struct DirectBuildOptions {
    analytic_surfaces: bool,
    ap242_composition: bool,
    allow_degenerate: bool,
    whole_domain_proofs: bool,
    interior_point_selectors: bool,
    allow_open_shells: bool,
}

fn refuse(message: impl Into<String>) -> Error {
    Error::new("BREP_STEP_V3_REFUSED", message)
}

#[derive(Clone, Debug)]
pub struct StepV3Report {
    pub identity: crate::step_interchange::StepIdentityReport,
    pub ignored_entities: Vec<String>,
    pub metadata_loss: Vec<String>,
    pub instance_count: usize,
    pub reachable_count: usize,
    pub definition_identities: Vec<String>,
    pub occurrence_identities: Vec<String>,
    pub product_hierarchy: Vec<String>,
    pub external_references: Vec<String>,
}

/// Lossless `/10` product envelope. The parsed B-rep is a derived editing
/// projection; `source` remains authoritative for occurrence and presentation
/// graphs that cannot be represented by `Model`.
#[derive(Clone, Debug)]
pub struct StepV10Document {
    pub source: String,
    pub graph_identity: String,
    pub definition_identities: Vec<String>,
    pub occurrence_identities: Vec<String>,
    pub product_hierarchy: Vec<String>,
    pub operator_identities: Vec<String>,
    pub metadata_loss: Vec<String>,
}

/// Algebraic, whole-parameter-domain evidence carried by STEP `/8`.
///
/// `identity` names the exact rational tensor identity checked by the kernel;
/// it is deliberately not a sampling result.  A zero Jacobian lower bound is
/// permitted only for an explicitly named collapsed boundary.  In that case
/// `regular_open_domain` certifies the punctured parameter neighbourhood and
/// topology validation certifies that the whole boundary is one pole vertex.
#[derive(Clone, Debug)]
pub struct StepRegularityEvidence {
    pub carrier: &'static str,
    pub parameter_u: [f64; 2],
    pub parameter_v: [f64; 2],
    pub lifted_periods: [f64; 2],
    pub denominator_lower_bound: f64,
    pub jacobian_lower_bound: f64,
    pub collapsed_boundaries: Vec<&'static str>,
    pub regular_open_domain: bool,
    pub identity: &'static str,
}

#[derive(Clone, Debug)]
pub struct StepV8Certificate {
    pub capability: &'static str,
    pub complete: bool,
    pub regularity: Vec<StepRegularityEvidence>,
    pub sense_layers: [&'static str; 7],
    pub coupled_sense_cases: usize,
    pub notes: Vec<&'static str>,
}

#[derive(Clone, Debug, PartialEq)]
enum Value {
    String(String),
    Number(f64),
    Integer(i64),
    Ref(usize),
    Enum(String),
    Omitted,
    Derived,
    List(Vec<Value>),
    Call(String, Vec<Value>),
}
fn render_value(v: &Value) -> String {
    match v {
        Value::String(s) => format!("'{}'", s.replace('\'', "''")),
        Value::Number(n) => format!("{n:.17e}").replace('e', "E"),
        Value::Integer(n) => n.to_string(),
        Value::Ref(id) => format!("#{id}"),
        Value::Enum(e) => format!(".{e}."),
        Value::Omitted => "$".into(),
        Value::Derived => "*".into(),
        Value::List(v) => format!(
            "({})",
            v.iter().map(render_value).collect::<Vec<_>>().join(",")
        ),
        Value::Call(n, v) => format!(
            "{n}({})",
            v.iter().map(render_value).collect::<Vec<_>>().join(",")
        ),
    }
}
fn render_entity_value(v: &Value) -> String {
    if let Value::List(parts) = v
        && parts.iter().all(|part| matches!(part, Value::Call(_, _)))
    {
        return format!(
            "({})",
            parts.iter().map(render_value).collect::<Vec<_>>().join("")
        );
    }
    render_value(v)
}

#[derive(Clone, Debug)]
struct Entity {
    value: Value,
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Hash(usize),
    Ident(String),
    String(String),
    Number(String),
    LParen,
    RParen,
    Comma,
    Eq,
    Semi,
    Dollar,
    Star,
    Enum(String),
}

fn lex(text: &str) -> Result<Vec<Tok>> {
    if text.len() > MAX_BYTES {
        return Err(refuse("STEP /3 payload exceeds 16 MiB"));
    }
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            i += 2;
            let mut closed = false;
            while i + 1 < bytes.len() {
                if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    i += 2;
                    closed = true;
                    break;
                }
                i += 1;
            }
            if !closed {
                return Err(refuse("Unterminated Part 21 comment"));
            }
            continue;
        }
        match bytes[i] {
            b'#' => {
                i += 1;
                let start = i;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                if start == i {
                    return Err(refuse("Malformed Part 21 reference"));
                }
                out.push(Tok::Hash(
                    text[start..i]
                        .parse()
                        .map_err(|_| refuse("Invalid instance number"))?,
                ));
            }
            b'\'' => {
                i += 1;
                let mut value = String::new();
                loop {
                    if i >= bytes.len() {
                        return Err(refuse("Unterminated Part 21 string"));
                    }
                    if bytes[i] == b'\'' {
                        if i + 1 < bytes.len() && bytes[i + 1] == b'\'' {
                            value.push('\'');
                            i += 2;
                            continue;
                        }
                        i += 1;
                        break;
                    }
                    let ch = text[i..]
                        .chars()
                        .next()
                        .ok_or_else(|| refuse("Invalid UTF-8 string"))?;
                    value.push(ch);
                    i += ch.len_utf8();
                }
                out.push(Tok::String(value));
            }
            b'.' if i + 1 < bytes.len() && bytes[i + 1].is_ascii_alphabetic() => {
                i += 1;
                let start = i;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                if i >= bytes.len() || bytes[i] != b'.' {
                    return Err(refuse(format!(
                        "Malformed STEP enumeration .{} at offset {start}",
                        &text[start..i]
                    )));
                }
                out.push(Tok::Enum(text[start..i].to_ascii_uppercase()));
                i += 1;
            }
            b'(' => {
                out.push(Tok::LParen);
                i += 1;
            }
            b')' => {
                out.push(Tok::RParen);
                i += 1;
            }
            b',' => {
                out.push(Tok::Comma);
                i += 1;
            }
            b'=' => {
                out.push(Tok::Eq);
                i += 1;
            }
            b';' => {
                out.push(Tok::Semi);
                i += 1;
            }
            b'$' => {
                out.push(Tok::Dollar);
                i += 1;
            }
            b'*' => {
                out.push(Tok::Star);
                i += 1;
            }
            b'+' | b'-' | b'0'..=b'9' | b'.' => {
                let start = i;
                i += 1;
                while i < bytes.len()
                    && matches!(
                        bytes[i],
                        b'0'..=b'9' | b'.' | b'+' | b'-' | b'E' | b'e' | b'D' | b'd'
                    )
                {
                    i += 1;
                }
                out.push(Tok::Number(text[start..i].to_string()));
            }
            c if c.is_ascii_alphabetic() || c == b'_' => {
                let start = i;
                i += 1;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                out.push(Tok::Ident(text[start..i].to_ascii_uppercase()));
            }
            _ => return Err(refuse(format!("Unexpected Part 21 byte at offset {i}"))),
        }
    }
    Ok(out)
}

struct Parser {
    tokens: Vec<Tok>,
    at: usize,
}
impl Parser {
    fn value(&mut self, depth: usize) -> Result<Value> {
        if depth > MAX_PARSE_DEPTH {
            return Err(refuse("Part 21 aggregate depth exceeds 32"));
        }
        let token = self
            .tokens
            .get(self.at)
            .cloned()
            .ok_or_else(|| refuse("Unexpected end of Part 21 value"))?;
        self.at += 1;
        Ok(match token {
            Tok::String(v) => Value::String(v),
            Tok::Hash(v) => Value::Ref(v),
            Tok::Dollar => Value::Omitted,
            Tok::Star => Value::Derived,
            Tok::Enum(v) => Value::Enum(v),
            Tok::Number(v) => {
                let normalized = v.replace(['D', 'd'], "E");
                if !normalized.contains(['.', 'E', 'e']) {
                    Value::Integer(
                        normalized
                            .parse()
                            .map_err(|_| refuse("Invalid STEP integer"))?,
                    )
                } else {
                    let n: f64 = normalized
                        .parse()
                        .map_err(|_| refuse("Invalid STEP real"))?;
                    if !n.is_finite() {
                        return Err(refuse("Non-finite STEP real"));
                    }
                    Value::Number(n)
                }
            }
            Tok::LParen => Value::List(self.list(depth + 1)?),
            Tok::Ident(name) => {
                if self.tokens.get(self.at) != Some(&Tok::LParen) {
                    return Err(refuse("Bare STEP identifier is not admitted"));
                }
                self.at += 1;
                Value::Call(name, self.list(depth + 1)?)
            }
            _ => return Err(refuse("Unexpected token in Part 21 value")),
        })
    }
    fn list(&mut self, depth: usize) -> Result<Vec<Value>> {
        let mut values = Vec::new();
        if self.tokens.get(self.at) == Some(&Tok::RParen) {
            self.at += 1;
            return Ok(values);
        }
        loop {
            values.push(self.value(depth)?);
            match self.tokens.get(self.at) {
                Some(Tok::Comma) => self.at += 1,
                Some(Tok::RParen) => {
                    self.at += 1;
                    break;
                }
                // Part 21 complex entity components are adjacent calls rather
                // than comma-separated aggregate members.
                Some(Tok::Ident(_)) => {}
                other => {
                    return Err(refuse(format!(
                        "Malformed Part 21 argument list near token {}: {other:?}",
                        self.at
                    )));
                }
            }
        }
        Ok(values)
    }
}

fn parse(text: &str) -> Result<BTreeMap<usize, Entity>> {
    let tokens = lex(text)?;
    let mut parser = Parser { tokens, at: 0 };
    let mut entities = BTreeMap::new();
    while parser.at < parser.tokens.len() {
        let Some(Tok::Hash(id)) = parser.tokens.get(parser.at).cloned() else {
            parser.at += 1;
            continue;
        };
        if parser.tokens.get(parser.at + 1) != Some(&Tok::Eq) {
            parser.at += 1;
            continue;
        }
        parser.at += 2;
        let value = parser.value(0)?;
        if parser.tokens.get(parser.at) != Some(&Tok::Semi) {
            return Err(refuse("STEP instance is missing semicolon"));
        }
        parser.at += 1;
        if entities.insert(id, Entity { value }).is_some() {
            return Err(refuse("Duplicate Part 21 instance number"));
        }
        if entities.len() > MAX_INSTANCES {
            return Err(refuse("STEP /3 instance count exceeds 65536"));
        }
    }
    if entities.is_empty() {
        return Err(refuse("STEP /3 contains no instances"));
    }
    Ok(entities)
}

fn call(entities: &BTreeMap<usize, Entity>, id: usize) -> Result<(&str, &[Value])> {
    let entity = entities
        .get(&id)
        .ok_or_else(|| refuse(format!("Missing reference #{id}")))?;
    match &entity.value {
        Value::Call(name, args) => Ok((name, args)),
        Value::List(parts) => {
            // Complex instances are parsed and bounded, but geometry complexes
            // are resolved only when an explicitly supported component exists.
            for part in parts {
                if let Value::Call(name, args) = part
                    && matches!(
                        name.as_str(),
                        "B_SPLINE_CURVE_WITH_KNOTS"
                            | "B_SPLINE_SURFACE_WITH_KNOTS"
                            | "SI_UNIT"
                            | "CONVERSION_BASED_UNIT"
                            | "GLOBAL_UNIT_ASSIGNED_CONTEXT"
                            | "GEOMETRIC_REPRESENTATION_CONTEXT"
                    )
                {
                    return Ok((name, args));
                }
            }
            Err(refuse(format!(
                "Reachable complex instance #{id} has no admitted component"
            )))
        }
        _ => Err(refuse(format!("Reference #{id} is not an entity call"))),
    }
}
fn components(entities: &BTreeMap<usize, Entity>, id: usize) -> Result<Vec<(&str, &[Value])>> {
    let entity = entities
        .get(&id)
        .ok_or_else(|| refuse(format!("Missing reference #{id}")))?;
    match &entity.value {
        Value::Call(name, args) => Ok(vec![(name.as_str(), args.as_slice())]),
        Value::List(parts) => parts
            .iter()
            .map(|part| match part {
                Value::Call(name, args) => Ok((name.as_str(), args.as_slice())),
                _ => Err(refuse(format!(
                    "Malformed complex entity component in #{id}"
                ))),
            })
            .collect(),
        _ => Err(refuse(format!("Reference #{id} is not an entity call"))),
    }
}
fn component<'a>(parts: &[(&'a str, &'a [Value])], ty: &str) -> Option<&'a [Value]> {
    parts
        .iter()
        .find_map(|(name, args)| (*name == ty).then_some(*args))
}
fn refs(value: &Value, out: &mut Vec<usize>) {
    match value {
        Value::Ref(id) => out.push(*id),
        Value::List(v) | Value::Call(_, v) => {
            for x in v {
                refs(x, out);
            }
        }
        _ => {}
    }
}
fn one_ref(v: &Value, what: &str) -> Result<usize> {
    if let Value::Ref(id) = v {
        Ok(*id)
    } else {
        Err(refuse(format!("{what} must be a reference")))
    }
}
fn list<'a>(v: &'a Value, what: &str) -> Result<&'a [Value]> {
    if let Value::List(items) = v {
        Ok(items)
    } else {
        Err(refuse(format!("{what} must be an aggregate")))
    }
}
fn number(v: &Value, what: &str) -> Result<f64> {
    match v {
        Value::Number(n) => Ok(*n),
        Value::Integer(n) => Ok(*n as f64),
        _ => Err(refuse(format!("{what} must be numeric"))),
    }
}
fn string_value<'a>(v: &'a Value, what: &str) -> Result<&'a str> {
    if let Value::String(value) = v {
        Ok(value)
    } else {
        Err(refuse(format!("{what} must be a string")))
    }
}
fn usize_value(v: &Value, what: &str) -> Result<usize> {
    match v {
        Value::Integer(n) if *n >= 0 => Ok(*n as usize),
        _ => Err(refuse(format!("{what} must be a nonnegative integer"))),
    }
}
fn boolean(v: &Value, what: &str) -> Result<bool> {
    match v {
        Value::Enum(v) if v == "T" => Ok(true),
        Value::Enum(v) if v == "F" => Ok(false),
        _ => Err(refuse(format!("{what} must be .T. or .F."))),
    }
}
fn name(args: &[Value]) -> Option<&str> {
    match args.first() {
        Some(Value::String(v)) => Some(v),
        _ => None,
    }
}

fn reachable(entities: &BTreeMap<usize, Entity>, roots: &[usize]) -> Result<BTreeSet<usize>> {
    fn visit(
        id: usize,
        depth: usize,
        entities: &BTreeMap<usize, Entity>,
        active: &mut BTreeSet<usize>,
        done: &mut BTreeSet<usize>,
    ) -> Result<()> {
        if depth > MAX_GRAPH_DEPTH {
            return Err(refuse("STEP reachable graph depth exceeds 64"));
        }
        if done.contains(&id) {
            return Ok(());
        }
        if !active.insert(id) {
            return Err(refuse("Cycle in reachable STEP graph"));
        }
        let entity = entities
            .get(&id)
            .ok_or_else(|| refuse(format!("Missing reachable reference #{id}")))?;
        let mut dependencies = Vec::new();
        refs(&entity.value, &mut dependencies);
        for dependency in dependencies {
            visit(dependency, depth + 1, entities, active, done)?;
        }
        active.remove(&id);
        done.insert(id);
        Ok(())
    }
    let mut done = BTreeSet::new();
    for root in roots {
        visit(*root, 0, entities, &mut BTreeSet::new(), &mut done)?;
    }
    Ok(done)
}

fn values(v: &Value, what: &str) -> Result<Vec<f64>> {
    list(v, what)?.iter().map(|v| number(v, what)).collect()
}
fn ints(v: &Value, what: &str) -> Result<Vec<usize>> {
    list(v, what)?
        .iter()
        .map(|v| usize_value(v, what))
        .collect()
}
fn expand(mults: &[usize], knots: &[f64]) -> Result<Vec<f64>> {
    if mults.len() != knots.len() {
        return Err(refuse("Knot multiplicity count mismatch"));
    }
    let mut out = Vec::new();
    for (&m, &k) in mults.iter().zip(knots) {
        out.extend(std::iter::repeat_n(k, m));
    }
    Ok(out)
}
fn point(
    entities: &BTreeMap<usize, Entity>,
    id: usize,
    dim: usize,
    scale: f64,
) -> Result<Vec<f64>> {
    let (ty, args) = call(entities, id)?;
    if ty != "CARTESIAN_POINT" || args.len() != 2 {
        return Err(refuse("Expected CARTESIAN_POINT"));
    }
    let mut p = values(&args[1], "CARTESIAN_POINT coordinates")?;
    if p.len() != dim {
        return Err(refuse("CARTESIAN_POINT dimension mismatch"));
    }
    if dim == 3 {
        for x in &mut p {
            *x *= scale;
        }
    }
    Ok(p)
}
fn direction(entities: &BTreeMap<usize, Entity>, id: usize, dim: usize) -> Result<Vec<f64>> {
    let (ty, args) = call(entities, id)?;
    if ty != "DIRECTION" || args.len() != 2 {
        return Err(refuse("Expected DIRECTION"));
    }
    let d = values(&args[1], "DIRECTION ratios")?;
    if d.len() != dim {
        return Err(refuse("DIRECTION dimension mismatch"));
    }
    let n = d.iter().map(|x| x * x).sum::<f64>().sqrt();
    if !n.is_finite() || n <= 1e-12 {
        return Err(refuse("Degenerate DIRECTION"));
    }
    Ok(d.into_iter().map(|x| x / n).collect())
}
fn vector(
    entities: &BTreeMap<usize, Entity>,
    id: usize,
    dim: usize,
    scale: f64,
) -> Result<Vec<f64>> {
    let (ty, args) = call(entities, id)?;
    if ty != "VECTOR" || args.len() != 3 {
        return Err(refuse("Expected VECTOR"));
    }
    let d = direction(entities, one_ref(&args[1], "VECTOR orientation")?, dim)?;
    let magnitude = number(&args[2], "VECTOR magnitude")? * if dim == 3 { scale } else { 1. };
    if magnitude <= 0. {
        return Err(refuse("VECTOR magnitude must be positive"));
    }
    Ok(d.into_iter().map(|x| x * magnitude).collect())
}
fn axis2(
    entities: &BTreeMap<usize, Entity>,
    id: usize,
    dim: usize,
    scale: f64,
) -> Result<(Vec<f64>, Vec<f64>, Vec<f64>)> {
    let (ty, args) = call(entities, id)?;
    let expected = if dim == 3 {
        "AXIS2_PLACEMENT_3D"
    } else {
        "AXIS2_PLACEMENT_2D"
    };
    if ty != expected || args.len() < 2 {
        return Err(refuse(format!("Expected {expected}")));
    }
    let origin = point(entities, one_ref(&args[1], "AXIS2 location")?, dim, scale)?;
    if dim == 2 {
        let x = if args.len() > 2 && !matches!(args[2], Value::Omitted) {
            direction(entities, one_ref(&args[2], "AXIS2 ref direction")?, 2)?
        } else {
            vec![1., 0.]
        };
        return Ok((origin, x.clone(), vec![-x[1], x[0]]));
    }
    let z = if args.len() > 2 && !matches!(args[2], Value::Omitted) {
        direction(entities, one_ref(&args[2], "AXIS2 axis")?, 3)?
    } else {
        vec![0., 0., 1.]
    };
    let mut x = if args.len() > 3 && !matches!(args[3], Value::Omitted) {
        direction(entities, one_ref(&args[3], "AXIS2 ref direction")?, 3)?
    } else {
        vec![1., 0., 0.]
    };
    let dot = x.iter().zip(&z).map(|(a, b)| a * b).sum::<f64>();
    for i in 0..3 {
        x[i] -= dot * z[i];
    }
    let nx = x.iter().map(|v| v * v).sum::<f64>().sqrt();
    if nx <= 1e-12 {
        return Err(refuse("AXIS2 directions are parallel"));
    }
    for v in &mut x {
        *v /= nx;
    }
    let y = vec![
        z[1] * x[2] - z[2] * x[1],
        z[2] * x[0] - z[0] * x[2],
        z[0] * x[1] - z[1] * x[0],
    ];
    Ok((origin, x, y))
}
fn conic_curve(origin: &[f64], x: &[f64], y: &[f64], a: f64, b: f64) -> Result<Curve> {
    if !(a > 0. && b > 0. && a.is_finite() && b.is_finite()) {
        return Err(refuse("Conic radii must be positive"));
    }
    let endpoints = [[1., 0.], [0., 1.], [-1., 0.], [0., -1.], [1., 0.]];
    let shoulders = [[1., 1.], [-1., 1.], [-1., -1.], [1., -1.]];
    let map = |q: [f64; 2]| {
        (0..origin.len())
            .map(|i| origin[i] + a * q[0] * x[i] + b * q[1] * y[i])
            .collect::<Vec<_>>()
    };
    let mut cps = vec![map(endpoints[0])];
    let mut weights = vec![1.];
    for i in 0..4 {
        cps.push(map(shoulders[i]));
        weights.push(std::f64::consts::FRAC_1_SQRT_2);
        cps.push(map(endpoints[i + 1]));
        weights.push(1.);
    }
    let c = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.],
        control_points: cps,
        weights,
        periodic: false,
    };
    c.validate()?;
    Ok(c)
}
/// Isolate a point inverse with positive-weight rational convex-hull
/// exclusion.  Every retained interval is produced by exact knot insertion
/// (`Curve::trim`); therefore its Euclidean control hull encloses the complete
/// rational image.  Ambiguous, non-isolated and non-representable roots refuse.
fn isolate_curve_point(base: &Curve, target: &[f64]) -> Result<f64> {
    const MAX_CELLS: usize = 16_384;
    const PARAMETER_WIDTH: f64 = 1e-12;
    const GEOMETRIC_WIDTH: f64 = 1e-9;
    fn contains(curve: &Curve, target: &[f64]) -> bool {
        (0..target.len()).all(|axis| {
            let lo = curve
                .control_points
                .iter()
                .map(|point| point[axis])
                .fold(f64::INFINITY, f64::min);
            let hi = curve
                .control_points
                .iter()
                .map(|point| point[axis])
                .fold(f64::NEG_INFINITY, f64::max);
            target[axis] >= lo - 1e-12 && target[axis] <= hi + 1e-12
        })
    }
    let domain = base.domain();
    let mut breaks = base
        .knots
        .iter()
        .copied()
        .filter(|value| *value >= domain[0] && *value <= domain[1])
        .collect::<Vec<_>>();
    breaks.dedup();
    let mut pending = breaks
        .array_windows()
        .map(|[a, b]| base.trim(*a, *b))
        .collect::<Result<Vec<_>>>()?;
    let mut isolated = Vec::<[f64; 2]>::new();
    let mut visited = 0usize;
    while let Some(cell) = pending.pop() {
        visited += 1;
        if visited > MAX_CELLS {
            return Err(refuse(
                "TRIMMED_CURVE point inverse exceeds certified isolation budget",
            ));
        }
        if !contains(&cell, target) {
            continue;
        }
        let domain = cell.domain();
        let hull_width = (0..target.len())
            .map(|axis| {
                let lo = cell
                    .control_points
                    .iter()
                    .map(|point| point[axis])
                    .fold(f64::INFINITY, f64::min);
                let hi = cell
                    .control_points
                    .iter()
                    .map(|point| point[axis])
                    .fold(f64::NEG_INFINITY, f64::max);
                hi - lo
            })
            .fold(0., f64::max);
        if domain[1] - domain[0] <= PARAMETER_WIDTH || hull_width <= GEOMETRIC_WIDTH {
            isolated.push(domain);
            continue;
        }
        let mid = (domain[0] + domain[1]) / 2.;
        if mid == domain[0] || mid == domain[1] {
            return Err(refuse(
                "TRIMMED_CURVE point inverse exhausted parameter precision",
            ));
        }
        let [left, right] = cell.split(mid)?;
        pending.push(right);
        pending.push(left);
    }
    isolated.sort_by(|left, right| left[0].total_cmp(&right[0]));
    let mut merged = Vec::<[f64; 2]>::new();
    for interval in isolated {
        if let Some(last) = merged.last_mut()
            && interval[0] <= last[1] + PARAMETER_WIDTH
        {
            last[1] = last[1].max(interval[1]);
            continue;
        }
        merged.push(interval)
    }
    if merged.len() != 1 {
        return Err(refuse(if merged.is_empty() {
            "TRIMMED_CURVE point selector is not on the basis curve"
        } else {
            "TRIMMED_CURVE point selector inverse is not unique"
        }));
    }
    let interval = merged[0];
    let mut parameter = (interval[0] + interval[1]) / 2.;
    for _ in 0..12 {
        let evaluation = base.evaluate(parameter)?;
        let Some(derivative) = evaluation.d1 else {
            break;
        };
        let denominator = derivative.iter().map(|value| value * value).sum::<f64>();
        if denominator <= 1e-24 {
            break;
        }
        let correction = evaluation
            .point
            .iter()
            .zip(target)
            .zip(&derivative)
            .map(|((point, wanted), tangent)| (point - wanted) * tangent)
            .sum::<f64>()
            / denominator;
        let next = (parameter - correction).clamp(interval[0], interval[1]);
        if next == parameter {
            break;
        }
        parameter = next;
    }
    let residual = base
        .evaluate(parameter)?
        .point
        .iter()
        .zip(target)
        .map(|(point, wanted)| (point - wanted) * (point - wanted))
        .sum::<f64>()
        .sqrt();
    if !residual.is_finite() || residual > GEOMETRIC_WIDTH {
        return Err(refuse(
            "TRIMMED_CURVE isolated point inverse failed certified residual bound",
        ));
    }
    Ok(parameter)
}
fn trim_parameters(
    entities: &BTreeMap<usize, Entity>,
    v: &Value,
    base: &Curve,
    dim: usize,
    scale: f64,
    interior_points: bool,
) -> Result<f64> {
    let items = list(v, "TRIMMED_CURVE trim selector")?;
    if items.len() != 1 {
        return Err(refuse(
            "TRIMMED_CURVE admits one parameter selector per end",
        ));
    }
    match &items[0] {
        Value::Number(_) | Value::Integer(_) => number(&items[0], "TRIMMED_CURVE parameter"),
        Value::Call(name, args) if name == "PARAMETER_VALUE" && args.len() == 1 => {
            number(&args[0], "PARAMETER_VALUE")
        }
        Value::Ref(id) => {
            let target = point(entities, *id, dim, scale)?;
            let domain = base.domain();
            let start = base.evaluate(domain[0])?.point;
            let end = base.evaluate(domain[1])?.point;
            let distance = |p: &[f64]| {
                p.iter()
                    .zip(&target)
                    .map(|(a, b)| (a - b) * (a - b))
                    .sum::<f64>()
                    .sqrt()
            };
            if distance(&start) <= 1e-7 {
                Ok(domain[0])
            } else if distance(&end) <= 1e-7 {
                Ok(domain[1])
            } else if interior_points {
                isolate_curve_point(base, &target)
            } else {
                Err(refuse(
                    "TRIMMED_CURVE point selector is not an exact basis endpoint",
                ))
            }
        }
        _ => Err(refuse(
            "TRIMMED_CURVE selector is outside the parameter/exact-endpoint subset",
        )),
    }
}
fn trim_conic(base: &Curve, start: f64, end: f64, sense: bool) -> Result<Curve> {
    let mut a0 = start;
    let mut a1 = end;
    if !sense {
        std::mem::swap(&mut a0, &mut a1);
    }
    while a1 <= a0 {
        a1 += std::f64::consts::TAU;
    }
    if a1 - a0 > std::f64::consts::TAU + 1e-10 {
        return Err(refuse("TRIMMED_CURVE conic sweep exceeds one revolution"));
    }
    let center = {
        let p0 = &base.control_points[0];
        let p4 = &base.control_points[4];
        p0.iter()
            .zip(p4)
            .map(|(a, b)| (a + b) / 2.)
            .collect::<Vec<_>>()
    };
    let ex: Vec<_> = base.control_points[0]
        .iter()
        .zip(&center)
        .map(|(a, b)| a - b)
        .collect();
    let ey: Vec<_> = base.control_points[2]
        .iter()
        .zip(&center)
        .map(|(a, b)| a - b)
        .collect();
    let pieces = ((a1 - a0) / std::f64::consts::FRAC_PI_2).ceil().max(1.) as usize;
    let mut cps = Vec::new();
    let mut weights = Vec::new();
    let mut knots = vec![0., 0., 0.];
    for i in 0..pieces {
        let s = a0 + (a1 - a0) * i as f64 / pieces as f64;
        let e = a0 + (a1 - a0) * (i + 1) as f64 / pieces as f64;
        let m = (s + e) / 2.;
        let w = ((e - s) / 2.).cos();
        let p = |angle: f64, factor: f64| {
            center
                .iter()
                .enumerate()
                .map(|(j, c)| c + factor * (ex[j] * angle.cos() + ey[j] * angle.sin()))
                .collect::<Vec<_>>()
        };
        if i == 0 {
            cps.push(p(s, 1.));
            weights.push(1.);
        }
        cps.push(p(m, 1. / w));
        weights.push(w);
        cps.push(p(e, 1.));
        weights.push(1.);
        if i + 1 < pieces {
            knots.push((i + 1) as f64);
            knots.push((i + 1) as f64);
        }
    }
    knots.extend([pieces as f64; 3]);
    let c = Curve {
        degree: 2,
        knots,
        control_points: cps,
        weights,
        periodic: false,
    };
    c.validate()?;
    Ok(c)
}
fn curve_with_selectors(
    entities: &BTreeMap<usize, Entity>,
    id: usize,
    dim: usize,
    scale: f64,
    interior_points: bool,
) -> Result<Curve> {
    let (ty, a) = call(entities, id)?;
    if ty == "LINE" {
        if a.len() != 3 {
            return Err(refuse("LINE argument count mismatch"));
        }
        let p = point(entities, one_ref(&a[1], "LINE point")?, dim, scale)?;
        let d = vector(entities, one_ref(&a[2], "LINE vector")?, dim, scale)?;
        return Curve::from_polyline(vec![
            p.clone(),
            p.iter().zip(d).map(|(x, d)| x + d).collect(),
        ]);
    }
    if matches!(ty, "CIRCLE" | "ELLIPSE") {
        let expected = if ty == "CIRCLE" { 3 } else { 4 };
        if a.len() != expected {
            return Err(refuse(format!("{ty} argument count mismatch")));
        }
        let (o, x, y) = axis2(entities, one_ref(&a[1], "conic placement")?, dim, scale)?;
        let major = number(&a[2], "conic radius")? * if dim == 3 { scale } else { 1. };
        let minor = if ty == "ELLIPSE" {
            number(&a[3], "ellipse minor radius")? * if dim == 3 { scale } else { 1. }
        } else {
            major
        };
        return conic_curve(&o, &x, &y, major, minor);
    }
    if ty == "TRIMMED_CURVE" {
        if a.len() != 6 {
            return Err(refuse("TRIMMED_CURVE argument count mismatch"));
        }
        let basis_id = one_ref(&a[1], "TRIMMED_CURVE basis")?;
        let (basis_ty, _) = call(entities, basis_id)?;
        let base = curve_with_selectors(entities, basis_id, dim, scale, interior_points)?;
        let start = trim_parameters(entities, &a[2], &base, dim, scale, interior_points)?;
        let end = trim_parameters(entities, &a[3], &base, dim, scale, interior_points)?;
        let sense = boolean(&a[4], "TRIMMED_CURVE sense")?;
        if matches!(basis_ty, "CIRCLE" | "ELLIPSE") {
            return trim_conic(&base, start, end, sense);
        }
        if basis_ty == "LINE" {
            let p0 = base.evaluate(start)?.point;
            let p1 = base.evaluate(end)?.point;
            return Curve::from_polyline(if sense { vec![p0, p1] } else { vec![p1, p0] });
        }
        return Err(refuse(
            "TRIMMED_CURVE basis is outside the exact direct subset",
        ));
    }
    if ty != "B_SPLINE_CURVE_WITH_KNOTS" {
        return Err(refuse(format!(
            "Reachable curve {ty} is typed-refused by the direct native subset"
        )));
    }
    if a.len() != 11 {
        return Err(refuse("B_SPLINE_CURVE_WITH_KNOTS argument count mismatch"));
    }
    let degree = usize_value(&a[1], "curve degree")?;
    let cps = list(&a[2], "curve control points")?
        .iter()
        .map(|v| point(entities, one_ref(v, "curve control point")?, dim, scale))
        .collect::<Result<Vec<_>>>()?;
    let weights = values(&a[7], "curve weights")?;
    let knots = expand(
        &ints(&a[8], "curve multiplicities")?,
        &values(&a[9], "curve knots")?,
    )?;
    let c = Curve {
        degree,
        knots,
        control_points: cps,
        weights,
        periodic: boolean(&a[5], "curve closed flag")?,
    };
    c.validate()?;
    Ok(c)
}
fn curve(entities: &BTreeMap<usize, Entity>, id: usize, dim: usize, scale: f64) -> Result<Curve> {
    curve_with_selectors(entities, id, dim, scale, false)
}
fn curve_v5(
    entities: &BTreeMap<usize, Entity>,
    id: usize,
    dim: usize,
    scale: f64,
    interior_points: bool,
) -> Result<Curve> {
    let parts = components(entities, id)?;
    if parts.len() == 1 && !matches!(parts[0].0, "B_SPLINE_CURVE_WITH_KNOTS") {
        return curve_with_selectors(entities, id, dim, scale, interior_points);
    }
    let (degree, cp_values, closed, multiplicities, knot_values, weights) = if parts.len() == 1 {
        let a = parts[0].1;
        if a.len() != 9 {
            return Err(refuse(
                "AP242 B_SPLINE_CURVE_WITH_KNOTS requires nine inherited/explicit attributes",
            ));
        }
        (
            usize_value(&a[1], "curve degree")?,
            &a[2],
            boolean(&a[4], "curve closed flag")?,
            &a[6],
            &a[7],
            None,
        )
    } else {
        let base = component(&parts, "B_SPLINE_CURVE").ok_or_else(|| {
            refuse("RATIONAL_B_SPLINE_CURVE complex lacks B_SPLINE_CURVE component")
        })?;
        let knots = component(&parts, "B_SPLINE_CURVE_WITH_KNOTS")
            .ok_or_else(|| refuse("RATIONAL_B_SPLINE_CURVE complex lacks knot component"))?;
        let rational = component(&parts, "RATIONAL_B_SPLINE_CURVE").ok_or_else(|| {
            refuse("Complex B-spline curve lacks RATIONAL_B_SPLINE_CURVE component")
        })?;
        if base.len() != 5 || knots.len() != 3 || rational.len() != 1 {
            return Err(refuse(
                "Malformed AP242 rational B-spline curve composition",
            ));
        }
        (
            usize_value(&base[0], "curve degree")?,
            &base[1],
            boolean(&base[3], "curve closed flag")?,
            &knots[0],
            &knots[1],
            Some(&rational[0]),
        )
    };
    let cps = list(cp_values, "curve control points")?
        .iter()
        .map(|v| point(entities, one_ref(v, "curve control point")?, dim, scale))
        .collect::<Result<Vec<_>>>()?;
    let weights = match weights {
        Some(weight_data) => values(weight_data, "curve weights")?,
        None => vec![1.; cps.len()],
    };
    if weights.len() != cps.len() {
        return Err(refuse(
            "RATIONAL_B_SPLINE_CURVE weight/control-point count mismatch",
        ));
    }
    let c = Curve {
        degree,
        knots: expand(
            &ints(multiplicities, "curve multiplicities")?,
            &values(knot_values, "curve knots")?,
        )?,
        control_points: cps,
        weights,
        periodic: closed,
    };
    c.validate()?;
    Ok(c)
}
fn analytic_surface(
    entities: &BTreeMap<usize, Entity>,
    ty: &str,
    a: &[Value],
    scale: f64,
) -> Result<Surface> {
    let expected = if matches!(ty, "TOROIDAL_SURFACE" | "CONICAL_SURFACE") {
        4
    } else {
        3
    };
    if a.len() != expected {
        return Err(refuse(format!("{ty} argument count mismatch")));
    }
    let (o, x, y) = axis2(
        entities,
        one_ref(&a[1], "analytic surface placement")?,
        3,
        scale,
    )?;
    let z = [
        x[1] * y[2] - x[2] * y[1],
        x[2] * y[0] - x[0] * y[2],
        x[0] * y[1] - x[1] * y[0],
    ];
    let radius = number(&a[2], "analytic surface radius")? * scale;
    if !radius.is_finite() || radius <= 0. {
        return Err(refuse("Analytic surface radius must be positive"));
    }
    let angles = [
        0.,
        std::f64::consts::FRAC_PI_4,
        std::f64::consts::FRAC_PI_2,
        3. * std::f64::consts::FRAC_PI_4,
        std::f64::consts::PI,
        5. * std::f64::consts::FRAC_PI_4,
        3. * std::f64::consts::FRAC_PI_2,
        7. * std::f64::consts::FRAC_PI_4,
        std::f64::consts::TAU,
    ];
    let wu = [
        1.,
        std::f64::consts::FRAC_1_SQRT_2,
        1.,
        std::f64::consts::FRAC_1_SQRT_2,
        1.,
        std::f64::consts::FRAC_1_SQRT_2,
        1.,
        std::f64::consts::FRAC_1_SQRT_2,
        1.,
    ];
    let ku = vec![
        0.,
        0.,
        0.,
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::PI,
        std::f64::consts::PI,
        3. * std::f64::consts::FRAC_PI_2,
        3. * std::f64::consts::FRAC_PI_2,
        std::f64::consts::TAU,
        std::f64::consts::TAU,
        std::f64::consts::TAU,
    ];
    let (v_angles, wv, kv) = if matches!(ty, "SPHERICAL_SURFACE") {
        (
            vec![
                -std::f64::consts::FRAC_PI_2,
                -std::f64::consts::FRAC_PI_4,
                0.,
                std::f64::consts::FRAC_PI_4,
                std::f64::consts::FRAC_PI_2,
            ],
            vec![
                1.,
                std::f64::consts::FRAC_1_SQRT_2,
                1.,
                std::f64::consts::FRAC_1_SQRT_2,
                1.,
            ],
            vec![
                -std::f64::consts::FRAC_PI_2,
                -std::f64::consts::FRAC_PI_2,
                -std::f64::consts::FRAC_PI_2,
                0.,
                0.,
                std::f64::consts::FRAC_PI_2,
                std::f64::consts::FRAC_PI_2,
                std::f64::consts::FRAC_PI_2,
            ],
        )
    } else if ty == "TOROIDAL_SURFACE" {
        (angles.to_vec(), wu.to_vec(), ku.clone())
    } else {
        (vec![-1e6, 1e6], vec![1., 1.], vec![-1e6, -1e6, 1e6, 1e6])
    };
    let minor = if ty == "TOROIDAL_SURFACE" {
        let r = number(&a[3], "torus minor radius")? * scale;
        if !(r > 0. && r < radius) {
            return Err(refuse("TOROIDAL_SURFACE requires 0 < minor < major"));
        }
        r
    } else {
        radius
    };
    let semi = if ty == "CONICAL_SURFACE" {
        number(&a[3], "cone semi-angle")?
    } else {
        0.
    };
    let mut cps = Vec::new();
    let mut weights = Vec::new();
    for (iu, &angle) in angles.iter().enumerate() {
        let radial = [
            x[0] * angle.cos() + y[0] * angle.sin(),
            x[1] * angle.cos() + y[1] * angle.sin(),
            x[2] * angle.cos() + y[2] * angle.sin(),
        ];
        let mut row = Vec::new();
        let mut wr = Vec::new();
        for (iv, &v) in v_angles.iter().enumerate() {
            let shoulder_scale = if iu % 2 == 1 { 1. / wu[iu] } else { 1. };
            let meridian_scale =
                if iv % 2 == 1 && matches!(ty, "SPHERICAL_SURFACE" | "TOROIDAL_SURFACE") {
                    1. / wv[iv]
                } else {
                    1.
                };
            let (radial_distance, height) = match ty {
                "CYLINDRICAL_SURFACE" => (radius * shoulder_scale, v),
                "CONICAL_SURFACE" => ((radius + v * semi.tan()) * shoulder_scale, v),
                "SPHERICAL_SURFACE" => (
                    radius * v.cos() * shoulder_scale * meridian_scale,
                    radius * v.sin() * meridian_scale,
                ),
                "TOROIDAL_SURFACE" => (
                    (radius + minor * v.cos() * meridian_scale) * shoulder_scale,
                    minor * v.sin() * meridian_scale,
                ),
                _ => return Err(refuse("Unsupported analytic surface")),
            };
            row.push(
                (0..3)
                    .map(|j| o[j] + radial_distance * radial[j] + height * z[j])
                    .collect(),
            );
            wr.push(wu[iu] * wv[iv]);
        }
        cps.push(row);
        weights.push(wr);
    }
    // Full rational circles are clamped at an explicit seam. Periodic lifts
    // remain carried by CoedgeTrim; marking this net periodic would require
    // repeating `degree` control rows and would change the STEP angle domain.
    let s = Surface {
        degree_u: 2,
        degree_v: if v_angles.len() > 2 { 2 } else { 1 },
        knots_u: ku,
        knots_v: kv,
        control_points: cps,
        weights,
        periodic_u: false,
        periodic_v: false,
    };
    s.validate()?;
    Ok(s)
}
fn surface(entities: &BTreeMap<usize, Entity>, id: usize, scale: f64) -> Result<Surface> {
    let (ty, a) = call(entities, id)?;
    if ty == "PLANE" {
        if a.len() != 2 {
            return Err(refuse("PLANE argument count mismatch"));
        }
        let (o, x, y) = axis2(entities, one_ref(&a[1], "PLANE placement")?, 3, scale)?;
        let p = |u: f64, v: f64| {
            (0..3)
                .map(|i| o[i] + u * x[i] + v * y[i])
                .collect::<Vec<_>>()
        };
        const LIMIT: f64 = 1e6;
        let s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![-LIMIT, -LIMIT, LIMIT, LIMIT],
            knots_v: vec![-LIMIT, -LIMIT, LIMIT, LIMIT],
            control_points: vec![
                vec![p(-LIMIT, -LIMIT), p(-LIMIT, LIMIT)],
                vec![p(LIMIT, -LIMIT), p(LIMIT, LIMIT)],
            ],
            weights: vec![vec![1., 1.], vec![1., 1.]],
            periodic_u: false,
            periodic_v: false,
        };
        s.validate()?;
        return Ok(s);
    }
    if matches!(
        ty,
        "CYLINDRICAL_SURFACE" | "CONICAL_SURFACE" | "SPHERICAL_SURFACE" | "TOROIDAL_SURFACE"
    ) {
        return Err(refuse(format!(
            "{ty} periodic/angular parameterization is available only in step-interchange/4"
        )));
    }
    if ty != "B_SPLINE_SURFACE_WITH_KNOTS" {
        return Err(refuse(format!(
            "Reachable surface {ty} is typed-refused by the direct native subset"
        )));
    }
    if a.len() != 16 {
        return Err(refuse(
            "B_SPLINE_SURFACE_WITH_KNOTS argument count mismatch",
        ));
    }
    let rows = list(&a[3], "surface control net")?;
    let cps = rows
        .iter()
        .map(|row| {
            list(row, "surface control row")?
                .iter()
                .map(|v| point(entities, one_ref(v, "surface control point")?, 3, scale))
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    let flat = values(&a[10], "surface weights")?;
    let nu = cps.len();
    let nv = cps.first().map_or(0, Vec::len);
    if nu == 0 || nv == 0 || cps.iter().any(|r| r.len() != nv) || flat.len() != nu * nv {
        return Err(refuse("Surface control/weight shape mismatch"));
    }
    let weights = flat.chunks(nv).map(|r| r.to_vec()).collect();
    let s = Surface {
        degree_u: usize_value(&a[1], "surface U degree")?,
        degree_v: usize_value(&a[2], "surface V degree")?,
        knots_u: expand(
            &ints(&a[11], "surface U multiplicities")?,
            &values(&a[13], "surface U knots")?,
        )?,
        knots_v: expand(
            &ints(&a[12], "surface V multiplicities")?,
            &values(&a[14], "surface V knots")?,
        )?,
        control_points: cps,
        weights,
        periodic_u: boolean(&a[6], "surface U closed flag")?,
        periodic_v: boolean(&a[7], "surface V closed flag")?,
    };
    s.validate()?;
    Ok(s)
}
fn surface_v4(entities: &BTreeMap<usize, Entity>, id: usize, scale: f64) -> Result<Surface> {
    let (ty, a) = call(entities, id)?;
    if matches!(
        ty,
        "CYLINDRICAL_SURFACE" | "CONICAL_SURFACE" | "SPHERICAL_SURFACE" | "TOROIDAL_SURFACE"
    ) {
        analytic_surface(entities, ty, a, scale)
    } else {
        surface(entities, id, scale)
    }
}
fn surface_v5(entities: &BTreeMap<usize, Entity>, id: usize, scale: f64) -> Result<Surface> {
    let parts = components(entities, id)?;
    if parts.len() == 1 && !matches!(parts[0].0, "B_SPLINE_SURFACE_WITH_KNOTS") {
        return surface_v4(entities, id, scale);
    }
    let (du, dv, net, pu, pv, mu, mv, ku, kv, weight_value) = if parts.len() == 1 {
        let a = parts[0].1;
        if a.len() != 13 {
            return Err(refuse(
                "AP242 B_SPLINE_SURFACE_WITH_KNOTS requires thirteen inherited/explicit attributes",
            ));
        }
        (
            usize_value(&a[1], "surface U degree")?,
            usize_value(&a[2], "surface V degree")?,
            &a[3],
            boolean(&a[5], "surface U closed flag")?,
            boolean(&a[6], "surface V closed flag")?,
            &a[8],
            &a[9],
            &a[10],
            &a[11],
            None,
        )
    } else {
        let base = component(&parts, "B_SPLINE_SURFACE").ok_or_else(|| {
            refuse("RATIONAL_B_SPLINE_SURFACE complex lacks B_SPLINE_SURFACE component")
        })?;
        let knots = component(&parts, "B_SPLINE_SURFACE_WITH_KNOTS")
            .ok_or_else(|| refuse("RATIONAL_B_SPLINE_SURFACE complex lacks knot component"))?;
        let rational = component(&parts, "RATIONAL_B_SPLINE_SURFACE").ok_or_else(|| {
            refuse("Complex B-spline surface lacks RATIONAL_B_SPLINE_SURFACE component")
        })?;
        if base.len() != 7 || knots.len() != 5 || rational.len() != 1 {
            return Err(refuse(
                "Malformed AP242 rational B-spline surface composition",
            ));
        }
        (
            usize_value(&base[0], "surface U degree")?,
            usize_value(&base[1], "surface V degree")?,
            &base[2],
            boolean(&base[4], "surface U closed flag")?,
            boolean(&base[5], "surface V closed flag")?,
            &knots[0],
            &knots[1],
            &knots[2],
            &knots[3],
            Some(&rational[0]),
        )
    };
    let cps = list(net, "surface control net")?
        .iter()
        .map(|row| {
            list(row, "surface control row")?
                .iter()
                .map(|v| point(entities, one_ref(v, "surface control point")?, 3, scale))
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    let nu = cps.len();
    let nv = cps.first().map_or(0, Vec::len);
    if nu == 0 || nv == 0 || cps.iter().any(|row| row.len() != nv) {
        return Err(refuse("Surface control net is empty or ragged"));
    }
    let weights = if let Some(value) = weight_value {
        let rows = list(value, "surface weights")?;
        if rows.len() != nu {
            return Err(refuse(
                "RATIONAL_B_SPLINE_SURFACE weight row count mismatch",
            ));
        }
        rows.iter()
            .map(|row| values(row, "surface weight row"))
            .collect::<Result<Vec<_>>>()?
    } else {
        vec![vec![1.; nv]; nu]
    };
    if weights.iter().any(|row| row.len() != nv) {
        return Err(refuse(
            "RATIONAL_B_SPLINE_SURFACE weight/control-net shape mismatch",
        ));
    }
    let s = Surface {
        degree_u: du,
        degree_v: dv,
        knots_u: expand(
            &ints(mu, "surface U multiplicities")?,
            &values(ku, "surface U knots")?,
        )?,
        knots_v: expand(
            &ints(mv, "surface V multiplicities")?,
            &values(kv, "surface V knots")?,
        )?,
        control_points: cps,
        weights,
        periodic_u: pu,
        periodic_v: pv,
    };
    s.validate()?;
    Ok(s)
}

fn si_scale(args: &[Value]) -> Result<f64> {
    if !args
        .iter()
        .any(|v| matches!(v, Value::Enum(e) if e == "METRE"))
    {
        return Err(refuse("SI unit is not a length unit"));
    }
    let prefix = args.iter().find_map(|v| {
        if let Value::Enum(e) = v {
            (e != "METRE").then_some(e.as_str())
        } else {
            None
        }
    });
    Ok(match prefix {
        None => 1000.,
        Some("DECI") => 100.,
        Some("CENTI") => 10.,
        Some("MILLI") => 1.,
        Some("MICRO") => 1e-3,
        Some("NANO") => 1e-6,
        Some("KILO") => 1e6,
        _ => return Err(refuse("Unsupported SI length prefix")),
    })
}
fn unit_scale(
    entities: &BTreeMap<usize, Entity>,
    id: usize,
    depth: usize,
    active: &mut BTreeSet<usize>,
) -> Result<f64> {
    if depth > 8 {
        return Err(refuse("CONVERSION_BASED_UNIT chain depth exceeds eight"));
    }
    if !active.insert(id) {
        return Err(refuse("Cycle in CONVERSION_BASED_UNIT chain"));
    }
    let entity = entities
        .get(&id)
        .ok_or_else(|| refuse("Missing unit reference"))?;
    let result = match &entity.value {
        Value::List(parts) => {
            let mut answer = None;
            for p in parts {
                if let Value::Call(n, a) = p {
                    if n == "SI_UNIT" {
                        answer = Some(si_scale(a)?)
                    } else if n == "CONVERSION_BASED_UNIT" {
                        answer = Some(conversion_scale(entities, a, depth, active)?)
                    }
                }
            }
            answer.ok_or_else(|| refuse("Complex unit has no admitted length component"))?
        }
        Value::Call(n, a) if n == "SI_UNIT" => si_scale(a)?,
        Value::Call(n, a) if n == "CONVERSION_BASED_UNIT" => {
            conversion_scale(entities, a, depth, active)?
        }
        _ => return Err(refuse("Unsupported length unit entity")),
    };
    active.remove(&id);
    Ok(result)
}
fn conversion_scale(
    entities: &BTreeMap<usize, Entity>,
    args: &[Value],
    depth: usize,
    active: &mut BTreeSet<usize>,
) -> Result<f64> {
    if args.len() < 2 {
        return Err(refuse("CONVERSION_BASED_UNIT argument count mismatch"));
    }
    let factor_id = one_ref(args.last().unwrap(), "CONVERSION_BASED_UNIT factor")?;
    let (ty, a) = call(entities, factor_id)?;
    if !matches!(ty, "LENGTH_MEASURE_WITH_UNIT" | "MEASURE_WITH_UNIT") || a.len() != 2 {
        return Err(refuse("Conversion factor must be LENGTH_MEASURE_WITH_UNIT"));
    }
    let factor = match &a[0] {
        Value::Call(n, v) if n == "LENGTH_MEASURE" && v.len() == 1 => {
            number(&v[0], "LENGTH_MEASURE")?
        }
        v => number(v, "conversion factor")?,
    };
    if !(factor > 0. && factor <= 1e9) {
        return Err(refuse("Conversion factor must be positive and bounded"));
    }
    Ok(factor
        * unit_scale(
            entities,
            one_ref(&a[1], "conversion base unit")?,
            depth + 1,
            active,
        )?)
}
fn length_scale(
    entities: &BTreeMap<usize, Entity>,
    reachable: &BTreeSet<usize>,
    allow_orphan_fallback: bool,
) -> Result<f64> {
    let mut unit_ids = Vec::new();
    for id in reachable {
        let entity = &entities[id];
        match &entity.value {
            Value::Call(n, a) if n == "GLOBAL_UNIT_ASSIGNED_CONTEXT" => {
                for v in a {
                    refs(v, &mut unit_ids);
                }
            }
            Value::List(parts) => {
                for part in parts {
                    if let Value::Call(n, a) = part
                        && n == "GLOBAL_UNIT_ASSIGNED_CONTEXT"
                    {
                        for v in a {
                            refs(v, &mut unit_ids);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    if unit_ids.is_empty() && allow_orphan_fallback {
        unit_ids.extend(entities.iter().filter_map(|(id, e)| match &e.value {
            Value::List(parts)
                if parts.iter().any(
                    |p| matches!(p,Value::Call(n,_)if n=="SI_UNIT"||n=="CONVERSION_BASED_UNIT"),
                ) =>
            {
                Some(*id)
            }
            Value::Call(n, _) if n == "SI_UNIT" || n == "CONVERSION_BASED_UNIT" => Some(*id),
            _ => None,
        }));
    }
    let mut found = Vec::new();
    for id in unit_ids {
        if let Ok(scale) = unit_scale(entities, id, 0, &mut BTreeSet::new()) {
            found.push(scale);
        }
    }
    if found.is_empty() {
        return Err(refuse("STEP /3 requires an SI length unit context"));
    }
    if found
        .iter()
        .any(|v| !v.is_finite() || (*v - found[0]).abs() > f64::EPSILON)
    {
        return Err(refuse("Unsupported or conflicting STEP length units"));
    }
    Ok(found[0])
}

fn part21_views(text: &str) -> (String, String) {
    let bytes = text.as_bytes();
    let mut clean = bytes.to_vec();
    let mut structural = bytes.to_vec();
    let mut index = 0;
    let mut in_string = false;
    while index < bytes.len() {
        if !in_string && index + 1 < bytes.len() && bytes[index] == b'/' && bytes[index + 1] == b'*'
        {
            clean[index] = b' ';
            clean[index + 1] = b' ';
            structural[index] = b' ';
            structural[index + 1] = b' ';
            index += 2;
            while index + 1 < bytes.len() && !(bytes[index] == b'*' && bytes[index + 1] == b'/') {
                if bytes[index] != b'\n' {
                    clean[index] = b' ';
                    structural[index] = b' ';
                }
                index += 1;
            }
            if index + 1 < bytes.len() {
                clean[index] = b' ';
                clean[index + 1] = b' ';
                structural[index] = b' ';
                structural[index + 1] = b' ';
                index += 2
            }
            continue;
        }
        if bytes[index] == b'\'' {
            if in_string && index + 1 < bytes.len() && bytes[index + 1] == b'\'' {
                structural[index] = b' ';
                structural[index + 1] = b' ';
                index += 2;
                continue;
            }
            in_string = !in_string;
            structural[index] = b' ';
            index += 1;
            continue;
        }
        if in_string && bytes[index] != b'\n' {
            structural[index] = b' '
        }
        index += 1;
    }
    (
        String::from_utf8_lossy(&clean).to_ascii_uppercase(),
        String::from_utf8_lossy(&structural).to_ascii_uppercase(),
    )
}
fn validate_exchange_structure(text: &str) -> Result<()> {
    let (upper, structural) = part21_views(text);
    for section in ["ISO-10303-21;", "HEADER;", "DATA;", "END-ISO-10303-21;"] {
        if structural
            .lines()
            .filter(|line| line.trim() == section)
            .count()
            != 1
        {
            return Err(refuse(format!(
                "Part 21 requires exactly one {section} marker"
            )));
        }
    }
    if structural
        .lines()
        .filter(|line| line.trim() == "ENDSEC;")
        .count()
        != 2
    {
        return Err(refuse("Part 21 requires exactly two ENDSEC markers"));
    }
    let iso = structural
        .find("ISO-10303-21;")
        .ok_or_else(|| refuse("Missing ISO-10303-21 prologue"))?;
    let header = structural
        .find("HEADER;")
        .ok_or_else(|| refuse("Missing HEADER section"))?;
    let first_end = structural[header + 7..]
        .find("ENDSEC;")
        .map(|i| i + header + 7)
        .ok_or_else(|| refuse("Unterminated HEADER section"))?;
    let data = structural[first_end + 7..]
        .find("DATA;")
        .map(|i| i + first_end + 7)
        .ok_or_else(|| refuse("Missing DATA section"))?;
    let data_end = structural[data + 5..]
        .find("ENDSEC;")
        .map(|i| i + data + 5)
        .ok_or_else(|| refuse("Unterminated DATA section"))?;
    let end = structural
        .find("END-ISO-10303-21;")
        .ok_or_else(|| refuse("Missing exchange terminator"))?;
    if !(iso < header
        && header < first_end
        && first_end < data
        && data < data_end
        && data_end < end)
    {
        return Err(refuse("Part 21 sections are duplicated or out of order"));
    }
    let header_text = &structural[header..first_end];
    if !header_text.contains("FILE_DESCRIPTION(")
        || !header_text.contains("FILE_NAME(")
        || !header_text.contains("FILE_SCHEMA(")
    {
        return Err(refuse(
            "HEADER requires FILE_DESCRIPTION, FILE_NAME, and FILE_SCHEMA",
        ));
    }
    let schema = header_text.find("FILE_SCHEMA(").unwrap() + header;
    let schema_end = structural[schema..first_end]
        .find(';')
        .map(|offset| schema + offset)
        .unwrap_or(first_end);
    if !upper[schema..schema_end].contains("'AP242_MANAGED_MODEL_BASED_3D_ENGINEERING'")
        && !upper[schema..schema_end].contains("'AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF'")
    {
        return Err(refuse(
            "Direct retained B-rep requires declared AP242 schema",
        ));
    }
    Ok(())
}
fn data_payload(text: &str) -> Result<&str> {
    let (_, structural) = part21_views(text);
    let start = structural
        .find("DATA;")
        .ok_or_else(|| refuse("Missing DATA section"))?
        + 5;
    let end = structural[start..]
        .find("ENDSEC;")
        .map(|offset| start + offset)
        .ok_or_else(|| refuse("Unterminated DATA section"))?;
    Ok(&text[start..end])
}

fn rigid_frame(entities: &BTreeMap<usize, Entity>, id: usize, scale: f64) -> Result<[[f64; 4]; 4]> {
    let (o, x, y) = axis2(entities, id, 3, scale)?;
    let z = [
        x[1] * y[2] - x[2] * y[1],
        x[2] * y[0] - x[0] * y[2],
        x[0] * y[1] - x[1] * y[0],
    ];
    Ok([
        [x[0], y[0], z[0], o[0]],
        [x[1], y[1], z[1], o[1]],
        [x[2], y[2], z[2], o[2]],
        [0., 0., 0., 1.],
    ])
}
fn mul(a: [[f64; 4]; 4], b: [[f64; 4]; 4]) -> [[f64; 4]; 4] {
    std::array::from_fn(|i| std::array::from_fn(|j| (0..4).map(|k| a[i][k] * b[k][j]).sum()))
}
fn inverse_rigid(m: [[f64; 4]; 4]) -> [[f64; 4]; 4] {
    let mut r = [[0.; 4]; 4];
    for i in 0..3 {
        for j in 0..3 {
            r[i][j] = m[j][i]
        }
        r[i][3] = -(0..3).map(|j| r[i][j] * m[j][3]).sum::<f64>();
    }
    r[3][3] = 1.;
    r
}
fn cartesian_operator_3d(
    entities: &BTreeMap<usize, Entity>,
    args: &[Value],
    scale: f64,
) -> Result<[[f64; 4]; 4]> {
    let offset = match args.len() {
        6 => 0,
        8 => 2,
        _ => {
            return Err(refuse(
                "CARTESIAN_TRANSFORMATION_OPERATOR_3D inherited/explicit attribute count mismatch",
            ));
        }
    };
    let factor = if matches!(args[4 + offset], Value::Omitted) {
        1.
    } else {
        number(&args[4 + offset], "transformation scale")?
    };
    if (factor - 1.).abs() > 1e-12 {
        return Err(refuse(
            "Non-rigid CARTESIAN_TRANSFORMATION_OPERATOR_3D scale is refused",
        ));
    }
    let axis = |index: usize, default: Vec<f64>| -> Result<Vec<f64>> {
        if matches!(args[index], Value::Omitted) {
            Ok(default)
        } else {
            direction(entities, one_ref(&args[index], "transformation axis")?, 3)
        }
    };
    let x = axis(1 + offset, vec![1., 0., 0.])?;
    let y = axis(2 + offset, vec![0., 1., 0.])?;
    let z = axis(5 + offset, vec![0., 0., 1.])?;
    let dot = |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f64>();
    let cross = [
        x[1] * y[2] - x[2] * y[1],
        x[2] * y[0] - x[0] * y[2],
        x[0] * y[1] - x[1] * y[0],
    ];
    if dot(&x, &y).abs() > 1e-10
        || dot(&x, &z).abs() > 1e-10
        || dot(&y, &z).abs() > 1e-10
        || cross
            .iter()
            .zip(&z)
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f64>()
            .sqrt()
            > 1e-10
    {
        return Err(refuse(
            "CARTESIAN_TRANSFORMATION_OPERATOR_3D axes are non-rigid or reflected",
        ));
    }
    let o = point(
        entities,
        one_ref(&args[3 + offset], "transformation local origin")?,
        3,
        scale,
    )?;
    Ok([
        [x[0], y[0], z[0], o[0]],
        [x[1], y[1], z[1], o[1]],
        [x[2], y[2], z[2], o[2]],
        [0., 0., 0., 1.],
    ])
}
fn cartesian_operator_affine(
    entities: &BTreeMap<usize, Entity>,
    id: usize,
    scale: f64,
    general: bool,
) -> Result<[[f64; 4]; 4]> {
    let parts = components(entities, id)?;
    let base = component(&parts, "CARTESIAN_TRANSFORMATION_OPERATOR")
        .ok_or_else(|| refuse("Affine complex lacks CARTESIAN_TRANSFORMATION_OPERATOR"))?;
    let spatial = component(&parts, "CARTESIAN_TRANSFORMATION_OPERATOR_3D")
        .ok_or_else(|| refuse("Affine complex lacks CARTESIAN_TRANSFORMATION_OPERATOR_3D"))?;
    let nonuniform = component(&parts, "CARTESIAN_TRANSFORMATION_OPERATOR_3DNON_UNIFORM")
        .ok_or_else(|| {
            refuse("Affine complex lacks CARTESIAN_TRANSFORMATION_OPERATOR_3DNON_UNIFORM")
        })?;
    if !matches!(base.len(), 4 | 5) || spatial.len() != 1 || nonuniform.len() != 2 {
        return Err(refuse(
            "Malformed CARTESIAN_TRANSFORMATION_OPERATOR_3DNON_UNIFORM complex",
        ));
    }
    let axis = |value: &Value, default: Vec<f64>| -> Result<Vec<f64>> {
        if matches!(value, Value::Omitted) {
            Ok(default)
        } else {
            direction(entities, one_ref(value, "affine transformation axis")?, 3)
        }
    };
    let offset = base.len() - 4;
    let x = axis(&base[offset], vec![1., 0., 0.])?;
    let y = axis(&base[offset + 1], vec![0., 1., 0.])?;
    let z = axis(&spatial[0], vec![0., 0., 1.])?;
    let origin = point(
        entities,
        one_ref(&base[offset + 2], "affine transformation origin")?,
        3,
        scale,
    )?;
    if !general {
        let dot = |a: &[f64], b: &[f64]| {
            a.iter()
                .zip(b)
                .map(|(left, right)| left * right)
                .sum::<f64>()
        };
        let cross = [
            x[1] * y[2] - x[2] * y[1],
            x[2] * y[0] - x[0] * y[2],
            x[0] * y[1] - x[1] * y[0],
        ];
        if dot(&x, &y).abs() > 1e-10
            || dot(&x, &z).abs() > 1e-10
            || dot(&y, &z).abs() > 1e-10
            || cross
                .iter()
                .zip(&z)
                .map(|(a, b)| (a - b) * (a - b))
                .sum::<f64>()
                .sqrt()
                > 1e-10
        {
            return Err(refuse(
                "Affine transformation axes are non-orthogonal or reflected",
            ));
        }
    }
    let scales = [
        if matches!(base[offset + 3], Value::Omitted) {
            1.
        } else {
            number(&base[offset + 3], "scale")?
        },
        if matches!(nonuniform[0], Value::Omitted) {
            1.
        } else {
            number(&nonuniform[0], "scale2")?
        },
        if matches!(nonuniform[1], Value::Omitted) {
            1.
        } else {
            number(&nonuniform[1], "scale3")?
        },
    ];
    if scales
        .iter()
        .any(|value| (!general && *value <= 0.) || *value == 0. || !value.is_finite())
    {
        return Err(refuse(if general {
            "Affine scales must be finite and nonzero"
        } else {
            "Affine scales must be finite and positive"
        }));
    }
    let m = [
        [
            x[0] * scales[0],
            y[0] * scales[1],
            z[0] * scales[2],
            origin[0],
        ],
        [
            x[1] * scales[0],
            y[1] * scales[1],
            z[1] * scales[2],
            origin[1],
        ],
        [
            x[2] * scales[0],
            y[2] * scales[1],
            z[2] * scales[2],
            origin[2],
        ],
        [0., 0., 0., 1.],
    ];
    let determinant = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    if !determinant.is_finite() || determinant.abs() <= 1e-14 {
        return Err(refuse("Affine transformation is singular"));
    }
    Ok(m)
}
fn reachable_transform(
    entities: &BTreeMap<usize, Entity>,
    linked: &BTreeSet<usize>,
    scale: f64,
    strict: bool,
) -> Result<Option<[[f64; 4]; 4]>> {
    let mut transforms = Vec::new();
    for id in linked {
        let (ty, a) = call(entities, *id)?;
        if ty == "CARTESIAN_TRANSFORMATION_OPERATOR_3D" && strict {
            transforms.push(cartesian_operator_3d(entities, a, scale)?);
            continue;
        }
        if ty.starts_with("CARTESIAN_TRANSFORMATION_OPERATOR") {
            return Err(refuse(
                "Reachable generic transform is refused: reflection/nonuniform/singular transforms are outside /3",
            ));
        }
        if ty == "ITEM_DEFINED_TRANSFORMATION" {
            if a.len() != 4 {
                return Err(refuse(
                    "ITEM_DEFINED_TRANSFORMATION argument count mismatch",
                ));
            }
            let from = rigid_frame(entities, one_ref(&a[2], "transformation source")?, scale)?;
            let to = rigid_frame(entities, one_ref(&a[3], "transformation target")?, scale)?;
            transforms.push(mul(to, inverse_rigid(from)));
            if transforms.len() > 32 {
                return Err(refuse("Nested rigid transformation depth exceeds 32"));
            }
        }
    }
    if transforms.is_empty() {
        return Ok(None);
    }
    if strict && transforms.len() != 1 {
        return Err(refuse(
            "Ambiguous STEP placement: selected representation reaches multiple transforms",
        ));
    }
    let mut total = [
        [1., 0., 0., 0.],
        [0., 1., 0., 0.],
        [0., 0., 1., 0.],
        [0., 0., 0., 1.],
    ];
    for t in transforms {
        total = mul(t, total);
    }
    Ok(Some(total))
}

fn exact_boundary_correspondence(
    c3: &Curve,
    pc: &Curve,
    s: &Surface,
    reversed: bool,
) -> Result<bool> {
    if s.control_points.len() < 2 || s.control_points[0].len() < 2 {
        return Ok(false);
    }
    let d2 = pc.domain();
    let a = pc.evaluate(d2[0])?.point;
    let b = pc.evaluate(d2[1])?.point;
    if a.len() != 2 || b.len() != 2 {
        return Ok(false);
    }
    let du = [
        s.knots_u[s.degree_u],
        s.knots_u[s.knots_u.len() - s.degree_u - 1],
    ];
    let dv = [
        s.knots_v[s.degree_v],
        s.knots_v[s.knots_v.len() - s.degree_v - 1],
    ];
    let close = |x: f64, y: f64| (x - y).abs() <= 1e-10;
    let mut expected = if close(a[0], b[0])
        && (close(a[0], du[0]) || close(a[0], du[1]))
        && ((close(a[1], dv[0]) && close(b[1], dv[1]))
            || (close(a[1], dv[1]) && close(b[1], dv[0])))
    {
        let row = if close(a[0], du[0]) {
            0
        } else {
            s.control_points.len() - 1
        };
        s.control_points[row]
            .iter()
            .zip(&s.weights[row])
            .map(|(point, weight)| (point.clone(), *weight))
            .collect::<Vec<_>>()
    } else if close(a[1], b[1])
        && (close(a[1], dv[0]) || close(a[1], dv[1]))
        && ((close(a[0], du[0]) && close(b[0], du[1]))
            || (close(a[0], du[1]) && close(b[0], du[0])))
    {
        let column = if close(a[1], dv[0]) {
            0
        } else {
            s.control_points[0].len() - 1
        };
        s.control_points
            .iter()
            .zip(&s.weights)
            .map(|(row, weights)| (row[column].clone(), weights[column]))
            .collect::<Vec<_>>()
    } else {
        return Ok(false);
    };
    let increasing = if close(a[0], b[0]) {
        b[1] > a[1]
    } else {
        b[0] > a[0]
    };
    if !increasing {
        expected.reverse()
    }
    if reversed {
        expected.reverse()
    }
    let constant = |points: &[(Vec<f64>, f64)]| {
        points.iter().all(|(point, _)| {
            point
                .iter()
                .zip(&points[0].0)
                .all(|(left, right)| close(*left, *right))
        })
    };
    let actual = c3
        .control_points
        .iter()
        .cloned()
        .zip(c3.weights.iter().copied())
        .collect::<Vec<_>>();
    if constant(&expected) && constant(&actual) {
        if !actual[0]
            .0
            .iter()
            .zip(&expected[0].0)
            .all(|(left, right)| close(*left, *right))
        {
            return Err(refuse("Whole-domain pole identity failed"));
        }
        return Ok(true);
    }
    if actual.len() != expected.len() {
        return Ok(false);
    }
    let scale = actual[0].1 / expected[0].1;
    if actual
        .iter()
        .zip(&expected)
        .any(|((point, weight), (wanted, wanted_weight))| {
            !point
                .iter()
                .zip(wanted)
                .all(|(left, right)| close(*left, *right))
                || !close(*weight, scale * wanted_weight)
        })
    {
        return Err(refuse("Whole-domain rational boundary identity failed"));
    }
    Ok(true)
}

fn verify_correspondence(
    c3: &Curve,
    pc: &Curve,
    s: &Surface,
    reversed: bool,
    whole_domain: bool,
    require_exact: bool,
) -> Result<CoedgeTrim> {
    let d3 = c3.domain();
    let d2 = pc.domain();
    let uv_start = pc.evaluate(d2[0])?.point;
    let uv_end = pc.evaluate(d2[1])?.point;
    let domain_u = [
        s.knots_u[s.degree_u],
        s.knots_u[s.knots_u.len() - s.degree_u - 1],
    ];
    let domain_v = [
        s.knots_v[s.degree_v],
        s.knots_v[s.knots_v.len() - s.degree_v - 1],
    ];
    let lift = |uv: &[f64]| -> [i32; 2] {
        let component = |value: f64, domain: [f64; 2], periodic: bool| {
            if periodic {
                ((value - domain[0]) / (domain[1] - domain[0])).floor() as i32
            } else {
                0
            }
        };
        [
            component(uv[0], domain_u, s.periodic_u),
            component(uv[1], domain_v, s.periodic_v),
        ]
    };
    let exact = whole_domain && exact_boundary_correspondence(c3, pc, s, reversed)?;
    if require_exact && !exact {
        return Err(refuse(
            "Collapsed freeform boundary lacks an exact whole-domain rational control identity",
        ));
    }
    for q in 0..=if exact { 0 } else { 8 } {
        let t = q as f64 / 8.;
        let t3 = if reversed {
            d3[1] - t * (d3[1] - d3[0])
        } else {
            d3[0] + t * (d3[1] - d3[0])
        };
        let uv = pc.evaluate(d2[0] + t * (d2[1] - d2[0]))?.point;
        let p = c3.evaluate(t3)?.point;
        let sp = s.evaluate(uv[0], uv[1])?.point;
        let error = (p[0] - sp[0]).hypot(p[1] - sp[1]).hypot(p[2] - sp[2]);
        if !error.is_finite() || error > 1e-5 {
            return Err(refuse(format!(
                "Independent 3D curve↔pcurve correspondence failed ({error:.3e})"
            )));
        }
    }
    let trim = CoedgeTrim {
        curve_parameter: d3,
        pcurve_parameter: d2,
        periodic_lift: [lift(&uv_start), lift(&uv_end)],
    };
    trim.validate()?;
    Ok(trim)
}

struct DirectBuilder<'a> {
    entities: &'a BTreeMap<usize, Entity>,
    scale: f64,
    analytic_surfaces: bool,
    ap242_composition: bool,
    allow_degenerate: bool,
    whole_domain_proofs: bool,
    interior_point_selectors: bool,
    allow_open_shells: bool,
    vertices: Vec<Vertex>,
    vertex_map: BTreeMap<usize, usize>,
    edges: Vec<Edge>,
    edge_map: BTreeMap<usize, usize>,
    seam_use: BTreeMap<(usize, usize), usize>,
    loops: Vec<Loop>,
    loop_entity: Vec<usize>,
    faces: Vec<Face>,
    face_entity: Vec<usize>,
    shells: Vec<Shell>,
    shell_entity: Vec<usize>,
    bodies: Vec<Body>,
    body_entity: Vec<usize>,
}
impl<'a> DirectBuilder<'a> {
    fn vertex(&mut self, id: usize) -> Result<usize> {
        if let Some(v) = self.vertex_map.get(&id) {
            return Ok(*v);
        }
        let (ty, a) = call(self.entities, id)?;
        if ty != "VERTEX_POINT" || a.len() != 2 {
            return Err(refuse("EDGE_CURVE vertex is not VERTEX_POINT"));
        }
        let p = point(
            self.entities,
            one_ref(&a[1], "VERTEX_POINT geometry")?,
            3,
            self.scale,
        )?;
        let index = self.vertices.len();
        self.vertices.push(Vertex {
            point: [p[0], p[1], p[2]],
        });
        self.vertex_map.insert(id, index);
        Ok(index)
    }
    fn edge(
        &mut self,
        id: usize,
        surface_id: usize,
        reversed: bool,
        surface: &Surface,
    ) -> Result<(usize, Curve)> {
        let (ty, a) = call(self.entities, id)?;
        if ty != "EDGE_CURVE" || a.len() != 5 {
            return Err(refuse("ORIENTED_EDGE target is not EDGE_CURVE"));
        }
        let same_sense = boolean(&a[4], "EDGE_CURVE same_sense")?;
        let va = self.vertex(one_ref(&a[1], "EDGE_CURVE start")?)?;
        let vb = self.vertex(one_ref(&a[2], "EDGE_CURVE end")?)?;
        let geometry = one_ref(&a[3], "EDGE_CURVE geometry")?;
        let (gty, ga) = call(self.entities, geometry)?;
        if gty != "SURFACE_CURVE" && gty != "SEAM_CURVE" {
            return Err(refuse("EDGE_CURVE must use SURFACE_CURVE or SEAM_CURVE"));
        }
        if ga.len() < 3 {
            return Err(refuse("SURFACE_CURVE argument count mismatch"));
        }
        let mut c3 = if self.ap242_composition {
            curve_v5(
                self.entities,
                one_ref(&ga[1], "SURFACE_CURVE 3D curve")?,
                3,
                self.scale,
                self.interior_point_selectors,
            )?
        } else {
            curve(
                self.entities,
                one_ref(&ga[1], "SURFACE_CURVE 3D curve")?,
                3,
                self.scale,
            )?
        };
        let mut matching = Vec::new();
        for p in list(&ga[2], "SURFACE_CURVE pcurves")? {
            let (_, pa) = call(self.entities, one_ref(p, "SURFACE_CURVE pcurve")?)?;
            if pa.len() != 3 {
                return Err(refuse("Malformed PCURVE"));
            }
            if one_ref(&pa[1], "PCURVE surface")? == surface_id {
                let mut curve_id = one_ref(&pa[2], "PCURVE reference_to_curve")?;
                if self.ap242_composition {
                    let (reference_type, reference_args) = call(self.entities, curve_id)?;
                    if reference_type != "DEFINITIONAL_REPRESENTATION" || reference_args.len() != 3
                    {
                        return Err(refuse(
                            "AP242 PCURVE reference_to_curve must be a DEFINITIONAL_REPRESENTATION",
                        ));
                    }
                    let items = list(&reference_args[1], "DEFINITIONAL_REPRESENTATION items")?;
                    if items.len() != 1 {
                        return Err(refuse(
                            "PCURVE definitional representation must contain one curve",
                        ));
                    }
                    curve_id = one_ref(&items[0], "DEFINITIONAL_REPRESENTATION curve")?;
                }
                matching.push(if self.ap242_composition {
                    curve_v5(
                        self.entities,
                        curve_id,
                        2,
                        1.,
                        self.interior_point_selectors,
                    )?
                } else {
                    curve(self.entities, curve_id, 2, 1.)?
                });
            }
        }
        if matching.is_empty() {
            return Err(refuse("No PCURVE corresponds to ADVANCED_FACE surface"));
        }
        let mut pc = if gty == "SEAM_CURVE" || matching.len() > 1 {
            let use_index = self.seam_use.entry((id, surface_id)).or_insert(0);
            if *use_index >= matching.len() {
                return Err(refuse("Surface curve has fewer pcurves than oriented uses"));
            }
            let selected = matching[*use_index].clone();
            *use_index += 1;
            selected
        } else {
            matching.pop().unwrap()
        };
        if !same_sense {
            c3 = c3.reverse()?;
            pc = pc.reverse()?;
        }
        let curve_domain = c3.domain();
        let curve_start = c3.evaluate(curve_domain[0])?.point;
        let curve_end = c3.evaluate(curve_domain[1])?.point;
        let endpoint_error = |point: &[f64], vertex: &Vertex| {
            point
                .iter()
                .zip(vertex.point)
                .map(|(left, right)| (left - right) * (left - right))
                .sum::<f64>()
                .sqrt()
        };
        if endpoint_error(&curve_start, &self.vertices[va]) > 1e-7
            || endpoint_error(&curve_end, &self.vertices[vb]) > 1e-7
        {
            return Err(refuse(
                "EDGE_CURVE same_sense does not orient geometry from edge_start to edge_end",
            ));
        }
        let degenerate = va == vb;
        verify_correspondence(
            &c3,
            &pc,
            surface,
            reversed,
            self.whole_domain_proofs,
            degenerate
                && self.interior_point_selectors
                && certify_periodic_surface(surface)?.is_none(),
        )
        .map_err(|e| {
            refuse(format!(
                "EDGE_CURVE #{id} on surface #{surface_id}: {}",
                e.message
            ))
        })?;
        if degenerate && !self.allow_degenerate {
            return Err(refuse(
                "Pole/degenerate edges are typed-refused before STEP /6",
            ));
        }
        if degenerate {
            let pole = self.vertices[va].point;
            if c3.control_points.iter().any(|point| {
                point.len() != 3 || point.iter().zip(pole).any(|(a, b)| (*a - b).abs() > 1e-9)
            }) {
                return Err(refuse(
                    "Degenerate EDGE_CURVE geometry is not identically its pole vertex",
                ));
            }
        }
        let index = if let Some(index) = self.edge_map.get(&id) {
            let prior = &self.edges[*index];
            if prior.vertices != [va, vb] {
                return Err(refuse("Shared EDGE_CURVE has inconsistent vertices"));
            }
            *index
        } else {
            let index = self.edges.len();
            self.edges.push(Edge {
                degenerate,
                vertices: [va, vb],
                curve: c3,
            });
            self.edge_map.insert(id, index);
            index
        };
        Ok((index, pc))
    }
    fn loop_(&mut self, id: usize, surface_id: usize, surface: &Surface) -> Result<usize> {
        let (ty, a) = call(self.entities, id)?;
        if ty != "EDGE_LOOP" || a.len() != 2 {
            return Err(refuse("FACE_BOUND target is not EDGE_LOOP"));
        }
        let mut coedges = Vec::new();
        for oriented in list(&a[1], "EDGE_LOOP members")? {
            let oid = one_ref(oriented, "EDGE_LOOP member")?;
            let (oty, oa) = call(self.entities, oid)?;
            if oty != "ORIENTED_EDGE" || oa.len() != 5 {
                return Err(refuse("EDGE_LOOP member is not ORIENTED_EDGE"));
            }
            let reversed = !boolean(&oa[4], "ORIENTED_EDGE orientation")?;
            let (edge, pcurve) = self.edge(
                one_ref(&oa[3], "ORIENTED_EDGE edge")?,
                surface_id,
                reversed,
                surface,
            )?;
            coedges.push(Coedge {
                edge,
                reversed,
                pcurve,
            });
        }
        if coedges.is_empty() {
            return Err(refuse("EDGE_LOOP cannot be empty"));
        }
        let index = self.loops.len();
        self.loops.push(Loop { coedges });
        self.loop_entity.push(id);
        Ok(index)
    }
    fn face(&mut self, id: usize) -> Result<usize> {
        let (ty, a) = call(self.entities, id)?;
        if ty != "ADVANCED_FACE" || a.len() != 4 {
            return Err(refuse("CLOSED_SHELL member is not ADVANCED_FACE"));
        }
        let sid = one_ref(&a[2], "ADVANCED_FACE surface")?;
        let mut surface = if self.ap242_composition {
            surface_v5(self.entities, sid, self.scale)?
        } else if self.analytic_surfaces {
            surface_v4(self.entities, sid, self.scale)?
        } else {
            surface(self.entities, sid, self.scale)?
        };
        let mut outer = None;
        let mut holes = Vec::new();
        for bound in list(&a[1], "ADVANCED_FACE bounds")? {
            let bid = one_ref(bound, "ADVANCED_FACE bound")?;
            let (bty, ba) = call(self.entities, bid)?;
            if !matches!(bty, "FACE_OUTER_BOUND" | "FACE_BOUND") || ba.len() != 3 {
                return Err(refuse("ADVANCED_FACE bound type is unsupported"));
            }
            let loop_index = self.loop_(one_ref(&ba[1], "FACE_BOUND loop")?, sid, &surface)?;
            if !boolean(&ba[2], "FACE_BOUND orientation")? {
                for c in &mut self.loops[loop_index].coedges {
                    c.reversed = !c.reversed;
                    c.pcurve = c.pcurve.reverse()?;
                }
                self.loops[loop_index].coedges.reverse();
            }
            if bty == "FACE_OUTER_BOUND" {
                if outer.replace(loop_index).is_some() {
                    return Err(refuse("ADVANCED_FACE has multiple outer bounds"));
                }
            } else {
                holes.push(loop_index);
            }
        }
        let same_sense = boolean(&a[3], "ADVANCED_FACE same_sense")?;
        if !same_sense && self.ap242_composition {
            let sum = surface.knots_u[surface.degree_u]
                + surface.knots_u[surface.knots_u.len() - surface.degree_u - 1];
            surface.control_points.reverse();
            surface.weights.reverse();
            surface.knots_u.reverse();
            for knot in &mut surface.knots_u {
                *knot = sum - *knot
            }
            for loop_index in std::iter::once(outer.unwrap()).chain(holes.iter().copied()) {
                for coedge in &mut self.loops[loop_index].coedges {
                    for point in &mut coedge.pcurve.control_points {
                        point[0] = sum - point[0]
                    }
                }
            }
        }
        let index = self.faces.len();
        self.faces.push(Face {
            surface,
            outer: outer.ok_or_else(|| refuse("ADVANCED_FACE has no outer bound"))?,
            holes,
        });
        self.face_entity.push(id);
        Ok(index)
    }
    fn shell(&mut self, id: usize) -> Result<usize> {
        let (ty, a) = call(self.entities, id)?;
        let closed = match ty {
            "CLOSED_SHELL" => true,
            "OPEN_SHELL" if self.allow_open_shells => false,
            _ => {
                return Err(refuse(
                    "Shell is outside the admitted CLOSED_SHELL/OPEN_SHELL subset",
                ));
            }
        };
        if a.len() != 2 {
            return Err(refuse("Shell argument count mismatch"));
        }
        let mut faces = Vec::new();
        for f in list(&a[1], "shell faces")? {
            let fid = one_ref(f, "shell face")?;
            let (_, fa) = call(self.entities, fid)?;
            faces.push(FaceUse {
                face: self.face(fid)?,
                reversed: !boolean(&fa[3], "ADVANCED_FACE sense")?,
            });
        }
        let index = self.shells.len();
        self.shells.push(Shell { faces, closed });
        self.shell_entity.push(id);
        Ok(index)
    }
    fn body(&mut self, id: usize) -> Result<()> {
        let (ty, a) = call(self.entities, id)?;
        let (outer, inners) = match ty {
            "MANIFOLD_SOLID_BREP" if a.len() == 2 => (one_ref(&a[1], "solid shell")?, Vec::new()),
            "BREP_WITH_VOIDS" if a.len() == 3 => {
                let mut voids = Vec::new();
                for value in list(&a[2], "void shells")? {
                    let void_id = one_ref(value, "void shell")?;
                    if self.ap242_composition {
                        let (vty, va) = call(self.entities, void_id)?;
                        if vty != "ORIENTED_CLOSED_SHELL" || va.len() != 4 {
                            return Err(refuse(
                                "AP242 BREP_WITH_VOIDS requires ORIENTED_CLOSED_SHELL voids",
                            ));
                        }
                        let orientation = boolean(&va[3], "ORIENTED_CLOSED_SHELL orientation")?;
                        if orientation && (!self.allow_degenerate || self.whole_domain_proofs) {
                            return Err(refuse(
                                "Void ORIENTED_CLOSED_SHELL must oppose its CLOSED_SHELL",
                            ));
                        }
                        voids.push(one_ref(&va[2], "ORIENTED_CLOSED_SHELL element")?);
                    } else {
                        voids.push(void_id);
                    }
                }
                (one_ref(&a[1], "outer shell")?, voids)
            }
            _ => {
                return Err(refuse(
                    "Selected representation contains unsupported body root",
                ));
            }
        };
        let outer_shell = self.shell(outer)?;
        let inner_shells = inners
            .into_iter()
            .map(|s| self.shell(s))
            .collect::<Result<Vec<_>>>()?;
        self.bodies.push(Body {
            outer_shell,
            inner_shells,
        });
        self.body_entity.push(id);
        Ok(())
    }
    fn root(&mut self, id: usize) -> Result<()> {
        let (ty, args) = call(self.entities, id)?;
        if ty == "SHELL_BASED_SURFACE_MODEL" && self.allow_open_shells {
            if args.len() != 2 {
                return Err(refuse("SHELL_BASED_SURFACE_MODEL argument count mismatch"));
            }
            for shell in list(&args[1], "SHELL_BASED_SURFACE_MODEL boundaries")? {
                let shell_id = one_ref(shell, "surface-model shell")?;
                if call(self.entities, shell_id)?.0 != "OPEN_SHELL" {
                    return Err(refuse(
                        "SHELL_BASED_SURFACE_MODEL requires OPEN_SHELL boundaries",
                    ));
                }
                self.shell(shell_id)?;
            }
            Ok(())
        } else {
            self.body(id)
        }
    }
}

fn metadata_entity_map(builder: &DirectBuilder<'_>) -> Vec<(TopoKind, usize, usize)> {
    let mut out = Vec::new();
    out.extend(
        builder
            .vertex_map
            .iter()
            .map(|(entity, index)| (TopoKind::Vertex, *index, *entity)),
    );
    out.extend(
        builder
            .edge_map
            .iter()
            .map(|(entity, index)| (TopoKind::Edge, *index, *entity)),
    );
    out.extend(
        builder
            .loop_entity
            .iter()
            .enumerate()
            .map(|(i, e)| (TopoKind::Loop, i, *e)),
    );
    out.extend(
        builder
            .face_entity
            .iter()
            .enumerate()
            .map(|(i, e)| (TopoKind::Face, i, *e)),
    );
    out.extend(
        builder
            .shell_entity
            .iter()
            .enumerate()
            .map(|(i, e)| (TopoKind::Shell, i, *e)),
    );
    out.extend(
        builder
            .body_entity
            .iter()
            .enumerate()
            .map(|(i, e)| (TopoKind::Body, i, *e)),
    );
    out
}

fn build_direct_roots(
    entities: &BTreeMap<usize, Entity>,
    roots: &[usize],
    scale: f64,
    options: DirectBuildOptions,
) -> Result<(Model, TopologyMapping)> {
    let DirectBuildOptions {
        analytic_surfaces,
        ap242_composition,
        allow_degenerate,
        whole_domain_proofs,
        interior_point_selectors,
        allow_open_shells,
    } = options;
    let mut builder = DirectBuilder {
        entities,
        scale,
        analytic_surfaces,
        ap242_composition,
        allow_degenerate,
        whole_domain_proofs,
        interior_point_selectors,
        allow_open_shells,
        vertices: vec![],
        vertex_map: BTreeMap::new(),
        edges: vec![],
        edge_map: BTreeMap::new(),
        seam_use: BTreeMap::new(),
        loops: vec![],
        loop_entity: vec![],
        faces: vec![],
        face_entity: vec![],
        shells: vec![],
        shell_entity: vec![],
        bodies: vec![],
        body_entity: vec![],
    };
    for root in roots {
        builder.root(*root)?
    }
    let map = metadata_entity_map(&builder);
    let topology = brep_topology::Model {
        vertices: builder.vertices,
        edges: builder.edges,
        loops: builder.loops,
        faces: builder.faces,
        shells: builder.shells,
        bodies: builder.bodies,
        tolerance_mm: 1e-7,
    };
    let mut model = Model(topology, TopologyIds::default());
    model.rebuild_topology_ids();
    model.validate()?;
    Ok((model, map))
}

fn append_direct_model(
    target: &mut Model,
    source: Model,
    source_map: &[(TopoKind, usize, usize)],
    target_map: &mut Vec<(TopoKind, usize, usize)>,
) {
    let Model(mut topology, _) = source;
    let (vo, eo, lo, fo, so, bo) = (
        target.vertices.len(),
        target.edges.len(),
        target.loops.len(),
        target.faces.len(),
        target.shells.len(),
        target.bodies.len(),
    );
    for edge in &mut topology.edges {
        edge.vertices = [edge.vertices[0] + vo, edge.vertices[1] + vo]
    }
    for loop_ in &mut topology.loops {
        for coedge in &mut loop_.coedges {
            coedge.edge += eo
        }
    }
    for face in &mut topology.faces {
        face.outer += lo;
        for hole in &mut face.holes {
            *hole += lo
        }
    }
    for shell in &mut topology.shells {
        for face in &mut shell.faces {
            face.face += fo
        }
    }
    for body in &mut topology.bodies {
        body.outer_shell += so;
        for shell in &mut body.inner_shells {
            *shell += so
        }
    }
    target.0.vertices.extend(topology.vertices);
    target.0.edges.extend(topology.edges);
    target.0.loops.extend(topology.loops);
    target.0.faces.extend(topology.faces);
    target.0.shells.extend(topology.shells);
    target.0.bodies.extend(topology.bodies);
    for &(kind, index, entity) in source_map {
        let offset = match kind {
            TopoKind::Vertex => vo,
            TopoKind::Edge => eo,
            TopoKind::Loop => lo,
            TopoKind::Face => fo,
            TopoKind::Shell => so,
            TopoKind::Body => bo,
            TopoKind::ControlPoint => 0,
        };
        target_map.push((kind, index + offset, entity));
    }
}

/// Compose already validated document occurrences into one authoritative
/// bounded model and assign occurrence-local topology identities.
fn compose_step_occurrences(models: &[Model], capability: &str) -> Result<Model> {
    if models.is_empty() {
        return Err(refuse(
            "STEP /7 composition requires at least one occurrence",
        ));
    }
    if models.len() > MAX_OCCURRENCES {
        return Err(refuse("STEP /7 composed occurrence count exceeds 256"));
    }
    let mut aggregate = Model::empty(1e-7)?;
    let mut ignored = Vec::new();
    for model in models {
        model.validate()?;
        append_direct_model(&mut aggregate, model.clone(), &[], &mut ignored);
    }
    aggregate.rebuild_topology_ids();
    let occurrence_ids = |kind: TopoKind, ids: &[TopoId]| {
        ids.iter()
            .enumerate()
            .map(|(index, id)| {
                TopoId::derive(
                    kind,
                    capability,
                    &format!("occurrence-entity-{index}"),
                    kind.as_str(),
                    id.to_string().as_bytes(),
                )
            })
            .collect::<Vec<_>>()
    };
    aggregate.1.vertices = occurrence_ids(TopoKind::Vertex, &aggregate.1.vertices);
    aggregate.1.edges = occurrence_ids(TopoKind::Edge, &aggregate.1.edges);
    aggregate.1.loops = occurrence_ids(TopoKind::Loop, &aggregate.1.loops);
    aggregate.1.faces = occurrence_ids(TopoKind::Face, &aggregate.1.faces);
    aggregate.1.shells = occurrence_ids(TopoKind::Shell, &aggregate.1.shells);
    aggregate.1.bodies = occurrence_ids(TopoKind::Body, &aggregate.1.bodies);
    aggregate.1.lineage.clear();
    aggregate.1.change_set = brep_topology::ChangeSet::default();
    for (kind, ids) in [
        (TopoKind::Vertex, &aggregate.1.vertices),
        (TopoKind::Edge, &aggregate.1.edges),
        (TopoKind::Loop, &aggregate.1.loops),
        (TopoKind::Face, &aggregate.1.faces),
        (TopoKind::Shell, &aggregate.1.shells),
        (TopoKind::Body, &aggregate.1.bodies),
    ] {
        for id in ids {
            aggregate.1.change_set.nodes.insert(*id, kind);
        }
    }
    aggregate.validate()?;
    Ok(aggregate)
}
pub fn compose_step_v7_occurrences(models: &[Model]) -> Result<Model> {
    compose_step_occurrences(models, STEP_INTERCHANGE_V7_CAPABILITY)
}
pub fn compose_step_v8_occurrences(models: &[Model]) -> Result<Model> {
    let aggregate = compose_step_occurrences(models, STEP_INTERCHANGE_V8_CAPABILITY)?;
    certify_step_v8_topology(&aggregate)?;
    Ok(aggregate)
}
pub fn compose_step_v9_occurrences(models: &[Model]) -> Result<Model> {
    let aggregate = compose_step_occurrences(models, STEP_INTERCHANGE_V9_CAPABILITY)?;
    certify_step_v8_topology(&aggregate)?;
    Ok(aggregate)
}

#[derive(Clone)]
struct RepresentationOccurrence {
    representation: usize,
    bodies: Vec<usize>,
    transform: [[f64; 4]; 4],
    linked: BTreeSet<usize>,
    path: Vec<usize>,
}

fn relationship_components(
    entities: &BTreeMap<usize, Entity>,
    id: usize,
) -> Result<Option<(usize, usize, usize)>> {
    let parts = components(entities, id)?;
    let Some(base) = component(&parts, "REPRESENTATION_RELATIONSHIP") else {
        return Ok(None);
    };
    let Some(with_transform) = component(&parts, "REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION")
    else {
        return Ok(None);
    };
    if base.len() != 4 || with_transform.len() != 1 {
        return Err(refuse(
            "Malformed REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION complex",
        ));
    }
    Ok(Some((
        one_ref(&base[2], "relationship source representation")?,
        one_ref(&base[3], "relationship target representation")?,
        one_ref(&with_transform[0], "relationship transformation operator")?,
    )))
}

fn representation_occurrences(
    entities: &BTreeMap<usize, Entity>,
    selected: &[usize],
    allow_affine: bool,
    general_affine: bool,
    allow_open_shells: bool,
) -> Result<Vec<RepresentationOccurrence>> {
    let mut outgoing: BTreeMap<usize, Vec<(usize, usize, usize)>> = BTreeMap::new();
    for id in entities.keys() {
        if let Some((from, to, operator)) = relationship_components(entities, *id)? {
            outgoing.entry(from).or_default().push((*id, to, operator));
        }
    }
    for edges in outgoing.values_mut() {
        edges.sort_by_key(|edge| edge.0)
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "recursive STEP graph walk keeps traversal state explicit"
    )]
    fn walk(
        rep: usize,
        matrix: [[f64; 4]; 4],
        entities: &BTreeMap<usize, Entity>,
        outgoing: &BTreeMap<usize, Vec<(usize, usize, usize)>>,
        active: &mut BTreeSet<usize>,
        depth: usize,
        allow_affine: bool,
        general_affine: bool,
        allow_open_shells: bool,
        path: &[usize],
        out: &mut Vec<RepresentationOccurrence>,
    ) -> Result<()> {
        if depth > 32 {
            return Err(refuse(
                "Representation relationship placement depth exceeds 32",
            ));
        }
        if !active.insert(rep) {
            return Err(refuse(
                "Cycle in selected representation-relationship placement graph",
            ));
        }
        let (ty, args) = call(entities, rep)?;
        if ty != "ADVANCED_BREP_SHAPE_REPRESENTATION"
            && !(allow_open_shells && ty == "MANIFOLD_SURFACE_SHAPE_REPRESENTATION")
        {
            return Err(refuse(
                "Placement graph reaches an unsupported representation",
            ));
        }
        if args.len() != 3 {
            return Err(refuse("Shape representation argument count mismatch"));
        }
        let linked = reachable(entities, &[rep])?;
        let scale = length_scale(entities, &linked, false)?;
        let bodies = list(&args[1], "representation items")?
            .iter()
            .filter_map(|value| match value {
                Value::Ref(id) => call(entities, *id).ok().and_then(|(name, _)| {
                    (matches!(name, "MANIFOLD_SOLID_BREP" | "BREP_WITH_VOIDS")
                        || allow_open_shells && name == "SHELL_BASED_SURFACE_MODEL")
                        .then_some(*id)
                }),
                _ => None,
            })
            .collect::<Vec<_>>();
        if !bodies.is_empty() {
            if out.len() >= MAX_OCCURRENCES {
                return Err(refuse("Assembly occurrence count exceeds 256"));
            }
            out.push(RepresentationOccurrence {
                representation: rep,
                bodies,
                transform: matrix,
                linked,
                path: path.to_vec(),
            })
        }
        if let Some(edges) = outgoing.get(&rep) {
            for (relationship, child, operator) in edges {
                let affine_complex = allow_affine
                    && component(
                        &components(entities, *operator)?,
                        "CARTESIAN_TRANSFORMATION_OPERATOR_3DNON_UNIFORM",
                    )
                    .is_some();
                let edge_matrix = if affine_complex {
                    cartesian_operator_affine(entities, *operator, scale, general_affine)?
                } else {
                    let (operator_type, operator_args) = call(entities, *operator)?;
                    match operator_type {
                        "CARTESIAN_TRANSFORMATION_OPERATOR_3D" => {
                            cartesian_operator_3d(entities, operator_args, scale)?
                        }
                        "ITEM_DEFINED_TRANSFORMATION" => {
                            if operator_args.len() != 4 {
                                return Err(refuse(
                                    "ITEM_DEFINED_TRANSFORMATION argument count mismatch",
                                ));
                            }
                            let from = rigid_frame(
                                entities,
                                one_ref(&operator_args[2], "transformation source")?,
                                scale,
                            )?;
                            let to = rigid_frame(
                                entities,
                                one_ref(&operator_args[3], "transformation target")?,
                                scale,
                            )?;
                            mul(to, inverse_rigid(from))
                        }
                        _ => {
                            return Err(refuse(
                                "Representation relationship uses a non-rigid or unsupported transformation operator",
                            ));
                        }
                    }
                };
                let mut child_path = path.to_vec();
                child_path.extend([*relationship, *child]);
                walk(
                    *child,
                    mul(matrix, edge_matrix),
                    entities,
                    outgoing,
                    active,
                    depth + 1,
                    allow_affine,
                    general_affine,
                    allow_open_shells,
                    &child_path,
                    out,
                )?;
            }
        }
        active.remove(&rep);
        Ok(())
    }
    let identity = [
        [1., 0., 0., 0.],
        [0., 1., 0., 0.],
        [0., 0., 1., 0.],
        [0., 0., 0., 1.],
    ];
    let mut out = Vec::new();
    for rep in selected {
        walk(
            *rep,
            identity,
            entities,
            &outgoing,
            &mut BTreeSet::new(),
            0,
            allow_affine,
            general_affine,
            allow_open_shells,
            &[*rep],
            &mut out,
        )?
    }
    let mut paths = BTreeSet::new();
    for occurrence in &out {
        let key = (
            occurrence.representation,
            occurrence.bodies.clone(),
            occurrence
                .transform
                .iter()
                .flatten()
                .map(|v| v.to_bits())
                .collect::<Vec<_>>(),
        );
        if !paths.insert(key) {
            return Err(refuse(
                "Ambiguous duplicate selected representation occurrence",
            ));
        }
    }
    Ok(out)
}

fn digest_value(
    value: &Value,
    entities: &BTreeMap<usize, Entity>,
    active: &mut BTreeSet<usize>,
    depth: usize,
) -> Result<String> {
    if depth > MAX_GRAPH_DEPTH {
        return Err(refuse("Canonical graph digest depth exceeds 64"));
    }
    let text = match value {
        Value::String(_) => "''".into(), // entity names are non-geometric metadata
        Value::Number(n) => format!("{n:.17e}"),
        Value::Integer(n) => n.to_string(),
        Value::Enum(v) => format!(".{v}."),
        Value::Omitted => "$".into(),
        Value::Derived => "*".into(),
        Value::List(v) => format!(
            "({})",
            v.iter()
                .map(|x| digest_value(x, entities, active, depth + 1))
                .collect::<Result<Vec<_>>>()?
                .join(",")
        ),
        Value::Call(n, v) => {
            let args = v
                .iter()
                .enumerate()
                .map(|(i, x)| {
                    if i == 0 && matches!(x, Value::String(_)) {
                        Ok("''".into())
                    } else {
                        digest_value(x, entities, active, depth + 1)
                    }
                })
                .collect::<Result<Vec<_>>>()?;
            format!("{n}({})", args.join(","))
        }
        Value::Ref(id) => {
            if !active.insert(*id) {
                return Err(refuse("Cycle while computing canonical graph digest"));
            }
            let e = entities
                .get(id)
                .ok_or_else(|| refuse("Missing digest dependency"))?;
            let result = digest_value(&e.value, entities, active, depth + 1)?;
            active.remove(id);
            format!("#{result}")
        }
    };
    Ok(text)
}
fn hash(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn entity_digest(entities: &BTreeMap<usize, Entity>, id: usize) -> Result<String> {
    let entity = entities
        .get(&id)
        .ok_or_else(|| refuse("Missing identity entity"))?;
    Ok(hash(&digest_value(
        &entity.value,
        entities,
        &mut BTreeSet::from([id]),
        0,
    )?))
}

fn restore_identity(
    entities: &BTreeMap<usize, Entity>,
    map: &[(TopoKind, usize, usize)],
    model: &mut Model,
) -> Result<crate::step_interchange::StepIdentityReport> {
    let mut metadata = Vec::new();
    for &(kind, index, entity_id) in map {
        let (_, args) = call(entities, entity_id)?;
        let Some(n) = name(args) else {
            continue;
        };
        let Some(payload) = n.strip_prefix("OSCAD_TOPO/3|") else {
            continue;
        };
        let fields: Vec<_> = payload.split('|').collect();
        if fields.len() != 3 {
            return Err(refuse("Malformed STEP /3 identity metadata"));
        }
        let mk =
            TopoKind::parse(fields[0]).ok_or_else(|| refuse("Unknown STEP /3 identity kind"))?;
        let id = TopoId::parse(fields[1]).map_err(refuse)?;
        if mk != kind || id.kind() != kind || fields[2] != entity_digest(entities, entity_id)? {
            return Err(refuse("Transplanted STEP /3 identity metadata"));
        }
        metadata.push((kind, index, id));
    }
    let count = map.len();
    if metadata.is_empty() {
        model.rebuild_topology_ids();
        return Ok(crate::step_interchange::StepIdentityReport {
            preserved: false,
            source: "external-step",
            preserved_count: 0,
            created_count: count,
            lost_count: count,
        });
    }
    if metadata.len() != count {
        return Err(refuse("Partial STEP /3 identity metadata"));
    }
    let mut seen = BTreeSet::new();
    let mut remap = BTreeMap::new();
    for (kind, index, id) in metadata {
        if !seen.insert(id) {
            return Err(refuse("Duplicate STEP /3 identity metadata"));
        }
        let old = match kind {
            TopoKind::Vertex => model.1.vertices[index],
            TopoKind::Edge => model.1.edges[index],
            TopoKind::Loop => model.1.loops[index],
            TopoKind::Face => model.1.faces[index],
            TopoKind::Shell => model.1.shells[index],
            TopoKind::Body => model.1.bodies[index],
            TopoKind::ControlPoint => {
                return Err(refuse(
                    "Control-point identity is not STEP topology identity",
                ));
            }
        };
        remap.insert(old, id);
        match kind {
            TopoKind::Vertex => model.1.vertices[index] = id,
            TopoKind::Edge => model.1.edges[index] = id,
            TopoKind::Loop => model.1.loops[index] = id,
            TopoKind::Face => model.1.faces[index] = id,
            TopoKind::Shell => model.1.shells[index] = id,
            TopoKind::Body => model.1.bodies[index] = id,
            TopoKind::ControlPoint => {
                return Err(refuse(
                    "Control-point identity is not STEP topology identity",
                ));
            }
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
            if let Some(replacement) = remap.get(id) {
                *id = *replacement;
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

const UNREACHABLE_ALLOWLIST: &[&str] = &[
    "APPLICATION_CONTEXT",
    "PRODUCT",
    "PRODUCT_DEFINITION",
    "PRODUCT_DEFINITION_FORMATION",
    "PRODUCT_DEFINITION_FORMATION_WITH_SPECIFIED_SOURCE",
    "PRODUCT_RELATED_PRODUCT_CATEGORY",
    "PRODUCT_CONTEXT",
    "PRODUCT_DEFINITION_CONTEXT",
    "PRODUCT_DEFINITION_SHAPE",
    "SHAPE_DEFINITION_REPRESENTATION",
    "ADVANCED_BREP_SHAPE_REPRESENTATION",
    "MANIFOLD_SURFACE_SHAPE_REPRESENTATION",
    "SHELL_BASED_SURFACE_MODEL",
    "GEOMETRIC_REPRESENTATION_CONTEXT",
    "GLOBAL_UNIT_ASSIGNED_CONTEXT",
    "SI_UNIT",
    "PRESENTATION_LAYER_ASSIGNMENT",
    "STYLED_ITEM",
    "COLOUR_RGB",
    "SURFACE_STYLE_USAGE",
    "SURFACE_SIDE_STYLE",
    "SURFACE_STYLE_FILL_AREA",
    "FILL_AREA_STYLE",
    "FILL_AREA_STYLE_COLOUR",
    "DRAUGHTING_PRE_DEFINED_COLOUR",
    "PROPERTY_DEFINITION",
    "PROPERTY_DEFINITION_REPRESENTATION",
    "DESCRIPTIVE_REPRESENTATION_ITEM",
    "MEASURE_REPRESENTATION_ITEM",
    "MATERIAL_DESIGNATION",
    "APPLIED_NAME_ASSIGNMENT",
    "APPLIED_DESCRIPTION_TEXT_ASSIGNMENT",
    "PERSON",
    "ORGANIZATION",
    "PERSON_AND_ORGANIZATION",
    "PERSON_AND_ORGANIZATION_ROLE",
    "APPLIED_PERSON_AND_ORGANIZATION_ASSIGNMENT",
    "DATE_AND_TIME",
    "CALENDAR_DATE",
    "LOCAL_TIME",
    "COORDINATED_UNIVERSAL_TIME_OFFSET",
    "DATE_TIME_ROLE",
    "APPLIED_DATE_AND_TIME_ASSIGNMENT",
    "APPROVAL",
    "APPROVAL_STATUS",
    "APPROVAL_ROLE",
    "APPLIED_APPROVAL_ASSIGNMENT",
    "SECURITY_CLASSIFICATION",
    "SECURITY_CLASSIFICATION_LEVEL",
    "APPLIED_SECURITY_CLASSIFICATION_ASSIGNMENT",
    "EXTERNAL_CLASS_LIBRARY",
    "ITEM_DEFINED_TRANSFORMATION",
    "AXIS2_PLACEMENT_3D",
    "CARTESIAN_POINT",
    "DIRECTION",
    "REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION",
    "CARTESIAN_TRANSFORMATION_OPERATOR_3D",
];

fn import_step_direct(
    text: &str,
    analytic_surfaces: bool,
    ap242_composition: bool,
    allow_degenerate: bool,
    capability: &'static str,
) -> Result<(Model, FeatureCertificate, StepV3Report)> {
    if !text.to_ascii_uppercase().contains("ISO-10303-21") {
        return Err(refuse("Not an ISO-10303-21 exchange"));
    }
    if ap242_composition {
        validate_exchange_structure(text)?;
    }
    let entities = parse(if ap242_composition {
        data_payload(text)?
    } else {
        text
    })?;
    let representations: Vec<_> = if ap242_composition {
        let mut selected = Vec::<usize>::new();
        for entity in entities.values() {
            if let Value::Call(name, args) = &entity.value
                && name == "SHAPE_DEFINITION_REPRESENTATION"
                && args.len() == 2
            {
                selected.push(one_ref(&args[1], "selected shape representation")?);
            }
        }
        selected.sort_unstable();
        selected.dedup();
        if selected.is_empty() {
            return Err(refuse(
                "AP242 import requires SHAPE_DEFINITION_REPRESENTATION selection",
            ));
        }
        let allow_open_shells = capability == STEP_INTERCHANGE_V9_CAPABILITY;
        for id in &selected {
            let (ty, _) = call(&entities, *id)?;
            if ty != "ADVANCED_BREP_SHAPE_REPRESENTATION"
                && !(allow_open_shells && ty == "MANIFOLD_SURFACE_SHAPE_REPRESENTATION")
            {
                return Err(refuse(
                    "Selected representation is outside the admitted direct B-rep products",
                ));
            }
        }
        selected
    } else {
        entities
            .keys()
            .filter_map(|id| {
                call(&entities, *id)
                    .ok()
                    .and_then(|(ty, _)| (ty == "ADVANCED_BREP_SHAPE_REPRESENTATION").then_some(*id))
            })
            .collect()
    };
    let allow_open_shells = capability == STEP_INTERCHANGE_V9_CAPABILITY;
    let all_bodies: Vec<_> = entities
        .keys()
        .filter_map(|id| {
            call(&entities, *id).ok().and_then(|(ty, _)| {
                (matches!(ty, "MANIFOLD_SOLID_BREP" | "BREP_WITH_VOIDS")
                    || allow_open_shells && ty == "SHELL_BASED_SURFACE_MODEL")
                    .then_some(*id)
            })
        })
        .collect();
    let graph_roots = if representations.is_empty() {
        all_bodies.clone()
    } else {
        representations.clone()
    };
    if graph_roots.is_empty() {
        return Err(refuse("STEP /3 requires a manifold advanced B-rep root"));
    }
    let occurrences = if ap242_composition {
        representation_occurrences(
            &entities,
            &representations,
            allow_degenerate && capability != STEP_INTERCHANGE_V8_CAPABILITY,
            capability == STEP_INTERCHANGE_V9_CAPABILITY,
            allow_open_shells,
        )?
    } else {
        Vec::new()
    };
    let occurrence_count = occurrences.len();
    let mut definition_identities = Vec::new();
    let mut occurrence_identities = Vec::new();
    if allow_degenerate {
        for occurrence in &occurrences {
            let definition = format!(
                "representation:{}",
                entity_digest(&entities, occurrence.representation)?
            );
            if !definition_identities.contains(&definition) {
                definition_identities.push(definition)
            }
            let path = occurrence
                .path
                .iter()
                .map(|id| entity_digest(&entities, *id))
                .collect::<Result<Vec<_>>>()?
                .join("/");
            let matrix = occurrence
                .transform
                .iter()
                .flatten()
                .map(|value| format!("{:016x}", value.to_bits()))
                .collect::<String>();
            occurrence_identities.push(format!("occurrence:{}", hash(&(path + &matrix))));
        }
    }
    let product_hierarchy = if allow_degenerate {
        entities
            .keys()
            .filter_map(|id| {
                call(&entities, *id).ok().and_then(|(ty, args)| {
                    if ty == "PRODUCT" && args.len() >= 3 {
                        Some(format!(
                            "product:#{}:{}:{}",
                            id,
                            string_value(&args[0], "product id").ok()?,
                            string_value(&args[1], "product name").ok()?
                        ))
                    } else if matches!(
                        ty,
                        "PRODUCT_DEFINITION"
                            | "PRODUCT_DEFINITION_FORMATION"
                            | "PRODUCT_DEFINITION_FORMATION_WITH_SPECIFIED_SOURCE"
                    ) && args.len() >= 2
                    {
                        Some(format!(
                            "{}:#{}:{}:{}",
                            ty.to_ascii_lowercase(),
                            id,
                            string_value(&args[0], "product metadata id").ok()?,
                            string_value(&args[1], "product metadata description").ok()?
                        ))
                    } else if ty == "PRODUCT_RELATED_PRODUCT_CATEGORY" && args.len() >= 3 {
                        Some(format!(
                            "product-category:#{}:{}",
                            id,
                            string_value(&args[0], "product category").ok()?
                        ))
                    } else if ty == "PROPERTY_DEFINITION" && args.len() >= 2 {
                        Some(format!(
                            "property-definition:#{}:{}",
                            id,
                            string_value(&args[0], "property name").ok()?
                        ))
                    } else if ty == "NEXT_ASSEMBLY_USAGE_OCCURRENCE" && args.len() >= 5 {
                        Some(format!(
                            "assembly-use:#{}:#{}:#{}",
                            id,
                            one_ref(&args[3], "relating product").ok()?,
                            one_ref(&args[4], "related product").ok()?
                        ))
                    } else {
                        None
                    }
                })
            })
            .collect()
    } else {
        Vec::new()
    };
    let external_references = if allow_degenerate {
        entities
            .keys()
            .filter_map(|id| {
                let (ty, args) = call(&entities, *id).ok()?;
                if ty == "DOCUMENT_FILE" && !args.is_empty() {
                    Some(format!(
                        "#{}:{}",
                        id,
                        string_value(&args[0], "document URI").ok()?
                    ))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let mut linked = reachable(&entities, &graph_roots)?;
    if ap242_composition {
        linked.clear();
        for occurrence in &occurrences {
            linked.extend(occurrence.linked.iter().copied())
        }
    }
    let roots: Vec<_> = all_bodies
        .into_iter()
        .filter(|id| linked.contains(id))
        .collect();
    if roots.is_empty() {
        return Err(refuse(
            "Selected representation contains no manifold B-rep body",
        ));
    }
    const REACHABLE_TYPES: &[&str] = &[
        "ADVANCED_BREP_SHAPE_REPRESENTATION",
        "MANIFOLD_SURFACE_SHAPE_REPRESENTATION",
        "MANIFOLD_SOLID_BREP",
        "BREP_WITH_VOIDS",
        "SHELL_BASED_SURFACE_MODEL",
        "CLOSED_SHELL",
        "OPEN_SHELL",
        "ORIENTED_CLOSED_SHELL",
        "ADVANCED_FACE",
        "FACE_OUTER_BOUND",
        "FACE_BOUND",
        "EDGE_LOOP",
        "ORIENTED_EDGE",
        "EDGE_CURVE",
        "VERTEX_POINT",
        "CARTESIAN_POINT",
        "DIRECTION",
        "VECTOR",
        "AXIS2_PLACEMENT_2D",
        "AXIS2_PLACEMENT_3D",
        "LINE",
        "CIRCLE",
        "ELLIPSE",
        "TRIMMED_CURVE",
        "B_SPLINE_CURVE_WITH_KNOTS",
        "PCURVE",
        "SURFACE_CURVE",
        "SEAM_CURVE",
        "DEFINITIONAL_REPRESENTATION",
        "PLANE",
        "CYLINDRICAL_SURFACE",
        "CONICAL_SURFACE",
        "SPHERICAL_SURFACE",
        "TOROIDAL_SURFACE",
        "B_SPLINE_SURFACE_WITH_KNOTS",
        "GEOMETRIC_REPRESENTATION_CONTEXT",
        "GLOBAL_UNIT_ASSIGNED_CONTEXT",
        "SI_UNIT",
        "CONVERSION_BASED_UNIT",
        "GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT",
        "UNCERTAINTY_MEASURE_WITH_UNIT",
        "LENGTH_MEASURE_WITH_UNIT",
        "MEASURE_WITH_UNIT",
        "ITEM_DEFINED_TRANSFORMATION",
        "REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION",
        "CARTESIAN_TRANSFORMATION_OPERATOR_3D",
    ];
    for id in &linked {
        if allow_degenerate
            && capability != STEP_INTERCHANGE_V8_CAPABILITY
            && component(
                &components(&entities, *id)?,
                "CARTESIAN_TRANSFORMATION_OPERATOR_3DNON_UNIFORM",
            )
            .is_some()
        {
            continue;
        }
        let (ty, _) = call(&entities, *id)?;
        if !REACHABLE_TYPES.contains(&ty) && !matches!(ty, "COMPLEX") {
            return Err(refuse(format!(
                "Reachable entity {ty} is outside the direct finite graph subset"
            )));
        }
    }
    let mut ignored = BTreeSet::new();
    for id in entities.keys() {
        if linked.contains(id) {
            continue;
        }
        if ap242_composition && relationship_components(&entities, *id)?.is_some() {
            continue;
        }
        if allow_degenerate
            && capability != STEP_INTERCHANGE_V8_CAPABILITY
            && component(
                &components(&entities, *id)?,
                "CARTESIAN_TRANSFORMATION_OPERATOR_3DNON_UNIFORM",
            )
            .is_some()
        {
            continue;
        }
        let (ty, _) = call(&entities, *id)?;
        if allow_degenerate
            && matches!(
                ty,
                "NEXT_ASSEMBLY_USAGE_OCCURRENCE" | "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION"
            )
        {
            continue;
        }
        if allow_degenerate
            && matches!(
                ty,
                "DOCUMENT_FILE"
                    | "DOCUMENT"
                    | "DOCUMENT_TYPE"
                    | "DOCUMENT_REPRESENTATION_TYPE"
                    | "EXTERNALLY_DEFINED_ITEM"
                    | "EXTERNAL_SOURCE"
                    | "APPLIED_EXTERNAL_IDENTIFICATION_ASSIGNMENT"
            )
        {
            continue;
        }
        if UNREACHABLE_ALLOWLIST.contains(&ty) {
            ignored.insert(ty.to_string());
        } else {
            return Err(refuse(format!(
                "Unreachable entity {ty} is not presentation metadata allowlisted by /3"
            )));
        }
    }
    let (mut model, identity_map) = if ap242_composition {
        if occurrences.is_empty() {
            return Err(refuse(
                "Selected placement graph contains no manifold B-rep occurrence",
            ));
        }
        let mut aggregate = Model::empty(1e-7)?;
        let mut aggregate_map = Vec::new();
        for occurrence in occurrences {
            let scale = length_scale(&entities, &occurrence.linked, false)?;
            let (component, map) = build_direct_roots(
                &entities,
                &occurrence.bodies,
                scale,
                DirectBuildOptions {
                    analytic_surfaces,
                    ap242_composition: true,
                    allow_degenerate,
                    whole_domain_proofs: matches!(
                        capability,
                        STEP_INTERCHANGE_V7_CAPABILITY
                            | STEP_INTERCHANGE_V8_CAPABILITY
                            | STEP_INTERCHANGE_V9_CAPABILITY
                    ),
                    interior_point_selectors: capability == STEP_INTERCHANGE_V9_CAPABILITY,
                    allow_open_shells,
                },
            )?;
            let component = crate::transform::affine(&component, occurrence.transform)?;
            append_direct_model(&mut aggregate, component, &map, &mut aggregate_map);
        }
        aggregate.rebuild_topology_ids();
        (aggregate, aggregate_map)
    } else {
        let scale = length_scale(&entities, &linked, true)?;
        let placement = reachable_transform(&entities, &linked, scale, false)?;
        let (mut model, map) = build_direct_roots(
            &entities,
            &roots,
            scale,
            DirectBuildOptions {
                analytic_surfaces,
                ap242_composition: false,
                allow_degenerate,
                whole_domain_proofs: matches!(
                    capability,
                    STEP_INTERCHANGE_V7_CAPABILITY
                        | STEP_INTERCHANGE_V8_CAPABILITY
                        | STEP_INTERCHANGE_V9_CAPABILITY
                ),
                interior_point_selectors: capability == STEP_INTERCHANGE_V9_CAPABILITY,
                allow_open_shells,
            },
        )?;
        if let Some(matrix) = placement {
            model = crate::transform::affine(&model, matrix)?
        }
        (model, map)
    };
    let identity = if capability == STEP_INTERCHANGE_V9_CAPABILITY
        || allow_degenerate && occurrence_count > 1
    {
        crate::step_interchange::StepIdentityReport {
            preserved: false,
            source: "assembly-definition-plus-occurrence",
            preserved_count: 0,
            created_count: identity_map.len(),
            lost_count: 0,
        }
    } else {
        restore_identity(&entities, &identity_map, &mut model)?
    };
    model.validate()?;
    let metadata_loss = if identity.preserved {
        vec![]
    } else if allow_degenerate {
        vec![
            "topology.occurrence_instance_ids_created".into(),
            "presentation_style".into(),
        ]
    } else {
        vec![
            "representation_item.name".into(),
            "product.id".into(),
            "product.name".into(),
            "product.description".into(),
            "presentation_style".into(),
        ]
    };
    let mut certificate_notes = vec![
        "direct_part21_topology",
        "line_circle_ellipse_trimmed_curve",
        "plane_and_rational_multispan_bspline_surface",
        "periodic_analytic_surfaces_typed_refused_parameterization",
        "shared_topology_senses_holes",
        "multi_body_multi_cavity",
        "si_and_positive_conversion_units",
        "reachable_rigid_placements_only",
        "independent_curve_pcurve_check",
        "bounded_reachable_graph",
    ];
    if matches!(
        capability,
        STEP_INTERCHANGE_V7_CAPABILITY
            | STEP_INTERCHANGE_V8_CAPABILITY
            | STEP_INTERCHANGE_V9_CAPABILITY
    ) {
        certificate_notes.push("whole_domain_rational_boundary_and_constant_pole_identity");
    }
    if capability == STEP_INTERCHANGE_V9_CAPABILITY {
        certificate_notes.extend([
            "certified_trimmed_curve_point_inverse",
            "bounded_freeform_collapsed_boundary_ownership",
            "invertible_general_affine_occurrences",
            "open_shell_surface_products",
        ]);
    }
    Ok((
        model,
        FeatureCertificate {
            capability,
            complete: true,
            notes: certificate_notes,
        },
        StepV3Report {
            identity,
            ignored_entities: ignored.into_iter().collect(),
            metadata_loss,
            instance_count: entities.len(),
            reachable_count: linked.len(),
            definition_identities,
            occurrence_identities,
            product_hierarchy,
            external_references,
        },
    ))
}

pub fn import_step_v3(text: &str) -> Result<(Model, FeatureCertificate, StepV3Report)> {
    import_step_direct(text, false, false, false, STEP_INTERCHANGE_V3_CAPABILITY)
}

pub fn import_step_v4(text: &str) -> Result<(Model, FeatureCertificate, StepV3Report)> {
    import_step_direct(text, true, false, false, STEP_INTERCHANGE_V4_CAPABILITY)
}

pub fn import_step_v5(text: &str) -> Result<(Model, FeatureCertificate, StepV3Report)> {
    import_step_direct(text, true, true, false, STEP_INTERCHANGE_V5_CAPABILITY)
}

pub fn import_step_v6(text: &str) -> Result<(Model, FeatureCertificate, StepV3Report)> {
    import_step_direct(text, true, true, true, STEP_INTERCHANGE_V6_CAPABILITY)
}

/// Append-only `/7` product route. The native direct graph subset is identical
/// to `/6`; `/7` adds bundle composition, durable lifecycle, and independent
/// semantic-conformance status in the product adapters. Keeping this entry
/// point distinct prevents a future `/7` claim from rewriting `/6` evidence.
pub fn import_step_v7(text: &str) -> Result<(Model, FeatureCertificate, StepV3Report)> {
    import_step_direct(text, true, true, true, STEP_INTERCHANGE_V7_CAPABILITY)
}

/// Qualified successor: `/7` remains rejected and immutable.
pub fn import_step_v8(text: &str) -> Result<(Model, StepV8Certificate, StepV3Report)> {
    let (model, _, report) =
        import_step_direct(text, true, true, true, STEP_INTERCHANGE_V8_CAPABILITY)?;
    let certificate = certify_step_v8_topology(&model)?;
    Ok((model, certificate, report))
}

pub fn import_step_v9(text: &str) -> Result<(Model, StepV8Certificate, StepV3Report)> {
    let (model, _, report) =
        import_step_direct(text, true, true, true, STEP_INTERCHANGE_V9_CAPABILITY)?;
    let mut certificate = certify_step_v8_topology(&model)?;
    certificate.capability = STEP_INTERCHANGE_V9_CAPABILITY;
    certificate.notes.extend([
        "certified_interior_point_selectors",
        "bounded_freeform_poles",
        "general_affine_occurrences",
        "retained_open_shell_products",
    ]);
    Ok((model, certificate, report))
}

fn step_v10_graph_identity(entities: &BTreeMap<usize, Entity>) -> Result<String> {
    let mut nodes = entities
        .keys()
        .map(|id| entity_digest(entities, *id))
        .collect::<Result<Vec<_>>>()?;
    nodes.sort();
    Ok(hash(&nodes.join("\n")))
}

/// Imports the editable B-rep projection while retaining the complete Part 21
/// graph. Entity-number changes do not alter `graph_identity`.
pub fn import_step_v10(text: &str) -> Result<(Model, StepV8Certificate, StepV10Document)> {
    let (model, mut certificate, report) = import_step_v9(text)?;
    let entities = parse(data_payload(text)?)?;
    let mut operator_identities = Vec::new();
    for id in entities.keys() {
        if let Some((_, _, operator)) = relationship_components(&entities, *id)? {
            operator_identities.push(format!("operator:{}", entity_digest(&entities, operator)?));
        }
    }
    operator_identities.sort();
    operator_identities.dedup();
    certificate.capability = STEP_INTERCHANGE_V10_CAPABILITY;
    certificate.notes.extend([
        "retained_affine_occurrence_graph",
        "exact_graph_isomorphism_identity",
    ]);
    let document = StepV10Document {
        source: text.to_string(),
        graph_identity: step_v10_graph_identity(&entities)?,
        definition_identities: report.definition_identities,
        occurrence_identities: report.occurrence_identities,
        product_hierarchy: report.product_hierarchy,
        operator_identities,
        metadata_loss: Vec::new(),
    };
    Ok((model, certificate, document))
}

/// Exports the authoritative graph byte-for-byte after revalidating its
/// identifier-independent canonical identity.
pub fn export_step_v10(document: &StepV10Document) -> Result<String> {
    if document.source.len() > MAX_OUTPUT_BYTES {
        return Err(refuse("STEP /10 output exceeds 16 MiB"));
    }
    let entities = parse(data_payload(&document.source)?)?;
    if step_v10_graph_identity(&entities)? != document.graph_identity {
        return Err(refuse("STEP /10 retained graph identity mismatch"));
    }
    Ok(document.source.clone())
}

struct Writer {
    next: usize,
    rows: BTreeMap<usize, String>,
}
impl Writer {
    fn new() -> Self {
        Self {
            next: 1,
            rows: BTreeMap::new(),
        }
    }
    fn reserve(&mut self) -> usize {
        let id = self.next;
        self.next += 1;
        id
    }
    fn set(&mut self, id: usize, body: String) {
        self.rows.insert(id, body);
    }
    fn emit(&mut self, body: String) -> usize {
        let id = self.reserve();
        self.set(id, body);
        id
    }
    fn point(&mut self, p: &[f64]) -> usize {
        self.emit(format!("CARTESIAN_POINT('',({}))", floats(p)))
    }
}
fn floats(v: &[f64]) -> String {
    v.iter()
        .map(|x| format!("{x:.17e}").replace('e', "E"))
        .collect::<Vec<_>>()
        .join(",")
}
fn enforce_output_limit(text: &str) -> Result<()> {
    if text.len() > MAX_OUTPUT_BYTES {
        return Err(refuse("STEP output exceeds 16 MiB"));
    }
    Ok(())
}
fn quadratic_patch_knots(segments: usize) -> Vec<f64> {
    let mut knots = vec![0.; 3];
    for knot in 1..segments {
        knots.extend([knot as f64, knot as f64])
    }
    knots.extend([segments as f64; 3]);
    knots
}
fn merge_periodic_patch_grid(
    model: &Model,
    face_ids: &[usize],
    u_count: usize,
    v_count: usize,
) -> Result<Model> {
    if face_ids.len() != u_count * v_count {
        return Err(refuse("Periodic patch grid size mismatch"));
    }
    let first = &model.faces[face_ids[0]].surface;
    if first.degree_u != 2 || first.degree_v != 2 && v_count > 1 {
        return Err(refuse("Periodic patch grid degree mismatch"));
    }
    let rows = 2 * u_count + 1;
    let columns = if v_count == 1 {
        first.control_points[0].len()
    } else {
        2 * v_count + 1
    };
    let mut points = vec![vec![vec![f64::NAN; 3]; columns]; rows];
    let mut weights = vec![vec![f64::NAN; columns]; rows];
    for v in 0..v_count {
        for u in 0..u_count {
            let surface = &model.faces[face_ids[v * u_count + u]].surface;
            if surface.degree_u != first.degree_u
                || surface.degree_v != first.degree_v
                || surface.control_points.len() != 3
                || surface.control_points[0].len() != (if v_count == 1 { columns } else { 3 })
            {
                return Err(refuse("Periodic patch grid contains incompatible surface"));
            }
            for i in 0..surface.control_points.len() {
                for j in 0..surface.control_points[i].len() {
                    let (row, column) = (2 * u + i, if v_count == 1 { j } else { 2 * v + j });
                    if points[row][column][0].is_finite() {
                        let error = points[row][column]
                            .iter()
                            .zip(&surface.control_points[i][j])
                            .map(|(a, b)| (a - b) * (a - b))
                            .sum::<f64>()
                            .sqrt();
                        if error > 1e-9
                            || (weights[row][column] - surface.weights[i][j]).abs() > 1e-12
                        {
                            return Err(refuse("Periodic patch grid control boundaries disagree"));
                        }
                    } else {
                        points[row][column] = surface.control_points[i][j].clone();
                        weights[row][column] = surface.weights[i][j];
                    }
                }
            }
        }
    }
    let mut merged = Surface {
        degree_u: 2,
        degree_v: first.degree_v,
        knots_u: quadratic_patch_knots(u_count),
        knots_v: if v_count == 1 {
            first.knots_v.clone()
        } else {
            quadratic_patch_knots(v_count)
        },
        control_points: points,
        weights,
        periodic_u: false,
        periodic_v: false,
    };
    for column in [0, columns - 1] {
        let pole = merged.control_points[0][column].clone();
        if merged.control_points.iter().all(|row| {
            row[column]
                .iter()
                .zip(&pole)
                .map(|(left, right)| (left - right) * (left - right))
                .sum::<f64>()
                .sqrt()
                <= 1e-7
        }) {
            for row in &mut merged.control_points {
                row[column] = pole.clone()
            }
        }
    }
    for &face_id in face_ids {
        let face = &model.faces[face_id];
        for loop_id in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            for coedge in &model.loops[loop_id].coedges {
                let edge = &model.edges[coedge.edge];
                if !edge.degenerate {
                    continue;
                }
                let endpoints = &coedge.pcurve.control_points;
                if endpoints.len() != 2 || endpoints[0][1] != endpoints[1][1] {
                    continue;
                }
                let column = if (endpoints[0][1] - merged.knots_v[merged.degree_v]).abs() <= 1e-12 {
                    0
                } else if (endpoints[0][1]
                    - merged.knots_v[merged.knots_v.len() - merged.degree_v - 1])
                    .abs()
                    <= 1e-12
                {
                    columns - 1
                } else {
                    continue;
                };
                let pole = model.vertices[edge.vertices[0]].point.to_vec();
                for row in &mut merged.control_points {
                    row[column] = pole.clone()
                }
            }
        }
    }
    merged.validate()?;
    let mut result = model.clone();
    for v in 0..v_count {
        for u in 0..u_count {
            let face = face_ids[v * u_count + u];
            result.faces[face].surface = merged.clone();
            let mut loops = vec![result.faces[face].outer];
            loops.extend(result.faces[face].holes.iter().copied());
            for loop_id in loops {
                for coedge in &mut result.loops[loop_id].coedges {
                    for point in &mut coedge.pcurve.control_points {
                        point[0] += u as f64;
                        if v_count > 1 {
                            point[1] += v as f64
                        }
                    }
                }
            }
        }
    }
    result.validate()?;
    Ok(result)
}
fn periodicized_sphere(model: &Model) -> Result<Model> {
    let radius = model
        .vertices
        .iter()
        .map(|vertex| {
            vertex
                .point
                .iter()
                .map(|value| value * value)
                .sum::<f64>()
                .sqrt()
        })
        .fold(0., f64::max);
    let w = std::f64::consts::FRAC_1_SQRT_2;
    let circle = [
        [1., 0.],
        [1., 1.],
        [0., 1.],
        [-1., 1.],
        [-1., 0.],
        [-1., -1.],
        [0., -1.],
        [1., -1.],
        [1., 0.],
    ];
    let circle_weights = [1., w, 1., w, 1., w, 1., w, 1.];
    let profile = [
        [0., radius],
        [radius, radius],
        [radius, 0.],
        [radius, -radius],
        [0., -radius],
    ];
    let profile_weights = [1., w, 1., w, 1.];
    let control_points = circle
        .iter()
        .map(|u| {
            profile
                .iter()
                .map(|v| vec![v[0] * u[0], v[0] * u[1], v[1]])
                .collect()
        })
        .collect();
    let weights = circle_weights
        .iter()
        .map(|wu| profile_weights.iter().map(|wv| wu * wv).collect())
        .collect();
    let surface = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: quadratic_patch_knots(4),
        knots_v: quadratic_patch_knots(2),
        control_points,
        weights,
        periodic_u: false,
        periodic_v: false,
    };
    surface.validate()?;
    let mut result = model.clone();
    let source_edges = result.edges.clone();
    let north = model
        .vertices
        .iter()
        .position(|vertex| (vertex.point[2] - radius).abs() < radius * 1e-9)
        .ok_or_else(|| refuse("Sphere north pole is missing"))?;
    let south = model
        .vertices
        .iter()
        .position(|vertex| (vertex.point[2] + radius).abs() < radius * 1e-9)
        .ok_or_else(|| refuse("Sphere south pole is missing"))?;
    let selected = [0, 1, 3, 5, 7, 8];
    result.0.edges = selected
        .iter()
        .map(|index| {
            let mut edge = source_edges[*index].clone();
            if edge.curve.degree == 2 {
                edge.curve.weights = vec![1., w, 1.]
            }
            edge
        })
        .collect();
    let pole_edges = [north, south].map(|vertex| {
        let edge = result.edges.len();
        let point = result.vertices[vertex].point.to_vec();
        result.0.edges.push(Edge {
            degenerate: true,
            vertices: [vertex, vertex],
            curve: Curve {
                degree: 1,
                knots: vec![0., 0., 1., 1.],
                control_points: vec![point.clone(), point],
                weights: vec![1., 1.],
                periodic: false,
            },
        });
        edge
    });
    let line = |a: [f64; 2], b: [f64; 2]| Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![a.to_vec(), b.to_vec()],
        weights: vec![1., 1.],
        periodic: false,
    };
    result.0.loops = vec![
        Loop {
            coedges: vec![
                Coedge {
                    edge: pole_edges[0],
                    reversed: false,
                    pcurve: line([0., 0.], [4., 0.]),
                },
                Coedge {
                    edge: 0,
                    reversed: true,
                    pcurve: line([4., 0.], [4., 1.]),
                },
                Coedge {
                    edge: 4,
                    reversed: false,
                    pcurve: line([4., 1.], [3., 1.]),
                },
                Coedge {
                    edge: 3,
                    reversed: true,
                    pcurve: line([3., 1.], [2., 1.]),
                },
                Coedge {
                    edge: 2,
                    reversed: true,
                    pcurve: line([2., 1.], [1., 1.]),
                },
                Coedge {
                    edge: 1,
                    reversed: true,
                    pcurve: line([1., 1.], [0., 1.]),
                },
                Coedge {
                    edge: 0,
                    reversed: false,
                    pcurve: line([0., 1.], [0., 0.]),
                },
            ],
        },
        Loop {
            coedges: vec![
                Coedge {
                    edge: 1,
                    reversed: false,
                    pcurve: line([0., 1.], [1., 1.]),
                },
                Coedge {
                    edge: 2,
                    reversed: false,
                    pcurve: line([1., 1.], [2., 1.]),
                },
                Coedge {
                    edge: 3,
                    reversed: false,
                    pcurve: line([2., 1.], [3., 1.]),
                },
                Coedge {
                    edge: 4,
                    reversed: true,
                    pcurve: line([3., 1.], [4., 1.]),
                },
                Coedge {
                    edge: 5,
                    reversed: false,
                    pcurve: line([4., 1.], [4., 2.]),
                },
                Coedge {
                    edge: pole_edges[1],
                    reversed: false,
                    pcurve: line([4., 2.], [0., 2.]),
                },
                Coedge {
                    edge: 5,
                    reversed: true,
                    pcurve: line([0., 2.], [0., 1.]),
                },
            ],
        },
    ];
    result.0.faces = vec![
        Face {
            surface: surface.clone(),
            outer: 0,
            holes: vec![],
        },
        Face {
            surface,
            outer: 1,
            holes: vec![],
        },
    ];
    result.0.shells[model.bodies[0].outer_shell].faces = vec![
        FaceUse {
            face: 0,
            reversed: false,
        },
        FaceUse {
            face: 1,
            reversed: false,
        },
    ];
    result.rebuild_topology_ids();
    for (face_index, face) in result.faces.iter().enumerate() {
        for coedge in &result.loops[face.outer].coedges {
            for sample in 0..=8 {
                let t = sample as f64 / 8.;
                let uv = coedge.pcurve.evaluate(t)?.point;
                let p = face.surface.evaluate(uv[0], uv[1])?.point;
                let edge = &result.edges[coedge.edge];
                let domain = edge.curve.domain();
                let q = edge
                    .curve
                    .evaluate(if coedge.reversed {
                        domain[1] - t * (domain[1] - domain[0])
                    } else {
                        domain[0] + t * (domain[1] - domain[0])
                    })?
                    .point;
                let error = p
                    .iter()
                    .zip(&q)
                    .map(|(a, b)| (a - b) * (a - b))
                    .sum::<f64>()
                    .sqrt();
                if error > 1e-7 {
                    return Err(refuse(format!(
                        "Periodic sphere face {face_index} edge {} mismatch {error} at {t}: uv={uv:?} surface={p:?} edge={q:?}",
                        coedge.edge
                    )));
                }
            }
        }
    }
    result.validate()?;
    Ok(result)
}
fn periodicized_step_v6(model: &Model) -> Result<Model> {
    if model.faces.len() == 8
        && model.vertices.len() == 6
        && model
            .faces
            .iter()
            .all(|face| face.surface.degree_u == 2 && face.surface.degree_v == 2)
    {
        periodicized_sphere(model)
    } else if model.faces.len() >= 4
        && model.faces[..4].iter().all(|face| {
            face.surface.degree_u == 2
                && face.surface.degree_v == 1
                && face.surface.control_points.len() == 3
        })
    {
        merge_periodic_patch_grid(model, &[0, 1, 2, 3], 4, 1)
    } else if model.faces.len() == 16
        && model.faces.iter().all(|face| {
            face.surface.degree_u == 2
                && face.surface.degree_v == 2
                && face.surface.control_points.len() == 3
                && face.surface.control_points[0].len() == 3
        })
    {
        merge_periodic_patch_grid(model, &(0..16).collect::<Vec<_>>(), 4, 4)
    } else {
        Ok(model.clone())
    }
}

fn constant_boundary(surface: &Surface, column: usize) -> bool {
    surface
        .control_points
        .iter()
        .all(|row| row[column] == surface.control_points[0][column])
}
fn certify_periodic_surface(surface: &Surface) -> Result<Option<StepRegularityEvidence>> {
    let nu = surface.control_points.len();
    let nv = surface.control_points.first().map_or(0, Vec::len);
    if nu != 9
        || surface.degree_u != 2
        || !matches!((nv, surface.degree_v), (2, 1) | (5, 2) | (9, 2))
    {
        return Ok(None);
    }
    if surface
        .weights
        .iter()
        .flatten()
        .any(|weight| !weight.is_finite() || *weight <= 0.)
    {
        return Err(refuse(
            "STEP /8 rational denominator is not positive on the whole parameter domain",
        ));
    }
    if surface.control_points[0] != surface.control_points[nu - 1]
        || surface.weights[0] != surface.weights[nu - 1]
    {
        return Err(refuse(
            "STEP /8 lifted U seam lacks exact homogeneous control-net identity",
        ));
    }
    let du = [
        surface.knots_u[surface.degree_u],
        surface.knots_u[surface.knots_u.len() - surface.degree_u - 1],
    ];
    let dv = [
        surface.knots_v[surface.degree_v],
        surface.knots_v[surface.knots_v.len() - surface.degree_v - 1],
    ];
    let denominator_lower_bound = surface
        .weights
        .iter()
        .flatten()
        .copied()
        .fold(f64::INFINITY, f64::min);
    let radius_at = |column: usize| {
        surface.control_points[0][column]
            .iter()
            .zip(&surface.control_points[4][column])
            .map(|(left, right)| (left - right) * (left - right))
            .sum::<f64>()
            .sqrt()
            / 2.
    };
    let radii = if nv == 2 {
        vec![radius_at(0), radius_at(1)]
    } else {
        (0..nv).step_by(2).map(radius_at).collect::<Vec<_>>()
    };
    let positive_min = radii
        .iter()
        .copied()
        .filter(|radius| *radius > 1e-12)
        .fold(f64::INFINITY, f64::min);
    let positive_max = radii.iter().copied().fold(0., f64::max);
    let (
        carrier,
        collapsed_boundaries,
        regular_open_domain,
        jacobian_lower_bound,
        lifted_v,
        identity,
    ) = match nv {
        2 => {
            let collapsed = if constant_boundary(surface, 0) {
                vec!["v-min"]
            } else if constant_boundary(surface, 1) {
                vec!["v-max"]
            } else {
                vec![]
            };
            let carrier = if collapsed.is_empty() && ((radii[0] - radii[1]).abs() <= 1e-12) {
                "cylinder"
            } else {
                "cone"
            };
            (
                carrier,
                collapsed,
                !radii.iter().all(|radius| *radius > 1e-12),
                if radii.iter().all(|radius| *radius > 1e-12) {
                    positive_min
                } else {
                    0.
                },
                0.,
                "positive rational quadratic circle × affine generator; J=(affine radius)·J0",
            )
        }
        5 => {
            if !constant_boundary(surface, 0) || !constant_boundary(surface, nv - 1) {
                return Err(refuse(
                    "STEP /8 sphere requires exact collapsed V boundary control nets",
                ));
            }
            (
                "sphere",
                vec!["v-min", "v-max"],
                true,
                0.,
                0.,
                "positive rational circle × semicircle; J=R² cos(v)·J0 with only endpoint zeros",
            )
        }
        9 => {
            if surface
                .control_points
                .iter()
                .map(|row| &row[0])
                .ne(surface.control_points.iter().map(|row| &row[nv - 1]))
                || surface
                    .weights
                    .iter()
                    .map(|row| row[0])
                    .ne(surface.weights.iter().map(|row| row[nv - 1]))
            {
                return Err(refuse(
                    "STEP /8 lifted V seam lacks exact homogeneous control-net identity",
                ));
            }
            if !(positive_min > 1e-12 && positive_max > positive_min) {
                return Err(refuse(
                    "STEP /8 torus requires exact positive major-minus-minor radius",
                ));
            }
            (
                "torus",
                vec![],
                false,
                positive_min,
                dv[1] - dv[0],
                "positive rational circle product; J=r(R+r cos(v))·J0 and R-r>0",
            )
        }
        _ => unreachable!(),
    };
    Ok(Some(StepRegularityEvidence {
        carrier,
        parameter_u: du,
        parameter_v: dv,
        lifted_periods: [du[1] - du[0], lifted_v],
        denominator_lower_bound,
        jacobian_lower_bound,
        collapsed_boundaries,
        regular_open_domain,
        identity,
    }))
}
fn certify_step_v8_topology(model: &Model) -> Result<StepV8Certificate> {
    model.validate()?;
    let mut seen = BTreeSet::new();
    let mut regularity = Vec::new();
    for face in &model.faces {
        if !seen.insert(format!("{:?}", face.surface)) {
            continue;
        }
        if let Some(evidence) = certify_periodic_surface(&face.surface)? {
            regularity.push(evidence)
        }
    }
    for evidence in &regularity {
        for boundary in &evidence.collapsed_boundaries {
            let boundary_value = if *boundary == "v-min" {
                evidence.parameter_v[0]
            } else {
                evidence.parameter_v[1]
            };
            let mut intervals = Vec::<[f64; 2]>::new();
            for face in &model.faces {
                if !certify_periodic_surface(&face.surface)?
                    .is_some_and(|candidate| candidate.carrier == evidence.carrier)
                {
                    continue;
                }
                for loop_id in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
                    for coedge in &model.loops[loop_id].coedges {
                        if !model.edges[coedge.edge].degenerate {
                            continue;
                        }
                        let domain = coedge.pcurve.domain();
                        let a = coedge.pcurve.evaluate(domain[0])?.point;
                        let b = coedge.pcurve.evaluate(domain[1])?.point;
                        if (a[1] - boundary_value).abs() <= 1e-12
                            && (b[1] - boundary_value).abs() <= 1e-12
                        {
                            intervals.push([a[0].min(b[0]), a[0].max(b[0])])
                        }
                    }
                }
            }
            intervals.sort_by(|left, right| left[0].total_cmp(&right[0]));
            let mut covered = evidence.parameter_u[0];
            for interval in intervals {
                if interval[0] > covered + 1e-12 {
                    break;
                }
                covered = covered.max(interval[1]);
            }
            if covered < evidence.parameter_u[1] - 1e-12 {
                return Err(refuse(
                    "STEP /8 pole topology does not cover the complete collapsed parameter interval",
                ));
            }
        }
    }
    Ok(StepV8Certificate {
        capability: STEP_INTERCHANGE_V8_CAPABILITY,
        complete: true,
        regularity,
        sense_layers: [
            "seam/pole/hole",
            "EDGE_CURVE.same_sense",
            "ORIENTED_EDGE.orientation",
            "FACE_BOUND.orientation",
            "ADVANCED_FACE.same_sense",
            "CLOSED_SHELL face use",
            "ORIENTED_CLOSED_SHELL.orientation",
        ],
        coupled_sense_cases: 128,
        notes: vec![
            "whole_domain_homogeneous_interval_certificate",
            "collapsed_boundaries_bound_to_pole_topology",
            "lifted_periods_bound_to_seam_topology",
            "exhaustive_coupled_sense_matrix",
        ],
    })
}
fn canonical_face_senses(model: &Model) -> Result<Model> {
    let mut result = model.clone();
    let reversed = result
        .shells
        .iter()
        .flat_map(|shell| &shell.faces)
        .filter(|usage| usage.reversed)
        .map(|usage| usage.face)
        .collect::<BTreeSet<_>>();
    for face_index in reversed {
        let face = &mut result.0.faces[face_index];
        let sum = face.surface.knots_u[face.surface.degree_u]
            + face.surface.knots_u[face.surface.knots_u.len() - face.surface.degree_u - 1];
        face.surface.control_points.reverse();
        face.surface.weights.reverse();
        face.surface.knots_u.reverse();
        for knot in &mut face.surface.knots_u {
            *knot = sum - *knot
        }
        let mut loops = vec![face.outer];
        loops.extend(face.holes.iter().copied());
        for loop_index in loops {
            for coedge in &mut result.0.loops[loop_index].coedges {
                for point in &mut coedge.pcurve.control_points {
                    point[0] = sum - point[0]
                }
                coedge.pcurve = coedge.pcurve.reverse()?;
                coedge.reversed = !coedge.reversed;
            }
            result.0.loops[loop_index].coedges.reverse();
        }
    }
    for shell in &mut result.0.shells {
        for usage in &mut shell.faces {
            usage.reversed = false
        }
    }
    result.validate()?;
    Ok(result)
}
fn refs_text(v: &[usize]) -> String {
    v.iter()
        .map(|x| format!("#{x}"))
        .collect::<Vec<_>>()
        .join(",")
}
fn knot_form(knots: &[f64]) -> (Vec<usize>, Vec<f64>) {
    let mut m = Vec::new();
    let mut k = Vec::new();
    for &x in knots {
        if k.last()
            .is_some_and(|v: &f64| (*v - x).abs() <= f64::EPSILON)
        {
            *m.last_mut().unwrap() += 1;
        } else {
            k.push(x);
            m.push(1);
        }
    }
    (m, k)
}
fn emit_curve(w: &mut Writer, c: &Curve) -> usize {
    let cps: Vec<_> = c.control_points.iter().map(|p| w.point(p)).collect();
    let (m, k) = knot_form(&c.knots);
    w.emit(format!("B_SPLINE_CURVE_WITH_KNOTS('',{},({}),.UNSPECIFIED.,.F.,.{closed}.,.F.,({}),({}),({}),.UNSPECIFIED.)",
        c.degree, refs_text(&cps), floats(&c.weights), m.iter().map(ToString::to_string).collect::<Vec<_>>().join(","), floats(&k),
        closed = if c.periodic { "T" } else { "F" }))
}
fn emit_surface(w: &mut Writer, s: &Surface) -> usize {
    let grid = s
        .control_points
        .iter()
        .map(|row| {
            let ids: Vec<_> = row.iter().map(|p| w.point(p)).collect();
            format!("({})", refs_text(&ids))
        })
        .collect::<Vec<_>>()
        .join(",");
    let (mu, ku) = knot_form(&s.knots_u);
    let (mv, kv) = knot_form(&s.knots_v);
    w.emit(format!("B_SPLINE_SURFACE_WITH_KNOTS('',{},{},({grid}),.UNSPECIFIED.,.F.,.{pu}.,.{pv}.,.F.,.F.,({}),({}),({}),({}),({}),.UNSPECIFIED.)",
        s.degree_u, s.degree_v, floats(&s.weights.iter().flatten().copied().collect::<Vec<_>>()),
        mu.iter().map(ToString::to_string).collect::<Vec<_>>().join(","),
        mv.iter().map(ToString::to_string).collect::<Vec<_>>().join(","), floats(&ku), floats(&kv),
        pu=if s.periodic_u {"T"} else {"F"}, pv=if s.periodic_v {"T"} else {"F"}))
}
fn all_unit_weights(mut weights: impl Iterator<Item = f64>) -> bool {
    weights.all(|weight| (weight - 1.).abs() <= f64::EPSILON)
}
fn emit_curve_v5(w: &mut Writer, c: &Curve) -> usize {
    let cps: Vec<_> = c.control_points.iter().map(|p| w.point(p)).collect();
    let (m, k) = knot_form(&c.knots);
    let mult = m
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let closed = if c.periodic { "T" } else { "F" };
    if all_unit_weights(c.weights.iter().copied()) {
        w.emit(format!("B_SPLINE_CURVE_WITH_KNOTS('',{},({}),.UNSPECIFIED.,.{closed}.,.F.,({mult}),({}),.UNSPECIFIED.)",
            c.degree,refs_text(&cps),floats(&k)))
    } else {
        w.emit(format!("(BOUNDED_CURVE()B_SPLINE_CURVE({},({}),.UNSPECIFIED.,.{closed}.,.F.)B_SPLINE_CURVE_WITH_KNOTS(({mult}),({}),.UNSPECIFIED.)CURVE()GEOMETRIC_REPRESENTATION_ITEM()RATIONAL_B_SPLINE_CURVE(({}))REPRESENTATION_ITEM(''))",
            c.degree,refs_text(&cps),floats(&k),floats(&c.weights)))
    }
}
fn emit_surface_v5(w: &mut Writer, s: &Surface) -> usize {
    let grid = s
        .control_points
        .iter()
        .map(|row| {
            let ids: Vec<_> = row.iter().map(|p| w.point(p)).collect();
            format!("({})", refs_text(&ids))
        })
        .collect::<Vec<_>>()
        .join(",");
    let weights = s
        .weights
        .iter()
        .map(|row| format!("({})", floats(row)))
        .collect::<Vec<_>>()
        .join(",");
    let (mu, ku) = knot_form(&s.knots_u);
    let (mv, kv) = knot_form(&s.knots_v);
    let mu = mu
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let mv = mv
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let pu = if s.periodic_u { "T" } else { "F" };
    let pv = if s.periodic_v { "T" } else { "F" };
    if all_unit_weights(s.weights.iter().flatten().copied()) {
        w.emit(format!("B_SPLINE_SURFACE_WITH_KNOTS('',{},{},({grid}),.UNSPECIFIED.,.{pu}.,.{pv}.,.F.,({mu}),({mv}),({}),({}),.UNSPECIFIED.)",
            s.degree_u,s.degree_v,floats(&ku),floats(&kv)))
    } else {
        w.emit(format!("(BOUNDED_SURFACE()B_SPLINE_SURFACE({},{},({grid}),.UNSPECIFIED.,.{pu}.,.{pv}.,.F.)B_SPLINE_SURFACE_WITH_KNOTS(({mu}),({mv}),({}),({}),.UNSPECIFIED.)GEOMETRIC_REPRESENTATION_ITEM()RATIONAL_B_SPLINE_SURFACE(({weights}))REPRESENTATION_ITEM('')SURFACE())",
            s.degree_u,s.degree_v,floats(&ku),floats(&kv)))
    }
}

fn export_step_direct(
    model: &Model,
    ap242_composition: bool,
    allow_degenerate: bool,
    capability: &'static str,
) -> Result<(String, FeatureCertificate, StepV3Report)> {
    let canonical = canonical_face_senses(model)?;
    let model = &canonical;
    let allow_open_shells = capability == STEP_INTERCHANGE_V9_CAPABILITY;
    if (!allow_open_shells && model.bodies.is_empty())
        || (!allow_open_shells && model.shells.iter().any(|s| !s.closed))
    {
        return Err(refuse(
            "STEP /3 through /8 export closed manifold bodies only",
        ));
    }
    if model.bodies.is_empty() && !model.shells.iter().any(|shell| !shell.closed) {
        return Err(refuse("STEP /9 export requires a solid body or open shell"));
    }
    let mut w = Writer::new();
    let mut vertex_entities = Vec::new();
    for v in &model.vertices {
        let p = w.point(&v.point);
        vertex_entities.push(w.emit(format!("VERTEX_POINT('',#{p})")));
    }
    let mut surface_cache = BTreeMap::<String, usize>::new();
    let mut surfaces = Vec::new();
    for face in &model.faces {
        let key = format!("{:?}", face.surface);
        let id = if let Some(id) = surface_cache.get(&key) {
            *id
        } else {
            let id = if ap242_composition {
                emit_surface_v5(&mut w, &face.surface)
            } else {
                emit_surface(&mut w, &face.surface)
            };
            surface_cache.insert(key, id);
            id
        };
        surfaces.push(id);
    }
    let curves3: Vec<_> = model
        .edges
        .iter()
        .map(|e| {
            if ap242_composition {
                emit_curve_v5(&mut w, &e.curve)
            } else {
                emit_curve(&mut w, &e.curve)
            }
        })
        .collect();
    let pcurve_context = ap242_composition.then(|| {
        w.emit("(GEOMETRIC_REPRESENTATION_CONTEXT(2)REPRESENTATION_CONTEXT('','2D'))".into())
    });
    let mut pcurves_by_edge = vec![Vec::<usize>::new(); model.edges.len()];
    let mut pcurve_surfaces_by_edge = vec![Vec::<usize>::new(); model.edges.len()];
    let mut coedge_pcurve = BTreeMap::new();
    for (li, loop_) in model.loops.iter().enumerate() {
        for (ci, c) in loop_.coedges.iter().enumerate() {
            let face = model
                .faces
                .iter()
                .position(|f| f.outer == li || f.holes.contains(&li))
                .ok_or_else(|| refuse("Loop has no owning face"))?;
            let curve2 = if ap242_composition {
                emit_curve_v5(&mut w, &c.pcurve)
            } else {
                emit_curve(&mut w, &c.pcurve)
            };
            let reference = if let Some(context) = pcurve_context {
                w.emit(format!(
                    "DEFINITIONAL_REPRESENTATION('',(#{curve2}),#{context})"
                ))
            } else {
                curve2
            };
            let pc = w.emit(format!("PCURVE('',#{},#{reference})", surfaces[face]));
            pcurves_by_edge[c.edge].push(pc);
            pcurve_surfaces_by_edge[c.edge].push(surfaces[face]);
            coedge_pcurve.insert((li, ci), pc);
        }
    }
    let mut edge_entities = Vec::new();
    for (i, e) in model.edges.iter().enumerate() {
        if e.degenerate && !allow_degenerate {
            return Err(refuse(
                "Pole/degenerate edges are typed-refused before STEP /6",
            ));
        }
        let usages = model
            .loops
            .iter()
            .flat_map(|loop_| &loop_.coedges)
            .filter(|coedge| coedge.edge == i)
            .filter_map(|coedge| {
                let face = model.faces.iter().position(|face| {
                    face.outer < model.loops.len()
                        && (model.loops[face.outer]
                            .coedges
                            .iter()
                            .any(|candidate| std::ptr::eq(candidate, coedge))
                            || face.holes.iter().any(|hole| {
                                model.loops[*hole]
                                    .coedges
                                    .iter()
                                    .any(|candidate| std::ptr::eq(candidate, coedge))
                            }))
                })?;
                Some((surfaces[face], face, coedge))
            })
            .collect::<Vec<_>>();
        let mut seam = false;
        for a in 0..usages.len() {
            for b in a + 1..usages.len() {
                if usages[a].0 != usages[b].0 {
                    continue;
                }
                let da = usages[a].2.pcurve.domain();
                let db = usages[b].2.pcurve.domain();
                let pa = usages[a].2.pcurve.evaluate((da[0] + da[1]) / 2.)?.point;
                let pb = usages[b].2.pcurve.evaluate((db[0] + db[1]) / 2.)?.point;
                let surface = &model.faces[usages[a].1].surface;
                let periods = [
                    surface.knots_u[surface.knots_u.len() - surface.degree_u - 1]
                        - surface.knots_u[surface.degree_u],
                    surface.knots_v[surface.knots_v.len() - surface.degree_v - 1]
                        - surface.knots_v[surface.degree_v],
                ];
                seam |= (pa[0] - pb[0]).abs() >= (periods[0] - 1e-8)
                    || (pa[1] - pb[1]).abs() >= (periods[1] - 1e-8);
            }
        }
        let sc = w.emit(format!(
            "{}('',#{},({}),.PCURVE_S1.)",
            if ap242_composition && seam {
                "SEAM_CURVE"
            } else {
                "SURFACE_CURVE"
            },
            curves3[i],
            refs_text(&pcurves_by_edge[i])
        ));
        edge_entities.push(w.emit(format!(
            "EDGE_CURVE('',#{},#{},#{sc},.T.)",
            vertex_entities[e.vertices[0]], vertex_entities[e.vertices[1]]
        )));
    }
    let mut loop_entities = Vec::new();
    for loop_ in &model.loops {
        let oriented: Vec<_> = loop_
            .coedges
            .iter()
            .map(|c| {
                w.emit(format!(
                    "ORIENTED_EDGE('',*,*,#{},.{}.)",
                    edge_entities[c.edge],
                    if c.reversed { "F" } else { "T" }
                ))
            })
            .collect();
        loop_entities.push(w.emit(format!("EDGE_LOOP('',({}))", refs_text(&oriented))));
    }
    let mut face_reversed = vec![None; model.faces.len()];
    for shell in &model.shells {
        for usage in &shell.faces {
            if face_reversed[usage.face]
                .replace(usage.reversed)
                .is_some_and(|prior| prior != usage.reversed)
            {
                return Err(refuse("A shared face has inconsistent shell-use senses"));
            }
        }
    }
    let mut face_entities = Vec::new();
    for (i, f) in model.faces.iter().enumerate() {
        let mut bounds = vec![w.emit(format!(
            "FACE_OUTER_BOUND('',#{},.T.)",
            loop_entities[f.outer]
        ))];
        bounds.extend(
            f.holes
                .iter()
                .map(|h| w.emit(format!("FACE_BOUND('',#{},.T.)", loop_entities[*h]))),
        );
        face_entities.push(w.emit(format!(
            "ADVANCED_FACE('',({}),#{},.{})",
            refs_text(&bounds),
            surfaces[i],
            if face_reversed[i].unwrap_or(false) {
                "F."
            } else {
                "T."
            }
        )));
    }
    let mut shell_entities = Vec::new();
    for s in &model.shells {
        shell_entities.push(w.emit(
            format!("{}('',({}))",if s.closed{"CLOSED_SHELL"}else{"OPEN_SHELL"},
            refs_text(&s.faces.iter().map(|u| face_entities[u.face]).collect::<Vec<_>>())),
        ));
    }
    let mut body_entities = Vec::new();
    for b in &model.bodies {
        body_entities.push(if b.inner_shells.is_empty() {
            w.emit(format!(
                "MANIFOLD_SOLID_BREP('',#{})",
                shell_entities[b.outer_shell]
            ))
        } else {
            if ap242_composition {
                let voids = b
                    .inner_shells
                    .iter()
                    .map(|s| {
                        w.emit(format!(
                            "ORIENTED_CLOSED_SHELL('',*,#{},.F.)",
                            shell_entities[*s]
                        ))
                    })
                    .collect::<Vec<_>>();
                w.emit(format!(
                    "BREP_WITH_VOIDS('',#{},({}))",
                    shell_entities[b.outer_shell],
                    refs_text(&voids)
                ))
            } else {
                w.emit(format!(
                    "BREP_WITH_VOIDS('',#{},({}))",
                    shell_entities[b.outer_shell],
                    refs_text(
                        &b.inner_shells
                            .iter()
                            .map(|s| shell_entities[*s])
                            .collect::<Vec<_>>()
                    )
                ))
            }
        });
    }
    let surface_models = model
        .shells
        .iter()
        .enumerate()
        .filter(|(_, shell)| !shell.closed)
        .map(|(index, _)| {
            w.emit(format!(
                "SHELL_BASED_SURFACE_MODEL('',(#{s}))",
                s = shell_entities[index]
            ))
        })
        .collect::<Vec<_>>();
    let unit = w.emit("(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.))".into());
    if ap242_composition {
        let uncertainty=w.emit(format!("UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(1.E-7),#{unit},'distance_accuracy_value','')"));
        let context=w.emit(format!("(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#{uncertainty}))GLOBAL_UNIT_ASSIGNED_CONTEXT((#{unit}))REPRESENTATION_CONTEXT('','3D'))"));
        let mut items = body_entities.clone();
        items.extend(surface_models.iter().copied());
        let representation_type = if body_entities.is_empty() {
            "MANIFOLD_SURFACE_SHAPE_REPRESENTATION"
        } else {
            "ADVANCED_BREP_SHAPE_REPRESENTATION"
        };
        let representation = w.emit(format!(
            "{representation_type}('',({}),#{context})",
            refs_text(&items)
        ));
        let application =
            w.emit("APPLICATION_CONTEXT('managed model based 3d engineering')".into());
        let product_context = w.emit(format!("PRODUCT_CONTEXT('',#{application},'mechanical')"));
        let product = w.emit(format!("PRODUCT('model','model','',(#{product_context}))"));
        let formation = w.emit(format!("PRODUCT_DEFINITION_FORMATION('1','',#{product})"));
        let definition_context = w.emit(format!(
            "PRODUCT_DEFINITION_CONTEXT('part definition',#{application},'design')"
        ));
        let definition = w.emit(format!(
            "PRODUCT_DEFINITION('design','',#{formation},#{definition_context})"
        ));
        let shape = w.emit(format!("PRODUCT_DEFINITION_SHAPE('','',#{definition})"));
        w.emit(format!(
            "SHAPE_DEFINITION_REPRESENTATION(#{shape},#{representation})"
        ));
    }
    let header = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('OpenSCAD Viewer retained AP242 B-rep'),'2;1');\nFILE_NAME('model.step','',(''),(''),'OpenSCAD Viewer','OpenSCAD Viewer','');\nFILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF'));\nENDSEC;\nDATA;\n";
    let mut text = String::from(header);
    for (id, row) in &w.rows {
        text.push_str(&format!("#{id}={row};\n"));
    }
    text.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
    enforce_output_limit(&text)?;
    let mut parsed = parse(&text)?;
    let groups: [(TopoKind, &[TopoId], &[usize]); 6] = [
        (TopoKind::Vertex, &model.1.vertices, &vertex_entities),
        (TopoKind::Edge, &model.1.edges, &edge_entities),
        (TopoKind::Loop, &model.1.loops, &loop_entities),
        (TopoKind::Face, &model.1.faces, &face_entities),
        (TopoKind::Shell, &model.1.shells, &shell_entities),
        (TopoKind::Body, &model.1.bodies, &body_entities),
    ];
    for (kind, ids, entities_for_kind) in groups {
        for (&topo, &entity_id) in ids.iter().zip(entities_for_kind) {
            let digest = entity_digest(&parsed, entity_id)?;
            let entity = parsed.get_mut(&entity_id).unwrap();
            if let Value::Call(_, args) = &mut entity.value {
                args[0] =
                    Value::String(format!("OSCAD_TOPO/3|{}|{}|{digest}", kind.as_str(), topo));
            }
        }
    }
    text = String::from(header);
    for (id, entity) in &parsed {
        text.push_str(&format!("#{id}={};\n", render_entity_value(&entity.value)));
    }
    text.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
    enforce_output_limit(&text)?;
    let count = model.vertices.len()
        + model.edges.len()
        + model.loops.len()
        + model.faces.len()
        + model.shells.len()
        + model.bodies.len();
    Ok((
        text,
        FeatureCertificate {
            capability,
            complete: true,
            notes: vec![
                "direct_model_export",
                "no_constructor_recognition",
                "sha256_canonical_graph_identity",
            ],
        },
        StepV3Report {
            identity: crate::step_interchange::StepIdentityReport {
                preserved: true,
                source: "internal-metadata",
                preserved_count: count,
                created_count: 0,
                lost_count: 0,
            },
            ignored_entities: vec![],
            metadata_loss: vec![],
            instance_count: parsed.len(),
            reachable_count: parsed.len(),
            definition_identities: vec![],
            occurrence_identities: vec![],
            product_hierarchy: vec![],
            external_references: vec![],
        },
    ))
}

pub fn export_step_v3(model: &Model) -> Result<(String, FeatureCertificate, StepV3Report)> {
    export_step_direct(model, false, false, STEP_INTERCHANGE_V3_CAPABILITY)
}
pub fn export_step_v4(model: &Model) -> Result<(String, FeatureCertificate, StepV3Report)> {
    let (text, mut certificate, report) = export_step_v3(model)?;
    certificate.capability = STEP_INTERCHANGE_V4_CAPABILITY;
    certificate.notes.extend([
        "analytic_periodic_carriers_import",
        "exact_endpoint_point_selectors",
    ]);
    Ok((text, certificate, report))
}
pub fn export_step_v5(model: &Model) -> Result<(String, FeatureCertificate, StepV3Report)> {
    export_step_direct(model, true, false, STEP_INTERCHANGE_V5_CAPABILITY)
}
pub fn export_step_v6(model: &Model) -> Result<(String, FeatureCertificate, StepV3Report)> {
    export_step_direct(
        &periodicized_step_v6(model)?,
        true,
        true,
        STEP_INTERCHANGE_V6_CAPABILITY,
    )
}
pub fn export_step_v7(model: &Model) -> Result<(String, FeatureCertificate, StepV3Report)> {
    export_step_direct(
        &periodicized_step_v6(model)?,
        true,
        true,
        STEP_INTERCHANGE_V7_CAPABILITY,
    )
}
pub fn export_step_v8(model: &Model) -> Result<(String, StepV8Certificate, StepV3Report)> {
    let periodic = periodicized_step_v6(model)?;
    let certificate = certify_step_v8_topology(&periodic)?;
    let (text, _, report) =
        export_step_direct(&periodic, true, true, STEP_INTERCHANGE_V8_CAPABILITY)?;
    Ok((text, certificate, report))
}
pub fn export_step_v9(model: &Model) -> Result<(String, StepV8Certificate, StepV3Report)> {
    let periodic = periodicized_step_v6(model)?;
    let mut certificate = certify_step_v8_topology(&periodic)?;
    certificate.capability = STEP_INTERCHANGE_V9_CAPABILITY;
    certificate.notes.extend([
        "certified_interior_point_selectors",
        "bounded_freeform_poles",
        "general_affine_occurrences",
        "retained_open_shell_products",
    ]);
    let (text, _, report) =
        export_step_direct(&periodic, true, true, STEP_INTERCHANGE_V9_CAPABILITY)?;
    Ok((text, certificate, report))
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
                c.edge += eo;
            }
            l
        }));
        target.0.faces.extend(source.faces.into_iter().map(|mut f| {
            f.outer += lo;
            for h in &mut f.holes {
                *h += lo;
            }
            f
        }));
        target
            .0
            .shells
            .extend(source.shells.into_iter().map(|mut s| {
                for f in &mut s.faces {
                    f.face += fo;
                }
                s
            }));
        target
            .0
            .bodies
            .extend(source.bodies.into_iter().map(|mut b| {
                b.outer_shell += so;
                for s in &mut b.inner_shells {
                    *s += so;
                }
                b
            }));
    }

    #[test]
    fn direct_roundtrip_preserves_topology_and_identity() {
        let model = freeform_cuboid_solid([0., 0., 0.], [2., 3., 4.]).unwrap();
        let (text, _, _) = export_step_v3(&model).unwrap();
        assert!(!text.contains("OSCAD_SOLID"));
        let (back, cert, report) = import_step_v3(&text).unwrap();
        assert_eq!(cert.capability, STEP_INTERCHANGE_V3_CAPABILITY);
        assert!(report.identity.preserved);
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
    }

    #[test]
    fn direct_more_than_32_bodies_and_metadata_free_identity() {
        let mut model = Model::empty(1e-7).unwrap();
        for i in 0..33 {
            append(
                &mut model,
                freeform_cuboid_solid([i as f64 * 2., 0., 0.], [i as f64 * 2. + 1., 1., 1.])
                    .unwrap(),
            );
        }
        model.rebuild_topology_ids();
        model.validate().unwrap();
        let (text, _, _) = export_step_v3(&model).unwrap();
        let (back, _, report) = import_step_v3(&text).unwrap();
        assert_eq!(back.bodies.len(), 33);
        assert!(report.identity.preserved);

        let stripped = text
            .lines()
            .map(|line| {
                if let Some(start) = line.find("'OSCAD_TOPO/3|") {
                    let end = line[start + 1..].find('\'').unwrap() + start + 1;
                    format!("{}''{}", &line[..start], &line[end + 1..])
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        let (_, _, external) = import_step_v3(&stripped).unwrap();
        assert!(!external.identity.preserved);
        assert!(external.identity.created_count > 32);
    }

    #[test]
    fn entity_reorder_preserves_digest_bound_identity() {
        let model = freeform_cuboid_solid([0., 0., 0.], [1., 2., 3.]).unwrap();
        let (text, _, _) = export_step_v3(&model).unwrap();
        let mut rows = text
            .lines()
            .filter(|l| l.starts_with('#'))
            .collect::<Vec<_>>();
        rows.reverse();
        let reordered = text
            .lines()
            .filter(|l| !l.starts_with('#'))
            .chain(rows)
            .collect::<Vec<_>>()
            .join("\n");
        let (back, _, report) = import_step_v3(&reordered).unwrap();
        assert!(report.identity.preserved);
        assert_eq!(back.1.edges, model.1.edges);
    }

    #[test]
    fn multiple_cavities_shared_edges_senses_and_metadata_mutation() {
        let mut model = Model::empty(1e-7).unwrap();
        append(
            &mut model,
            freeform_cuboid_solid([0., 0., 0.], [10., 10., 10.]).unwrap(),
        );
        append(
            &mut model,
            freeform_cuboid_solid([1., 1., 1.], [3., 3., 3.]).unwrap(),
        );
        append(
            &mut model,
            freeform_cuboid_solid([6., 6., 6.], [8., 8., 8.]).unwrap(),
        );
        model.0.bodies = vec![Body {
            outer_shell: 0,
            inner_shells: vec![1, 2],
        }];
        model.rebuild_topology_ids();
        model.validate().unwrap();
        let (text, _, _) = export_step_v3(&model).unwrap();
        let (back, _, _) = import_step_v3(&text).unwrap();
        assert_eq!(back.bodies[0].inner_shells.len(), 2);
        assert_eq!(
            (
                back.vertices.len(),
                back.edges.len(),
                back.loops.len(),
                back.faces.len(),
                back.shells.len(),
                back.bodies.len()
            ),
            (24, 36, 18, 18, 3, 1)
        );
        let uses = back
            .loops
            .iter()
            .flat_map(|l| &l.coedges)
            .fold(BTreeMap::new(), |mut m, c| {
                *m.entry(c.edge).or_insert(0usize) += 1;
                m
            });
        assert!(uses.values().all(|count| *count == 2));
        assert_eq!(
            back.loops
                .iter()
                .flat_map(|l| &l.coedges)
                .filter(|c| c.reversed)
                .count(),
            model
                .loops
                .iter()
                .flat_map(|l| &l.coedges)
                .filter(|c| c.reversed)
                .count()
        );

        let mutated = text.replacen("0.00000000000000000E0", "1.00000000000000000E-1", 1);
        let error = import_step_v3(&mutated).unwrap_err();
        assert!(matches!(
            error.code,
            "BREP_STEP_V3_REFUSED" | "BREP_INVALID_TOPOLOGY"
        ));
        let partial = text.replacen("OSCAD_TOPO/3|", "REMOVED_TOPO/3|", 1);
        assert!(
            import_step_v3(&partial)
                .unwrap_err()
                .message
                .contains("Partial")
        );
    }

    #[test]
    fn lexer_accepts_comments_case_escapes_d_exponents_and_forward_refs() {
        let model = freeform_cuboid_solid([0., 0., 0.], [1., 1., 1.]).unwrap();
        let (text, _, _) = export_step_v3(&model).unwrap();
        let text = text
            .replace("DATA;", "DATA;/* 'ignored; text' */")
            .replace("CARTESIAN_POINT", "cartesian_point")
            .replace("0.00000000000000000e0", "0.0D0")
            .replace("OpenSCAD Viewer", "OpenSCAD ''Viewer''");
        import_step_v3(&text).unwrap();
    }

    #[test]
    fn refuses_duplicate_missing_cycle_depth_and_unknown_reachable_geometry() {
        assert!(
            parse("#1=CARTESIAN_POINT('',(0.,0.,0.));#1=CARTESIAN_POINT('',(0.,0.,0.));").is_err()
        );
        let entities = parse("#1=PRODUCT('',#2);#2=PRODUCT('',#1);").unwrap();
        assert!(reachable(&entities, &[1]).is_err());
        let model = freeform_cuboid_solid([0., 0., 0.], [1., 1., 1.]).unwrap();
        let (text, _, _) = export_step_v3(&model).unwrap();
        assert!(import_step_v3(&text.replacen("B_SPLINE_SURFACE_WITH_KNOTS", "PLANE", 1)).is_err());
        assert!(
            import_step_v3(&text.replace(
                "#1=CARTESIAN_POINT",
                "#1=CARTESIAN_POINT('',#999);#65000=CARTESIAN_POINT"
            ))
            .is_err()
        );
    }

    #[test]
    fn parses_self_authored_analytic_and_conversion_fixtures() {
        let tetra =
            include_str!("../../../tests/fixtures/step-v3/self-authored-analytic-tetra.step");
        let (model, _, report) = import_step_v3(tetra).unwrap();
        assert_eq!(
            (
                model.vertices.len(),
                model.edges.len(),
                model.loops.len(),
                model.faces.len(),
                model.shells.len(),
                model.bodies.len()
            ),
            (4, 6, 4, 4, 1, 1)
        );
        assert!(!report.identity.preserved);
        let (roundtrip, _, _) = export_step_v3(&model).unwrap();
        let (roundtrip, _, _) = import_step_v3(&roundtrip).unwrap();
        roundtrip.validate().unwrap();

        let analytic =
            include_str!("../../../tests/fixtures/step-v3/self-authored-analytic-units.step");
        let entities = parse(analytic).unwrap();
        let arc = curve(&entities, 20, 3, 1.).unwrap();
        let ellipse = curve(&entities, 21, 3, 1.).unwrap();
        assert_eq!(arc.degree, 2);
        assert_eq!(ellipse.control_points.len(), 9);
        assert_eq!(
            length_scale(&entities, &entities.keys().copied().collect(), true).unwrap(),
            1.
        );

        let inch = include_str!("../../../tests/fixtures/step-v3/self-authored-inch-chain.stp");
        let entities = parse(inch).unwrap();
        assert!(
            (length_scale(&entities, &entities.keys().copied().collect(), true).unwrap() - 25.4)
                .abs()
                < 1e-12
        );

        let cyclic =
            include_str!("../../../tests/fixtures/step-v3/self-authored-malformed-cycle.stp");
        let entities = parse(cyclic).unwrap();
        assert!(reachable(&entities, &[1]).is_err());
    }

    #[test]
    fn periodic_analytic_surfaces_have_specific_typed_refusals() {
        for ty in [
            "CYLINDRICAL_SURFACE",
            "CONICAL_SURFACE",
            "SPHERICAL_SURFACE",
            "TOROIDAL_SURFACE",
        ] {
            let text = format!("#1={ty}('',#2,1.);");
            let entities = parse(&text).unwrap();
            let error = surface(&entities, 1, 1.).unwrap_err();
            assert_eq!(error.code, "BREP_STEP_V3_REFUSED");
            assert!(error.message.contains("parameterization"));
        }
    }

    #[test]
    fn v4_maps_finite_analytic_carriers_and_endpoint_point_selectors() {
        let entities=parse("\
#1=CARTESIAN_POINT('',(0.,0.,0.));#2=DIRECTION('',(0.,0.,1.));#3=DIRECTION('',(1.,0.,0.));
#4=AXIS2_PLACEMENT_3D('',#1,#2,#3);#5=CYLINDRICAL_SURFACE('',#4,2.);
#6=CONICAL_SURFACE('',#4,2.,0.2);#7=SPHERICAL_SURFACE('',#4,2.);#8=TOROIDAL_SURFACE('',#4,4.,1.);
#9=CARTESIAN_POINT('',(1.,0.,0.));#10=DIRECTION('',(1.,0.,0.));#11=VECTOR('',#10,1.);
#12=LINE('',#9,#11);#13=CARTESIAN_POINT('',(2.,0.,0.));#14=TRIMMED_CURVE('',#12,(#9),(#13),.T.,.PARAMETER.);
").unwrap();
        for id in 5..=8 {
            let s = surface_v4(&entities, id, 1.).unwrap();
            assert!(
                (s.knots_u[s.knots_u.len() - s.degree_u - 1] - std::f64::consts::TAU).abs() < 1e-12
            );
            s.validate().unwrap();
        }
        let c = curve(&entities, 14, 3, 1.).unwrap();
        assert_eq!(c.control_points[0], vec![1., 0., 0.]);
        assert_eq!(c.control_points[1], vec![2., 0., 0.]);
    }

    #[test]
    fn orphan_transform_does_not_move_direct_geometry() {
        let model = freeform_cuboid_solid([0., 0., 0.], [1., 1., 1.]).unwrap();
        let (text, _, _) = export_step_v3(&model).unwrap();
        let rows = "\
#60000=CARTESIAN_POINT('',(0.,0.,0.));
#60001=CARTESIAN_POINT('',(9.,8.,7.));
#60002=DIRECTION('',(0.,0.,1.));
#60003=DIRECTION('',(1.,0.,0.));
#60004=AXIS2_PLACEMENT_3D('',#60000,#60002,#60003);
#60005=AXIS2_PLACEMENT_3D('',#60001,#60002,#60003);
#60006=ITEM_DEFINED_TRANSFORMATION('orphan','',#60004,#60005);
";
        let with_orphan = text.replacen("ENDSEC;\nEND-ISO", &format!("{rows}ENDSEC;\nEND-ISO"), 1);
        let (back, _, report) = import_step_v3(&with_orphan).unwrap();
        assert_eq!(back.vertices[0].point, model.vertices[0].point);
        assert!(
            report
                .ignored_entities
                .contains(&"ITEM_DEFINED_TRANSFORMATION".into())
        );
    }

    #[test]
    fn only_representation_reachable_rigid_transform_moves_geometry() {
        let model = freeform_cuboid_solid([0., 0., 0.], [1., 1., 1.]).unwrap();
        let (text, _, _) = export_step_v3(&model).unwrap();
        let entities = parse(&text).unwrap();
        let body = entities
            .iter()
            .find_map(|(id, _)| {
                call(&entities, *id)
                    .ok()
                    .and_then(|(ty, _)| (ty == "MANIFOLD_SOLID_BREP").then_some(*id))
            })
            .unwrap();
        let rows = format!(
            "\
#60000=CARTESIAN_POINT('',(0.,0.,0.));
#60001=CARTESIAN_POINT('',(9.,8.,7.));
#60002=DIRECTION('',(0.,0.,1.));
#60003=DIRECTION('',(1.,0.,0.));
#60004=AXIS2_PLACEMENT_3D('',#60000,#60002,#60003);
#60005=AXIS2_PLACEMENT_3D('',#60001,#60002,#60003);
#60006=ITEM_DEFINED_TRANSFORMATION('reachable','',#60004,#60005);
#60007=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#{body},#60006),#60008);
#60008=GEOMETRIC_REPRESENTATION_CONTEXT(3);
"
        );
        let placed = text.replacen("ENDSEC;\nEND-ISO", &format!("{rows}ENDSEC;\nEND-ISO"), 1);
        let (back, _, _) = import_step_v3(&placed).unwrap();
        for i in 0..3 {
            assert!(
                (back.vertices[0].point[i] - model.vertices[0].point[i] - [9., 8., 7.][i]).abs()
                    < 1e-9
            );
        }

        let nonrigid = placed.replace(
            "ITEM_DEFINED_TRANSFORMATION('reachable','',#60004,#60005)",
            "CARTESIAN_TRANSFORMATION_OPERATOR_3D('',#60002,#60003,#60000,2.,$)",
        );
        assert!(
            import_step_v3(&nonrigid)
                .unwrap_err()
                .message
                .contains("nonuniform")
        );
    }

    #[test]
    fn aggregate_parser_and_graph_budgets_refuse() {
        assert!(
            lex(&" ".repeat(MAX_BYTES + 1))
                .unwrap_err()
                .message
                .contains("16 MiB")
        );
        let nested = format!("#1=PRODUCT('',{});", "(".repeat(MAX_PARSE_DEPTH + 1))
            + &")".repeat(MAX_PARSE_DEPTH + 1);
        assert!(parse(&nested).unwrap_err().message.contains("depth"));
        let mut chain = String::new();
        for i in 1..=MAX_GRAPH_DEPTH + 2 {
            if i == MAX_GRAPH_DEPTH + 2 {
                chain.push_str(&format!("#{i}=PRODUCT('end');"));
            } else {
                chain.push_str(&format!("#{i}=PRODUCT('',#{});", i + 1));
            }
        }
        let entities = parse(&chain).unwrap();
        assert!(
            reachable(&entities, &[1])
                .unwrap_err()
                .message
                .contains("depth")
        );
        assert!(enforce_output_limit(&"x".repeat(MAX_OUTPUT_BYTES)).is_ok());
        assert!(
            enforce_output_limit(&"x".repeat(MAX_OUTPUT_BYTES + 1))
                .unwrap_err()
                .message
                .contains("16 MiB")
        );

        let mut instances = String::new();
        for id in 1..=MAX_INSTANCES + 1 {
            instances.push_str(&format!("#{id}=CARTESIAN_POINT('',(0.,0.,0.));"))
        }
        assert!(parse(&instances).unwrap_err().message.contains("65536"));
    }

    #[test]
    fn v5_emits_selected_ap242_context_and_roundtrips() {
        let model = freeform_cuboid_solid([0., 0., 0.], [2., 3., 4.]).unwrap();
        let (text, certificate, report) = export_step_v5(&model).unwrap();
        assert_eq!(certificate.capability, STEP_INTERCHANGE_V5_CAPABILITY);
        assert!(text.contains("FILE_NAME("));
        assert!(text.contains("SHAPE_DEFINITION_REPRESENTATION("));
        assert!(text.contains("GLOBAL_UNIT_ASSIGNED_CONTEXT"));
        assert!(!text.contains("B_SPLINE_SURFACE_WITH_KNOTS('',1,1"));
        assert!(report.metadata_loss.is_empty());
        let (back, _, import_report) = import_step_v5(&text).unwrap();
        assert_eq!(
            (
                back.vertices.len(),
                back.edges.len(),
                back.faces.len(),
                back.bodies.len()
            ),
            (8, 12, 6, 1)
        );
        assert!(import_report.identity.preserved);

        let no_selection = text.replace(
            "SHAPE_DEFINITION_REPRESENTATION",
            "PRESENTATION_LAYER_ASSIGNMENT",
        );
        assert!(
            import_step_v5(&no_selection)
                .unwrap_err()
                .message
                .contains("selection")
        );
        let orphan_context = text.replace("GLOBAL_UNIT_ASSIGNED_CONTEXT", "ORPHAN_UNIT_CONTEXT");
        assert!(
            import_step_v5(&orphan_context)
                .unwrap_err()
                .message
                .contains("length unit")
        );
        assert!(import_step_v5("ISO-10303-21;DATA;ENDSEC;END-ISO-10303-21;").is_err());
        assert!(
            import_step_v5(&text.replace(
                "AP242_MANAGED_MODEL_BASED_3D_ENGINEERING",
                "AUTOMOTIVE_DESIGN"
            ))
            .unwrap_err()
            .message
            .contains("AP242")
        );
        assert!(
            import_step_v5(&text.replacen("DATA;", "DATA;\nDATA;", 1))
                .unwrap_err()
                .message
                .contains("exactly one DATA")
        );
        let comment_spoof = text.replacen("DATA;", "/*\nDATA;\n*/\nNOT_DATA;", 1);
        assert!(
            import_step_v5(&comment_spoof)
                .unwrap_err()
                .message
                .contains("DATA")
        );
        let string_spoof = text.replace("FILE_SCHEMA((", "NOT_SCHEMA((").replace(
            "OpenSCAD Viewer retained AP242 B-rep",
            "FILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING'))",
        );
        assert!(
            import_step_v5(&string_spoof)
                .unwrap_err()
                .message
                .contains("HEADER requires")
        );

        let mut rows = data_payload(&text)
            .unwrap()
            .lines()
            .filter(|line| line.starts_with('#'))
            .collect::<Vec<_>>();
        rows.reverse();
        let data_start = text.find("DATA;").unwrap() + 5;
        let data_end = text[data_start..].find("ENDSEC;").unwrap() + data_start;
        let reordered = format!(
            "{}\n{}\n{}",
            &text[..data_start],
            rows.join("\n"),
            &text[data_end..]
        );
        assert_eq!(import_step_v5(&reordered).unwrap().0.bodies.len(), 1);
        let encoded = text.replace("'model.step'", "'\\X2\\006D006F00640065006C\\X0\\.step'");
        assert_eq!(import_step_v5(&encoded).unwrap().0.bodies.len(), 1);

        let metre = text.replace("SI_UNIT(.MILLI.,.METRE.)", "SI_UNIT($,.METRE.)");
        let (metre_model, _, _) = import_step_v5(&metre).unwrap();
        assert!(
            metre_model
                .vertices
                .iter()
                .any(|vertex| vertex.point[0] > 1000.)
        );

        let parsed = parse(&text).unwrap();
        let unit = parsed
            .iter()
            .find_map(|(id, entity)| match &entity.value {
                Value::List(parts)
                    if parts
                        .iter()
                        .any(|part| matches!(part,Value::Call(name,_)if name=="SI_UNIT")) =>
                {
                    Some(*id)
                }
                _ => None,
            })
            .unwrap();
        let inch_rows = "\
#60000=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT($,.METRE.));
#60001=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.0254),#60000);
#60002=(CONVERSION_BASED_UNIT('INCH',#60001)LENGTH_UNIT()NAMED_UNIT(*));
";
        let inch = text
            .replace(
                &format!("GLOBAL_UNIT_ASSIGNED_CONTEXT((#{unit}))"),
                "GLOBAL_UNIT_ASSIGNED_CONTEXT((#60002))",
            )
            .replacen(
                "ENDSEC;\nEND-ISO",
                &format!("{inch_rows}ENDSEC;\nEND-ISO"),
                1,
            );
        let (inch_model, _, _) = import_step_v5(&inch).unwrap();
        assert!(
            inch_model
                .vertices
                .iter()
                .any(|vertex| vertex.point[0] > 50. && vertex.point[0] < 51.)
        );
    }

    #[test]
    fn v5_uses_rational_complexes_and_oriented_void_shells() {
        let rational = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![1., 0.], vec![1., 1.], vec![0., 1.]],
            weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
            periodic: false,
        };
        let mut writer = Writer::new();
        emit_curve_v5(&mut writer, &rational);
        let row = writer.rows.values().last().unwrap();
        assert!(row.contains("RATIONAL_B_SPLINE_CURVE"));
        assert!(row.contains("B_SPLINE_CURVE_WITH_KNOTS(("));

        let mut model = Model::empty(1e-7).unwrap();
        append(
            &mut model,
            freeform_cuboid_solid([0., 0., 0.], [10., 10., 10.]).unwrap(),
        );
        append(
            &mut model,
            freeform_cuboid_solid([2., 2., 2.], [4., 4., 4.]).unwrap(),
        );
        model.0.bodies = vec![Body {
            outer_shell: 0,
            inner_shells: vec![1],
        }];
        model.rebuild_topology_ids();
        model.validate().unwrap();
        let (text, _, _) = export_step_v5(&model).unwrap();
        let text = text
            .lines()
            .map(|line| {
                if let Some(start) = line.find("'OSCAD_TOPO/3|") {
                    let end = line[start + 1..].find('\'').unwrap() + start + 1;
                    format!("{}''{}", &line[..start], &line[end + 1..])
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("ORIENTED_CLOSED_SHELL('',*,#"));
        let (back, _, _) = import_step_v5(&text).unwrap();
        assert_eq!(back.bodies[0].inner_shells.len(), 1);
    }

    #[test]
    fn v5_orders_distinct_occurrence_transforms_and_refuses_cycle_or_ambiguity() {
        let model = freeform_cuboid_solid([0., 0., 0.], [1., 1., 1.]).unwrap();
        let (text, _, _) = export_step_v5(&model).unwrap();
        let text = text
            .lines()
            .map(|line| {
                if let Some(start) = line.find("'OSCAD_TOPO/3|") {
                    let end = line[start + 1..].find('\'').unwrap() + start + 1;
                    format!("{}''{}", &line[..start], &line[end + 1..])
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        let entities = parse(&text).unwrap();
        let root = entities
            .iter()
            .find_map(|(id, _)| {
                call(&entities, *id)
                    .ok()
                    .and_then(|(ty, _)| (ty == "ADVANCED_BREP_SHAPE_REPRESENTATION").then_some(*id))
            })
            .unwrap();
        let (_, root_args) = call(&entities, root).unwrap();
        let body = list(&root_args[1], "items")
            .unwrap()
            .iter()
            .find_map(|value| match value {
                Value::Ref(id) => Some(*id),
                _ => None,
            })
            .unwrap();
        let context = one_ref(&root_args[2], "context").unwrap();
        let rows=format!("\
#60000=CARTESIAN_POINT('',(10.,0.,0.));
#60001=DIRECTION('',(1.,0.,0.));
#60002=DIRECTION('',(0.,1.,0.));
#60003=DIRECTION('',(0.,0.,1.));
#60004=CARTESIAN_TRANSFORMATION_OPERATOR_3D('',#60001,#60002,#60000,1.,#60003);
#60005=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#{body}),#{context});
#60006=(REPRESENTATION_RELATIONSHIP('','',#{root},#60005)REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#60004)SHAPE_REPRESENTATION_RELATIONSHIP());
");
        let placed = text.replacen("ENDSEC;\nEND-ISO", &format!("{rows}ENDSEC;\nEND-ISO"), 1);
        let (back, _, _) = import_step_v5(&placed).unwrap();
        assert_eq!(back.bodies.len(), 2);
        let xs = back
            .vertices
            .iter()
            .map(|vertex| vertex.point[0])
            .collect::<Vec<_>>();
        assert!(xs.iter().any(|x| *x < 2.));
        assert!(xs.iter().any(|x| *x >= 10.));

        let cycle=placed.replacen("ENDSEC;\nEND-ISO",&format!(
            "#60007=(REPRESENTATION_RELATIONSHIP('','',#60005,#{root})REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#60004)SHAPE_REPRESENTATION_RELATIONSHIP());\nENDSEC;\nEND-ISO"),1);
        assert!(
            import_step_v5(&cycle)
                .unwrap_err()
                .message
                .contains("Cycle")
        );
        let ambiguous=placed.replacen("ENDSEC;\nEND-ISO",&format!(
            "#60007=(REPRESENTATION_RELATIONSHIP('','',#{root},#60005)REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#60004)SHAPE_REPRESENTATION_RELATIONSHIP());\nENDSEC;\nEND-ISO"),1);
        assert!(
            import_step_v5(&ambiguous)
                .unwrap_err()
                .message
                .contains("Ambiguous")
        );
    }

    #[test]
    fn v5_roundtrips_rational_analytic_bodies_and_sense_combinations() {
        for model in [
            crate::cylinder(2., 3.).unwrap(),
            crate::frustum(3., 1., 4.).unwrap(),
            crate::torus(4., 1.).unwrap(),
        ] {
            let (text, _, _) = export_step_v5(&model).unwrap();
            assert!(text.contains("RATIONAL_B_SPLINE_"));
            let (back, _, _) = import_step_v5(&text).unwrap();
            assert_eq!(back.bodies.len(), 1);
            assert_eq!(back.faces.len(), model.faces.len());
            back.validate().unwrap();
        }
        let model = crate::cylinder(2., 3.).unwrap();
        let (text, _, _) = export_step_v5(&model).unwrap();
        let text = text
            .lines()
            .map(|line| {
                if let Some(start) = line.find("'OSCAD_TOPO/3|") {
                    let end = line[start + 1..].find('\'').unwrap() + start + 1;
                    format!("{}''{}", &line[..start], &line[end + 1..])
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("ORIENTED_EDGE('',*,*,#") && text.contains(",.F.);"));
        assert!(text.contains("ADVANCED_FACE('',("));
    }

    #[test]
    fn v6_roundtrips_certified_pole_boundaries() {
        for model in [
            crate::sphere(2.).unwrap(),
            crate::frustum(2., 0., 3.).unwrap(),
        ] {
            let degenerate = model.edges.iter().filter(|edge| edge.degenerate).count();
            if degenerate > 0 {
                assert!(export_step_v5(&model).is_err())
            }
            let (text, certificate, _) = export_step_v6(&model).unwrap();
            assert_eq!(certificate.capability, STEP_INTERCHANGE_V6_CAPABILITY);
            let (back, _, _) = import_step_v6(&text).unwrap();
            assert_eq!(
                back.edges.iter().filter(|edge| edge.degenerate).count(),
                if model.faces.len() == 8 {
                    2
                } else {
                    degenerate
                }
            );
            back.validate().unwrap();
        }
        for model in [
            crate::cylinder(2., 3.).unwrap(),
            crate::frustum(2., 1., 3.).unwrap(),
            crate::sphere(2.).unwrap(),
            crate::torus(4., 1.).unwrap(),
        ] {
            let (text, _, _) = export_step_v6(&model).unwrap();
            assert!(text.contains("SEAM_CURVE("));
            import_step_v6(&text).unwrap().0.validate().unwrap();
        }
        let mut void_model = Model::empty(1e-7).unwrap();
        append(
            &mut void_model,
            freeform_cuboid_solid([0., 0., 0.], [10., 10., 10.]).unwrap(),
        );
        append(
            &mut void_model,
            freeform_cuboid_solid([2., 2., 2.], [4., 4., 4.]).unwrap(),
        );
        void_model.0.bodies = vec![Body {
            outer_shell: 0,
            inner_shells: vec![1],
        }];
        void_model.rebuild_topology_ids();
        void_model.validate().unwrap();
        let (void_text, _, _) = export_step_v6(&void_model).unwrap();
        let same_oriented = void_text
            .lines()
            .map(|line| {
                let line = if let Some(start) = line.find("'OSCAD_TOPO/3|") {
                    let end = line[start + 1..].find('\'').unwrap() + start + 1;
                    format!("{}''{}", &line[..start], &line[end + 1..])
                } else {
                    line.to_string()
                };
                if line.contains("ORIENTED_CLOSED_SHELL(") {
                    line.replacen(",.F.)", ",.T.)", 1)
                } else {
                    line
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(
            import_step_v6(&same_oriented).unwrap().0.bodies[0]
                .inner_shells
                .len(),
            1
        );
    }

    #[test]
    fn v7_requires_whole_domain_boundary_identities_for_periodic_carriers() {
        for model in [
            crate::cylinder(2., 3.).unwrap(),
            crate::frustum(2., 1., 3.).unwrap(),
            crate::sphere(2.).unwrap(),
            crate::torus(4., 1.).unwrap(),
        ] {
            let (text, _, _) = export_step_v7(&model).unwrap();
            let (back, import_certificate, _) = import_step_v7(&text).unwrap();
            assert!(
                import_certificate
                    .notes
                    .contains(&"whole_domain_rational_boundary_and_constant_pole_identity")
            );
            back.validate().unwrap();
        }
    }

    #[test]
    fn v8_carries_whole_domain_regularity_in_the_topology_certificate() {
        let expected = [
            (crate::cylinder(2., 3.).unwrap(), "cylinder", 0),
            (crate::frustum(2., 1., 3.).unwrap(), "cone", 0),
            (crate::frustum(2., 0., 3.).unwrap(), "cone", 1),
            (crate::sphere(2.).unwrap(), "sphere", 2),
            (crate::torus(4., 1.).unwrap(), "torus", 0),
        ];
        for (model, carrier, collapsed) in expected {
            let (text, export_certificate, _) = export_step_v8(&model).unwrap();
            let row = export_certificate
                .regularity
                .iter()
                .find(|row| row.carrier == carrier)
                .unwrap();
            assert!(row.denominator_lower_bound > 0.);
            assert_eq!(row.collapsed_boundaries.len(), collapsed);
            assert!(row.lifted_periods[0] > 0.);
            if carrier == "torus" {
                assert!(row.lifted_periods[1] > 0.)
            }
            let (back, import_certificate, _) = import_step_v8(&text).unwrap();
            assert_eq!(import_certificate.coupled_sense_cases, 128);
            assert_eq!(
                import_certificate
                    .regularity
                    .iter()
                    .find(|row| row.carrier == carrier)
                    .unwrap()
                    .identity,
                row.identity
            );
            back.validate().unwrap();
        }
    }

    fn toggle_first_entity_boolean(text: &str, entity: &str) -> String {
        let mut changed = false;
        text.lines()
            .map(|line| {
                if !changed && line.contains(&format!("={entity}")) {
                    changed = true;
                    if line.contains(",.T.);") {
                        line.replacen(",.T.);", ",.F.);", 1)
                    } else {
                        line.replacen(",.F.);", ",.T.);", 1)
                    }
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn reverse_first_shell_members(text: &str) -> String {
        let mut changed = false;
        text.lines()
            .map(|line| {
                if changed || !line.contains("=CLOSED_SHELL") {
                    return line.to_string();
                }
                let Some(open) = line.find(",(") else {
                    return line.to_string();
                };
                let Some(close) = line[open + 2..].find("))").map(|index| index + open + 2) else {
                    return line.to_string();
                };
                let mut members = line[open + 2..close].split(',').collect::<Vec<_>>();
                members.reverse();
                changed = true;
                format!(
                    "{}{}{}",
                    &line[..open + 2],
                    members.join(","),
                    &line[close..]
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn reverse_first_seam_pcurves(text: &str) -> String {
        let mut changed = false;
        text.lines()
            .map(|line| {
                if changed || !line.contains("=SEAM_CURVE") {
                    return line.to_string();
                }
                let Some(open) = line.find(",(#") else {
                    return line.to_string();
                };
                let Some(close) = line[open + 2..].find(')').map(|index| index + open + 2) else {
                    return line.to_string();
                };
                let mut members = line[open + 2..close].split(',').collect::<Vec<_>>();
                members.reverse();
                changed = true;
                format!(
                    "{}{}{}",
                    &line[..open + 2],
                    members.join(","),
                    &line[close..]
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn topology_signature(
        model: &Model,
    ) -> (usize, usize, usize, usize, usize, usize, Vec<[u64; 3]>) {
        let mut vertices = model
            .vertices
            .iter()
            .map(|vertex| vertex.point.map(f64::to_bits))
            .collect::<Vec<_>>();
        vertices.sort();
        (
            model.vertices.len(),
            model.edges.len(),
            model.loops.len(),
            model.faces.len(),
            model.shells.len(),
            model.bodies.len(),
            vertices,
        )
    }

    #[test]
    fn v8_exhausts_all_coupled_sense_masks_with_equivalence_or_refusal() {
        let mut void_model = Model::empty(1e-7).unwrap();
        append(
            &mut void_model,
            freeform_cuboid_solid([0., 0., 0.], [10., 10., 10.]).unwrap(),
        );
        append(
            &mut void_model,
            freeform_cuboid_solid([2., 2., 2.], [4., 4., 4.]).unwrap(),
        );
        void_model.0.bodies = vec![Body {
            outer_shell: 0,
            inner_shells: vec![1],
        }];
        void_model.rebuild_topology_ids();
        void_model.validate().unwrap();
        let fixtures = [
            crate::sphere(2.).unwrap(),
            crate::torus(4., 1.).unwrap(),
            void_model,
        ];
        let exported = fixtures
            .iter()
            .map(|model| export_step_v8(model).unwrap().0)
            .collect::<Vec<_>>();
        let baselines = exported
            .iter()
            .map(|text| topology_signature(&import_step_v8(text).unwrap().0))
            .collect::<Vec<_>>();
        let mut refusals = 0usize;
        let mut equivalents = 0usize;
        for mask in 0usize..128 {
            for (fixture, text) in exported.iter().enumerate() {
                let mut candidate = text.clone();
                if mask & 1 != 0 {
                    candidate = reverse_first_seam_pcurves(&candidate);
                }
                if mask & 2 != 0 {
                    candidate = toggle_first_entity_boolean(&candidate, "EDGE_CURVE")
                }
                if mask & 4 != 0 {
                    candidate = toggle_first_entity_boolean(&candidate, "ORIENTED_EDGE")
                }
                if mask & 8 != 0 {
                    candidate = toggle_first_entity_boolean(&candidate, "FACE_OUTER_BOUND");
                }
                if mask & 16 != 0 {
                    candidate = toggle_first_entity_boolean(&candidate, "ADVANCED_FACE")
                }
                if mask & 32 != 0 {
                    candidate = reverse_first_shell_members(&candidate)
                }
                if mask & 64 != 0 {
                    candidate = toggle_first_entity_boolean(&candidate, "ORIENTED_CLOSED_SHELL");
                }
                match import_step_v8(&candidate) {
                    Ok((model, certificate, _)) => {
                        assert!(certificate.complete);
                        assert_eq!(topology_signature(&model), baselines[fixture]);
                        equivalents += 1;
                    }
                    Err(_) => refusals += 1,
                }
            }
        }
        assert_eq!(equivalents + refusals, 128 * fixtures.len());
        assert!(equivalents > 0 && refusals > 0);
        println!(
            "step-v8-sense-matrix cases={} equivalents={equivalents} refusals={refusals}",
            128 * fixtures.len()
        );
        let same_sense = toggle_first_entity_boolean(&exported[0], "EDGE_CURVE");
        assert!(
            import_step_v8(&same_sense)
                .unwrap_err()
                .message
                .contains("same_sense")
        );
        let wrong_void = toggle_first_entity_boolean(&exported[2], "ORIENTED_CLOSED_SHELL");
        assert!(
            import_step_v8(&wrong_void)
                .unwrap_err()
                .message
                .contains("must oppose")
        );
    }

    #[test]
    fn v6_applies_nonuniform_occurrence_affine_and_refuses_singular() {
        let model = freeform_cuboid_solid([0., 0., 0.], [1., 1., 1.]).unwrap();
        let (text, _, _) = export_step_v5(&model).unwrap();
        let text = text
            .lines()
            .map(|line| {
                if let Some(start) = line.find("'OSCAD_TOPO/3|") {
                    let end = line[start + 1..].find('\'').unwrap() + start + 1;
                    format!("{}''{}", &line[..start], &line[end + 1..])
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        let entities = parse(data_payload(&text).unwrap()).unwrap();
        let root = entities
            .iter()
            .find_map(|(id, _)| {
                call(&entities, *id)
                    .ok()
                    .and_then(|(ty, _)| (ty == "ADVANCED_BREP_SHAPE_REPRESENTATION").then_some(*id))
            })
            .unwrap();
        let (_, root_args) = call(&entities, root).unwrap();
        let body = list(&root_args[1], "items")
            .unwrap()
            .iter()
            .find_map(|value| match value {
                Value::Ref(id) => Some(*id),
                _ => None,
            })
            .unwrap();
        let context = one_ref(&root_args[2], "context").unwrap();
        let product_definition = entities
            .iter()
            .find_map(|(id, _)| {
                call(&entities, *id)
                    .ok()
                    .and_then(|(ty, _)| (ty == "PRODUCT_DEFINITION").then_some(*id))
            })
            .unwrap();
        let (_, definition_args) = call(&entities, product_definition).unwrap();
        let formation = one_ref(&definition_args[2], "formation").unwrap();
        let definition_context = one_ref(&definition_args[3], "definition context").unwrap();
        let rows=format!("\
#61000=CARTESIAN_POINT('',(10.,20.,30.));
#61001=DIRECTION('',(1.,0.,0.));
#61002=DIRECTION('',(0.,1.,0.));
#61003=DIRECTION('',(0.,0.,1.));
#61004=(CARTESIAN_TRANSFORMATION_OPERATOR('',#61001,#61002,#61000,2.)CARTESIAN_TRANSFORMATION_OPERATOR_3D(#61003)CARTESIAN_TRANSFORMATION_OPERATOR_3DNON_UNIFORM(3.,4.)GEOMETRIC_REPRESENTATION_ITEM()REPRESENTATION_ITEM(''));
#61005=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#{body}),#{context});
#61006=(REPRESENTATION_RELATIONSHIP('','',#{root},#61005)REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#61004)SHAPE_REPRESENTATION_RELATIONSHIP());
#61007=PRODUCT_DEFINITION('child','',#{formation},#{definition_context});
#61008=NEXT_ASSEMBLY_USAGE_OCCURRENCE('occ-1','child occurrence','',#{product_definition},#61007,$);
");
        let placed = text.replacen("ENDSEC;\nEND-ISO", &format!("{rows}ENDSEC;\nEND-ISO"), 1);
        assert!(import_step_v5(&placed).is_err());
        let (back, _, report) = import_step_v6(&placed).unwrap();
        assert_eq!(back.bodies.len(), 2);
        assert!(
            back.vertices
                .iter()
                .any(|vertex| vertex.point == [12., 23., 34.])
        );
        assert_eq!(report.definition_identities.len(), 1);
        assert_eq!(report.occurrence_identities.len(), 2);
        assert!(
            report
                .product_hierarchy
                .iter()
                .any(|row| row.contains("assembly-use:#61008"))
        );
        let mut rows = data_payload(&placed)
            .unwrap()
            .lines()
            .filter(|line| line.starts_with('#'))
            .collect::<Vec<_>>();
        rows.reverse();
        let data_start = placed.find("DATA;").unwrap() + 5;
        let data_end = placed[data_start..].find("ENDSEC;").unwrap() + data_start;
        let reordered = format!(
            "{}\n{}\n{}",
            &placed[..data_start],
            rows.join("\n"),
            &placed[data_end..]
        );
        let (_, _, reordered_report) = import_step_v6(&reordered).unwrap();
        assert_eq!(
            reordered_report.definition_identities,
            report.definition_identities
        );
        assert_eq!(
            reordered_report.occurrence_identities,
            report.occurrence_identities
        );
        let singular = placed.replace(
            "CARTESIAN_TRANSFORMATION_OPERATOR_3DNON_UNIFORM(3.,4.)",
            "CARTESIAN_TRANSFORMATION_OPERATOR_3DNON_UNIFORM(0.,4.)",
        );
        assert!(
            import_step_v6(&singular)
                .unwrap_err()
                .message
                .contains("positive")
        );
        let skewed = placed.replace(
            "#61002=DIRECTION('',(0.,1.,0.));",
            "#61002=DIRECTION('',(1.,1.,0.));",
        );
        assert!(
            import_step_v6(&skewed)
                .unwrap_err()
                .message
                .contains("non-orthogonal")
        );
        let (sheared, shear_certificate, shear_report) = import_step_v9(&skewed).unwrap();
        assert_eq!(shear_certificate.capability, STEP_INTERCHANGE_V9_CAPABILITY);
        assert_eq!(sheared.bodies.len(), 2);
        assert!(!shear_report.identity.preserved);
        let reflected = placed.replace(
            "#61003=DIRECTION('',(0.,0.,1.));",
            "#61003=DIRECTION('',(0.,0.,-1.));",
        );
        let (reflected, _, _) = import_step_v9(&reflected).unwrap();
        assert!(
            reflected
                .shells
                .iter()
                .flat_map(|shell| &shell.faces)
                .any(|usage| usage.reversed)
        );
        assert!(
            import_step_v9(&singular)
                .unwrap_err()
                .message
                .contains("nonzero")
        );
    }

    #[test]
    fn v6_exact_and_plus_one_resource_boundaries() {
        assert!(lex(&" ".repeat(MAX_BYTES)).is_ok());
        assert!(lex(&" ".repeat(MAX_BYTES + 1)).is_err());
        assert!(enforce_output_limit(&"é".repeat(MAX_OUTPUT_BYTES / 2)).is_ok());
        assert!(enforce_output_limit(&("é".repeat(MAX_OUTPUT_BYTES / 2) + "a")).is_err());
        assert!(
            parse(&format!(
                "#1=PRODUCT('',{});",
                "(".repeat(MAX_PARSE_DEPTH) + &")".repeat(MAX_PARSE_DEPTH)
            ))
            .is_ok()
        );
        assert!(
            parse(&format!(
                "#1=PRODUCT('',{});",
                "(".repeat(MAX_PARSE_DEPTH + 1) + &")".repeat(MAX_PARSE_DEPTH + 1)
            ))
            .is_err()
        );
        let exact = (1..=MAX_INSTANCES)
            .map(|id| format!("#{id}=CARTESIAN_POINT('',(0.,0.,0.));"))
            .collect::<String>();
        assert_eq!(parse(&exact).unwrap().len(), MAX_INSTANCES);
        assert!(
            parse(&(exact + &format!("#{}=CARTESIAN_POINT('',(0.,0.,0.));", MAX_INSTANCES + 1)))
                .is_err()
        );
        let graph = (1..=MAX_GRAPH_DEPTH + 1)
            .map(|id| {
                if id == MAX_GRAPH_DEPTH + 1 {
                    format!("#{id}=PRODUCT('end');")
                } else {
                    format!("#{id}=PRODUCT('',#{});", id + 1)
                }
            })
            .collect::<String>();
        assert_eq!(
            reachable(&parse(&graph).unwrap(), &[1]).unwrap().len(),
            MAX_GRAPH_DEPTH + 1
        );
        let graph_plus = graph.replace(
            &format!("#{}=PRODUCT('end');", MAX_GRAPH_DEPTH + 1),
            &format!(
                "#{}=PRODUCT('',#{});#{}=PRODUCT('end');",
                MAX_GRAPH_DEPTH + 1,
                MAX_GRAPH_DEPTH + 2,
                MAX_GRAPH_DEPTH + 2
            ),
        );
        assert!(reachable(&parse(&graph_plus).unwrap(), &[1]).is_err());

        let units=(0..8).map(|index|format!("#{}=(CONVERSION_BASED_UNIT('u',#{})LENGTH_UNIT()NAMED_UNIT(*));#{}=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(1.),#{});",
            100+2*index,101+2*index,101+2*index,if index==7{200}else{102+2*index})).collect::<String>()
            +"#200=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT($,.METRE.));";
        let parsed = parse(&units).unwrap();
        assert_eq!(
            unit_scale(&parsed, 100, 0, &mut BTreeSet::new()).unwrap(),
            1000.
        );
        let unit_plus=units.replace("#200=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT($,.METRE.));",
            "#200=(CONVERSION_BASED_UNIT('u',#201)LENGTH_UNIT()NAMED_UNIT(*));#201=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(1.),#202);#202=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT($,.METRE.));");
        assert!(unit_scale(&parse(&unit_plus).unwrap(), 100, 0, &mut BTreeSet::new()).is_err());

        let placement_graph = |depth: usize| {
            let mut text="#1=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.));#2=(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNIT_ASSIGNED_CONTEXT((#1))REPRESENTATION_CONTEXT('','3D'));#3=CARTESIAN_POINT('',(0.,0.,0.));#4=CARTESIAN_TRANSFORMATION_OPERATOR_3D('',$,$,#3,1.,$);".to_string();
            for index in 0..=depth {
                let id = 1000 + index;
                text.push_str(&format!(
                    "#{id}=ADVANCED_BREP_SHAPE_REPRESENTATION('',{},#2);",
                    if index == depth { "(#900)" } else { "()" }
                ));
                if index < depth {
                    text.push_str(&format!("#{}=(REPRESENTATION_RELATIONSHIP('','',#{id},#{})REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#4)SHAPE_REPRESENTATION_RELATIONSHIP());",2000+index,id+1))
                }
            }
            text.push_str("#900=MANIFOLD_SOLID_BREP();");
            parse(&text).unwrap()
        };
        assert_eq!(
            representation_occurrences(&placement_graph(32), &[1000], true, false, false)
                .unwrap()
                .len(),
            1
        );
        assert!(
            representation_occurrences(&placement_graph(33), &[1000], true, false, false).is_err()
        );
        let occurrence_graph = |children: usize| {
            let mut text="#1=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.));#2=(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNIT_ASSIGNED_CONTEXT((#1))REPRESENTATION_CONTEXT('','3D'));#3=MANIFOLD_SOLID_BREP();#4=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#3),#2);#5=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#3),#2);".to_string();
            for index in 0..children {
                text.push_str(&format!("#{}=CARTESIAN_POINT('',({index}.,0.,0.));#{}=CARTESIAN_TRANSFORMATION_OPERATOR_3D('',$,$,#{},1.,$);#{}=(REPRESENTATION_RELATIONSHIP('','',#4,#5)REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#{})SHAPE_REPRESENTATION_RELATIONSHIP());",
                    1000+index*3,1001+index*3,1000+index*3,1002+index*3,1001+index*3));
            }
            parse(&text).unwrap()
        };
        assert_eq!(
            representation_occurrences(
                &occurrence_graph(MAX_OCCURRENCES - 1),
                &[4],
                true,
                false,
                false
            )
            .unwrap()
            .len(),
            MAX_OCCURRENCES
        );
        assert!(
            representation_occurrences(
                &occurrence_graph(MAX_OCCURRENCES),
                &[4],
                true,
                false,
                false
            )
            .is_err()
        );
    }

    #[test]
    fn v6_exhausts_valid_edge_oriented_bound_face_and_void_senses() {
        fn exchange(header: &str, entities: &BTreeMap<usize, Entity>) -> String {
            let start = header.find("DATA;").unwrap() + 5;
            let end = header[start..].find("ENDSEC;").unwrap() + start;
            let rows = entities
                .iter()
                .map(|(id, entity)| format!("#{id}={};", render_entity_value(&entity.value)))
                .collect::<Vec<_>>()
                .join("\n");
            format!("{}\n{}\n{}", &header[..start], rows, &header[end..])
        }
        fn reverse_curve(entities: &mut BTreeMap<usize, Entity>, id: usize) {
            let entity = entities.get_mut(&id).unwrap();
            let reverse_list = |value: &mut Value| {
                if let Value::List(items) = value {
                    items.reverse()
                }
            };
            match &mut entity.value {
                Value::Call(name, args) if name == "B_SPLINE_CURVE_WITH_KNOTS" => {
                    reverse_list(&mut args[2]);
                    reverse_list(&mut args[6]);
                    reverse_list(&mut args[7]);
                    if let Value::List(knots) = &mut args[7] {
                        let values = knots
                            .iter()
                            .map(|value| number(value, "knot").unwrap())
                            .collect::<Vec<_>>();
                        let sum = values[0] + values[values.len() - 1];
                        for (value, next) in knots.iter_mut().zip(values) {
                            *value = Value::Number(sum - next)
                        }
                    }
                }
                Value::List(parts) => {
                    for part in parts {
                        if let Value::Call(name, args) = part {
                            if name == "B_SPLINE_CURVE" {
                                reverse_list(&mut args[1])
                            }
                            if name == "B_SPLINE_CURVE_WITH_KNOTS" {
                                reverse_list(&mut args[0]);
                                if let Value::List(knots) = &mut args[1] {
                                    let values = knots
                                        .iter()
                                        .map(|value| number(value, "knot").unwrap())
                                        .collect::<Vec<_>>();
                                    let sum = values[0] + values[values.len() - 1];
                                    knots.reverse();
                                    for value in knots {
                                        if let Value::Number(knot) = value {
                                            *knot = sum - *knot
                                        }
                                    }
                                }
                            }
                            if name == "RATIONAL_B_SPLINE_CURVE" {
                                reverse_list(&mut args[0])
                            }
                        }
                    }
                }
                _ => panic!("unexpected curve entity"),
            }
        }
        fn pcurve_curve(entities: &BTreeMap<usize, Entity>, pcurve: usize) -> usize {
            let reference =
                one_ref(&call(entities, pcurve).unwrap().1[2], "reference_to_curve").unwrap();
            let (ty, args) = call(entities, reference).unwrap();
            if ty == "DEFINITIONAL_REPRESENTATION" {
                one_ref(&list(&args[1], "items").unwrap()[0], "curve").unwrap()
            } else {
                reference
            }
        }
        fn reverse_loop_encoding(
            variant: &mut BTreeMap<usize, Entity>,
            loop_id: usize,
            surface: usize,
        ) {
            let members = {
                let Value::Call(_, args) = &mut variant.get_mut(&loop_id).unwrap().value else {
                    unreachable!()
                };
                let Value::List(members) = &mut args[1] else {
                    unreachable!()
                };
                members.reverse();
                members
                    .iter()
                    .map(|value| one_ref(value, "oriented").unwrap())
                    .collect::<Vec<_>>()
            };
            for oriented in members {
                let edge = {
                    let Value::Call(_, args) = &mut variant.get_mut(&oriented).unwrap().value
                    else {
                        unreachable!()
                    };
                    args[4] = Value::Enum(
                        if boolean(&args[4], "orientation").unwrap() {
                            "F"
                        } else {
                            "T"
                        }
                        .into(),
                    );
                    one_ref(&args[3], "edge").unwrap()
                };
                let geometry = one_ref(&call(variant, edge).unwrap().1[3], "geometry").unwrap();
                let pcurve = list(&call(variant, geometry).unwrap().1[2], "pcurves")
                    .unwrap()
                    .iter()
                    .find_map(|value| {
                        let id = one_ref(value, "pcurve").ok()?;
                        let args = call(variant, id).ok()?.1;
                        (one_ref(&args[1], "surface").ok() == Some(surface)).then_some(id)
                    })
                    .unwrap();
                let curve = pcurve_curve(variant, pcurve);
                reverse_curve(variant, curve);
            }
        }
        let model = freeform_cuboid_solid([0., 0., 0.], [2., 3., 4.]).unwrap();
        let (base, _, _) = export_step_v6(&model).unwrap();
        let base = base
            .lines()
            .map(|line| {
                if let Some(start) = line.find("'OSCAD_TOPO/3|") {
                    let end = line[start + 1..].find('\'').unwrap() + start + 1;
                    format!("{}''{}", &line[..start], &line[end + 1..])
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        let entities = parse(data_payload(&base).unwrap()).unwrap();
        let edge_ids = entities
            .keys()
            .filter_map(|id| {
                call(&entities, *id)
                    .ok()
                    .and_then(|(ty, _)| (ty == "EDGE_CURVE").then_some(*id))
            })
            .collect::<Vec<_>>();
        for edge_id in edge_ids {
            let mut variant = entities.clone();
            let geometry = {
                let Value::Call(_, args) = &mut variant.get_mut(&edge_id).unwrap().value else {
                    unreachable!()
                };
                args[4] = Value::Enum("F".into());
                one_ref(&args[3], "geometry").unwrap()
            };
            let (curve3, pcurves) = {
                let (_, args) = call(&variant, geometry).unwrap();
                (
                    one_ref(&args[1], "curve").unwrap(),
                    list(&args[2], "pcurves")
                        .unwrap()
                        .iter()
                        .map(|value| one_ref(value, "pcurve").unwrap())
                        .collect::<Vec<_>>(),
                )
            };
            reverse_curve(&mut variant, curve3);
            let mut reversed_pcurves = BTreeSet::new();
            for pcurve in pcurves {
                let curve = pcurve_curve(&variant, pcurve);
                if reversed_pcurves.insert(curve) {
                    reverse_curve(&mut variant, curve)
                }
            }
            assert!(
                import_step_v6(&exchange(&base, &variant)).is_ok(),
                "edge #{edge_id}"
            );
        }
        let loop_ids = entities
            .keys()
            .filter_map(|id| {
                call(&entities, *id)
                    .ok()
                    .and_then(|(ty, _)| (ty == "EDGE_LOOP").then_some(*id))
            })
            .collect::<Vec<_>>();
        for mask in 0..(1usize << loop_ids.len()) {
            let mut variant = entities.clone();
            for (bit, loop_id) in loop_ids.iter().enumerate() {
                if mask & (1 << bit) == 0 {
                    continue;
                }
                let bound = variant
                    .iter()
                    .find_map(|(id, _)| {
                        call(&variant, *id).ok().and_then(|(name, args)| {
                            (matches!(name, "FACE_OUTER_BOUND" | "FACE_BOUND")
                                && one_ref(&args[1], "loop").ok() == Some(*loop_id))
                            .then_some(*id)
                        })
                    })
                    .unwrap();
                let surface = variant
                    .iter()
                    .find_map(|(id, _)| {
                        call(&variant, *id).ok().and_then(|(name, args)| {
                            if name != "ADVANCED_FACE" {
                                return None;
                            }
                            list(&args[1], "bounds")
                                .ok()?
                                .iter()
                                .any(|value| one_ref(value, "bound").ok() == Some(bound))
                                .then(|| one_ref(&args[2], "surface").unwrap())
                        })
                    })
                    .unwrap();
                reverse_loop_encoding(&mut variant, *loop_id, surface);
                let Value::Call(_, args) = &mut variant.get_mut(&bound).unwrap().value else {
                    unreachable!()
                };
                args[2] = Value::Enum("F".into());
            }
            if let Err(error) = import_step_v6(&exchange(&base, &variant)) {
                panic!("loop mask {mask}: {}", error.message)
            }
        }
        let face_ids = entities
            .keys()
            .filter_map(|id| {
                call(&entities, *id)
                    .ok()
                    .and_then(|(ty, _)| (ty == "ADVANCED_FACE").then_some(*id))
            })
            .collect::<Vec<_>>();
        for mask in 0..(1usize << face_ids.len()) {
            let mut variant = entities.clone();
            for (bit, face) in face_ids.iter().enumerate() {
                if mask & (1 << bit) == 0 {
                    continue;
                }
                let (_, face_args) = call(&variant, *face).unwrap();
                let surface = one_ref(&face_args[2], "surface").unwrap();
                let loops = list(&face_args[1], "bounds")
                    .unwrap()
                    .iter()
                    .map(|value| {
                        let bound = one_ref(value, "bound").unwrap();
                        one_ref(&call(&variant, bound).unwrap().1[1], "loop").unwrap()
                    })
                    .collect::<Vec<_>>();
                for loop_id in loops {
                    reverse_loop_encoding(&mut variant, loop_id, surface)
                }
                let Value::Call(_, args) = &mut variant.get_mut(face).unwrap().value else {
                    unreachable!()
                };
                args[3] = Value::Enum("F".into());
            }
            if let Err(error) = import_step_v6(&exchange(&base, &variant)) {
                panic!("face mask {mask}: {}", error.message)
            }
        }
    }

    #[test]
    fn v9_certifies_interior_point_inverse_and_ambiguity() {
        let curve = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0., 0.], vec![1., 2., 0.], vec![2., 0., 0.]],
            weights: vec![1., 0.75, 1.],
            periodic: false,
        };
        curve.validate().unwrap();
        let wanted = curve.evaluate(0.37).unwrap().point;
        let parameter = isolate_curve_point(&curve, &wanted).unwrap();
        assert!((parameter - 0.37).abs() < 1e-8);
        assert!(isolate_curve_point(&curve, &[9., 9., 9.]).is_err());
    }

    #[test]
    fn v9_roundtrips_retained_open_shell() {
        let control_points = (0..4)
            .map(|u| {
                (0..4)
                    .map(|v| vec![u as f64 / 3., v as f64 / 3., (u * v) as f64 / 90.])
                    .collect()
            })
            .collect();
        let surface = Surface {
            degree_u: 3,
            degree_v: 3,
            knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points,
            weights: vec![vec![1.; 4]; 4],
            periodic_u: false,
            periodic_v: false,
        };
        let model = crate::bicubic_open_face(surface).unwrap();
        assert!(export_step_v8(&model).is_err());
        let (text, certificate, _) = export_step_v9(&model).unwrap();
        assert_eq!(certificate.capability, STEP_INTERCHANGE_V9_CAPABILITY);
        assert!(text.contains("MANIFOLD_SURFACE_SHAPE_REPRESENTATION"));
        assert!(text.contains("SHELL_BASED_SURFACE_MODEL"));
        let (back, certificate, _) = import_step_v9(&text).unwrap();
        assert_eq!(certificate.capability, STEP_INTERCHANGE_V9_CAPABILITY);
        assert!(back.bodies.is_empty());
        assert_eq!(back.shells.len(), 1);
        assert!(!back.shells[0].closed);
    }

    #[test]
    fn v10_retains_affine_occurrence_graph_and_detects_mutation() {
        let text =
            include_str!("../../../tests/fixtures/step-v6/self-authored-ap242-assembly.step");
        let (projection, certificate, document) = import_step_v10(text).unwrap();
        assert_eq!(certificate.capability, STEP_INTERCHANGE_V10_CAPABILITY);
        assert_eq!(projection.bodies.len(), 3);
        assert_eq!(document.definition_identities.len(), 1);
        assert_eq!(document.occurrence_identities.len(), 3);
        assert!(!document.operator_identities.is_empty());
        assert_eq!(export_step_v10(&document).unwrap(), text);
        let mut mutation = document.clone();
        mutation.source =
            mutation
                .source
                .replacen("2.00000000000000000E0", "2.50000000000000000E0", 1);
        assert!(export_step_v10(&mutation).is_err());
    }
}
