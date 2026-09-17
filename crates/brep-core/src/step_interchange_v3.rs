//! Bounded, direct Part 21 topology interchange.
//!
//! Unlike the compatibility `/1` and `/2` routes this module never recognizes
//! constructors or reconstructs an AABB.  It translates the selected
//! MANIFOLD_SOLID_BREP/BREP_WITH_VOIDS graph directly to/from `Model`.

use crate::{Body, Coedge, Edge, Face, FaceUse, Loop, Model, Shell, TopoId, TopoKind, TopologyIds};
use crate::analytic_features::FeatureCertificate;
use brep_topology::{CoedgeTrim, Vertex};
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};
use std::collections::{BTreeMap, BTreeSet};

pub const STEP_INTERCHANGE_V3_CAPABILITY: &str = "step-interchange/3";
pub const STEP_INTERCHANGE_V4_CAPABILITY: &str = "step-interchange/4";
const MAX_BYTES: usize = 16 * 1024 * 1024;
const MAX_INSTANCES: usize = 65_536;
const MAX_PARSE_DEPTH: usize = 32;
const MAX_GRAPH_DEPTH: usize = 64;
const MAX_OUTPUT_BYTES: usize = 16 * 1024 * 1024;

fn refuse(message: impl Into<String>) -> Error {
    Error::new("BREP_STEP_V3_REFUSED", message)
}

#[derive(Clone, Debug)]
pub struct StepV3Report {
    pub identity: crate::step_interchange::StepIdentityReport,
    pub ignored_entities: Vec<String>,
    pub instance_count: usize,
    pub reachable_count: usize,
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
                while i < bytes.len() && bytes[i].is_ascii_digit() { i += 1; }
                if start == i { return Err(refuse("Malformed Part 21 reference")); }
                out.push(Tok::Hash(text[start..i].parse().map_err(|_| refuse("Invalid instance number"))?));
            }
            b'\'' => {
                i += 1;
                let mut value = String::new();
                loop {
                    if i >= bytes.len() { return Err(refuse("Unterminated Part 21 string")); }
                    if bytes[i] == b'\'' {
                        if i + 1 < bytes.len() && bytes[i + 1] == b'\'' {
                            value.push('\''); i += 2; continue;
                        }
                        i += 1; break;
                    }
                    let ch = text[i..].chars().next().ok_or_else(|| refuse("Invalid UTF-8 string"))?;
                    value.push(ch);
                    i += ch.len_utf8();
                }
                out.push(Tok::String(value));
            }
            b'.' if i + 1 < bytes.len() && bytes[i + 1].is_ascii_alphabetic() => {
                i += 1;
                let start = i;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') { i += 1; }
                if i >= bytes.len() || bytes[i] != b'.' {
                    return Err(refuse(format!("Malformed STEP enumeration .{} at offset {start}", &text[start..i])));
                }
                out.push(Tok::Enum(text[start..i].to_ascii_uppercase()));
                i += 1;
            }
            b'(' => { out.push(Tok::LParen); i += 1; }
            b')' => { out.push(Tok::RParen); i += 1; }
            b',' => { out.push(Tok::Comma); i += 1; }
            b'=' => { out.push(Tok::Eq); i += 1; }
            b';' => { out.push(Tok::Semi); i += 1; }
            b'$' => { out.push(Tok::Dollar); i += 1; }
            b'*' => { out.push(Tok::Star); i += 1; }
            b'+' | b'-' | b'0'..=b'9' | b'.' => {
                let start = i;
                i += 1;
                while i < bytes.len() && matches!(bytes[i], b'0'..=b'9' | b'.' | b'+' | b'-' | b'E' | b'e' | b'D' | b'd') { i += 1; }
                out.push(Tok::Number(text[start..i].to_string()));
            }
            c if c.is_ascii_alphabetic() || c == b'_' => {
                let start = i;
                i += 1;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') { i += 1; }
                out.push(Tok::Ident(text[start..i].to_ascii_uppercase()));
            }
            _ => return Err(refuse(format!("Unexpected Part 21 byte at offset {i}"))),
        }
    }
    Ok(out)
}

struct Parser { tokens: Vec<Tok>, at: usize }
impl Parser {
    fn value(&mut self, depth: usize) -> Result<Value> {
        if depth > MAX_PARSE_DEPTH { return Err(refuse("Part 21 aggregate depth exceeds 32")); }
        let token = self.tokens.get(self.at).cloned().ok_or_else(|| refuse("Unexpected end of Part 21 value"))?;
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
                    Value::Integer(normalized.parse().map_err(|_| refuse("Invalid STEP integer"))?)
                } else {
                    let n: f64 = normalized.parse().map_err(|_| refuse("Invalid STEP real"))?;
                    if !n.is_finite() { return Err(refuse("Non-finite STEP real")); }
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
        if self.tokens.get(self.at) == Some(&Tok::RParen) { self.at += 1; return Ok(values); }
        loop {
            values.push(self.value(depth)?);
            match self.tokens.get(self.at) {
                Some(Tok::Comma) => self.at += 1,
                Some(Tok::RParen) => { self.at += 1; break; }
                // Part 21 complex entity components are adjacent calls rather
                // than comma-separated aggregate members.
                Some(Tok::Ident(_)) => {}
                other => return Err(refuse(format!("Malformed Part 21 argument list near token {}: {other:?}", self.at))),
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
        let Some(Tok::Hash(id)) = parser.tokens.get(parser.at).cloned() else { parser.at += 1; continue; };
        if parser.tokens.get(parser.at + 1) != Some(&Tok::Eq) { parser.at += 1; continue; }
        parser.at += 2;
        let value = parser.value(0)?;
        if parser.tokens.get(parser.at) != Some(&Tok::Semi) { return Err(refuse("STEP instance is missing semicolon")); }
        parser.at += 1;
        if entities.insert(id, Entity { value }).is_some() { return Err(refuse("Duplicate Part 21 instance number")); }
        if entities.len() > MAX_INSTANCES { return Err(refuse("STEP /3 instance count exceeds 65536")); }
    }
    if entities.is_empty() { return Err(refuse("STEP /3 contains no instances")); }
    Ok(entities)
}

fn call<'a>(entities: &'a BTreeMap<usize, Entity>, id: usize) -> Result<(&'a str, &'a [Value])> {
    let entity = entities.get(&id).ok_or_else(|| refuse(format!("Missing reference #{id}")))?;
    match &entity.value {
        Value::Call(name, args) => Ok((name, args)),
        Value::List(parts) => {
            // Complex instances are parsed and bounded, but geometry complexes
            // are resolved only when an explicitly supported component exists.
            for part in parts {
                if let Value::Call(name, args) = part {
                    if matches!(name.as_str(), "B_SPLINE_CURVE_WITH_KNOTS" | "B_SPLINE_SURFACE_WITH_KNOTS" | "SI_UNIT" | "CONVERSION_BASED_UNIT") {
                        return Ok((name, args));
                    }
                }
            }
            Err(refuse(format!("Reachable complex instance #{id} has no admitted component")))
        }
        _ => Err(refuse(format!("Reference #{id} is not an entity call"))),
    }
}
fn refs(value: &Value, out: &mut Vec<usize>) {
    match value {
        Value::Ref(id) => out.push(*id),
        Value::List(v) | Value::Call(_, v) => for x in v { refs(x, out); },
        _ => {}
    }
}
fn one_ref(v: &Value, what: &str) -> Result<usize> {
    if let Value::Ref(id) = v { Ok(*id) } else { Err(refuse(format!("{what} must be a reference"))) }
}
fn list<'a>(v: &'a Value, what: &str) -> Result<&'a [Value]> {
    if let Value::List(items) = v { Ok(items) } else { Err(refuse(format!("{what} must be an aggregate"))) }
}
fn number(v: &Value, what: &str) -> Result<f64> {
    match v {
        Value::Number(n) => Ok(*n),
        Value::Integer(n) => Ok(*n as f64),
        _ => Err(refuse(format!("{what} must be numeric"))),
    }
}
fn usize_value(v: &Value, what: &str) -> Result<usize> {
    match v {
        Value::Integer(n) if *n >= 0 => Ok(*n as usize),
        _ => Err(refuse(format!("{what} must be a nonnegative integer"))),
    }
}
fn boolean(v: &Value, what: &str) -> Result<bool> {
    match v { Value::Enum(v) if v == "T" => Ok(true), Value::Enum(v) if v == "F" => Ok(false), _ => Err(refuse(format!("{what} must be .T. or .F."))) }
}
fn name(args: &[Value]) -> Option<&str> {
    match args.first() { Some(Value::String(v)) => Some(v), _ => None }
}

fn reachable(entities: &BTreeMap<usize, Entity>, roots: &[usize]) -> Result<BTreeSet<usize>> {
    fn visit(id: usize, depth: usize, entities: &BTreeMap<usize, Entity>, active: &mut BTreeSet<usize>, done: &mut BTreeSet<usize>) -> Result<()> {
        if depth > MAX_GRAPH_DEPTH { return Err(refuse("STEP reachable graph depth exceeds 64")); }
        if done.contains(&id) { return Ok(()); }
        if !active.insert(id) { return Err(refuse("Cycle in reachable STEP graph")); }
        let entity = entities.get(&id).ok_or_else(|| refuse(format!("Missing reachable reference #{id}")))?;
        let mut dependencies = Vec::new();
        refs(&entity.value, &mut dependencies);
        for dependency in dependencies { visit(dependency, depth + 1, entities, active, done)?; }
        active.remove(&id); done.insert(id); Ok(())
    }
    let mut done = BTreeSet::new();
    for root in roots { visit(*root, 0, entities, &mut BTreeSet::new(), &mut done)?; }
    Ok(done)
}

