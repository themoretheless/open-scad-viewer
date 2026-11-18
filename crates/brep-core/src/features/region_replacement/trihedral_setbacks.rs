//! Reference-plane end layout for three independently radiused convex edges.
//! Plane approximation, prescribed radius/G2 and a constructed fillet differ.
use super::{EdgeRadius, plan_radius_network};
use crate::{Model, TopoId};
use nurbs_core::{Error, Result, interval_eval::Interval as I};
use std::collections::BTreeMap;
type V = [I; 3];
#[derive(Debug)]
pub struct FacePort {
    pub face: usize,
    pub identity: TopoId,
    pub edges: [usize; 2],
    pub offsets_mm: [[f64; 2]; 2],
    pub feet_mm: [[f64; 2]; 2],
    pub position_mm: [[f64; 2]; 3],
    pub plane_error_upper_mm: f64,
}
#[derive(Debug)]
pub struct EdgeSetback {
    pub edge: usize,
    pub identity: TopoId,
    pub radius_mm: f64,
    pub contact_feet_mm: [f64; 2],
    pub distance_mm: f64,
}
pub struct TrihedralSetbacks {
    pub vertex: usize,
    pub identity: TopoId,
    pub ports: Vec<FacePort>,
    pub edges: Vec<EdgeSetback>,
    pub work: usize,
}
fn refuse(s: &str) -> Error {
    Error::new("BREP_TRIHEDRAL_SETBACK_UNPROVEN", s)
}
fn vector(p: [f64; 3]) -> V {
    p.map(I::point)
}
fn sub(a: V, b: V) -> Result<V> {
    Ok([a[0].sub(b[0])?, a[1].sub(b[1])?, a[2].sub(b[2])?])
}
fn dot(a: V, b: V) -> Result<I> {
    a[0].mul(b[0])?.add(a[1].mul(b[1])?)?.add(a[2].mul(b[2])?)
}
fn cross(a: V, b: V) -> Result<V> {
    Ok([
        a[1].mul(b[2])?.sub(a[2].mul(b[1])?)?,
        a[2].mul(b[0])?.sub(a[0].mul(b[2])?)?,
        a[0].mul(b[1])?.sub(a[1].mul(b[0])?)?,
    ])
}
fn sqrt(a: I) -> Result<I> {
    if a.lo < 0. {
        return Err(refuse("Unresolved squared-length sign"));
    }
    I::new(a.lo.sqrt().next_down().max(0.), a.hi.sqrt().next_up())
}
fn unit(a: V) -> Result<(V, I)> {
    let n = sqrt(dot(a, a)?)?;
    Ok(([a[0].div(n)?, a[1].div(n)?, a[2].div(n)?], n))
}
fn bounds(a: I) -> [f64; 2] {
    [a.lo, a.hi]
}
fn abs(a: I) -> f64 {
    a.lo.abs().max(a.hi.abs())
}
struct Budget {
    used: usize,
    max: usize,
}
impl Budget {
    fn visit(&mut self) -> Result<()> {
        if self.used == self.max {
            return Err(refuse("Setback geometry work exhausted"));
        }
        self.used += 1;
        Ok(())
    }
}
struct Plane {
    normal: V,
    error: f64,
}
fn plane(
    source: &Model,
    shell: usize,
    face: usize,
    origin: V,
    budget: &mut Budget,
) -> Result<Plane> {
    let f = &source.faces[face];
    if !f.holes.is_empty() {
        return Err(refuse(
            "Corner support holes need separate membership proof",
        ));
    }
    let mut area = [I::point(0.); 3];
    for c in &source.loops[f.outer].coedges {
        budget.visit()?;
        let e = &source.edges[c.edge];
        let a = sub(
            vector(source.vertices[e.vertices[usize::from(c.reversed)]].point),
            origin,
        )?;
        let b = sub(
            vector(source.vertices[e.vertices[usize::from(!c.reversed)]].point),
            origin,
        )?;
        let term = cross(a, b)?;
        for k in 0..3 {
            area[k] = area[k].add(term[k])?;
        }
    }
    let reversed = source.shells[shell]
        .faces
        .iter()
        .find(|u| u.face == face)
        .ok_or_else(|| refuse("Missing original face use"))?
        .reversed;
    if reversed {
        for v in &mut area {
            *v = I::point(0.).sub(*v)?;
        }
    }
    let (normal, _) = unit(area)?;
    let mut error = 0_f64;
    for p in f.surface.control_points.iter().flatten() {
        budget.visit()?;
        error = error.max(abs(dot(normal, sub(vector([p[0], p[1], p[2]]), origin)?)?));
    }
    if error > source.tolerance_mm {
        return Err(refuse("Support exceeds the unchanged planar tolerance"));
    }
    Ok(Plane { normal, error })
}
/// A planar wedge's tangent offset is r*sqrt((1-c)/(1+c)) for outward
/// normal dot product c. Bounds cover reference-plane geometry, not actual
/// curved-surface contacts. A common target lies beyond both face feet plus
/// explicit runout. No source cut or fillet surface is authored by this plan.
pub fn plan_trihedral_setbacks(
    source: &Model,
    shell: usize,
    vertex: usize,
    requested: &[EdgeRadius],
    runout_mm: f64,
    max_work: usize,
) -> Result<TrihedralSetbacks> {
    if !(1..=1000000).contains(&max_work) || !runout_mm.is_finite() || runout_mm <= 0. {
        return Err(refuse("Need bounded work and positive explicit runout"));
    }
    let network = plan_radius_network(source, shell, requested, max_work)?;
    if source
        .bodies
        .iter()
        .filter(|b| b.outer_shell == shell)
        .count()
        != 1
        || source
            .bodies
            .iter()
            .any(|b| b.inner_shells.contains(&shell))
    {
        return Err(refuse(
            "Need one uniquely owned outer shell; cavity material roles are unqualified",
        ));
    }
    let corner = network
        .corners
        .iter()
        .find(|c| c.vertex == vertex)
        .ok_or_else(|| refuse("Missing selected corner"))?;
    if corner.incident.len() != 3 || corner.selected.len() != 3 || network.edges.len() != 3 {
        return Err(refuse("Need all three edges of a trihedral corner"));
    }
    let mut budget = Budget {
        used: network.topology_visits,
        max: max_work,
    };
    let origin = vector(source.vertices[vertex].point);
    let mut rays = BTreeMap::new();
    let mut lengths = BTreeMap::new();
    let mut faces = BTreeMap::<usize, Vec<usize>>::new();
    for e in &network.edges {
        budget.visit()?;
        let edge = &source.edges[e.edge];
        if edge.curve.degree != 1
            || edge.curve.control_points.len() != 2
            || edge.curve.weights[0] != edge.curve.weights[1]
        {
            return Err(refuse(
                "Reference-plane rays need affine straight original edges",
            ));
        }
        let other = *edge.vertices.iter().find(|&&v| v != vertex).unwrap();
        let (ray, len) = unit(sub(vector(source.vertices[other].point), origin)?)?;
        rays.insert(e.edge, ray);
        lengths.insert(e.edge, len);
        for f in e.supports {
            faces.entry(f).or_default().push(e.edge);
        }
    }
    if faces.len() != 3 || faces.values().any(|v| v.len() != 2) {
        return Err(refuse("Support incidence is not trihedral"));
    }
    let mut planes = BTreeMap::new();
    for f in faces.keys() {
        planes.insert(*f, plane(source, shell, *f, origin, &mut budget)?);
    }
    for e in &network.edges {
        let third = *faces.keys().find(|f| !e.supports.contains(f)).unwrap();
        if dot(planes[&third].normal, rays[&e.edge])?.hi >= 0. {
            return Err(refuse("Corner is not proved locally convex"));
        }
    }
    let mut offsets = BTreeMap::new();
    for e in &network.edges {
        let c = dot(planes[&e.supports[0]].normal, planes[&e.supports[1]].normal)?
            .intersect(-1., 1.)?;
        let ratio = I::point(1.).sub(c)?.div(I::point(1.).add(c)?)?;
        offsets.insert(e.edge, I::point(e.radius_mm).mul(sqrt(ratio)?)?);
    }
    let mut feet = BTreeMap::<usize, Vec<I>>::new();
    let mut ports = Vec::new();
    for (f, es) in faces {
        budget.visit()?;
        let edges = [es[0], es[1]];
        let u = rays[&edges[0]];
        let v = rays[&edges[1]];
        let c = dot(u, v)?.intersect(-1., 1.)?;
        let sin = sqrt(I::point(1.).sub(c.mul(c)?)?)?;
        let du = offsets[&edges[0]];
        let dv = offsets[&edges[1]];
        let a = dv.div(sin)?;
        let b = du.div(sin)?;
        let along = [a.add(b.mul(c)?)?, b.add(a.mul(c)?)?];
        if along.iter().any(|v| v.lo <= 0.) {
            return Err(refuse("Contact crossing leaves a forward ray"));
        }
        let mut position = origin;
        for k in 0..3 {
            position[k] = origin[k].add(a.mul(u[k])?)?.add(b.mul(v[k])?)?;
        }
        for k in 0..2 {
            feet.entry(edges[k]).or_default().push(along[k]);
        }
        ports.push(FacePort {
            face: f,
            identity: source.1.faces[f],
            edges,
            offsets_mm: [bounds(du), bounds(dv)],
            feet_mm: along.map(bounds),
            position_mm: position.map(bounds),
            plane_error_upper_mm: planes[&f].error,
        });
    }
    let mut edges = Vec::new();
    for e in &network.edges {
        let feet = &feet[&e.edge];
        let maximum = I::new(
            feet.iter().map(|v| v.lo).fold(f64::NEG_INFINITY, f64::max),
            feet.iter().map(|v| v.hi).fold(f64::NEG_INFINITY, f64::max),
        )?;
        let distance = maximum.add(I::point(runout_mm))?.hi;
        if !distance.is_finite() || distance >= lengths[&e.edge].lo {
            return Err(refuse("Setback reaches another original vertex"));
        }
        edges.push(EdgeSetback {
            edge: e.edge,
            identity: e.identity,
            radius_mm: e.radius_mm,
            contact_feet_mm: bounds(maximum),
            distance_mm: distance,
        });
    }
    Ok(TrihedralSetbacks {
        vertex,
        identity: source.1.vertices[vertex],
        ports,
        edges,
        work: budget.used,
    })
}
