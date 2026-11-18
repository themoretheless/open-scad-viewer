//! Constructive six-valent height-graph corner, three unequal paired profiles.
//! Exact rational construction precedes conversion to literal binary poles.
//! Proposal family only; final G2, curvature and embedding require fresh audits.
use super::corner_patch_body::Build;
use crate::{Body, Curve, FaceUse, Model, Shell, Surface};
use num_rational::BigRational as Q;
use num_traits::{FromPrimitive, One, ToPrimitive, Zero};
use nurbs_core::{Error, Result};
use std::collections::BTreeMap;
type Point = [i64; 2];
type Poly = BTreeMap<(usize, usize), Q>;
const A: i64 = 3;
const R: i64 = 12;
const H: i64 = 40;
/// Fixed exact physical scale of this canonical control, not a post-hoc repair.
const SCALE: f64 = 138600. / 262144.;
fn fail(s: &str) -> Error {
    Error::new("BREP_SIX_EDGE_UNPROVEN", s)
}
fn q(n: i64) -> Q {
    Q::from_integer(n.into())
}
fn choose(n: usize, k: usize) -> i64 {
    if k > n {
        return 0;
    }
    (0..k.min(n - k)).fold(1i64, |a, i| a * (n - i) as i64 / (i + 1) as i64)
}
fn exact(x: &Q) -> Result<f64> {
    let f = x
        .to_f64()
        .ok_or_else(|| fail("Polynomial pole outside binary range"))?;
    if Q::from_f64(f).as_ref() != Some(x) {
        return Err(fail(&format!("Non-binary polynomial pole: {x}")));
    }
    Ok(f)
}
fn scale() -> Q {
    Q::from_f64(SCALE).unwrap()
}
fn constant(x: Q) -> Poly {
    BTreeMap::from([((0, 0), x)])
}
fn add(a: &mut Poly, b: &Poly, k: &Q) {
    for (&ij, v) in b {
        *a.entry(ij).or_default() += v * k;
    }
    a.retain(|_, v| !v.is_zero());
}
fn product(a: &Poly, b: &Poly) -> Poly {
    let mut p = Poly::new();
    for (&(i, j), x) in a {
        for (&(k, l), y) in b {
            *p.entry((i + k, j + l)).or_default() += x * y;
        }
    }
    p.retain(|_, v| !v.is_zero());
    p
}
fn power(a: &Poly, n: usize) -> Poly {
    (0..n).fold(constant(Q::one()), |b, _| product(&b, a))
}
fn axis(i: usize) -> Poly {
    match i {
        0 => BTreeMap::from([((1, 0), q(1))]),
        1 => BTreeMap::from([((0, 1), q(1))]),
        _ => BTreeMap::from([((1, 0), q(1)), ((0, 1), q(1))]),
    }
}
fn field(i: usize, state: i8, h: f64) -> Poly {
    let l = axis(i);
    if state != 0 {
        let mut p = Poly::new();
        add(&mut p, &l, &q(state as i64));
        return p;
    }
    let mut t = constant(q(A));
    add(&mut t, &l, &q(1));
    for v in t.values_mut() {
        *v /= q(2 * A);
    }
    let mut complement = constant(q(1));
    add(&mut complement, &t, &q(-1));
    let profile = [q(A), q(2), q(1), Q::from_f64(h).unwrap(), q(1), q(2), q(A)];
    let mut p = Poly::new();
    for (k, v) in profile.iter().enumerate() {
        let term = product(&power(&t, k), &power(&complement, 6 - k));
        add(&mut p, &term, &(v * q(choose(6, k))));
    }
    p
}
fn roof(states: [i8; 3], handles: [f64; 3]) -> Poly {
    let mut p = constant(q(H));
    for i in 0..3 {
        add(&mut p, &field(i, states[i], handles[i]), &q(-1));
    }
    p
}
fn value(p: &Poly, a: Point) -> Q {
    p.iter().fold(Q::zero(), |v, (&(i, j), c)| {
        v + c * q(a[0]).pow(i as i32) * q(a[1]).pow(j as i32)
    })
}
fn linear(p: Point, i: usize) -> i64 {
    match i {
        0 => p[0],
        1 => p[1],
        _ => p[0] + p[1],
    }
}
fn hexagon() -> Vec<Point> {
    vec![[R, 0], [0, R], [-R, R], [-R, 0], [0, -R], [R, -R]]
}
fn clip(poly: &[Point], axis: usize, bound: i64, less: bool) -> Result<Vec<Point>> {
    let mut out = Vec::new();
    if poly.is_empty() {
        return Ok(out);
    }
    let inside = |a: Point| {
        if less {
            linear(a, axis) <= bound
        } else {
            linear(a, axis) >= bound
        }
    };
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        let ia = inside(a);
        let ib = inside(b);
        if ia {
            out.push(a);
        }
        if ia != ib {
            let den = linear(b, axis) - linear(a, axis);
            let num = bound - linear(a, axis);
            let mut p = a;
            for k in 0..2 {
                let n = (b[k] - a[k]) * num;
                if n % den != 0 {
                    return Err(fail("Nonlattice arrangement root"));
                }
                p[k] += n / den;
            }
            out.push(p);
        }
    }
    out.dedup();
    if out.len() > 1 && out.first() == out.last() {
        out.pop();
    }
    Ok(out)
}
fn area(p: &[Point]) -> i64 {
    (0..p.len())
        .map(|i| p[i][0] * p[(i + 1) % p.len()][1] - p[i][1] * p[(i + 1) % p.len()][0])
        .sum()
}
struct Cell {
    states: [i8; 3],
    polygon: Vec<Point>,
    height: Poly,
}
fn cells(rounded: bool, handles: [f64; 3]) -> Result<Vec<Cell>> {
    let states: Vec<i8> = if rounded { vec![-1, 0, 1] } else { vec![-1, 1] };
    let mut out = Vec::new();
    for &a in &states {
        for &b in &states {
            for &c in &states {
                let state = [a, b, c];
                let mut p = hexagon();
                for i in 0..3 {
                    match state[i] {
                        -1 => p = clip(&p, i, if rounded { -A } else { 0 }, true)?,
                        1 => p = clip(&p, i, if rounded { A } else { 0 }, false)?,
                        _ => {
                            p = clip(&p, i, -A, false)?;
                            p = clip(&p, i, A, true)?;
                        }
                    }
                }
                if p.len() >= 3 && area(&p) > 0 {
                    out.push(Cell {
                        states: state,
                        polygon: p,
                        height: roof(state, handles),
                    });
                }
            }
        }
    }
    Ok(out)
}
fn bounds(p: &[Point]) -> [[i64; 2]; 2] {
    std::array::from_fn(|k| {
        [
            p.iter().map(|p| p[k]).min().unwrap(),
            p.iter().map(|p| p[k]).max().unwrap(),
        ]
    })
}
fn local(p: &Poly, b: [[i64; 2]; 2]) -> Poly {
    let x = BTreeMap::from([((0, 0), q(b[0][0])), ((1, 0), q(b[0][1] - b[0][0]))]);
    let y = BTreeMap::from([((0, 0), q(b[1][0])), ((0, 1), q(b[1][1] - b[1][0]))]);
    let mut out = Poly::new();
    for (&(i, j), v) in p {
        add(&mut out, &product(&power(&x, i), &power(&y, j)), v);
    }
    out
}
fn graph(p: &Poly, b: [[i64; 2]; 2], degree: usize) -> Result<Surface> {
    let poly = local(p, b);
    let mut cp = vec![vec![vec![0.; 3]; degree + 1]; degree + 1];
    for i in 0..=degree {
        for j in 0..=degree {
            cp[i][j][0] = exact(
                &(scale() * (q(b[0][0]) + q(b[0][1] - b[0][0]) * q(i as i64) / q(degree as i64))),
            )?;
            cp[i][j][1] = exact(
                &(scale() * (q(b[1][0]) + q(b[1][1] - b[1][0]) * q(j as i64) / q(degree as i64))),
            )?;
            let z = poly.iter().filter(|((a, b), _)| *a <= i && *b <= j).fold(
                Q::zero(),
                |v, (&(a, b), c)| {
                    v + c * q(choose(i, a)) * q(choose(j, b))
                        / q(choose(degree, a))
                        / q(choose(degree, b))
                },
            );
            cp[i][j][2] = exact(&(scale() * z))?;
        }
    }
    Ok(Surface {
        degree_u: degree,
        degree_v: degree,
        knots_u: [
            vec![b[0][0] as f64; degree + 1],
            vec![b[0][1] as f64; degree + 1],
        ]
        .concat(),
        knots_v: [
            vec![b[1][0] as f64; degree + 1],
            vec![b[1][1] as f64; degree + 1],
        ]
        .concat(),
        control_points: cp,
        weights: vec![vec![1.; degree + 1]; degree + 1],
        periodic_u: false,
        periodic_v: false,
    })
}
fn restriction(p: &Poly, a: Point, b: Point) -> Vec<Q> {
    let mut out = vec![Q::zero(); 7];
    for (&(i, j), v) in p {
        for r in 0..=i {
            for s in 0..=j {
                out[r + s] += v
                    * q(choose(i, r))
                    * q(a[0]).pow((i - r) as i32)
                    * q(b[0] - a[0]).pow(r as i32)
                    * q(choose(j, s))
                    * q(a[1]).pow((j - s) as i32)
                    * q(b[1] - a[1]).pow(s as i32);
            }
        }
    }
    out
}
fn edge_curve(p: &Poly, a: Point, b: Point) -> Result<Curve> {
    let coefficients = restriction(p, a, b);
    let mut cp = Vec::new();
    for i in 0..=6 {
        let z = (0..=i).fold(Q::zero(), |v, k| {
            v + &coefficients[k] * q(choose(i, k)) / q(choose(6, k))
        });
        cp.push(vec![
            exact(&(scale() * (q(a[0]) + q(b[0] - a[0]) * q(i as i64) / q(6))))?,
            exact(&(scale() * (q(a[1]) + q(b[1] - a[1]) * q(i as i64) / q(6))))?,
            exact(&(scale() * z))?,
        ]);
    }
    Ok(Curve {
        degree: 6,
        knots: vec![0., 0., 0., 0., 0., 0., 0., 1., 1., 1., 1., 1., 1., 1.],
        control_points: cp,
        weights: vec![1.; 7],
        periodic: false,
    })
}
fn pc(a: Point, b: Point) -> Result<Curve> {
    Curve::from_polyline(vec![
        vec![a[0] as f64, a[1] as f64],
        vec![b[0] as f64, b[1] as f64],
    ])
}
struct Construction {
    b: Build,
    nodes: BTreeMap<(Point, bool), usize>,
    top_faces: Vec<usize>,
    retained: Vec<([i8; 3], usize)>,
}
impl Construction {
    fn node(&mut self, p: Point, top: bool, height: &Poly) -> Result<usize> {
        if let Some(&n) = self.nodes.get(&(p, top)) {
            return Ok(n);
        }
        let z = if top { value(height, p) } else { q(0) };
        let n = self.b.node([
            exact(&(scale() * q(p[0])))?,
            exact(&(scale() * q(p[1])))?,
            exact(&(scale() * z))?,
        ]);
        self.nodes.insert((p, top), n);
        Ok(n)
    }
}
fn wall(a: Point, b: Point) -> Result<(Surface, usize, bool)> {
    let free = if a[0] != b[0] { 0 } else { 1 };
    let low = a[free].min(b[free]);
    let high = a[free].max(b[free]);
    let mut cp = vec![vec![vec![0.; 3]; 2]; 2];
    for i in 0..2 {
        let u = if i == 0 { low } else { high };
        let t = q(u - a[free]) / q(b[free] - a[free]);
        for j in 0..2 {
            for k in 0..2 {
                cp[i][j][k] = exact(&(scale() * (q(a[k]) + q(b[k] - a[k]) * &t)))?;
            }
            cp[i][j][2] = exact(&(scale() * q(if j == 0 { 0 } else { H })))?;
        }
    }
    let u = [exact(&(scale() * q(low)))?, exact(&(scale() * q(high)))?];
    let v = [0., exact(&(scale() * q(H)))?];
    Ok((
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![u[0], u[0], u[1], u[1]],
            knots_v: vec![v[0], v[0], v[1], v[1]],
            control_points: cp,
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        },
        free,
        b[free] < a[free],
    ))
}
fn construct(
    rounded: bool,
    handles: [f64; 3],
) -> Result<(Model, Vec<usize>, Vec<([i8; 3], usize)>)> {
    let cells = cells(rounded, handles)?;
    let mut c = Construction {
        b: Build::empty(1e-7),
        nodes: BTreeMap::new(),
        top_faces: vec![],
        retained: vec![],
    };
    let mut senses = Vec::new();
    for cell in &cells {
        let retained = cell.states.iter().all(|s| *s != 0);
        let surface = graph(
            &cell.height,
            if retained {
                [[-R, R]; 2]
            } else {
                bounds(&cell.polygon)
            },
            if retained { 1 } else { 6 },
        )?;
        let mut segments = Vec::new();
        for i in 0..cell.polygon.len() {
            let a = cell.polygon[i];
            let b = cell.polygon[(i + 1) % cell.polygon.len()];
            let na = c.node(a, true, &cell.height)?;
            let nb = c.node(b, true, &cell.height)?;
            segments.push((na, nb, edge_curve(&cell.height, a, b)?, pc(a, b)?));
        }
        let f = c.b.face(surface, segments)?;
        c.top_faces.push(f);
        if retained {
            c.retained.push((cell.states, f));
        }
        senses.push(false);
    }
    let hex = hexagon();
    for i in 0..6 {
        let a = hex[i];
        let b = hex[(i + 1) % 6];
        let (surface, free, reversed) = wall(a, b)?;
        let mut points: Vec<_> = c
            .nodes
            .keys()
            .filter(|(p, top)| {
                *top && (b[0] - a[0]) * (p[1] - a[1]) == (b[1] - a[1]) * (p[0] - a[0])
                    && p[0] >= a[0].min(b[0])
                    && p[0] <= a[0].max(b[0])
                    && p[1] >= a[1].min(b[1])
                    && p[1] <= a[1].max(b[1])
            })
            .map(|(p, _)| *p)
            .collect();
        points.sort_by_key(|p| if b[free] > a[free] { p[free] } else { -p[free] });
        if points.first() != Some(&a) || points.last() != Some(&b) {
            return Err(fail("Incomplete outer arrangement side"));
        }
        let empty = Poly::new();
        let ba = c.node(a, false, &empty)?;
        let bb = c.node(b, false, &empty)?;
        let mut path = vec![ba, bb];
        path.extend(points.iter().rev().map(|p| c.nodes[&(*p, true)]));
        let mut segments = Vec::new();
        for j in 0..path.len() {
            let u = path[j];
            let v = path[(j + 1) % path.len()];
            let curve = c.b.curve(u, v)?;
            let mut pcurve = curve.clone();
            pcurve.control_points = curve
                .control_points
                .iter()
                .map(|p| vec![p[free], p[2]])
                .collect();
            segments.push((u, v, curve, pcurve));
        }
        c.b.face(surface, segments)?;
        senses.push(reversed);
    }
    let bottom = graph(&Poly::new(), [[-R, R]; 2], 1)?;
    let mut segments = Vec::new();
    for i in 0..6 {
        let a = c.nodes[&(hex[i], false)];
        let b = c.nodes[&(hex[(i + 1) % 6], false)];
        segments.push((a, b, c.b.curve(a, b)?, pc(hex[i], hex[(i + 1) % 6])?));
    }
    c.b.face(bottom, segments)?;
    senses.push(true);
    c.b.m.shells = vec![Shell {
        faces: senses
            .into_iter()
            .enumerate()
            .map(|(face, reversed)| FaceUse { face, reversed })
            .collect(),
        closed: true,
    }];
    c.b.m.bodies = vec![Body {
        outer_shell: 0,
        inner_shells: vec![],
    }];
    c.b.m.rebuild_topology_ids();
    c.b.m.validate()?;
    Ok((c.b.m, c.top_faces, c.retained))
}
pub struct SixEdgeCorner {
    pub model: Model,
    pub top_faces: Vec<usize>,
    pub smooth_edges: Vec<usize>,
    pub handles: [f64; 3],
    pub physical_scale: f64,
    pub requested_mid_radii: Option<[f64; 3]>,
    pub mid_radius_intervals: Option<[[f64; 2]; 3]>,
}
pub fn six_edge_corner_stock() -> Result<Model> {
    Ok(construct(false, [0.; 3])?.0)
}
/// Canonical six-valent stock only. Profiles pair opposite edges; three groups
/// may differ. Handles use a 2^-24 dyadic grid; non-binary stored poles refuse.
/// End contacts to the six vertical walls remain intentionally sharp.
/// No report from this constructor grants G2/curvature/embedding qualification.
pub fn propose_six_edge_corner(source: &Model, handles: [f64; 3]) -> Result<SixEdgeCorner> {
    source.validate()?;
    if handles
        .iter()
        .any(|h| !h.is_finite() || *h < 0. || *h > 1. || (*h * 16777216.).fract() != 0.)
    {
        return Err(fail("Need bounded dyadic handles in [0,1]"));
    }
    if source.0 != six_edge_corner_stock()?.0 {
        return Err(fail(
            "Six-edge source differs from canonical original stock",
        ));
    }
    let (mut model, top, retained) = construct(true, handles)?;
    let (_, _, original) = construct(false, [0.; 3])?;
    for (state, f) in &retained {
        let old = original.iter().find(|(s, _)| s == state).unwrap().1;
        if model.faces[*f].surface != source.faces[old].surface {
            return Err(fail("Original retained roof support changed"));
        }
    }
    model.inherit_topology_ids(&[source]);
    let mut lineage = Vec::new();
    for (state, &face) in retained.iter().map(|(s, f)| (s, f)) {
        let old = original.iter().find(|(s, _)| s == state).unwrap().1;
        lineage.push((old, face));
    }
    let source_top = original.len();
    for i in 0..7 {
        lineage.push((source_top + i, top.len() + i));
    }
    for (old, new) in lineage {
        if source.faces[old].surface != model.faces[new].surface {
            return Err(fail("Retained original support changed"));
        }
        let parent = source.1.faces[old];
        let child = model.1.faces[new];
        if parent != child
            && !model.1.lineage.iter().any(|r| {
                r.entity_kind == "face"
                    && r.parents.contains(&parent)
                    && r.children.contains(&child)
            })
        {
            model.1.lineage.push(crate::TopologyLineageRecord {
                operation: "persist".into(),
                entity_kind: "face".into(),
                parents: vec![parent],
                children: vec![child],
            });
        }
    }
    model.refresh_change_set(&[source]);
    let mut smooth = Vec::new();
    for e in 0..model.edges.len() {
        let uses: Vec<_> = top
            .iter()
            .filter(|&&f| {
                model.loops[model.faces[f].outer]
                    .coedges
                    .iter()
                    .any(|c| c.edge == e)
            })
            .collect();
        if uses.len() == 2 {
            smooth.push(e);
        }
    }
    Ok(SixEdgeCorner {
        model,
        top_faces: top,
        smooth_edges: smooth,
        handles,
        physical_scale: SCALE,
        requested_mid_radii: None,
        mid_radius_intervals: None,
    })
}