fn values(v: &Value, what: &str) -> Result<Vec<f64>> {
    list(v, what)?.iter().map(|v| number(v, what)).collect()
}
fn ints(v: &Value, what: &str) -> Result<Vec<usize>> {
    list(v, what)?.iter().map(|v| usize_value(v, what)).collect()
}
fn expand(mults: &[usize], knots: &[f64]) -> Result<Vec<f64>> {
    if mults.len() != knots.len() { return Err(refuse("Knot multiplicity count mismatch")); }
    let mut out = Vec::new();
    for (&m, &k) in mults.iter().zip(knots) { out.extend(std::iter::repeat_n(k, m)); }
    Ok(out)
}
fn point(entities: &BTreeMap<usize, Entity>, id: usize, dim: usize, scale: f64) -> Result<Vec<f64>> {
    let (ty, args) = call(entities, id)?;
    if ty != "CARTESIAN_POINT" || args.len() != 2 { return Err(refuse("Expected CARTESIAN_POINT")); }
    let mut p = values(&args[1], "CARTESIAN_POINT coordinates")?;
    if p.len() != dim { return Err(refuse("CARTESIAN_POINT dimension mismatch")); }
    if dim == 3 { for x in &mut p { *x *= scale; } }
    Ok(p)
}
fn direction(entities: &BTreeMap<usize, Entity>, id: usize, dim: usize) -> Result<Vec<f64>> {
    let (ty, args) = call(entities, id)?;
    if ty != "DIRECTION" || args.len() != 2 { return Err(refuse("Expected DIRECTION")); }
    let d = values(&args[1], "DIRECTION ratios")?;
    if d.len() != dim { return Err(refuse("DIRECTION dimension mismatch")); }
    let n = d.iter().map(|x| x * x).sum::<f64>().sqrt();
    if !n.is_finite() || n <= 1e-12 { return Err(refuse("Degenerate DIRECTION")); }
    Ok(d.into_iter().map(|x| x / n).collect())
}
fn vector(entities: &BTreeMap<usize, Entity>, id: usize, dim: usize, scale: f64) -> Result<Vec<f64>> {
    let (ty, args) = call(entities, id)?;
    if ty != "VECTOR" || args.len() != 3 { return Err(refuse("Expected VECTOR")); }
    let d = direction(entities, one_ref(&args[1], "VECTOR orientation")?, dim)?;
    let magnitude = number(&args[2], "VECTOR magnitude")? * if dim == 3 { scale } else { 1. };
    if magnitude <= 0. { return Err(refuse("VECTOR magnitude must be positive")); }
    Ok(d.into_iter().map(|x| x * magnitude).collect())
}
fn axis2(entities: &BTreeMap<usize, Entity>, id: usize, dim: usize, scale: f64) -> Result<(Vec<f64>, Vec<f64>, Vec<f64>)> {
    let (ty, args) = call(entities, id)?;
    let expected = if dim == 3 { "AXIS2_PLACEMENT_3D" } else { "AXIS2_PLACEMENT_2D" };
    if ty != expected || args.len() < 2 { return Err(refuse(format!("Expected {expected}"))); }
    let origin = point(entities, one_ref(&args[1], "AXIS2 location")?, dim, scale)?;
    if dim == 2 {
        let x = if args.len() > 2 && !matches!(args[2], Value::Omitted) {
            direction(entities, one_ref(&args[2], "AXIS2 ref direction")?, 2)?
        } else { vec![1., 0.] };
        return Ok((origin, x.clone(), vec![-x[1], x[0]]));
    }
    let z = if args.len() > 2 && !matches!(args[2], Value::Omitted) {
        direction(entities, one_ref(&args[2], "AXIS2 axis")?, 3)?
    } else { vec![0., 0., 1.] };
    let mut x = if args.len() > 3 && !matches!(args[3], Value::Omitted) {
        direction(entities, one_ref(&args[3], "AXIS2 ref direction")?, 3)?
    } else { vec![1., 0., 0.] };
    let dot = x.iter().zip(&z).map(|(a,b)| a*b).sum::<f64>();
    for i in 0..3 { x[i] -= dot * z[i]; }
    let nx = x.iter().map(|v| v*v).sum::<f64>().sqrt();
    if nx <= 1e-12 { return Err(refuse("AXIS2 directions are parallel")); }
    for v in &mut x { *v /= nx; }
    let y = vec![z[1]*x[2]-z[2]*x[1], z[2]*x[0]-z[0]*x[2], z[0]*x[1]-z[1]*x[0]];
    Ok((origin, x, y))
}
fn conic_curve(origin: &[f64], x: &[f64], y: &[f64], a: f64, b: f64) -> Result<Curve> {
    if !(a > 0. && b > 0. && a.is_finite() && b.is_finite()) { return Err(refuse("Conic radii must be positive")); }
    let endpoints = [[1.,0.],[0.,1.],[-1.,0.],[0.,-1.],[1.,0.]];
    let shoulders = [[1.,1.],[-1.,1.],[-1.,-1.],[1.,-1.]];
    let map = |q: [f64;2]| (0..origin.len()).map(|i| origin[i] + a*q[0]*x[i] + b*q[1]*y[i]).collect::<Vec<_>>();
    let mut cps = vec![map(endpoints[0])]; let mut weights = vec![1.];
    for i in 0..4 { cps.push(map(shoulders[i])); weights.push(std::f64::consts::FRAC_1_SQRT_2); cps.push(map(endpoints[i+1])); weights.push(1.); }
    let c = Curve { degree: 2, knots: vec![0.,0.,0.,1.,1.,2.,2.,3.,3.,4.,4.,4.], control_points: cps, weights, periodic: false };
    c.validate()?; Ok(c)
}
fn trim_parameters(entities: &BTreeMap<usize, Entity>, v: &Value, base: &Curve, dim: usize, scale: f64) -> Result<f64> {
    let items = list(v, "TRIMMED_CURVE trim selector")?;
    if items.len() != 1 { return Err(refuse("TRIMMED_CURVE admits one parameter selector per end")); }
    match &items[0] {
        Value::Number(_) | Value::Integer(_) => number(&items[0], "TRIMMED_CURVE parameter"),
        Value::Call(name, args) if name == "PARAMETER_VALUE" && args.len() == 1 => number(&args[0], "PARAMETER_VALUE"),
        Value::Ref(id) => {
            let target = point(entities, *id, dim, scale)?;
            let domain = base.domain();
            let start = base.evaluate(domain[0])?.point;
            let end = base.evaluate(domain[1])?.point;
            let distance = |p: &[f64]| p.iter().zip(&target).map(|(a,b)|(a-b)*(a-b)).sum::<f64>().sqrt();
            if distance(&start) <= 1e-7 { Ok(domain[0]) }
            else if distance(&end) <= 1e-7 { Ok(domain[1]) }
            else { Err(refuse("TRIMMED_CURVE point selector is not an exact basis endpoint")) }
        }
        _ => Err(refuse("TRIMMED_CURVE selector is outside the parameter/exact-endpoint subset")),
    }
}
fn trim_conic(base: &Curve, start: f64, end: f64, sense: bool) -> Result<Curve> {
    let mut a0 = start; let mut a1 = end;
    if !sense { std::mem::swap(&mut a0, &mut a1); }
    while a1 <= a0 { a1 += std::f64::consts::TAU; }
    if a1 - a0 > std::f64::consts::TAU + 1e-10 { return Err(refuse("TRIMMED_CURVE conic sweep exceeds one revolution")); }
    let center = {
        let p0=&base.control_points[0]; let p4=&base.control_points[4];
        p0.iter().zip(p4).map(|(a,b)| (a+b)/2.).collect::<Vec<_>>()
    };
    let ex: Vec<_> = base.control_points[0].iter().zip(&center).map(|(a,b)| a-b).collect();
    let ey: Vec<_> = base.control_points[2].iter().zip(&center).map(|(a,b)| a-b).collect();
    let pieces = ((a1-a0)/std::f64::consts::FRAC_PI_2).ceil().max(1.) as usize;
    let mut cps=Vec::new(); let mut weights=Vec::new(); let mut knots=vec![0.,0.,0.];
    for i in 0..pieces {
        let s=a0+(a1-a0)*i as f64/pieces as f64; let e=a0+(a1-a0)*(i+1) as f64/pieces as f64; let m=(s+e)/2.; let w=((e-s)/2.).cos();
        let p = |angle:f64, factor:f64| center.iter().enumerate().map(|(j,c)| c+factor*(ex[j]*angle.cos()+ey[j]*angle.sin())).collect::<Vec<_>>();
        if i==0 { cps.push(p(s,1.)); weights.push(1.); }
        cps.push(p(m,1./w)); weights.push(w); cps.push(p(e,1.)); weights.push(1.);
        if i+1<pieces { knots.push((i+1) as f64); knots.push((i+1) as f64); }
    }
    knots.extend([pieces as f64;3]);
    let c=Curve { degree:2, knots, control_points:cps, weights, periodic:false }; c.validate()?; Ok(c)
}
fn curve(entities: &BTreeMap<usize, Entity>, id: usize, dim: usize, scale: f64) -> Result<Curve> {
    let (ty, a) = call(entities, id)?;
    if ty == "LINE" {
        if a.len()!=3 { return Err(refuse("LINE argument count mismatch")); }
        let p=point(entities,one_ref(&a[1],"LINE point")?,dim,scale)?;
        let d=vector(entities,one_ref(&a[2],"LINE vector")?,dim,scale)?;
        return Curve::from_polyline(vec![p.clone(),p.iter().zip(d).map(|(x,d)|x+d).collect()]);
    }
    if matches!(ty, "CIRCLE" | "ELLIPSE") {
        let expected=if ty=="CIRCLE"{3}else{4};
        if a.len()!=expected { return Err(refuse(format!("{ty} argument count mismatch"))); }
        let (o,x,y)=axis2(entities,one_ref(&a[1],"conic placement")?,dim,scale)?;
        let major=number(&a[2],"conic radius")? * if dim==3 {scale}else{1.};
        let minor=if ty=="ELLIPSE"{number(&a[3],"ellipse minor radius")? * if dim==3 {scale}else{1.}}else{major};
        return conic_curve(&o,&x,&y,major,minor);
    }
    if ty == "TRIMMED_CURVE" {
        if a.len()!=6 { return Err(refuse("TRIMMED_CURVE argument count mismatch")); }
        let basis_id=one_ref(&a[1],"TRIMMED_CURVE basis")?;
        let (basis_ty,_)=call(entities,basis_id)?;
        let base=curve(entities,basis_id,dim,scale)?;
        let start=trim_parameters(entities,&a[2],&base,dim,scale)?; let end=trim_parameters(entities,&a[3],&base,dim,scale)?; let sense=boolean(&a[4],"TRIMMED_CURVE sense")?;
        if matches!(basis_ty,"CIRCLE"|"ELLIPSE") { return trim_conic(&base,start,end,sense); }
        if basis_ty=="LINE" {
            let p0=base.evaluate(start)?.point; let p1=base.evaluate(end)?.point;
            return Curve::from_polyline(if sense {vec![p0,p1]} else {vec![p1,p0]});
        }
        return Err(refuse("TRIMMED_CURVE basis is outside the exact direct subset"));
    }
    if ty != "B_SPLINE_CURVE_WITH_KNOTS" {
        return Err(refuse(format!("Reachable curve {ty} is typed-refused by the direct native subset")));
    }
    if a.len() != 11 { return Err(refuse("B_SPLINE_CURVE_WITH_KNOTS argument count mismatch")); }
    let degree = usize_value(&a[1], "curve degree")?;
    let cps = list(&a[2], "curve control points")?.iter().map(|v| point(entities, one_ref(v, "curve control point")?, dim, scale)).collect::<Result<Vec<_>>>()?;
    let weights = values(&a[7], "curve weights")?;
    let knots = expand(&ints(&a[8], "curve multiplicities")?, &values(&a[9], "curve knots")?)?;
    let c = Curve { degree, knots, control_points: cps, weights, periodic: boolean(&a[5], "curve closed flag")? };
    c.validate()?;
    Ok(c)
}
fn analytic_surface(entities:&BTreeMap<usize,Entity>,ty:&str,a:&[Value],scale:f64)->Result<Surface>{
    let expected=if ty=="TOROIDAL_SURFACE"{4}else if ty=="CONICAL_SURFACE"{4}else{3};
    if a.len()!=expected{return Err(refuse(format!("{ty} argument count mismatch")))}
    let (o,x,y)=axis2(entities,one_ref(&a[1],"analytic surface placement")?,3,scale)?;
    let z=[x[1]*y[2]-x[2]*y[1],x[2]*y[0]-x[0]*y[2],x[0]*y[1]-x[1]*y[0]];
    let radius=number(&a[2],"analytic surface radius")?*scale;
    if !(radius>0.){return Err(refuse("Analytic surface radius must be positive"))}
    let angles=[0.,std::f64::consts::FRAC_PI_4,std::f64::consts::FRAC_PI_2,3.*std::f64::consts::FRAC_PI_4,std::f64::consts::PI,5.*std::f64::consts::FRAC_PI_4,3.*std::f64::consts::FRAC_PI_2,7.*std::f64::consts::FRAC_PI_4,std::f64::consts::TAU];
    let wu=[1.,std::f64::consts::FRAC_1_SQRT_2,1.,std::f64::consts::FRAC_1_SQRT_2,1.,std::f64::consts::FRAC_1_SQRT_2,1.,std::f64::consts::FRAC_1_SQRT_2,1.];
    let ku=vec![0.,0.,0.,std::f64::consts::FRAC_PI_2,std::f64::consts::FRAC_PI_2,std::f64::consts::PI,std::f64::consts::PI,3.*std::f64::consts::FRAC_PI_2,3.*std::f64::consts::FRAC_PI_2,std::f64::consts::TAU,std::f64::consts::TAU,std::f64::consts::TAU];
    let (v_angles,wv,kv)=if matches!(ty,"SPHERICAL_SURFACE"){
        (vec![-std::f64::consts::FRAC_PI_2,-std::f64::consts::FRAC_PI_4,0.,std::f64::consts::FRAC_PI_4,std::f64::consts::FRAC_PI_2],
         vec![1.,std::f64::consts::FRAC_1_SQRT_2,1.,std::f64::consts::FRAC_1_SQRT_2,1.],
         vec![-std::f64::consts::FRAC_PI_2,-std::f64::consts::FRAC_PI_2,-std::f64::consts::FRAC_PI_2,0.,0.,std::f64::consts::FRAC_PI_2,std::f64::consts::FRAC_PI_2,std::f64::consts::FRAC_PI_2])
    }else if ty=="TOROIDAL_SURFACE"{(angles.to_vec(),wu.to_vec(),ku.clone())}
    else{(vec![-1e6,1e6],vec![1.,1.],vec![-1e6,-1e6,1e6,1e6])};
    let minor=if ty=="TOROIDAL_SURFACE"{let r=number(&a[3],"torus minor radius")?*scale;if !(r>0.&&r<radius){return Err(refuse("TOROIDAL_SURFACE requires 0 < minor < major"))}r}else{radius};
    let semi=if ty=="CONICAL_SURFACE"{number(&a[3],"cone semi-angle")?}else{0.};
    let mut cps=Vec::new();let mut weights=Vec::new();
    for (iu,&angle) in angles.iter().enumerate(){
        let radial=[x[0]*angle.cos()+y[0]*angle.sin(),x[1]*angle.cos()+y[1]*angle.sin(),x[2]*angle.cos()+y[2]*angle.sin()];
        let mut row=Vec::new();let mut wr=Vec::new();
        for (iv,&v) in v_angles.iter().enumerate(){
            let shoulder_scale=if iu%2==1{1./wu[iu]}else{1.};
            let meridian_scale=if iv%2==1&&matches!(ty,"SPHERICAL_SURFACE"|"TOROIDAL_SURFACE"){1./wv[iv]}else{1.};
            let (radial_distance,height)=match ty{
                "CYLINDRICAL_SURFACE"=>(radius*shoulder_scale,v),
                "CONICAL_SURFACE"=>((radius+v*semi.tan())*shoulder_scale,v),
                "SPHERICAL_SURFACE"=>(radius*v.cos()*shoulder_scale*meridian_scale,radius*v.sin()*meridian_scale),
                "TOROIDAL_SURFACE"=>((radius+minor*v.cos()*meridian_scale)*shoulder_scale,minor*v.sin()*meridian_scale),
                _=>return Err(refuse("Unsupported analytic surface")),
            };
            row.push((0..3).map(|j|o[j]+radial_distance*radial[j]+height*z[j]).collect());
            wr.push(wu[iu]*wv[iv]);
        }
        cps.push(row);weights.push(wr);
    }
    // Full rational circles are clamped at an explicit seam. Periodic lifts
    // remain carried by CoedgeTrim; marking this net periodic would require
    // repeating `degree` control rows and would change the STEP angle domain.
    let s=Surface{degree_u:2,degree_v:if v_angles.len()>2{2}else{1},knots_u:ku,knots_v:kv,control_points:cps,weights,periodic_u:false,periodic_v:false};
    s.validate()?;Ok(s)
}
fn surface(entities: &BTreeMap<usize, Entity>, id: usize, scale: f64) -> Result<Surface> {
    let (ty, a) = call(entities, id)?;
    if ty == "PLANE" {
        if a.len()!=2 { return Err(refuse("PLANE argument count mismatch")); }
        let (o,x,y)=axis2(entities,one_ref(&a[1],"PLANE placement")?,3,scale)?;
        let p=|u:f64,v:f64| (0..3).map(|i|o[i]+u*x[i]+v*y[i]).collect::<Vec<_>>();
        const LIMIT:f64=1e6;
        let s=Surface { degree_u:1,degree_v:1,knots_u:vec![-LIMIT,-LIMIT,LIMIT,LIMIT],knots_v:vec![-LIMIT,-LIMIT,LIMIT,LIMIT],
            control_points:vec![vec![p(-LIMIT,-LIMIT),p(-LIMIT,LIMIT)],vec![p(LIMIT,-LIMIT),p(LIMIT,LIMIT)]],weights:vec![vec![1.,1.],vec![1.,1.]],periodic_u:false,periodic_v:false };
        s.validate()?; return Ok(s);
    }
    if matches!(ty,"CYLINDRICAL_SURFACE"|"CONICAL_SURFACE"|"SPHERICAL_SURFACE"|"TOROIDAL_SURFACE") {
        return Err(refuse(format!("{ty} periodic/angular parameterization is available only in step-interchange/4")));
    }
    if ty != "B_SPLINE_SURFACE_WITH_KNOTS" {
        return Err(refuse(format!("Reachable surface {ty} is typed-refused by the direct native subset")));
    }
    if a.len() != 16 { return Err(refuse("B_SPLINE_SURFACE_WITH_KNOTS argument count mismatch")); }
    let rows = list(&a[3], "surface control net")?;
    let cps = rows.iter().map(|row| list(row, "surface control row")?.iter().map(|v| point(entities, one_ref(v, "surface control point")?, 3, scale)).collect::<Result<Vec<_>>>()).collect::<Result<Vec<_>>>()?;
    let flat = values(&a[10], "surface weights")?;
    let nu = cps.len(); let nv = cps.first().map_or(0, Vec::len);
    if nu == 0 || nv == 0 || cps.iter().any(|r| r.len() != nv) || flat.len() != nu * nv { return Err(refuse("Surface control/weight shape mismatch")); }
    let weights = flat.chunks(nv).map(|r| r.to_vec()).collect();
    let s = Surface {
        degree_u: usize_value(&a[1], "surface U degree")?,
        degree_v: usize_value(&a[2], "surface V degree")?,
        knots_u: expand(&ints(&a[11], "surface U multiplicities")?, &values(&a[13], "surface U knots")?)?,
        knots_v: expand(&ints(&a[12], "surface V multiplicities")?, &values(&a[14], "surface V knots")?)?,
        control_points: cps, weights,
        periodic_u: boolean(&a[6], "surface U closed flag")?,
        periodic_v: boolean(&a[7], "surface V closed flag")?,
    };
    s.validate()?;
    Ok(s)
}
fn surface_v4(entities:&BTreeMap<usize,Entity>,id:usize,scale:f64)->Result<Surface>{
    let (ty,a)=call(entities,id)?;
    if matches!(ty,"CYLINDRICAL_SURFACE"|"CONICAL_SURFACE"|"SPHERICAL_SURFACE"|"TOROIDAL_SURFACE"){
        analytic_surface(entities,ty,a,scale)
    }else{surface(entities,id,scale)}
}

