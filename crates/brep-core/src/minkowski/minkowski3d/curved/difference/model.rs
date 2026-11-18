use super::*;
use crate::{Curve, Surface, TopologyIds};
use brep_topology::{Body, Coedge, Edge, Face, FaceUse, Loop, Shell, Vertex};
fn exact(q: &Q) -> Result<f64> {
    let x = q
        .to_f64()
        .ok_or_else(|| fail("NURBS control coordinate overflow"))?;
    if !x.is_finite() || Q::from_float(x).as_ref() != Some(q) {
        return Err(fail("exact NURBS control is not representable in binary64"));
    }
    Ok(x)
}
fn lift(p: &P, k: &Q) -> P {
    let mut p = p.clone();
    let shift = &p[0] * &p[0] * k;
    p[2] += shift;
    p
}
fn curve(a: &P, b: &P, k: &Q) -> Result<Curve> {
    let start = lift(a, k);
    let end = lift(b, k);
    let delta = &b[0] - &a[0];
    let mut middle = std::array::from_fn::<_, 3, _>(|i| (&start[i] + &end[i]) / integer(2));
    middle[2] -= k * &delta * &delta / integer(2);
    Ok(Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: [start, middle, end]
            .iter()
            .map(|p| p.iter().map(exact).collect())
            .collect::<Result<_>>()?,
        weights: vec![1.; 3],
        periodic: false,
    })
}
fn surface(a: &P, b: &P, c: &P, k: &Q) -> Result<Surface> {
    let u = sub(b, a);
    let v = sub(c, a);
    let mut controls = vec![vec![vec![0.; 3]; 3]; 3];
    // Tensor Bernstein degree (2,2) coefficients of F(a+u*s+v*t).
    for i in 0..3 {
        for j in 0..3 {
            let s = Q::new(i.into(), 2.into());
            let t = Q::new(j.into(), 2.into());
            let mut p = std::array::from_fn::<_, 3, _>(|d| &a[d] + &u[d] * &s + &v[d] * &t);
            let square = &a[0] * &a[0]
                + integer(2) * &a[0] * (&u[0] * &s + &v[0] * &t)
                + &u[0] * &u[0] * integer(if i == 2 { 1 } else { 0 })
                + &v[0] * &v[0] * integer(if j == 2 { 1 } else { 0 })
                + integer(2) * &u[0] * &v[0] * &s * &t;
            p[2] += k * square;
            controls[i][j] = p.iter().map(exact).collect::<Result<_>>()?;
        }
    }
    Ok(Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: controls,
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    })
}
fn pcurve(a: [f64; 2], b: [f64; 2]) -> Curve {
    Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![a.to_vec(), b.to_vec()],
        weights: vec![1.; 2],
        periodic: false,
    }
}
const MAX_TRIANGLES: usize = 1024;
const TOLERANCE_MM: f64 = 1e-7;
type EdgeKey = (u32, u32);
/// Triangles across each edge; every edge must join exactly two triangles.
fn adjacency(triangles: &[[u32; 3]]) -> Result<Vec<Vec<usize>>> {
    let mut incident = BTreeMap::<EdgeKey, Vec<usize>>::new();
    for (f, t) in triangles.iter().enumerate() {
        for i in 0..3 {
            let (a, b) = (t[i], t[(i + 1) % 3]);
            incident.entry((a.min(b), a.max(b))).or_default().push(f);
        }
    }
    let mut adjacent = vec![vec![]; triangles.len()];
    for faces in incident.values() {
        if faces.len() != 2 {
            return Err(fail("quadratic B-rep boundary edge is not paired"));
        }
        adjacent[faces[0]].push(faces[1]);
        adjacent[faces[1]].push(faces[0]);
    }
    Ok(adjacent)
}
/// Triangles reachable from `start` through `adjacent` that satisfy `keep`.
fn flood(start: usize, adjacent: &[Vec<usize>], mut keep: impl FnMut(usize) -> bool) -> Vec<usize> {
    let mut todo = vec![start];
    let mut group = vec![];
    while let Some(f) = todo.pop() {
        if keep(f) {
            group.push(f);
            todo.extend(&adjacent[f]);
        }
    }
    group
}
/// Directed boundary edges of a connected oriented plane region: edges
/// internal to the region cancel against their reverse.
fn region_boundary(group: &[usize], triangles: &[[u32; 3]]) -> BTreeSet<EdgeKey> {
    let mut boundary = BTreeSet::new();
    for f in group {
        let t = triangles[*f];
        for i in 0..3 {
            let e = (t[i], t[(i + 1) % 3]);
            if !boundary.remove(&(e.1, e.0)) {
                boundary.insert(e);
            }
        }
    }
    boundary
}
/// Closed vertex rings of a boundary that must not branch.
fn boundary_rings(boundary: BTreeSet<EdgeKey>) -> Result<Vec<Vec<u32>>> {
    let mut next = BTreeMap::new();
    for (a, b) in boundary {
        if next.insert(a, b).is_some() {
            return Err(fail("coplanar face has a branching boundary"));
        }
    }
    let mut rings = vec![];
    while let Some((&start, _)) = next.iter().next() {
        let mut ring = vec![];
        let mut a = start;
        loop {
            ring.push(a);
            a = next
                .remove(&a)
                .ok_or_else(|| fail("coplanar boundary is not a cycle"))?;
            if a == start {
                break;
            }
        }
        if ring.len() < 3 {
            return Err(fail("coplanar region boundary is degenerate"));
        }
        rings.push(ring);
    }
    Ok(rings)
}
/// A plane parameterized by the two coordinates it does not depend on
/// least, oriented so that outer loops have positive area.
struct PlaneChart<'a> {
    h: &'a [Q],
    drop: usize,
    axes: [usize; 2],
}
impl<'a> PlaneChart<'a> {
    fn new(h: &'a [Q]) -> Result<Self> {
        let drop = (0..3)
            .find(|i| !h[*i].is_zero())
            .ok_or_else(|| fail("coplanar region has a degenerate plane"))?;
        let mut axes = [(drop + 1) % 3, (drop + 2) % 3];
        if h[drop].is_negative() {
            axes.swap(0, 1);
        }
        Ok(Self { h, drop, axes })
    }
    fn uv(&self, p: &P) -> [Q; 2] {
        [p[self.axes[0]].clone(), p[self.axes[1]].clone()]
    }
    fn corner(&self, u: &Q, v: &Q) -> P {
        let (h, [a, b], d) = (self.h, self.axes, self.drop);
        let mut p = [Q::zero(), Q::zero(), Q::zero()];
        p[a] = u.clone();
        p[b] = v.clone();
        p[d] = (&h[3] - &h[a] * u - &h[b] * v) / &h[d];
        p
    }
}
/// Running chart bounds of the loop vertices of one face.
#[derive(Default)]
struct ChartBounds([[Option<Q>; 2]; 2]);
impl ChartBounds {
    fn include(&mut self, p: &[Q; 2]) {
        for (axis, bounds) in self.0.iter_mut().enumerate() {
            let [low, high] = bounds;
            if low.as_ref().is_none_or(|old| p[axis] < *old) {
                *low = Some(p[axis].clone());
            }
            if high.as_ref().is_none_or(|old| p[axis] > *old) {
                *high = Some(p[axis].clone());
            }
        }
    }
    fn finish(self) -> Option<[[Q; 2]; 2]> {
        let [[a, b], [c, d]] = self.0;
        Some([[a?, b?], [c?, d?]])
    }
}
/// The quadratic B-rep under construction over a lifted triangle mesh.
struct QuadraticModel<'a> {
    model: Model,
    points: &'a [P],
    triangles: &'a [[u32; 3]],
    k: &'a Q,
    edges: BTreeMap<EdgeKey, usize>,
}
impl QuadraticModel<'_> {
    fn lift_vertices(&mut self) -> Result<()> {
        for p in self.points {
            guard(p)?;
            let p = lift(p, self.k);
            guard(&p)?;
            self.model.vertices.push(Vertex {
                point: [exact(&p[0])?, exact(&p[1])?, exact(&p[2])?],
            });
        }
        Ok(())
    }
    /// The shared lifted edge between two mesh vertices.
    fn edge(&mut self, key: EdgeKey) -> Result<usize> {
        if let Some(id) = self.edges.get(&key) {
            return Ok(*id);
        }
        let id = self.model.edges.len();
        self.model.edges.push(Edge {
            degenerate: false,
            vertices: [key.0 as usize, key.1 as usize],
            curve: curve(
                &self.points[key.0 as usize],
                &self.points[key.1 as usize],
                self.k,
            )?,
        });
        self.edges.insert(key, id);
        Ok(id)
    }
    /// One boundary ring as a loop with linear pcurves; returns the loop and
    /// its signed chart area.
    fn ring_loop(
        &mut self,
        ring: &[u32],
        chart: &PlaneChart<'_>,
        bounds: &mut ChartBounds,
    ) -> Result<(usize, Q)> {
        let mut area = Q::zero();
        let mut coedges = vec![];
        for i in 0..ring.len() {
            let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
            let (p, q) = (
                chart.uv(&self.points[a as usize]),
                chart.uv(&self.points[b as usize]),
            );
            area += &p[0] * &q[1] - &q[0] * &p[1];
            bounds.include(&p);
            let edge = self.edge((a.min(b), a.max(b)))?;
            coedges.push(Coedge::new(
                edge,
                a > b,
                pcurve(
                    [exact(&p[0])?, exact(&p[1])?],
                    [exact(&q[0])?, exact(&q[1])?],
                ),
            ));
        }
        let id = self.model.loops.len();
        self.model.loops.push(Loop { coedges });
        Ok((id, area))
    }
    /// One face over a connected coplanar triangle group: its boundary rings
    /// become one outer loop and holes on the lifted plane patch.
    fn face(&mut self, group: &[usize], h: &[Q]) -> Result<()> {
        let chart = PlaneChart::new(h)?;
        let rings = boundary_rings(region_boundary(group, self.triangles))?;
        let mut outer = None;
        let mut holes = vec![];
        let mut bounds = ChartBounds::default();
        for ring in rings {
            let (id, area) = self.ring_loop(&ring, &chart, &mut bounds)?;
            if area.is_positive() {
                if outer.replace(id).is_some() {
                    return Err(fail("coplanar region has multiple outer loops"));
                }
            } else if area.is_negative() {
                holes.push(id);
            } else {
                return Err(fail("coplanar boundary has zero area"));
            }
        }
        let bounds = bounds
            .finish()
            .ok_or_else(|| fail("coplanar region has no outer loop"))?;
        let a = chart.corner(&bounds[0][0], &bounds[1][0]);
        let b = chart.corner(&bounds[0][1], &bounds[1][0]);
        let c = chart.corner(&bounds[0][0], &bounds[1][1]);
        let mut s = surface(&a, &b, &c, self.k)?;
        s.knots_u = vec![exact(&bounds[0][0])?; 3];
        s.knots_u.extend(vec![exact(&bounds[0][1])?; 3]);
        s.knots_v = vec![exact(&bounds[1][0])?; 3];
        s.knots_v.extend(vec![exact(&bounds[1][1])?; 3]);
        self.model.faces.push(Face {
            surface: s,
            outer: outer.ok_or_else(|| fail("coplanar region has no outer loop"))?,
            holes,
        });
        Ok(())
    }
    /// Faces of all coplanar connected groups; returns the face of every
    /// triangle.
    fn faces(&mut self, adjacent: &[Vec<usize>]) -> Result<Vec<usize>> {
        let planes = self
            .triangles
            .iter()
            .map(|t| normalized(&plane(&t.map(|v| self.points[v as usize].clone())), true))
            .collect::<Vec<_>>();
        let mut face_of = vec![usize::MAX; self.triangles.len()];
        for start in 0..self.triangles.len() {
            if face_of[start] != usize::MAX {
                continue;
            }
            let face_id = self.model.faces.len();
            let group = flood(start, adjacent, |f| {
                let take = face_of[f] == usize::MAX && planes[f] == planes[start];
                if take {
                    face_of[f] = face_id;
                }
                take
            });
            self.face(&group, &planes[start])?;
        }
        Ok(face_of)
    }
    /// Discard vertices used only by cancelled internal triangles. Native
    /// Model vertices must belong to the editable boundary graph.
    fn compact_vertices(&mut self) {
        let used = self
            .model
            .edges
            .iter()
            .flat_map(|e| e.vertices)
            .collect::<BTreeSet<_>>();
        let mut remap = BTreeMap::new();
        let old = std::mem::take(&mut self.model.vertices);
        for id in used {
            remap.insert(id, self.model.vertices.len());
            self.model.vertices.push(old[id].clone());
        }
        for e in &mut self.model.edges {
            e.vertices = e.vertices.map(|id| remap[&id]);
        }
    }
    /// Signed reference volume of a triangle component.
    fn volume(&self, component: &[usize]) -> Q {
        component
            .iter()
            .map(|f| {
                let t = self.triangles[*f];
                dot(
                    &self.points[t[0] as usize],
                    &cross(&self.points[t[1] as usize], &self.points[t[2] as usize]),
                )
            })
            .sum()
    }
    /// Connected triangle components with their nonzero signed volumes.
    fn components(&self, adjacent: &[Vec<usize>]) -> Result<Vec<(Vec<usize>, Q)>> {
        let mut assigned = BTreeSet::new();
        let mut components = vec![];
        for start in 0..self.triangles.len() {
            if assigned.contains(&start) {
                continue;
            }
            let mut component = flood(start, adjacent, |f| assigned.insert(f));
            component.sort();
            let volume = self.volume(&component);
            if volume.is_zero() {
                return Err(fail("quadratic B-rep shell has zero reference volume"));
            }
            components.push((component, volume));
        }
        Ok(components)
    }
    /// One closed shell per component; positive shells become bodies and
    /// every negative shell is a cavity of the unique body containing it.
    fn shells_and_bodies(
        &mut self,
        components: &[(Vec<usize>, Q)],
        face_of: &[usize],
        work: &mut Work,
    ) -> Result<()> {
        for (component, _) in components {
            let faces = component
                .iter()
                .map(|f| face_of[*f])
                .collect::<BTreeSet<_>>();
            self.model.shells.push(Shell {
                faces: faces
                    .into_iter()
                    .map(|face| FaceUse {
                        face,
                        reversed: false,
                    })
                    .collect(),
                closed: true,
            });
        }
        for (s, (_, volume)) in components.iter().enumerate() {
            if volume.is_positive() {
                self.model.bodies.push(Body {
                    outer_shell: s,
                    inner_shells: vec![],
                });
            }
        }
        for (s, (component, volume)) in components.iter().enumerate() {
            if volume.is_negative() {
                let owner = self.cavity_owner(component, components, work)?;
                self.model.bodies[owner].inner_shells.push(s);
            }
        }
        Ok(())
    }
    /// The unique body whose outer shell winds around a cavity.
    fn cavity_owner(
        &self,
        cavity: &[usize],
        components: &[(Vec<usize>, Q)],
        work: &mut Work,
    ) -> Result<usize> {
        let p = &self.points[self.triangles[cavity[0]][0] as usize];
        let mut owners = vec![];
        for (id, body) in self.model.bodies.iter().enumerate() {
            let faces = components[body.outer_shell]
                .0
                .iter()
                .map(|f| {
                    self.triangles[*f]
                        .map(|v| self.points[v as usize].clone())
                        .to_vec()
                })
                .collect::<Vec<_>>();
            if winding(p, &faces, work)? > 0 {
                owners.push(id);
            }
        }
        match owners[..] {
            [owner] => Ok(owner),
            _ => Err(fail("quadratic cavity has no unique exact container")),
        }
    }
}
pub(super) fn build(points: &[P], triangles: &[[u32; 3]], k: &Q, work: &mut Work) -> Result<Model> {
    if triangles.is_empty() {
        return Model::empty(TOLERANCE_MM);
    }
    if triangles.len() > MAX_TRIANGLES {
        return Err(fail("quadratic B-rep face budget exceeded"));
    }
    let mut builder = QuadraticModel {
        model: Model(
            brep_topology::Model {
                vertices: vec![],
                edges: vec![],
                loops: vec![],
                faces: vec![],
                shells: vec![],
                bodies: vec![],
                tolerance_mm: TOLERANCE_MM,
            },
            TopologyIds::default(),
        ),
        points,
        triangles,
        k,
        edges: BTreeMap::new(),
    };
    builder.lift_vertices()?;
    let adjacent = adjacency(triangles)?;
    let face_of = builder.faces(&adjacent)?;
    builder.compact_vertices();
    let components = builder.components(&adjacent)?;
    builder.shells_and_bodies(&components, &face_of, work)?;
    let mut model = builder.model;
    model.rebuild_topology_ids();
    model.validate()?;
    Ok(model)
}
