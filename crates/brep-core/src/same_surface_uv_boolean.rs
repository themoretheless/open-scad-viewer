//! Deterministic regularized Boolean operations on exact linear UV trims.
//!
//! This is a two-dimensional retained-region kernel, not a sheet/solid CSG
//! shortcut. Every input boundary segment is split at every arrangement
//! intersection. Output edges are admitted only when the operation has
//! opposite regularized material membership on their two sides. Nonlinear
//! pcurves, non-finite data and exhausted budgets are rejected before a result
//! is constructed. The certificate replays the complete operation.
use crate::boolean_robustness::{surface_uv_domains, UvEquivalence};
use crate::{Error, Face, Result, Surface};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UvBooleanOperation {
    Union,
    Intersection,
    Difference,
    Xor,
}

impl UvBooleanOperation {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "union" => Ok(Self::Union),
            "intersection" => Ok(Self::Intersection),
            "difference" => Ok(Self::Difference),
            "xor" => Ok(Self::Xor),
            _ => Err(refused(
                "UV operation must be union, intersection, difference or xor",
            )),
        }
    }
    fn apply(self, a: bool, b: bool) -> bool {
        match self {
            Self::Union => a || b,
            Self::Intersection => a && b,
            Self::Difference => a && !b,
            Self::Xor => a != b,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UvRegion {
    pub outer: Vec<[f64; 2]>,
    pub holes: Vec<Vec<[f64; 2]>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SameSurfaceUvBooleanCertificate {
    pub version: u32,
    pub operation: UvBooleanOperation,
    pub first: Vec<UvRegion>,
    pub second: Vec<UvRegion>,
    pub equivalence: Option<UvEquivalence>,
    pub limits: SameSurfaceUvLimits,
    pub work: u64,
    pub result: Vec<UvRegion>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SameSurfaceUvBoolean {
    regions: Vec<UvRegion>,
    certificate: SameSurfaceUvBooleanCertificate,
}

impl SameSurfaceUvBoolean {
    pub fn regions(&self) -> &[UvRegion] {
        &self.regions
    }
    pub fn certificate(&self) -> &SameSurfaceUvBooleanCertificate {
        &self.certificate
    }
    pub fn into_regions(self) -> Vec<UvRegion> {
        self.regions
    }
    /// Rehydrate a serialized result; call the verifier before trusting it.
    pub fn from_parts(
        regions: Vec<UvRegion>,
        certificate: SameSurfaceUvBooleanCertificate,
    ) -> Self {
        Self {
            regions,
            certificate,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SameSurfaceUvLimits {
    pub tolerance_uv: f64,
    pub max_segments: usize,
    pub max_work: u64,
}

impl Default for SameSurfaceUvLimits {
    fn default() -> Self {
        Self {
            tolerance_uv: 1e-9,
            max_segments: 2048,
            max_work: 2_000_000,
        }
    }
}

fn refused(message: impl Into<String>) -> Error {
    Error::new("BREP_SAME_SURFACE_UV_UNPROVEN", message)
}

#[derive(Clone, Copy, Debug)]
struct Segment {
    a: [f64; 2],
    b: [f64; 2],
}

fn cross(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
fn sub(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}
fn dot(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
fn length(a: [f64; 2]) -> f64 {
    dot(a, a).sqrt()
}
fn area(loop_: &[[f64; 2]]) -> f64 {
    loop_
        .iter()
        .zip(loop_.iter().cycle().skip(1))
        .map(|(a, b)| cross(*a, *b))
        .sum::<f64>()
        / 2.
}

fn clean_loop(points: Vec<[f64; 2]>, tolerance: f64) -> Result<Vec<[f64; 2]>> {
    let mut out: Vec<[f64; 2]> = Vec::new();
    for p in points {
        if p.iter().any(|x| !x.is_finite()) {
            return Err(refused("Non-finite UV trim"));
        }
        if out.last().is_none_or(|q| length(sub(p, *q)) > tolerance) {
            out.push(p);
        }
    }
    if out.len() > 1 && length(sub(out[0], *out.last().unwrap())) <= tolerance {
        out.pop();
    }
    if out.len() < 3 || area(&out).abs() <= tolerance * tolerance {
        return Err(refused("Degenerate UV trim loop"));
    }
    Ok(out)
}

fn face_regions(model: &crate::Model, face: &Face) -> Result<Vec<UvRegion>> {
    let mut loops = Vec::new();
    for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
        let mut points = Vec::new();
        for coedge in &model.loops[wire].coedges {
            let p = &coedge.pcurve;
            p.validate()?;
            if p.degree != 1 || p.control_points.len() != 2 || p.weights.iter().any(|w| *w != 1.) {
                return Err(refused(
                    "Only exact degree-one two-pole UV trims belong to this kernel",
                ));
            }
            // UV pcurves are stored in traversal order. Coedge::reversed belongs
            // to the independent 3D edge and must not reverse this UV chain again.
            points.push(
                <[f64; 2]>::try_from(p.control_points[0].as_slice())
                    .map_err(|_| refused("UV pcurve is not two-dimensional"))?,
            );
        }
        loops.push(clean_loop(points, 0.)?);
    }
    if area(&loops[0]) <= 0. || loops.iter().skip(1).any(|l| area(l) >= 0.) {
        return Err(refused("UV outer/hole winding is not material-left"));
    }
    Ok(vec![UvRegion {
        outer: loops.remove(0),
        holes: loops,
    }])
}

fn transform_regions(
    regions: &mut [UvRegion],
    map: UvEquivalence,
    second: &Surface,
    first: &Surface,
) {
    for region in regions {
        for p in std::iter::once(&mut region.outer)
            .chain(region.holes.iter_mut())
            .flatten()
        {
            *p = map.second_to_first(second, first, *p);
        }
        if map.transpose ^ map.reverse_u ^ map.reverse_v {
            for loop_ in std::iter::once(&mut region.outer).chain(region.holes.iter_mut()) {
                loop_.reverse();
            }
        }
    }
}

fn point_on_segment(p: [f64; 2], s: Segment, tolerance: f64) -> bool {
    let d = sub(s.b, s.a);
    let scale = length(d).max(1.);
    cross(sub(p, s.a), d).abs() <= tolerance * scale
        && dot(sub(p, s.a), sub(p, s.b)) <= tolerance * scale
        && dot(sub(p, s.a), d) >= -tolerance * scale
}

fn point_in_loop(loop_: &[[f64; 2]], p: [f64; 2]) -> bool {
    let mut winding = 0i32;
    for (&a, &b) in loop_.iter().zip(loop_.iter().cycle().skip(1)) {
        let side = cross(sub(b, a), sub(p, a));
        if a[1] <= p[1] {
            if b[1] > p[1] && side > 0. {
                winding += 1;
            }
        } else if b[1] <= p[1] && side < 0. {
            winding -= 1;
        }
    }
    winding != 0
}

fn inside(regions: &[UvRegion], p: [f64; 2]) -> bool {
    regions
        .iter()
        .any(|r| point_in_loop(&r.outer, p) && !r.holes.iter().any(|h| point_in_loop(h, p)))
}

fn on_boundary(regions: &[UvRegion], p: [f64; 2], tolerance: f64) -> bool {
    regions.iter().any(|r| {
        std::iter::once(&r.outer).chain(&r.holes).any(|l| {
            l.iter()
                .zip(l.iter().cycle().skip(1))
                .any(|(&a, &b)| point_on_segment(p, Segment { a, b }, tolerance))
        })
    })
}

fn membership(
    regions: &[UvRegion],
    p: [f64; 2],
    normal: [f64; 2],
    tolerance: f64,
) -> Result<(bool, bool)> {
    let left = [
        p[0] + normal[0] * tolerance * 4.,
        p[1] + normal[1] * tolerance * 4.,
    ];
    let right = [
        p[0] - normal[0] * tolerance * 4.,
        p[1] - normal[1] * tolerance * 4.,
    ];
    if on_boundary(regions, left, tolerance) || on_boundary(regions, right, tolerance) {
        return Err(refused(
            "UV offset membership probe intersects another trim",
        ));
    }
    Ok((inside(regions, left), inside(regions, right)))
}

fn key(p: [f64; 2], tolerance: f64) -> Result<(i64, i64)> {
    if p.iter().any(|x| !x.is_finite()) {
        return Err(refused("Non-finite arrangement coordinate"));
    }
    let q = |x: f64| {
        let y = x / tolerance;
        if !y.is_finite() || y.abs() > i64::MAX as f64 {
            Err(refused("UV coordinate exceeds deterministic key range"))
        } else {
            Ok(y.round() as i64)
        }
    };
    Ok((q(p[0])?, q(p[1])?))
}

fn snap(p: [f64; 2], tolerance: f64) -> [f64; 2] {
    [
        (p[0] / tolerance).round() * tolerance,
        (p[1] / tolerance).round() * tolerance,
    ]
}

fn split_segments(input: &[Segment], limits: SameSurfaceUvLimits) -> Result<(Vec<Segment>, u64)> {
    let mut cuts = vec![vec![0., 1.]; input.len()];
    let mut work = 0u64;
    for i in 0..input.len() {
        for j in i + 1..input.len() {
            work += 1;
            if work > limits.max_work {
                return Err(Error::new(
                    "BREP_RESOURCE_LIMIT",
                    "UV arrangement intersection budget exhausted",
                ));
            }
            let a = input[i];
            let b = input[j];
            let d = sub(a.b, a.a);
            let e = sub(b.b, b.a);
            let denominator = cross(d, e);
            let delta = sub(b.a, a.a);
            if denominator.abs() > limits.tolerance_uv {
                let t = cross(delta, e) / denominator;
                let u = cross(delta, d) / denominator;
                if t >= -limits.tolerance_uv
                    && t <= 1. + limits.tolerance_uv
                    && u >= -limits.tolerance_uv
                    && u <= 1. + limits.tolerance_uv
                {
                    cuts[i].push(t.clamp(0., 1.));
                    cuts[j].push(u.clamp(0., 1.));
                }
            } else if cross(delta, d).abs() <= limits.tolerance_uv * length(d).max(1.) {
                let dd = dot(d, d);
                let ee = dot(e, e);
                if dd <= limits.tolerance_uv * limits.tolerance_uv
                    || ee <= limits.tolerance_uv * limits.tolerance_uv
                {
                    continue;
                }
                for t in [dot(sub(b.a, a.a), d) / dd, dot(sub(b.b, a.a), d) / dd] {
                    if t > 0. && t < 1. {
                        cuts[i].push(t);
                    }
                }
                for u in [dot(sub(a.a, b.a), e) / ee, dot(sub(a.b, b.a), e) / ee] {
                    if u > 0. && u < 1. {
                        cuts[j].push(u);
                    }
                }
            }
        }
    }
    let mut out = Vec::new();
    for (s, mut ts) in input.iter().copied().zip(cuts) {
        ts.sort_by(f64::total_cmp);
        ts.dedup_by(|a, b| (*a - *b).abs() <= limits.tolerance_uv);
        for pair in ts.windows(2) {
            let a = [
                s.a[0] + (s.b[0] - s.a[0]) * pair[0],
                s.a[1] + (s.b[1] - s.a[1]) * pair[0],
            ];
            let b = [
                s.a[0] + (s.b[0] - s.a[0]) * pair[1],
                s.a[1] + (s.b[1] - s.a[1]) * pair[1],
            ];
            let a = snap(a, limits.tolerance_uv);
            let b = snap(b, limits.tolerance_uv);
            if length(sub(b, a)) > limits.tolerance_uv {
                out.push(Segment { a, b });
            }
            if out.len() > limits.max_segments {
                return Err(Error::new(
                    "BREP_RESOURCE_LIMIT",
                    "UV atomic segment budget exhausted",
                ));
            }
        }
    }
    Ok((out, work))
}

fn arrangement(
    a: &[UvRegion],
    b: &[UvRegion],
    operation: UvBooleanOperation,
    limits: SameSurfaceUvLimits,
) -> Result<(Vec<UvRegion>, u64)> {
    let segments = a
        .iter()
        .chain(b)
        .flat_map(|r| std::iter::once(&r.outer).chain(&r.holes))
        .flat_map(|l| {
            l.iter()
                .zip(l.iter().cycle().skip(1))
                .map(|(&a, &b)| Segment { a, b })
        })
        .collect::<Vec<_>>();
    if segments.len() > limits.max_segments {
        return Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "UV input segment budget exhausted",
        ));
    }
    let (atoms, pair_work) = split_segments(&segments, limits)?;
    let atom_count = atoms.len() as u64;
    let mut unique = BTreeMap::<(i64, i64, i64, i64), Segment>::new();
    for s in atoms {
        let ka = key(s.a, limits.tolerance_uv)?;
        let kb = key(s.b, limits.tolerance_uv)?;
        let k = if ka <= kb {
            (ka.0, ka.1, kb.0, kb.1)
        } else {
            (kb.0, kb.1, ka.0, ka.1)
        };
        unique.entry(k).or_insert(s);
    }
    let mut directed = Vec::new();
    for s in unique.into_values() {
        let d = sub(s.b, s.a);
        let l = length(d);
        let n = [-d[1] / l, d[0] / l];
        let m = [(s.a[0] + s.b[0]) / 2., (s.a[1] + s.b[1]) / 2.];
        let (al, ar) = membership(a, m, n, limits.tolerance_uv)?;
        let (bl, br) = membership(b, m, n, limits.tolerance_uv)?;
        let left = operation.apply(al, bl);
        let right = operation.apply(ar, br);
        if left != right {
            directed.push(if left { s } else { Segment { a: s.b, b: s.a } });
        }
    }
    let work = pair_work
        .checked_add(segments.len() as u64)
        .and_then(|w| w.checked_add(atom_count))
        .and_then(|w| w.checked_add(directed.len() as u64))
        .ok_or_else(|| Error::new("BREP_RESOURCE_LIMIT", "UV work counter overflow"))?;
    if work > limits.max_work {
        return Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "UV total work budget exhausted",
        ));
    }
    Ok((stitch(directed, limits)?, work))
}

fn stitch(edges: Vec<Segment>, limits: SameSurfaceUvLimits) -> Result<Vec<UvRegion>> {
    let mut outgoing = BTreeMap::<(i64, i64), Vec<usize>>::new();
    let endpoints = edges
        .iter()
        .map(|s| {
            Ok((
                key(s.a, limits.tolerance_uv)?,
                key(s.b, limits.tolerance_uv)?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    for (i, (a, _)) in endpoints.iter().enumerate() {
        outgoing.entry(*a).or_default().push(i);
    }
    for list in outgoing.values_mut() {
        list.sort_by(|&i, &j| {
            let ai = (edges[i].b[1] - edges[i].a[1]).atan2(edges[i].b[0] - edges[i].a[0]);
            let aj = (edges[j].b[1] - edges[j].a[1]).atan2(edges[j].b[0] - edges[j].a[0]);
            ai.total_cmp(&aj)
        });
    }
    let positions = outgoing
        .iter()
        .map(|(v, list)| {
            (
                *v,
                list.iter()
                    .enumerate()
                    .map(|(p, e)| (*e, p))
                    .collect::<BTreeMap<_, _>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut used = BTreeSet::new();
    let mut loops = Vec::new();
    for start in 0..edges.len() {
        if used.contains(&start) {
            continue;
        }
        let mut current = start;
        let mut points = Vec::new();
        loop {
            if !used.insert(current) {
                return Err(refused("UV output walk is not a simple cycle"));
            }
            points.push(snap(edges[current].a, limits.tolerance_uv));
            let end = endpoints[current].1;
            let list = &outgoing[&end];
            let twin = list.iter().copied().find(|&candidate| {
                endpoints[candidate].1 == endpoints[current].0
                    && key(edges[candidate].a, limits.tolerance_uv) == Ok(end)
                    && key(edges[candidate].b, limits.tolerance_uv) == Ok(endpoints[current].0)
            });
            let next = if let Some(reverse) = twin {
                let p = positions[&end][&reverse];
                list[(p + list.len() - 1) % list.len()]
            } else {
                // At a manifold output vertex, choose the clockwise-most
                // outgoing relative to the incoming direction.
                let incoming = (edges[current].a[1] - edges[current].b[1])
                    .atan2(edges[current].a[0] - edges[current].b[0]);
                list.iter()
                    .copied()
                    .min_by(|&i, &j| {
                        let turn = |k: usize| {
                            let out = (edges[k].b[1] - edges[k].a[1])
                                .atan2(edges[k].b[0] - edges[k].a[0]);
                            (incoming - out).rem_euclid(std::f64::consts::TAU)
                        };
                        turn(i).total_cmp(&turn(j))
                    })
                    .unwrap()
            };
            if next == start {
                break;
            }
            if used.contains(&next) || points.len() > limits.max_segments {
                return Err(refused("UV output topology branches or is open"));
            }
            current = next;
        }
        loops.push(clean_loop(points, limits.tolerance_uv)?);
    }
    let mut outers = Vec::new();
    let mut holes = Vec::new();
    for l in loops {
        if area(&l) > 0. {
            outers.push(l)
        } else {
            holes.push(l)
        }
    }
    let mut regions = outers
        .into_iter()
        .map(|outer| UvRegion {
            outer,
            holes: Vec::new(),
        })
        .collect::<Vec<_>>();
    for hole in holes {
        let p = hole[0];
        let mut candidates = regions
            .iter()
            .enumerate()
            .filter(|(_, r)| point_in_loop(&r.outer, p))
            .map(|(i, r)| (i, area(&r.outer).abs()))
            .collect::<Vec<_>>();
        candidates.sort_by(|a, b| a.1.total_cmp(&b.1));
        let Some((index, _)) = candidates.first() else {
            return Err(refused("UV output hole has no containing outer"));
        };
        regions[*index].holes.push(hole);
    }
    regions.sort_by(|a, b| {
        key(a.outer[0], limits.tolerance_uv)
            .unwrap()
            .cmp(&key(b.outer[0], limits.tolerance_uv).unwrap())
    });
    for r in &mut regions {
        r.holes.sort_by(|a, b| {
            key(a[0], limits.tolerance_uv)
                .unwrap()
                .cmp(&key(b[0], limits.tolerance_uv).unwrap())
        });
    }
    Ok(regions)
}

fn unwrap_periodic_regions(
    regions: &mut [UvRegion],
    surface: &Surface,
    tolerance: f64,
) -> Result<()> {
    let domains = surface_uv_domains(surface);
    let periodic = [surface.periodic_u, surface.periodic_v];
    let period = [domains[0][1] - domains[0][0], domains[1][1] - domains[1][0]];
    for region in regions {
        for loop_ in std::iter::once(&mut region.outer).chain(region.holes.iter_mut()) {
            let authored = loop_.clone();
            for axis in 0..2 {
                if periodic[axis] {
                    loop_[0][axis] = domains[axis][0]
                        + (loop_[0][axis] - domains[axis][0]).rem_euclid(period[axis]);
                }
            }
            for i in 1..loop_.len() {
                for axis in 0..2 {
                    if periodic[axis] {
                        while loop_[i][axis] - loop_[i - 1][axis] > period[axis] / 2. {
                            loop_[i][axis] -= period[axis];
                        }
                        while loop_[i][axis] - loop_[i - 1][axis] < -period[axis] / 2. {
                            loop_[i][axis] += period[axis];
                        }
                    }
                }
            }
            let authored_close = sub(authored[0], *authored.last().unwrap());
            let unwrapped_close = sub(loop_[0], *loop_.last().unwrap());
            for axis in 0..2 {
                let discrepancy = (unwrapped_close[axis] - authored_close[axis]).abs();
                let allowed = if periodic[axis] { period[axis] } else { 0. };
                if discrepancy > tolerance && (discrepancy - allowed).abs() > tolerance {
                    return Err(refused(
                        "Periodic UV loop does not close on one lifted chart",
                    ));
                }
                if periodic[axis]
                    && (0..loop_.len()).any(|i| {
                        let j = (i + 1) % loop_.len();
                        (loop_[j][axis] - loop_[i][axis]).abs() >= period[axis] - tolerance
                    })
                {
                    return Err(refused(
                        "Periodic UV loop spans a full period in one segment",
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Execute an atomic UV Boolean with default resource limits.
pub fn boolean_same_surface_uv(
    first: &Surface,
    a: &crate::Model,
    face_a: usize,
    second: &Surface,
    b: &crate::Model,
    face_b: usize,
    equivalence: Option<UvEquivalence>,
    operation: UvBooleanOperation,
) -> Result<SameSurfaceUvBoolean> {
    boolean_same_surface_uv_with_limits(
        first,
        a,
        face_a,
        second,
        b,
        face_b,
        equivalence,
        operation,
        SameSurfaceUvLimits::default(),
    )
}

/// The result is a certified UV-region arrangement, not an authored 3D solid.
/// Every limit used by a successful run is replayed from its certificate.
pub fn boolean_same_surface_uv_with_limits(
    first: &Surface,
    a: &crate::Model,
    face_a: usize,
    second: &Surface,
    b: &crate::Model,
    face_b: usize,
    equivalence: Option<UvEquivalence>,
    operation: UvBooleanOperation,
    limits: SameSurfaceUvLimits,
) -> Result<SameSurfaceUvBoolean> {
    first.validate()?;
    second.validate()?;
    if !limits.tolerance_uv.is_finite()
        || limits.tolerance_uv <= 0.
        || limits.max_segments < 3
        || limits.max_work == 0
    {
        return Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "UV limits must be finite, positive and usable",
        ));
    }
    let fa = a
        .faces
        .get(face_a)
        .ok_or_else(|| refused("Missing first UV face"))?;
    let fb = b
        .faces
        .get(face_b)
        .ok_or_else(|| refused("Missing second UV face"))?;
    if fa.surface != *first || fb.surface != *second {
        return Err(refused("Carrier arguments do not match the selected faces"));
    }
    if first != second && equivalence.is_none() {
        return Err(refused(
            "Distinct carriers need an exact UV equivalence map",
        ));
    }
    if let Some(map) = equivalence {
        if crate::boolean_robustness::certify_nurbs_uv_equivalence(first, second) != Some(map) {
            return Err(refused(
                "UV equivalence evidence does not certify these carriers",
            ));
        }
    }
    let mut ar = face_regions(a, fa)?;
    let mut br = face_regions(b, fb)?;
    if let Some(map) = equivalence {
        transform_regions(&mut br, map, second, first);
    }
    if first.periodic_u || first.periodic_v {
        unwrap_periodic_regions(&mut ar, first, limits.tolerance_uv)?;
        unwrap_periodic_regions(&mut br, first, limits.tolerance_uv)?;
    }
    let (regions, work) = arrangement(&ar, &br, operation, limits)?;
    let certificate = SameSurfaceUvBooleanCertificate {
        version: 1,
        operation,
        first: ar,
        second: br,
        equivalence,
        limits,
        work,
        result: regions.clone(),
    };
    Ok(SameSurfaceUvBoolean {
        regions,
        certificate,
    })
}

pub fn verify_same_surface_uv_boolean(
    first: &Surface,
    a: &crate::Model,
    face_a: usize,
    second: &Surface,
    b: &crate::Model,
    face_b: usize,
    result: &SameSurfaceUvBoolean,
) -> Result<()> {
    let c = &result.certificate;
    if c.version != 1 {
        return Err(refused("UV Boolean certificate version mismatch"));
    }
    let fresh = boolean_same_surface_uv_with_limits(
        first,
        a,
        face_a,
        second,
        b,
        face_b,
        c.equivalence,
        c.operation,
        c.limits,
    )?;
    if fresh != *result {
        return Err(refused(
            "UV Boolean source, mapping, operation, limits or result changed",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    // Integration fixtures construct ordinary B-rep faces and exercise this
    // public certified API from `tests/same_surface_uv_boolean.rs`.
}