fn si_scale(args: &[Value]) -> Result<f64> {
    if !args.iter().any(|v| matches!(v, Value::Enum(e) if e == "METRE")) { return Err(refuse("SI unit is not a length unit")); }
    let prefix = args.iter().find_map(|v| if let Value::Enum(e)=v {(e!="METRE").then_some(e.as_str())}else{None});
    Ok(match prefix {
        None => 1000., Some("DECI") => 100., Some("CENTI") => 10., Some("MILLI") => 1.,
        Some("MICRO") => 1e-3, Some("NANO") => 1e-6, Some("KILO") => 1e6,
        _ => return Err(refuse("Unsupported SI length prefix")),
    })
}
fn unit_scale(entities:&BTreeMap<usize,Entity>, id:usize, depth:usize, active:&mut BTreeSet<usize>)->Result<f64>{
    if depth>8{return Err(refuse("CONVERSION_BASED_UNIT chain depth exceeds eight"))}
    if !active.insert(id){return Err(refuse("Cycle in CONVERSION_BASED_UNIT chain"))}
    let entity=entities.get(&id).ok_or_else(||refuse("Missing unit reference"))?;
    let result=match &entity.value {
        Value::List(parts)=>{
            let mut answer=None;
            for p in parts { if let Value::Call(n,a)=p { if n=="SI_UNIT"{answer=Some(si_scale(a)?)} else if n=="CONVERSION_BASED_UNIT"{answer=Some(conversion_scale(entities,a,depth,active)?)} } }
            answer.ok_or_else(||refuse("Complex unit has no admitted length component"))?
        }
        Value::Call(n,a) if n=="SI_UNIT"=>si_scale(a)?,
        Value::Call(n,a) if n=="CONVERSION_BASED_UNIT"=>conversion_scale(entities,a,depth,active)?,
        _=>return Err(refuse("Unsupported length unit entity")),
    };
    active.remove(&id); Ok(result)
}
fn conversion_scale(entities:&BTreeMap<usize,Entity>,args:&[Value],depth:usize,active:&mut BTreeSet<usize>)->Result<f64>{
    if args.len()<2{return Err(refuse("CONVERSION_BASED_UNIT argument count mismatch"))}
    let factor_id=one_ref(args.last().unwrap(),"CONVERSION_BASED_UNIT factor")?;
    let (ty,a)=call(entities,factor_id)?;
    if !matches!(ty,"LENGTH_MEASURE_WITH_UNIT"|"MEASURE_WITH_UNIT")||a.len()!=2{return Err(refuse("Conversion factor must be LENGTH_MEASURE_WITH_UNIT"))}
    let factor=match &a[0]{Value::Call(n,v) if n=="LENGTH_MEASURE"&&v.len()==1=>number(&v[0],"LENGTH_MEASURE")?,v=>number(v,"conversion factor")?};
    if !(factor>0.&&factor<=1e9){return Err(refuse("Conversion factor must be positive and bounded"))}
    Ok(factor*unit_scale(entities,one_ref(&a[1],"conversion base unit")?,depth+1,active)?)
}
fn length_scale(entities: &BTreeMap<usize, Entity>, reachable: &BTreeSet<usize>) -> Result<f64> {
    let mut unit_ids=Vec::new();
    for id in reachable {
        let entity=&entities[id];
        if let Value::Call(n,a)=&entity.value {
            if n=="GLOBAL_UNIT_ASSIGNED_CONTEXT" { for v in a { refs(v,&mut unit_ids); } }
        }
    }
    if unit_ids.is_empty() {
        unit_ids.extend(entities.iter().filter_map(|(id,e)| match &e.value {
            Value::List(parts) if parts.iter().any(|p|matches!(p,Value::Call(n,_)if n=="SI_UNIT"||n=="CONVERSION_BASED_UNIT"))=>Some(*id),
            Value::Call(n,_) if n=="SI_UNIT"||n=="CONVERSION_BASED_UNIT"=>Some(*id), _=>None
        }));
    }
    let mut found=Vec::new();
    for id in unit_ids { if let Ok(scale)=unit_scale(entities,id,0,&mut BTreeSet::new()){found.push(scale);} }
    if found.is_empty() { return Err(refuse("STEP /3 requires an SI length unit context")); }
    if found.iter().any(|v| !v.is_finite() || (*v - found[0]).abs() > f64::EPSILON) { return Err(refuse("Unsupported or conflicting STEP length units")); }
    Ok(found[0])
}

