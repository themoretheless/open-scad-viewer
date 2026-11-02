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
pub(super) fn build(points: &[P], triangles: &[[u32; 3]], k: &Q, work: &mut Work) -> Result<Model> {
    if triangles.is_empty() {
        return Model::empty(1e-7);
    }
    if triangles.len() > 1024 {
        return Err(fail("quadratic B-rep face budget exceeded"));
    }
    let mut model = Model(
        brep_topology::Model {
            vertices: vec![],
            edges: vec![],
            loops: vec![],
            faces: vec![],
            shells: vec![],
            bodies: vec![],
            tolerance_mm: 1e-7,
        },
        TopologyIds::default(),
    );
    for p in points {
        guard(p)?;
        let p = lift(p, k);
        guard(&p)?;
        model.vertices.push(Vertex {
            point: [exact(&p[0])?, exact(&p[1])?, exact(&p[2])?],
        });
    }
    let mut incident = BTreeMap::<(u32, u32), Vec<usize>>::new();
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
    let planes = triangles
        .iter()
        .map(|t| normalized(&plane(&t.map(|v| points[v as usize].clone())), true))
        .collect::<Vec<_>>();
    let mut face_of = vec![usize::MAX; triangles.len()];
    let mut edge_map = BTreeMap::new();
    for start in 0..triangles.len() {
        if face_of[start] != usize::MAX {
            continue;
        }
        let face_id = model.faces.len();
        let mut todo = vec![start];
        let mut group = vec![];
        while let Some(f) = todo.pop() {
            if face_of[f] == usize::MAX && planes[f] == planes[start] {
                face_of[f] = face_id;
                group.push(f);
                todo.extend(&adjacent[f]);
            }
        }
        // Cancel only edges internal to this connected oriented plane region.
        let mut boundary = BTreeSet::new();
        for f in &group {
            let t = triangles[*f];
            for i in 0..3 {
                let e = (t[i], t[(i + 1) % 3]);
                if !boundary.remove(&(e.1, e.0)) {
                    boundary.insert(e);
                }
            }
        }
        let h = &planes[start];
        let drop = (0..3).find(|i| !h[*i].is_zero()).unwrap();
        let mut axes = [(drop + 1) % 3, (drop + 2) % 3];
        if h[drop].is_negative() {
            axes.swap(0, 1);
        }
        let uv = |id: u32| {
            [
                points[id as usize][axes[0]].clone(),
                points[id as usize][axes[1]].clone(),
            ]
        };
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
        let mut outer = None;
        let mut holes = vec![];
        let mut bounds = [[None, None], [None, None]];
        for ring in rings {
            let mut area = Q::zero();
            let mut coedges = vec![];
            for i in 0..ring.len() {
                let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
                let (p, q) = (uv(a), uv(b));
                area += &p[0] * &q[1] - &q[0] * &p[1];
                for j in 0..2 {
                    for k in 0..2 {
                        if bounds[j][k].as_ref().is_none_or(|old: &Q| {
                            if k == 0 {
                                p[j] < *old
                            } else {
                                p[j] > *old
                            }
                        }) {
                            bounds[j][k] = Some(p[j].clone());
                        }
                    }
                }
                let key = (a.min(b), a.max(b));
                let edge = if let Some(id) = edge_map.get(&key) {
                    *id
                } else {
                    let id = model.edges.len();
                    model.edges.push(Edge {
                        degenerate: false,
                        vertices: [key.0 as usize, key.1 as usize],
                        curve: curve(&points[key.0 as usize], &points[key.1 as usize], k)?,
                    });
                    edge_map.insert(key, id);
                    id
                };
                coedges.push(Coedge::new(
                    edge,
                    a > b,
                    pcurve(
                        [exact(&p[0])?, exact(&p[1])?],
                        [exact(&q[0])?, exact(&q[1])?],
                    ),
                ));
            }
            let id = model.loops.len();
            model.loops.push(Loop { coedges });
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
        let bounds = bounds.map(|b| b.map(Option::unwrap));
        let corner = |u: &Q, v: &Q| {
            let mut p = [Q::zero(), Q::zero(), Q::zero()];
            p[axes[0]] = u.clone();
            p[axes[1]] = v.clone();
            p[drop] = (&h[3] - &h[axes[0]] * u - &h[axes[1]] * v) / &h[drop];
            p
        };
        let a = corner(&bounds[0][0], &bounds[1][0]);
        let b = corner(&bounds[0][1], &bounds[1][0]);
        let c = corner(&bounds[0][0], &bounds[1][1]);
        let mut s = surface(&a, &b, &c, k)?;
        s.knots_u = vec![exact(&bounds[0][0])?; 3];
        s.knots_u.extend(vec![exact(&bounds[0][1])?; 3]);
        s.knots_v = vec![exact(&bounds[1][0])?; 3];
        s.knots_v.extend(vec![exact(&bounds[1][1])?; 3]);
        model.faces.push(Face {
            surface: s,
            outer: outer.ok_or_else(|| fail("coplanar region has no outer loop"))?,
            holes,
        });
    }
    // Discard vertices used only by cancelled internal triangles. Native Model
    // vertices must belong to the editable boundary graph.
    let used = model
        .edges
        .iter()
        .flat_map(|e| e.vertices)
        .collect::<BTreeSet<_>>();
    let mut remap = BTreeMap::new();
    let old = std::mem::take(&mut model.vertices);
    for id in used {
        remap.insert(id, model.vertices.len());
        model.vertices.push(old[id].clone());
    }
    for e in &mut model.edges {
        e.vertices = e.vertices.map(|id| remap[&id]);
    }
    let mut assigned = BTreeSet::new();
    let mut components = vec![];
    let mut volumes = vec![];
    for start in 0..triangles.len() {
        if assigned.contains(&start) {
            continue;
        }
        let mut todo = vec![start];
        let mut component = vec![];
        while let Some(f) = todo.pop() {
            if assigned.insert(f) {
                component.push(f);
                todo.extend(&adjacent[f]);
            }
        }
        component.sort();
        let volume: Q = component
            .iter()
            .map(|f| {
                let t = triangles[*f];
                dot(
                    &points[t[0] as usize],
                    &cross(&points[t[1] as usize], &points[t[2] as usize]),
                )
            })
            .sum();
        if volume.is_zero() {
            return Err(fail("quadratic B-rep shell has zero reference volume"));
        }
        volumes.push(volume);
        components.push(component);
    }
    for component in &components {
        model.shells.push(Shell {
            faces: component
                .iter()
                .map(|f| face_of[*f])
                .collect::<BTreeSet<_>>()
                .into_iter()
                .map(|face| FaceUse {
                    face,
                    reversed: false,
                })
                .collect(),
            closed: true,
        });
    }
    for (s, volume) in volumes.iter().enumerate() {
        if volume.is_positive() {
            model.bodies.push(Body {
                outer_shell: s,
                inner_shells: vec![],
            });
        }
    }
    for (s, volume) in volumes.iter().enumerate() {
        if volume.is_negative() {
            let p = &points[triangles[components[s][0]][0] as usize];
            let mut owners = vec![];
            for (id, body) in model.bodies.iter().enumerate() {
                let faces = components[body.outer_shell]
                    .iter()
                    .map(|f| triangles[*f].map(|v| points[v as usize].clone()).to_vec())
                    .collect::<Vec<_>>();
                if winding(p, &faces, work)? > 0 {
                    owners.push(id);
                }
            }
            if owners.len() != 1 {
                return Err(fail("quadratic cavity has no unique exact container"));
            }
            model.bodies[owners[0]].inner_shells.push(s);
        }
    }
    model.rebuild_topology_ids();
    model.validate()?;
    Ok(model)
}