#[path = "six_edge_corner/graph_proof.rs"]
mod graph_proof;
pub use graph_proof::{SixEdgeGraphProof,inspect_six_edge_graph};

fn radius_intervals(handles: [f64; 3]) -> Result<[[f64; 2]; 3]> {
    use nurbs_core::interval_eval::Interval as I;
    let mut out = [[0.; 2]; 3];
    for i in 0..3 {
        let root = if i < 2 { 6f64.sqrt() } else { 3f64.sqrt() };
        let sqrt = I::new(root.next_down(), root.next_up())?;
        let numerator = I::point(if i < 2 { 144. * SCALE } else { 12. * SCALE }).mul(sqrt)?;
        let denominator =
            I::point(if i < 2 { 25. } else { 5. }).mul(I::point(3.).sub(I::point(handles[i]))?)?;
        let radius = numerator.div(denominator)?;
        out[i] = [radius.lo, radius.hi];
    }
    Ok(out)
}
/// Prescribe three arm MIDLINE principal radii, pairing opposite selected edges
/// (x=0, y=0, x+y=0). G2 profiles have varying transverse curvature toward the
/// planar supports. Quantisation is 2^-24; complete interval radius admission
/// requires error <=1e-7 mm. The bounded convex family may refuse other radii.
pub fn propose_six_edge_corner_radii(source: &Model, radii: [f64; 3]) -> Result<SixEdgeCorner> {
    let mut handles = [0.; 3];
    for i in 0..3 {
        if !radii[i].is_finite() || radii[i] <= 0. {
            return Err(fail("Need three finite positive paired midline radii"));
        }
        let c = if i < 2 {
            144. * 6f64.sqrt() * SCALE / 25.
        } else {
            12. * 3f64.sqrt() * SCALE / 5.
        };
        handles[i] = ((3. - c / radii[i]) * 16777216.).round() / 16777216.;
    }
    if handles.iter().any(|h| !h.is_finite() || *h < 0. || *h > 1.) {
        return Err(fail(
            "Requested radius lies outside the convex paired profile family",
        ));
    }
    let bounds = radius_intervals(handles)?;
    if (0..3).any(|i| bounds[i][0] < radii[i] - 1e-7 || bounds[i][1] > radii[i] + 1e-7) {
        return Err(fail("Quantised radius not within 1e-7 mm"));
    }
    let mut corner = propose_six_edge_corner(source, handles)?;
    corner.requested_mid_radii = Some(radii);
    corner.mid_radius_intervals = Some(bounds);
    Ok(corner)
}