fn rigid_frame(entities:&BTreeMap<usize,Entity>,id:usize,scale:f64)->Result<[[f64;4];4]>{
    let (o,x,y)=axis2(entities,id,3,scale)?;
    let z=[x[1]*y[2]-x[2]*y[1],x[2]*y[0]-x[0]*y[2],x[0]*y[1]-x[1]*y[0]];
    Ok([[x[0],y[0],z[0],o[0]],[x[1],y[1],z[1],o[1]],[x[2],y[2],z[2],o[2]],[0.,0.,0.,1.]])
}
fn mul(a:[[f64;4];4],b:[[f64;4];4])->[[f64;4];4]{std::array::from_fn(|i|std::array::from_fn(|j|(0..4).map(|k|a[i][k]*b[k][j]).sum()))}
fn inverse_rigid(m:[[f64;4];4])->[[f64;4];4]{
    let mut r=[[0.;4];4]; for i in 0..3{for j in 0..3{r[i][j]=m[j][i]} r[i][3]=-(0..3).map(|j|r[i][j]*m[j][3]).sum::<f64>();} r[3][3]=1.;r
}
fn reachable_transform(entities:&BTreeMap<usize,Entity>,linked:&BTreeSet<usize>,scale:f64)->Result<Option<[[f64;4];4]>>{
    let mut transforms=Vec::new();
    for id in linked {
        let (ty,a)=call(entities,*id)?;
        if ty.starts_with("CARTESIAN_TRANSFORMATION_OPERATOR") {
            return Err(refuse("Reachable generic transform is refused: reflection/nonuniform/singular transforms are outside /3"));
        }
        if ty=="ITEM_DEFINED_TRANSFORMATION" {
            if a.len()!=4{return Err(refuse("ITEM_DEFINED_TRANSFORMATION argument count mismatch"))}
            let from=rigid_frame(entities,one_ref(&a[2],"transformation source")?,scale)?;
            let to=rigid_frame(entities,one_ref(&a[3],"transformation target")?,scale)?;
            transforms.push(mul(to,inverse_rigid(from)));
            if transforms.len()>32{return Err(refuse("Nested rigid transformation depth exceeds 32"))}
        }
    }
    if transforms.is_empty(){return Ok(None)}
    let mut total=[[1.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,1.,0.],[0.,0.,0.,1.]];
    for t in transforms { total=mul(t,total); }
    Ok(Some(total))
}

fn curve_and_pcurves(entities: &BTreeMap<usize, Entity>, id: usize, face_surface: usize, scale: f64) -> Result<(Curve, Curve)> {
    let (ty, a) = call(entities, id)?;
    if ty != "SURFACE_CURVE" && ty != "SEAM_CURVE" {
        return Err(refuse("EDGE_CURVE must use SURFACE_CURVE or SEAM_CURVE in /3"));
    }
    if a.len() < 3 { return Err(refuse("SURFACE_CURVE argument count mismatch")); }
    let c3 = curve(entities, one_ref(&a[1], "SURFACE_CURVE 3D curve")?, 3, scale)?;
    let mut match_curve = None;
    for p in list(&a[2], "SURFACE_CURVE pcurves")? {
        let pid = one_ref(p, "SURFACE_CURVE pcurve")?;
        let (pty, pa) = call(entities, pid)?;
        if pty != "PCURVE" || pa.len() != 3 { return Err(refuse("Malformed PCURVE")); }
        if one_ref(&pa[1], "PCURVE surface")? == face_surface {
            if match_curve.is_some() { return Err(refuse("Duplicate PCURVE for face surface")); }
            match_curve = Some(curve(entities, one_ref(&pa[2], "PCURVE curve")?, 2, 1.)?);
        }
    }
    Ok((c3, match_curve.ok_or_else(|| refuse("No PCURVE corresponds to ADVANCED_FACE surface"))?))
}

fn verify_correspondence(c3: &Curve, pc: &Curve, s: &Surface, reversed: bool) -> Result<CoedgeTrim> {
    let d3 = c3.domain(); let d2 = pc.domain();
    let uv_start = pc.evaluate(d2[0])?.point;
    let uv_end = pc.evaluate(d2[1])?.point;
    let domain_u = [s.knots_u[s.degree_u], s.knots_u[s.knots_u.len() - s.degree_u - 1]];
    let domain_v = [s.knots_v[s.degree_v], s.knots_v[s.knots_v.len() - s.degree_v - 1]];
    let lift = |uv: &[f64]| -> [i32; 2] {
        let component = |value: f64, domain: [f64; 2], periodic: bool| {
            if periodic {
                ((value - domain[0]) / (domain[1] - domain[0])).floor() as i32
            } else { 0 }
        };
        [component(uv[0], domain_u, s.periodic_u), component(uv[1], domain_v, s.periodic_v)]
    };
    for q in 0..=8 {
        let t = q as f64 / 8.;
        let t3 = if reversed { d3[1] - t * (d3[1] - d3[0]) } else { d3[0] + t * (d3[1] - d3[0]) };
        let uv = pc.evaluate(d2[0] + t * (d2[1] - d2[0]))?.point;
        let p = c3.evaluate(t3)?.point;
        let sp = s.evaluate(uv[0], uv[1])?.point;
        let error = (p[0] - sp[0]).hypot(p[1] - sp[1]).hypot(p[2] - sp[2]);
        if !error.is_finite() || error > 1e-5 { return Err(refuse(format!("Independent 3D curve↔pcurve correspondence failed ({error:.3e})"))); }
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
    entities: &'a BTreeMap<usize, Entity>, scale: f64, analytic_surfaces: bool,
    vertices: Vec<Vertex>, vertex_map: BTreeMap<usize, usize>,
    edges: Vec<Edge>, edge_map: BTreeMap<usize, usize>,
    loops: Vec<Loop>, loop_entity: Vec<usize>, faces: Vec<Face>, face_entity: Vec<usize>,
    shells: Vec<Shell>, shell_entity: Vec<usize>, bodies: Vec<Body>, body_entity: Vec<usize>,
}
impl<'a> DirectBuilder<'a> {
    fn vertex(&mut self, id: usize) -> Result<usize> {
        if let Some(v) = self.vertex_map.get(&id) { return Ok(*v); }
        let (ty, a) = call(self.entities, id)?;
        if ty != "VERTEX_POINT" || a.len() != 2 { return Err(refuse("EDGE_CURVE vertex is not VERTEX_POINT")); }
        let p = point(self.entities, one_ref(&a[1], "VERTEX_POINT geometry")?, 3, self.scale)?;
        let index = self.vertices.len();
        self.vertices.push(Vertex { point: [p[0], p[1], p[2]] });
        self.vertex_map.insert(id, index); Ok(index)
    }
    fn edge(&mut self, id: usize, surface_id: usize, reversed: bool, surface: &Surface) -> Result<(usize, Curve)> {
        let (ty, a) = call(self.entities, id)?;
        if ty != "EDGE_CURVE" || a.len() != 5 { return Err(refuse("ORIENTED_EDGE target is not EDGE_CURVE")); }
        if !boolean(&a[4], "EDGE_CURVE same_sense")? { return Err(refuse("EDGE_CURVE same_sense=.F. is not yet representable independently")); }
        let va = self.vertex(one_ref(&a[1], "EDGE_CURVE start")?)?;
        let vb = self.vertex(one_ref(&a[2], "EDGE_CURVE end")?)?;
        let (c3, pc) = curve_and_pcurves(self.entities, one_ref(&a[3], "EDGE_CURVE geometry")?, surface_id, self.scale)?;
        verify_correspondence(&c3, &pc, surface, reversed)
            .map_err(|e| refuse(format!("EDGE_CURVE #{id} on surface #{surface_id}: {}", e.message)))?;
        let index = if let Some(index) = self.edge_map.get(&id) {
            let prior = &self.edges[*index];
            if prior.vertices != [va, vb] { return Err(refuse("Shared EDGE_CURVE has inconsistent vertices")); }
            *index
        } else {
            let index = self.edges.len();
            self.edges.push(Edge { degenerate: false, vertices: [va, vb], curve: c3 });
            self.edge_map.insert(id, index); index
        };
        Ok((index, pc))
    }
    fn loop_(&mut self, id: usize, surface_id: usize, surface: &Surface) -> Result<usize> {
        let (ty, a) = call(self.entities, id)?;
        if ty != "EDGE_LOOP" || a.len() != 2 { return Err(refuse("FACE_BOUND target is not EDGE_LOOP")); }
        let mut coedges = Vec::new();
        for oriented in list(&a[1], "EDGE_LOOP members")? {
            let oid = one_ref(oriented, "EDGE_LOOP member")?;
            let (oty, oa) = call(self.entities, oid)?;
            if oty != "ORIENTED_EDGE" || oa.len() != 5 { return Err(refuse("EDGE_LOOP member is not ORIENTED_EDGE")); }
            let reversed = !boolean(&oa[4], "ORIENTED_EDGE orientation")?;
            let (edge, pcurve) = self.edge(one_ref(&oa[3], "ORIENTED_EDGE edge")?, surface_id, reversed, surface)?;
            coedges.push(Coedge { edge, reversed, pcurve });
        }
        if coedges.is_empty() { return Err(refuse("EDGE_LOOP cannot be empty")); }
        let index = self.loops.len(); self.loops.push(Loop { coedges }); self.loop_entity.push(id); Ok(index)
    }
    fn face(&mut self, id: usize) -> Result<usize> {
        let (ty, a) = call(self.entities, id)?;
        if ty != "ADVANCED_FACE" || a.len() != 4 { return Err(refuse("CLOSED_SHELL member is not ADVANCED_FACE")); }
        let sid = one_ref(&a[2], "ADVANCED_FACE surface")?;
        let surface = if self.analytic_surfaces { surface_v4(self.entities,sid,self.scale)? } else { surface(self.entities,sid,self.scale)? };
        let mut outer = None; let mut holes = Vec::new();
        for bound in list(&a[1], "ADVANCED_FACE bounds")? {
            let bid = one_ref(bound, "ADVANCED_FACE bound")?;
            let (bty, ba) = call(self.entities, bid)?;
            if !matches!(bty, "FACE_OUTER_BOUND" | "FACE_BOUND") || ba.len() != 3 { return Err(refuse("ADVANCED_FACE bound type is unsupported")); }
            let loop_index = self.loop_(one_ref(&ba[1], "FACE_BOUND loop")?, sid, &surface)?;
            if !boolean(&ba[2], "FACE_BOUND orientation")? {
                for c in &mut self.loops[loop_index].coedges { c.reversed = !c.reversed; }
                self.loops[loop_index].coedges.reverse();
            }
            if bty == "FACE_OUTER_BOUND" {
                if outer.replace(loop_index).is_some() { return Err(refuse("ADVANCED_FACE has multiple outer bounds")); }
            } else { holes.push(loop_index); }
        }
        let index = self.faces.len();
        self.faces.push(Face { surface, outer: outer.ok_or_else(|| refuse("ADVANCED_FACE has no outer bound"))?, holes });
        self.face_entity.push(id);
        // Shell use carries face sense, keeping geometry itself unchanged.
        let _ = boolean(&a[3], "ADVANCED_FACE same_sense")?;
        Ok(index)
    }
    fn shell(&mut self, id: usize) -> Result<usize> {
        let (ty, a) = call(self.entities, id)?;
        if ty != "CLOSED_SHELL" || a.len() != 2 { return Err(refuse("Body shell is not CLOSED_SHELL")); }
        let mut faces = Vec::new();
        for f in list(&a[1], "CLOSED_SHELL faces")? {
            let fid = one_ref(f, "CLOSED_SHELL face")?;
            let (_, fa) = call(self.entities, fid)?;
            faces.push(FaceUse { face: self.face(fid)?, reversed: !boolean(&fa[3], "ADVANCED_FACE sense")? });
        }
        let index = self.shells.len(); self.shells.push(Shell { faces, closed: true }); self.shell_entity.push(id); Ok(index)
    }
    fn body(&mut self, id: usize) -> Result<()> {
        let (ty, a) = call(self.entities, id)?;
        let (outer, inners) = match ty {
            "MANIFOLD_SOLID_BREP" if a.len() == 2 => (one_ref(&a[1], "solid shell")?, Vec::new()),
            "BREP_WITH_VOIDS" if a.len() == 3 => (one_ref(&a[1], "outer shell")?, list(&a[2], "void shells")?.iter().map(|v| one_ref(v, "void shell")).collect::<Result<Vec<_>>>()?),
            _ => return Err(refuse("Selected representation contains unsupported body root")),
        };
        let outer_shell = self.shell(outer)?;
        let inner_shells = inners.into_iter().map(|s| self.shell(s)).collect::<Result<Vec<_>>>()?;
        self.bodies.push(Body { outer_shell, inner_shells }); self.body_entity.push(id); Ok(())
    }
}

fn metadata_entity_map(builder: &DirectBuilder<'_>) -> Vec<(TopoKind, usize, usize)> {
    let mut out = Vec::new();
    out.extend(builder.vertex_map.iter().map(|(entity, index)| (TopoKind::Vertex, *index, *entity)));
    out.extend(builder.edge_map.iter().map(|(entity, index)| (TopoKind::Edge, *index, *entity)));
    out.extend(builder.loop_entity.iter().enumerate().map(|(i, e)| (TopoKind::Loop, i, *e)));
    out.extend(builder.face_entity.iter().enumerate().map(|(i, e)| (TopoKind::Face, i, *e)));
    out.extend(builder.shell_entity.iter().enumerate().map(|(i, e)| (TopoKind::Shell, i, *e)));
    out.extend(builder.body_entity.iter().enumerate().map(|(i, e)| (TopoKind::Body, i, *e)));
    out
}

fn digest_value(value: &Value, entities: &BTreeMap<usize, Entity>, active: &mut BTreeSet<usize>, depth: usize) -> Result<String> {
    if depth > MAX_GRAPH_DEPTH { return Err(refuse("Canonical graph digest depth exceeds 64")); }
    let text = match value {
        Value::String(_) => "''".into(), // entity names are non-geometric metadata
        Value::Number(n) => format!("{n:.17e}"), Value::Integer(n) => n.to_string(),
        Value::Enum(v) => format!(".{v}."), Value::Omitted => "$".into(), Value::Derived => "*".into(),
        Value::List(v) => format!("({})", v.iter().map(|x| digest_value(x, entities, active, depth + 1)).collect::<Result<Vec<_>>>()?.join(",")),
        Value::Call(n, v) => {
            let args = v.iter().enumerate().map(|(i, x)| if i == 0 && matches!(x, Value::String(_)) { Ok("''".into()) } else { digest_value(x, entities, active, depth + 1) }).collect::<Result<Vec<_>>>()?;
            format!("{n}({})", args.join(","))
        }
        Value::Ref(id) => {
            if !active.insert(*id) { return Err(refuse("Cycle while computing canonical graph digest")); }
            let e = entities.get(id).ok_or_else(|| refuse("Missing digest dependency"))?;
            let result = digest_value(&e.value, entities, active, depth + 1)?;
            active.remove(id);
            format!("#{result}")
        }
    };
    Ok(text)
}
fn hash(text: &str) -> String {
    let mut h = 0xcbf29ce484222325u64;
    for b in text.bytes() { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
    format!("{h:016x}")
}
fn entity_digest(entities: &BTreeMap<usize, Entity>, id: usize) -> Result<String> {
    let entity = entities.get(&id).ok_or_else(|| refuse("Missing identity entity"))?;
    Ok(hash(&digest_value(&entity.value, entities, &mut BTreeSet::from([id]), 0)?))
}

fn restore_identity(entities: &BTreeMap<usize, Entity>, map: &[(TopoKind, usize, usize)], model: &mut Model) -> Result<crate::step_interchange::StepIdentityReport> {
    let mut metadata = Vec::new();
    for &(kind, index, entity_id) in map {
        let (_, args) = call(entities, entity_id)?;
        let Some(n) = name(args) else { continue; };
        let Some(payload) = n.strip_prefix("OSCAD_TOPO/3|") else { continue; };
        let fields: Vec<_> = payload.split('|').collect();
        if fields.len() != 3 { return Err(refuse("Malformed STEP /3 identity metadata")); }
        let mk = TopoKind::parse(fields[0]).ok_or_else(|| refuse("Unknown STEP /3 identity kind"))?;
        let id = TopoId::parse(fields[1]).map_err(refuse)?;
        if mk != kind || id.kind() != kind || fields[2] != entity_digest(entities, entity_id)? { return Err(refuse("Transplanted STEP /3 identity metadata")); }
        metadata.push((kind, index, id));
    }
    let count = map.len();
    if metadata.is_empty() {
        model.rebuild_topology_ids();
        return Ok(crate::step_interchange::StepIdentityReport { preserved: false, source: "external-step", preserved_count: 0, created_count: count, lost_count: count });
    }
    if metadata.len() != count { return Err(refuse("Partial STEP /3 identity metadata")); }
    let mut seen = BTreeSet::new();
    let mut remap = BTreeMap::new();
    for (kind, index, id) in metadata {
        if !seen.insert(id) { return Err(refuse("Duplicate STEP /3 identity metadata")); }
        let old = match kind {
            TopoKind::Vertex => model.1.vertices[index], TopoKind::Edge => model.1.edges[index],
            TopoKind::Loop => model.1.loops[index], TopoKind::Face => model.1.faces[index],
            TopoKind::Shell => model.1.shells[index], TopoKind::Body => model.1.bodies[index],
            TopoKind::ControlPoint => return Err(refuse("Control-point identity is not STEP topology identity")),
        };
        remap.insert(old, id);
        match kind {
            TopoKind::Vertex => model.1.vertices[index] = id, TopoKind::Edge => model.1.edges[index] = id,
            TopoKind::Loop => model.1.loops[index] = id, TopoKind::Face => model.1.faces[index] = id,
            TopoKind::Shell => model.1.shells[index] = id, TopoKind::Body => model.1.bodies[index] = id,
            TopoKind::ControlPoint => return Err(refuse("Control-point identity is not STEP topology identity")),
        }
    }
    model.1.change_set.nodes = model.1.change_set.nodes.iter()
        .map(|(id, kind)| (remap.get(id).copied().unwrap_or(*id), *kind)).collect();
    for change in &mut model.1.change_set.changes {
        for id in change.parents.iter_mut().chain(&mut change.children) {
            if let Some(replacement) = remap.get(id) { *id = *replacement; }
        }
    }
    Ok(crate::step_interchange::StepIdentityReport { preserved: true, source: "internal-metadata", preserved_count: count, created_count: 0, lost_count: 0 })
}

const UNREACHABLE_ALLOWLIST: &[&str] = &[
    "APPLICATION_CONTEXT", "PRODUCT", "PRODUCT_DEFINITION", "PRODUCT_DEFINITION_FORMATION",
    "PRODUCT_DEFINITION_SHAPE", "SHAPE_DEFINITION_REPRESENTATION", "ADVANCED_BREP_SHAPE_REPRESENTATION",
    "GEOMETRIC_REPRESENTATION_CONTEXT", "GLOBAL_UNIT_ASSIGNED_CONTEXT", "SI_UNIT",
    "PRESENTATION_LAYER_ASSIGNMENT", "STYLED_ITEM", "COLOUR_RGB", "SURFACE_STYLE_USAGE",
    "ITEM_DEFINED_TRANSFORMATION", "AXIS2_PLACEMENT_3D", "CARTESIAN_POINT", "DIRECTION",
    "REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION",
];

fn import_step_direct(text: &str, analytic_surfaces: bool, capability: &'static str) -> Result<(Model, FeatureCertificate, StepV3Report)> {
    if !text.to_ascii_uppercase().contains("ISO-10303-21") { return Err(refuse("Not an ISO-10303-21 exchange")); }
    let entities = parse(text)?;
    let representations:Vec<_>=entities.iter().filter_map(|(id,_)|call(&entities,*id).ok().and_then(|(ty,_)|(ty=="ADVANCED_BREP_SHAPE_REPRESENTATION").then_some(*id))).collect();
    let all_bodies:Vec<_>=entities.iter().filter_map(|(id,_)|call(&entities,*id).ok().and_then(|(ty,_)|matches!(ty,"MANIFOLD_SOLID_BREP"|"BREP_WITH_VOIDS").then_some(*id))).collect();
    let graph_roots=if representations.is_empty(){all_bodies.clone()}else{representations};
    if graph_roots.is_empty() { return Err(refuse("STEP /3 requires a manifold advanced B-rep root")); }
    let linked = reachable(&entities, &graph_roots)?;
    let roots:Vec<_>=all_bodies.into_iter().filter(|id|linked.contains(id)).collect();
    if roots.is_empty(){return Err(refuse("Selected representation contains no manifold B-rep body"))}
    const REACHABLE_TYPES:&[&str]=&[
        "ADVANCED_BREP_SHAPE_REPRESENTATION","MANIFOLD_SOLID_BREP","BREP_WITH_VOIDS","CLOSED_SHELL",
        "ADVANCED_FACE","FACE_OUTER_BOUND","FACE_BOUND","EDGE_LOOP","ORIENTED_EDGE","EDGE_CURVE",
        "VERTEX_POINT","CARTESIAN_POINT","DIRECTION","VECTOR","AXIS2_PLACEMENT_2D","AXIS2_PLACEMENT_3D",
        "LINE","CIRCLE","ELLIPSE","TRIMMED_CURVE","B_SPLINE_CURVE_WITH_KNOTS","PCURVE","SURFACE_CURVE","SEAM_CURVE",
        "PLANE","CYLINDRICAL_SURFACE","CONICAL_SURFACE","SPHERICAL_SURFACE","TOROIDAL_SURFACE","B_SPLINE_SURFACE_WITH_KNOTS",
        "GEOMETRIC_REPRESENTATION_CONTEXT","GLOBAL_UNIT_ASSIGNED_CONTEXT","SI_UNIT","CONVERSION_BASED_UNIT",
        "LENGTH_MEASURE_WITH_UNIT","MEASURE_WITH_UNIT","ITEM_DEFINED_TRANSFORMATION","REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION",
        "CARTESIAN_TRANSFORMATION_OPERATOR_3D",
    ];
    for id in &linked {
        let (ty,_)=call(&entities,*id)?;
        if !REACHABLE_TYPES.contains(&ty) && !matches!(ty,"COMPLEX") {
            return Err(refuse(format!("Reachable entity {ty} is outside the direct finite graph subset")));
        }
    }
    let mut ignored = BTreeSet::new();
    for (id, _) in &entities {
        if linked.contains(id) { continue; }
        let (ty, _) = call(&entities, *id)?;
        if UNREACHABLE_ALLOWLIST.contains(&ty) { ignored.insert(ty.to_string()); }
        else { return Err(refuse(format!("Unreachable entity {ty} is not presentation metadata allowlisted by /3"))); }
    }
    let scale = length_scale(&entities,&linked)?;
    let placement=reachable_transform(&entities,&linked,scale)?;
    let mut builder = DirectBuilder {
        entities: &entities, scale, analytic_surfaces, vertices: vec![], vertex_map: BTreeMap::new(),
        edges: vec![], edge_map: BTreeMap::new(), loops: vec![], loop_entity: vec![],
        faces: vec![], face_entity: vec![], shells: vec![], shell_entity: vec![],
        bodies: vec![], body_entity: vec![],
    };
    for root in roots { builder.body(root)?; }
    let identity_map = metadata_entity_map(&builder);
    let topology = brep_topology::Model {
        vertices: builder.vertices, edges: builder.edges, loops: builder.loops,
        faces: builder.faces, shells: builder.shells, bodies: builder.bodies, tolerance_mm: 1e-7,
    };
    let mut model = Model(topology, TopologyIds::default());
    model.rebuild_topology_ids();
    if let Some(matrix)=placement { model=crate::transform::affine(&model,matrix)?; }
    let identity = restore_identity(&entities, &identity_map, &mut model)?;
    model.validate()?;
    Ok((model, FeatureCertificate {
        capability, complete: true,
        notes: vec!["direct_part21_topology", "line_circle_ellipse_trimmed_curve", "plane_and_rational_multispan_bspline_surface", "periodic_analytic_surfaces_typed_refused_parameterization", "shared_topology_senses_holes", "multi_body_multi_cavity", "si_and_positive_conversion_units", "reachable_rigid_placements_only", "independent_curve_pcurve_check", "bounded_reachable_graph"],
    }, StepV3Report { identity, ignored_entities: ignored.into_iter().collect(), instance_count: entities.len(), reachable_count: linked.len() }))
}

pub fn import_step_v3(text: &str) -> Result<(Model, FeatureCertificate, StepV3Report)> {
    import_step_direct(text, false, STEP_INTERCHANGE_V3_CAPABILITY)
}

pub fn import_step_v4(text: &str) -> Result<(Model, FeatureCertificate, StepV3Report)> {
    import_step_direct(text, true, STEP_INTERCHANGE_V4_CAPABILITY)
}

struct Writer { next: usize, rows: BTreeMap<usize, String> }
impl Writer {
    fn new() -> Self { Self { next: 1, rows: BTreeMap::new() } }
    fn reserve(&mut self) -> usize { let id = self.next; self.next += 1; id }
    fn set(&mut self, id: usize, body: String) { self.rows.insert(id, body); }
    fn emit(&mut self, body: String) -> usize { let id = self.reserve(); self.set(id, body); id }
    fn point(&mut self, p: &[f64]) -> usize { self.emit(format!("CARTESIAN_POINT('',({}))", floats(p))) }
}
fn floats(v: &[f64]) -> String { v.iter().map(|x| format!("{x:.17e}")).collect::<Vec<_>>().join(",") }
fn refs_text(v: &[usize]) -> String { v.iter().map(|x| format!("#{x}")).collect::<Vec<_>>().join(",") }
fn knot_form(knots: &[f64]) -> (Vec<usize>, Vec<f64>) {
    let mut m = Vec::new(); let mut k = Vec::new();
    for &x in knots {
        if k.last().is_some_and(|v: &f64| (*v - x).abs() <= f64::EPSILON) { *m.last_mut().unwrap() += 1; }
        else { k.push(x); m.push(1); }
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
    let grid = s.control_points.iter().map(|row| {
        let ids: Vec<_> = row.iter().map(|p| w.point(p)).collect(); format!("({})", refs_text(&ids))
    }).collect::<Vec<_>>().join(",");
    let (mu, ku) = knot_form(&s.knots_u); let (mv, kv) = knot_form(&s.knots_v);
    w.emit(format!("B_SPLINE_SURFACE_WITH_KNOTS('',{},{},({grid}),.UNSPECIFIED.,.F.,.{pu}.,.{pv}.,.F.,.F.,({}),({}),({}),({}),({}),.UNSPECIFIED.)",
        s.degree_u, s.degree_v, floats(&s.weights.iter().flatten().copied().collect::<Vec<_>>()),
        mu.iter().map(ToString::to_string).collect::<Vec<_>>().join(","),
        mv.iter().map(ToString::to_string).collect::<Vec<_>>().join(","), floats(&ku), floats(&kv),
        pu=if s.periodic_u {"T"} else {"F"}, pv=if s.periodic_v {"T"} else {"F"}))
}

pub fn export_step_v3(model: &Model) -> Result<(String, FeatureCertificate, StepV3Report)> {
    model.validate()?;
    if model.bodies.is_empty() || model.shells.iter().any(|s| !s.closed) { return Err(refuse("STEP /3 exports closed manifold bodies only")); }
    let mut w = Writer::new();
    let mut vertex_entities = Vec::new();
    for v in &model.vertices { let p = w.point(&v.point); vertex_entities.push(w.emit(format!("VERTEX_POINT('',#{p})"))); }
    let surfaces: Vec<_> = model.faces.iter().map(|f| emit_surface(&mut w, &f.surface)).collect();
    let curves3: Vec<_> = model.edges.iter().map(|e| emit_curve(&mut w, &e.curve)).collect();
    let mut pcurves_by_edge = vec![Vec::<usize>::new(); model.edges.len()];
    let mut coedge_pcurve = BTreeMap::new();
    for (li, loop_) in model.loops.iter().enumerate() {
        for (ci, c) in loop_.coedges.iter().enumerate() {
            let face = model.faces.iter().position(|f| f.outer == li || f.holes.contains(&li)).ok_or_else(|| refuse("Loop has no owning face"))?;
            let curve2 = emit_curve(&mut w, &c.pcurve);
            let pc = w.emit(format!("PCURVE('',#{},#{curve2})", surfaces[face]));
            pcurves_by_edge[c.edge].push(pc); coedge_pcurve.insert((li, ci), pc);
        }
    }
    let mut edge_entities = Vec::new();
    for (i, e) in model.edges.iter().enumerate() {
        if e.degenerate { return Err(refuse("Pole/degenerate edges are typed-refused by STEP /3")); }
        let sc = w.emit(format!("SURFACE_CURVE('',#{},({}),.PCURVE_S1.)", curves3[i], refs_text(&pcurves_by_edge[i])));
        edge_entities.push(w.emit(format!("EDGE_CURVE('',#{},#{},#{sc},.T.)", vertex_entities[e.vertices[0]], vertex_entities[e.vertices[1]])));
    }
    let mut loop_entities = Vec::new();
    for loop_ in &model.loops {
        let oriented: Vec<_> = loop_.coedges.iter().map(|c| w.emit(format!("ORIENTED_EDGE('',*,*,#{},.{}.)", edge_entities[c.edge], if c.reversed {"F"} else {"T"}))).collect();
        loop_entities.push(w.emit(format!("EDGE_LOOP('',({}))", refs_text(&oriented))));
    }
    let mut face_reversed=vec![None;model.faces.len()];
    for shell in &model.shells {
        for usage in &shell.faces {
            if face_reversed[usage.face].replace(usage.reversed).is_some_and(|prior|prior!=usage.reversed) {
                return Err(refuse("A shared face has inconsistent shell-use senses"));
            }
        }
    }
    let mut face_entities = Vec::new();
    for (i, f) in model.faces.iter().enumerate() {
        let mut bounds = vec![w.emit(format!("FACE_OUTER_BOUND('',#{},.T.)", loop_entities[f.outer]))];
        bounds.extend(f.holes.iter().map(|h| w.emit(format!("FACE_BOUND('',#{},.T.)", loop_entities[*h]))));
        face_entities.push(w.emit(format!("ADVANCED_FACE('',({}),#{},.{})", refs_text(&bounds), surfaces[i],if face_reversed[i].unwrap_or(false){"F."}else{"T."})));
    }
    let mut shell_entities = Vec::new();
    for s in &model.shells {
        shell_entities.push(w.emit(format!("CLOSED_SHELL('',({}))", refs_text(&s.faces.iter().map(|u| face_entities[u.face]).collect::<Vec<_>>()))));
    }
    let mut body_entities = Vec::new();
    for b in &model.bodies {
        body_entities.push(if b.inner_shells.is_empty() {
            w.emit(format!("MANIFOLD_SOLID_BREP('',#{})", shell_entities[b.outer_shell]))
        } else {
            w.emit(format!("BREP_WITH_VOIDS('',#{},({}))", shell_entities[b.outer_shell], refs_text(&b.inner_shells.iter().map(|s| shell_entities[*s]).collect::<Vec<_>>())))
        });
    }
    w.emit("(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.))".into());
    let mut text = String::from("ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('OpenSCAD Viewer direct B-rep'),'2;1');\nFILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING'));\nENDSEC;\nDATA;\n");
    for (id, row) in &w.rows { text.push_str(&format!("#{id}={row};\n")); }
    text.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
    if text.len() > MAX_OUTPUT_BYTES { return Err(refuse("STEP /3 output exceeds 16 MiB")); }
    let mut parsed = parse(&text)?;
    let groups: [(TopoKind, &[TopoId], &[usize]); 6] = [
        (TopoKind::Vertex, &model.1.vertices, &vertex_entities), (TopoKind::Edge, &model.1.edges, &edge_entities),
        (TopoKind::Loop, &model.1.loops, &loop_entities), (TopoKind::Face, &model.1.faces, &face_entities),
        (TopoKind::Shell, &model.1.shells, &shell_entities), (TopoKind::Body, &model.1.bodies, &body_entities),
    ];
    for (kind, ids, entities_for_kind) in groups {
        for (&topo, &entity_id) in ids.iter().zip(entities_for_kind) {
            let digest = entity_digest(&parsed, entity_id)?;
            let entity = parsed.get_mut(&entity_id).unwrap();
            if let Value::Call(_, args) = &mut entity.value {
                args[0] = Value::String(format!("OSCAD_TOPO/3|{}|{}|{digest}", kind.as_str(), topo));
            }
        }
    }
    fn render(v: &Value) -> String {
        match v {
            Value::String(s) => format!("'{}'", s.replace('\'', "''")), Value::Number(n) => format!("{n:.17e}"),
            Value::Integer(n) => n.to_string(), Value::Ref(id) => format!("#{id}"), Value::Enum(e) => format!(".{e}."),
            Value::Omitted => "$".into(), Value::Derived => "*".into(), Value::List(v) => format!("({})", v.iter().map(render).collect::<Vec<_>>().join(",")),
            Value::Call(n, v) => format!("{n}({})", v.iter().map(render).collect::<Vec<_>>().join(",")),
        }
    }
    text = String::from("ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('OpenSCAD Viewer direct B-rep'),'2;1');\nFILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING'));\nENDSEC;\nDATA;\n");
    for (id, entity) in &parsed { text.push_str(&format!("#{id}={};\n", render(&entity.value))); }
    text.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
    let count = model.vertices.len()+model.edges.len()+model.loops.len()+model.faces.len()+model.shells.len()+model.bodies.len();
    Ok((text, FeatureCertificate { capability: STEP_INTERCHANGE_V3_CAPABILITY, complete: true, notes: vec!["direct_model_export", "no_constructor_recognition", "canonical_local_graph_identity"] },
        StepV3Report { identity: crate::step_interchange::StepIdentityReport { preserved: true, source: "internal-metadata", preserved_count: count, created_count: 0, lost_count: 0 }, ignored_entities: vec![], instance_count: parsed.len(), reachable_count: parsed.len() }))
}

pub fn export_step_v4(model: &Model) -> Result<(String, FeatureCertificate, StepV3Report)> {
    let (text, mut certificate, report) = export_step_v3(model)?;
    certificate.capability = STEP_INTERCHANGE_V4_CAPABILITY;
    certificate.notes.extend(["analytic_periodic_carriers_import", "exact_endpoint_point_selectors"]);
    Ok((text, certificate, report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nurbs_step_solid::freeform_cuboid_solid;

    fn append(target: &mut Model, source: Model) {
        let Model(source, _) = source;
        let (vo, eo, lo, fo, so) = (
            target.vertices.len(), target.edges.len(), target.loops.len(),
            target.faces.len(), target.shells.len(),
        );
        target.0.vertices.extend(source.vertices);
        target.0.edges.extend(source.edges.into_iter().map(|mut e| {
            e.vertices = [e.vertices[0] + vo, e.vertices[1] + vo]; e
        }));
        target.0.loops.extend(source.loops.into_iter().map(|mut l| {
            for c in &mut l.coedges { c.edge += eo; } l
        }));
        target.0.faces.extend(source.faces.into_iter().map(|mut f| {
            f.outer += lo; for h in &mut f.holes { *h += lo; } f
        }));
        target.0.shells.extend(source.shells.into_iter().map(|mut s| {
            for f in &mut s.faces { f.face += fo; } s
        }));
        target.0.bodies.extend(source.bodies.into_iter().map(|mut b| {
            b.outer_shell += so; for s in &mut b.inner_shells { *s += so; } b
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
        assert_eq!((back.vertices.len(), back.edges.len(), back.faces.len(), back.bodies.len()), (8, 12, 6, 1));
        assert_eq!(back.1.faces, model.1.faces);
    }

    #[test]
    fn direct_more_than_32_bodies_and_metadata_free_identity() {
        let mut model = Model::empty(1e-7).unwrap();
        for i in 0..33 {
            append(&mut model, freeform_cuboid_solid([i as f64 * 2., 0., 0.], [i as f64 * 2. + 1., 1., 1.]).unwrap());
        }
        model.rebuild_topology_ids();
        model.validate().unwrap();
        let (text, _, _) = export_step_v3(&model).unwrap();
        let (back, _, report) = import_step_v3(&text).unwrap();
        assert_eq!(back.bodies.len(), 33);
        assert!(report.identity.preserved);

        let stripped = text.lines().map(|line| {
            if let Some(start) = line.find("'OSCAD_TOPO/3|") {
                let end = line[start + 1..].find('\'').unwrap() + start + 1;
                format!("{}''{}", &line[..start], &line[end + 1..])
            } else { line.to_string() }
        }).collect::<Vec<_>>().join("\n");
        let (_, _, external) = import_step_v3(&stripped).unwrap();
        assert!(!external.identity.preserved);
        assert!(external.identity.created_count > 32);
    }

    #[test]
    fn entity_reorder_preserves_digest_bound_identity() {
        let model = freeform_cuboid_solid([0., 0., 0.], [1., 2., 3.]).unwrap();
        let (text, _, _) = export_step_v3(&model).unwrap();
        let mut rows = text.lines().filter(|l| l.starts_with('#')).collect::<Vec<_>>();
        rows.reverse();
        let reordered = text.lines().filter(|l| !l.starts_with('#'))
            .chain(rows).collect::<Vec<_>>().join("\n");
        let (back, _, report) = import_step_v3(&reordered).unwrap();
        assert!(report.identity.preserved);
        assert_eq!(back.1.edges, model.1.edges);
    }

    #[test]
    fn multiple_cavities_shared_edges_senses_and_metadata_mutation() {
        let mut model = Model::empty(1e-7).unwrap();
        append(&mut model, freeform_cuboid_solid([0.,0.,0.],[10.,10.,10.]).unwrap());
        append(&mut model, freeform_cuboid_solid([1.,1.,1.],[3.,3.,3.]).unwrap());
        append(&mut model, freeform_cuboid_solid([6.,6.,6.],[8.,8.,8.]).unwrap());
        model.0.bodies = vec![Body { outer_shell: 0, inner_shells: vec![1,2] }];
        model.rebuild_topology_ids();
        model.validate().unwrap();
        let (text,_,_)=export_step_v3(&model).unwrap();
        let (back,_,_)=import_step_v3(&text).unwrap();
        assert_eq!(back.bodies[0].inner_shells.len(),2);
        assert_eq!((back.vertices.len(),back.edges.len(),back.loops.len(),back.faces.len(),back.shells.len(),back.bodies.len()),(24,36,18,18,3,1));
        let uses=back.loops.iter().flat_map(|l|&l.coedges).fold(BTreeMap::new(),|mut m,c|{*m.entry(c.edge).or_insert(0usize)+=1;m});
        assert!(uses.values().all(|count|*count==2));
        assert_eq!(back.loops.iter().flat_map(|l|&l.coedges).filter(|c|c.reversed).count(),
            model.loops.iter().flat_map(|l|&l.coedges).filter(|c|c.reversed).count());

        let mutated=text.replacen("0.00000000000000000e0","1.00000000000000000e-1",1);
        let error=import_step_v3(&mutated).unwrap_err();
        assert!(error.message.contains("identity metadata"));
        let partial=text.replacen("OSCAD_TOPO/3|","REMOVED_TOPO/3|",1);
        assert!(import_step_v3(&partial).unwrap_err().message.contains("Partial"));
    }

    #[test]
    fn lexer_accepts_comments_case_escapes_d_exponents_and_forward_refs() {
        let model = freeform_cuboid_solid([0., 0., 0.], [1., 1., 1.]).unwrap();
        let (text, _, _) = export_step_v3(&model).unwrap();
        let text = text.replace("DATA;", "DATA;/* 'ignored; text' */")
            .replace("CARTESIAN_POINT", "cartesian_point")
            .replace("0.00000000000000000e0", "0.0D0")
            .replace("OpenSCAD Viewer", "OpenSCAD ''Viewer''");
        import_step_v3(&text).unwrap();
    }

    #[test]
    fn refuses_duplicate_missing_cycle_depth_and_unknown_reachable_geometry() {
        assert!(parse("#1=CARTESIAN_POINT('',(0.,0.,0.));#1=CARTESIAN_POINT('',(0.,0.,0.));").is_err());
        let entities = parse("#1=PRODUCT('',#2);#2=PRODUCT('',#1);").unwrap();
        assert!(reachable(&entities, &[1]).is_err());
        let model = freeform_cuboid_solid([0., 0., 0.], [1., 1., 1.]).unwrap();
        let (text, _, _) = export_step_v3(&model).unwrap();
        assert!(import_step_v3(&text.replacen("B_SPLINE_SURFACE_WITH_KNOTS", "PLANE", 1)).is_err());
        assert!(import_step_v3(&text.replace("#1=CARTESIAN_POINT", "#1=CARTESIAN_POINT('',#999);#65000=CARTESIAN_POINT")).is_err());
    }

    #[test]
    fn parses_self_authored_analytic_and_conversion_fixtures() {
        let tetra = include_str!("../../../tests/fixtures/step-v3/self-authored-analytic-tetra.step");
        let (model, _, report) = import_step_v3(tetra).unwrap();
        assert_eq!((model.vertices.len(), model.edges.len(), model.loops.len(), model.faces.len(), model.shells.len(), model.bodies.len()), (4, 6, 4, 4, 1, 1));
        assert!(!report.identity.preserved);
        let (roundtrip,_,_)=export_step_v3(&model).unwrap();
        let (roundtrip,_,_)=import_step_v3(&roundtrip).unwrap();
        assert_eq!(roundtrip.shells[0].faces.iter().map(|f|f.reversed).collect::<Vec<_>>(),
            model.shells[0].faces.iter().map(|f|f.reversed).collect::<Vec<_>>());

        let analytic = include_str!("../../../tests/fixtures/step-v3/self-authored-analytic-units.step");
        let entities = parse(analytic).unwrap();
        let arc = curve(&entities, 20, 3, 1.).unwrap();
        let ellipse = curve(&entities, 21, 3, 1.).unwrap();
        assert_eq!(arc.degree, 2);
        assert_eq!(ellipse.control_points.len(), 9);
        assert_eq!(length_scale(&entities, &entities.keys().copied().collect()).unwrap(), 1.);

        let inch = include_str!("../../../tests/fixtures/step-v3/self-authored-inch-chain.stp");
        let entities = parse(inch).unwrap();
        assert!((length_scale(&entities, &entities.keys().copied().collect()).unwrap() - 25.4).abs() < 1e-12);

        let cyclic = include_str!("../../../tests/fixtures/step-v3/self-authored-malformed-cycle.stp");
        let entities = parse(cyclic).unwrap();
        assert!(reachable(&entities, &[1]).is_err());
    }

    #[test]
    fn periodic_analytic_surfaces_have_specific_typed_refusals() {
        for ty in ["CYLINDRICAL_SURFACE", "CONICAL_SURFACE", "SPHERICAL_SURFACE", "TOROIDAL_SURFACE"] {
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
            let s=surface_v4(&entities,id,1.).unwrap();
            assert!((s.knots_u[s.knots_u.len()-s.degree_u-1]-std::f64::consts::TAU).abs()<1e-12);
            s.validate().unwrap();
        }
        let c=curve(&entities,14,3,1.).unwrap();
        assert_eq!(c.control_points[0],vec![1.,0.,0.]);
        assert_eq!(c.control_points[1],vec![2.,0.,0.]);
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
        assert!(report.ignored_entities.contains(&"ITEM_DEFINED_TRANSFORMATION".into()));
    }

    #[test]
    fn only_representation_reachable_rigid_transform_moves_geometry() {
        let model = freeform_cuboid_solid([0., 0., 0.], [1., 1., 1.]).unwrap();
        let (text, _, _) = export_step_v3(&model).unwrap();
        let entities=parse(&text).unwrap();
        let body=entities.iter().find_map(|(id,_)|call(&entities,*id).ok().and_then(|(ty,_)|(ty=="MANIFOLD_SOLID_BREP").then_some(*id))).unwrap();
        let rows = format!("\
#60000=CARTESIAN_POINT('',(0.,0.,0.));
#60001=CARTESIAN_POINT('',(9.,8.,7.));
#60002=DIRECTION('',(0.,0.,1.));
#60003=DIRECTION('',(1.,0.,0.));
#60004=AXIS2_PLACEMENT_3D('',#60000,#60002,#60003);
#60005=AXIS2_PLACEMENT_3D('',#60001,#60002,#60003);
#60006=ITEM_DEFINED_TRANSFORMATION('reachable','',#60004,#60005);
#60007=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#{body},#60006),#60008);
#60008=GEOMETRIC_REPRESENTATION_CONTEXT(3);
");
        let placed=text.replacen("ENDSEC;\nEND-ISO",&format!("{rows}ENDSEC;\nEND-ISO"),1);
        let (back,_,_)=import_step_v3(&placed).unwrap();
        for i in 0..3 { assert!((back.vertices[0].point[i]-model.vertices[0].point[i]-[9.,8.,7.][i]).abs()<1e-9); }

        let nonrigid=placed.replace("ITEM_DEFINED_TRANSFORMATION('reachable','',#60004,#60005)",
            "CARTESIAN_TRANSFORMATION_OPERATOR_3D('',#60002,#60003,#60000,2.,$)");
        assert!(import_step_v3(&nonrigid).unwrap_err().message.contains("nonuniform"));
    }

    #[test]
    fn aggregate_parser_and_graph_budgets_refuse() {
        assert!(lex(&" ".repeat(MAX_BYTES + 1)).unwrap_err().message.contains("16 MiB"));
        let nested=format!("#1=PRODUCT('',{});","(".repeat(MAX_PARSE_DEPTH+1))+&")".repeat(MAX_PARSE_DEPTH+1);
        assert!(parse(&nested).unwrap_err().message.contains("depth"));
        let mut chain=String::new();
        for i in 1..=MAX_GRAPH_DEPTH+2 {
            if i==MAX_GRAPH_DEPTH+2 { chain.push_str(&format!("#{i}=PRODUCT('end');")); }
            else { chain.push_str(&format!("#{i}=PRODUCT('',#{});",i+1)); }
        }
        let entities=parse(&chain).unwrap();
        assert!(reachable(&entities,&[1]).unwrap_err().message.contains("depth"));
    }
}