/// Owned admitted canonical body. Geometry is exposed read-only; extracting a
/// mutable Model consumes the receipt instead of retaining a stale admission.
pub struct QualifiedSixEdgeBody {
    corner: SixEdgeCorner,
    graph: SixEdgeGraphProof,
    closure: crate::watertight::QualifiedReport,
}
impl QualifiedSixEdgeBody {
    pub fn model(&self) -> &Model {
        &self.corner.model
    }
    pub fn corner(&self) -> &SixEdgeCorner {
        &self.corner
    }
    pub fn proof(&self) -> &SixEdgeGraphProof {
        &self.graph
    }
    pub fn closure(&self) -> &crate::watertight::QualifiedReport {
        &self.closure
    }
    pub fn into_model(self) -> Model {
        self.corner.model
    }
}
/// Atomic native admission of the bounded six-edge paired-radius family.
/// Original source is never changed. Every exact saved-geometry/G2/material/
/// embedding/<=10-mm^-1 gate and the generic 3D edge/lift closure gate must pass.
pub fn build_six_edge_corner_radii(
    source: &Model,
    radii: [f64; 3],
) -> Result<QualifiedSixEdgeBody> {
    let corner = propose_six_edge_corner_radii(source, radii)?;
    let graph = inspect_six_edge_graph(source, &corner)?;
    let closure = crate::watertight::inspect_with_exact(&corner.model, 1000000, 1000000)?;
    if closure.closure.status != crate::watertight::Closure::WithinTolerance {
        return Err(fail("Complete saved boundary closure remains unresolved"));
    }
    Ok(QualifiedSixEdgeBody {
        corner,
        graph,
        closure,
    })
}
