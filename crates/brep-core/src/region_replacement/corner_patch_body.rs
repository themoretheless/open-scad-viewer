//! Explicit canonical-cube topology for the constructive corner patch family.
//! C0 closure is mandatory; G2, curvature, trims and material are separate gates.
use super::CornerPatchNetwork;
use crate::{Body, Coedge, Curve, Edge, Face, FaceUse, Loop, Model, Shell, Surface, Vertex};
use nurbs_core::{Error, Result, surface::Axis};
use std::collections::BTreeMap;
pub struct CornerPatchBody {
    pub model: Model,
    pub cap_faces: [usize; 3],
    pub strip_faces: [usize; 3],
    pub stop_faces: [usize; 3],
    /// Central interfaces and strip/support contacts; original cube edges and
    /// planar far stops remain sharp and are not included in this G2 scope.
    pub smooth_edges: Vec<usize>,
}
fn check(ok: bool, s: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(Error::new("BREP_CORNER_PATCH_BODY", s))
    }
}
fn line(a: [f64; 2], b: [f64; 2]) -> Result<Curve> {
    Curve::from_polyline(vec![a.to_vec(), b.to_vec()])
}
fn side(s: &Surface, e: usize) -> Result<Curve> {
    match e {
        0 => s.iso(Axis::V, 0.),
        1 => s.iso(Axis::U, 1.),
        2 => s.iso(Axis::V, 1.)?.reverse(),
        _ => s.iso(Axis::U, 0.)?.reverse(),
    }
}
fn projected(c: &Curve, s: &Surface) -> Result<Curve> {
    check(
        s.degree_u == 1
            && s.degree_v == 1
            && s.control_points.len() == 2
            && s.control_points[0].len() == 2
            && s.weights.iter().flatten().all(|w| *w == 1.),
        "Plane pullback needs an authored affine chart",
    )?;
    let o: [f64; 3] = s.control_points[0][0].clone().try_into().unwrap();
    let u: [f64; 3] = s.control_points[1][0].clone().try_into().unwrap();
    let v: [f64; 3] = s.control_points[0][1].clone().try_into().unwrap();
    let u_span = s.knots_u[s.control_points.len()] - s.knots_u[s.degree_u];
    let v_span = s.knots_v[s.control_points[0].len()] - s.knots_v[s.degree_v];
    let u = std::array::from_fn::<_, 3, _>(|k| (u[k] - o[k]) / u_span);
    let v = std::array::from_fn::<_, 3, _>(|k| (v[k] - o[k]) / v_span);
    let dot = |a: [f64; 3], b: [f64; 3]| (0..3).map(|k| a[k] * b[k]).sum::<f64>();
    let mut p = c.clone();
    p.control_points = c
        .control_points
        .iter()
        .map(|p| {
            let d = std::array::from_fn(|k| p[k] - o[k]);
            vec![dot(d, u) / dot(u, u), dot(d, v) / dot(v, v)]
        })
        .collect();
    p.validate()?;
    Ok(p)
}
pub(super) struct Build {
    pub(super) m: Model,
    edges: BTreeMap<(usize, usize), usize>,
}
impl Build {
    pub(super) fn empty(tolerance_mm:f64)->Self {
        Self {m:Model(brep_topology::Model {vertices:vec![],edges:vec![],loops:vec![],faces:vec![],shells:vec![],bodies:vec![],tolerance_mm},Default::default()),edges:BTreeMap::new()}
    }
    pub(super) fn node(&mut self, p: [f64; 3]) -> usize {
        let i = self.m.vertices.len();
        self.m.vertices.push(Vertex { point: p });
        i
    }
    fn edge(&mut self, a: usize, b: usize, mut c: Curve) -> Result<usize> {
        let key = (a.min(b), a.max(b));
        if let Some(&e) = self.edges.get(&key) {
            return Ok(e);
        }
        // Binding follows explicit vertex roles, never a spatial nearest match.
        for (node, last) in [(a, false), (b, true)] {
            let p = if last {
                c.control_points.last_mut().unwrap()
            } else {
                &mut c.control_points[0]
            };
            check(
                (0..3).all(|k| {
                    (p[k] - self.m.vertices[node].point[k]).abs() <= self.m.tolerance_mm / 16.
                }),
                "Curve endpoint does not match its owned vertex role",
            )?;
            *p = self.m.vertices[node].point.to_vec();
        }
        c.validate()?;
        let e = self.m.edges.len();
        self.m.edges.push(Edge {
            vertices: [a, b],
            curve: c,
            degenerate: false,
        });
        self.edges.insert(key, e);
        Ok(e)
    }
    pub(super) fn curve(&self, a: usize, b: usize) -> Result<Curve> {
        if let Some(&i) = self.edges.get(&(a.min(b), a.max(b))) {
            let e = &self.m.edges[i];
            if e.vertices[0] == a {
                Ok(e.curve.clone())
            } else {
                e.curve.reverse()
            }
        } else {
            Curve::from_polyline(vec![
                self.m.vertices[a].point.to_vec(),
                self.m.vertices[b].point.to_vec(),
            ])
        }
    }
    pub(super) fn face(&mut self, s: Surface, segments: Vec<(usize, usize, Curve, Curve)>) -> Result<usize> {
        let mut coedges = Vec::new();
        for (a, b, c, p) in segments {
            let e = self.edge(a, b, c)?;
            coedges.push(Coedge::new(e, self.m.edges[e].vertices[0] != a, p));
        }
        let area = coedges
            .iter()
            .map(|c| {
                let a = &c.pcurve.control_points[0];
                let b = c.pcurve.control_points.last().unwrap();
                a[0] * b[1] - a[1] * b[0]
            })
            .sum::<f64>();
        check(area.is_finite() && area != 0., "No UV orientation proposal")?;
        if area < 0. {
            coedges.reverse();
            for c in &mut coedges {
                c.reversed = !c.reversed;
                c.pcurve = c.pcurve.reverse()?;
            }
        }
        let w = self.m.loops.len();
        self.m.loops.push(Loop { coedges });
        let f = self.m.faces.len();
        self.m.faces.push(Face {
            surface: s,
            outer: w,
            holes: vec![],
        });
        Ok(f)
    }
    fn plane_face(
        &mut self,
        mut s: Surface,
        path: &[usize],
        curves: &BTreeMap<(usize, usize), Curve>,
    ) -> Result<usize> {
        // Coordinate planes use physical UV clocks. Normalising a dyadic
        // edge pole by a non-power-of-two extent and rounding loses exact
        // edge/lift identity, although the surface itself is unchanged.
        for axis in 0..2 {
            let a = &s.control_points[0][0];
            let z = if axis == 0 {
                &s.control_points[1][0]
            } else {
                &s.control_points[0][1]
            };
            let components = (0..3).filter(|&k| z[k] != a[k]).collect::<Vec<_>>();
            check(
                components.len() == 1,
                "Physical UV chart needs coordinate-plane axes",
            )?;
            let length = (z[components[0]] - a[components[0]]).abs();
            if axis == 0 {
                s.knots_u = vec![0., 0., length, length];
            } else {
                s.knots_v = vec![0., 0., length, length];
            }
        }
        let mut segments = Vec::new();
        for i in 0..path.len() {
            let a = path[i];
            let b = path[(i + 1) % path.len()];
            let c = if let Some(c) = curves.get(&(a, b)) {
                c.clone()
            } else if let Some(c) = curves.get(&(b, a)) {
                c.reverse()?
            } else {
                self.curve(a, b)?
            };
            self.edge(a, b, c)?;
            let authored = self.curve(a, b)?;
            let p = projected(&authored, &s)?;
            segments.push((a, b, authored, p));
        }
        self.face(s, segments)
    }
}
/// Build a single closed proposal on the exact canonical 10 mm cube. No
/// general arbitrary-B-rep trimming is claimed. All shared curves and vertices
/// have explicit construction roles. Source plane geometry and identities are
/// retained; their UV clocks are reparameterised to physical coordinate units.
pub fn propose_corner_patch_body(
    source: &Model,
    net: &CornerPatchNetwork,
) -> Result<CornerPatchBody> {
    source.validate()?;
    check(
        source.0 == crate::cuboid([0.; 3], [10.; 3])?.0,
        "This constructor requires the canonical 10 mm cube",
    )?;
    check(
        net.caps.len() == 3
            && net.strips.len() == 3
            && net.cap_nodes == [[0, 5, 6, 4], [1, 3, 6, 5], [2, 4, 6, 3]],
        "Unsupported corner chart ownership",
    )?;
    check(
        net.caps.iter().all(|s| {
            s.degree_u == 7
                && s.degree_v == 7
                && s.control_points.len() == 8
                && s.control_points.iter().all(|r| r.len() == 8)
        }) && net.strips.iter().all(|s| {
            s.degree_u == 5
                && s.degree_v == 5
                && s.control_points.len() == 6
                && s.control_points.iter().all(|r| r.len() == 6)
        }),
        "Unsupported corner support dimensions",
    )?;
    for s in net.caps.iter().chain(&net.strips) {
        s.validate()?;
    }
    let radius = net.strips[0].control_points[0][0][0];
    let stop = net.strips[0].control_points[0][5][0];
    check(
        radius > 0. && stop > radius && stop < 10.,
        "Corner reaches another original cube corner",
    )?;
    let empty = brep_topology::Model {
        vertices: vec![],
        edges: vec![],
        loops: vec![],
        faces: vec![],
        shells: vec![],
        bodies: vec![],
        tolerance_mm: source.tolerance_mm,
    };
    let mut b = Build {
        m: Model(empty, Default::default()),
        edges: BTreeMap::new(),
    };
    let mut originals = BTreeMap::new();
    for i in 1..8 {
        originals.insert(i, b.node(source.vertices[i].point));
    }
    let mut core = [0usize; 7];
    for k in 0..3 {
        let mut p = [radius; 3];
        p[k] = 0.;
        core[k] = b.node(p);
    }
    for k in 0..3 {
        let near = net.strips[k].iso(Axis::V, 0.)?.trim(0., 0.5)?;
        core[3 + k] = b.node(
            near.control_points
                .last()
                .unwrap()
                .clone()
                .try_into()
                .unwrap(),
        );
    }
    core[6] = b.node(net.caps[0].control_points[7][7].clone().try_into().unwrap());
    let mut stops = [0usize; 3];
    let mut far = [[0usize; 2]; 3];
    for k in 0..3 {
        let mut p = [0.; 3];
        p[k] = stop;
        stops[k] = b.node(p);
        for s in 0..2 {
            far[k][s] = b.node(
                net.strips[k].control_points[s * 5][5]
                    .clone()
                    .try_into()
                    .unwrap(),
            );
        }
    }
    // Seed original edges in their original direction; root-trim the three
    // incident tails at the explicit stop plane, without proximity welding.
    let mut old_edges = BTreeMap::new();
    for (i, e) in source.edges.iter().enumerate() {
        let (a, z, c) = if e.vertices.contains(&0) {
            let other = *e.vertices.iter().find(|&&v| v != 0).unwrap();
            let axis = (0..3)
                .find(|&k| source.vertices[other].point[k] == 10.)
                .unwrap();
            let d = e.curve.domain();
            let t = d[0] + (d[1] - d[0]) * stop / 10.;
            if e.vertices[0] == 0 {
                (stops[axis], originals[&other], e.curve.trim(t, d[1])?)
            } else {
                (
                    originals[&other],
                    stops[axis],
                    e.curve.trim(d[0], d[1] - t + d[0])?,
                )
            }
        } else {
            (
                originals[&e.vertices[0]],
                originals[&e.vertices[1]],
                e.curve.clone(),
            )
        };
        old_edges.insert(i, b.edge(a, z, c)?);
    }
    let mut contacts = BTreeMap::new();
    for axis in 0..3 {
        let first = core[(axis + 2) % 3];
        let last = core[(axis + 1) % 3];
        contacts.insert((first, far[axis][0]), net.strips[axis].iso(Axis::U, 0.)?);
        contacts.insert((last, far[axis][1]), net.strips[axis].iso(Axis::U, 1.)?);
    }
    let original_node = |axis_a: usize, axis_b: Option<usize>| -> usize {
        let mut p = [0.; 3];
        p[axis_a] = 10.;
        if let Some(k) = axis_b {
            p[k] = 10.;
        }
        let i = source.vertices.iter().position(|v| v.point == p).unwrap();
        originals[&i]
    };
    for (f, sf) in source.faces.iter().enumerate() {
        let plane = (0..3)
            .find_map(|k| {
                let x = sf.surface.control_points[0][0][k];
                sf.surface
                    .control_points
                    .iter()
                    .flatten()
                    .all(|p| p[k] == x)
                    .then_some((k, x))
            })
            .unwrap();
        if plane.1 == 10. {
            let mut seg = Vec::new();
            for c in &source.loops[sf.outer].coedges {
                let oe = &source.edges[c.edge];
                let a = originals[&oe.vertices[usize::from(c.reversed)]];
                let z = originals[&oe.vertices[usize::from(!c.reversed)]];
                let curve = b.curve(a, z)?;
                seg.push((a, z, curve, c.pcurve.clone()));
            }
            check(
                b.face(sf.surface.clone(), seg)? == f,
                "Source face order changed",
            )?;
        } else {
            let a = plane.0;
            let y = (a + 1) % 3;
            let z = (a + 2) % 3;
            let path = [
                stops[y],
                original_node(y, None),
                original_node(y, Some(z)),
                original_node(z, None),
                stops[z],
                far[z][1],
                core[a],
                far[y][0],
            ];
            check(
                b.plane_face(sf.surface.clone(), &path, &contacts)? == f,
                "Source face order changed",
            )?;
        }
    }
    let mut strip_faces = [0; 3];
    for axis in 0..3 {
        let s = &net.strips[axis];
        let a = core[(axis + 2) % 3];
        let mid = core[3 + axis];
        let z = core[(axis + 1) % 3];
        let near = s.iso(Axis::V, 0.)?;
        strip_faces[axis] = b.face(
            s.clone(),
            vec![
                (a, mid, near.trim(0., 0.5)?, line([0., 0.], [0.5, 0.])?),
                (mid, z, near.trim(0.5, 1.)?, line([0.5, 0.], [1., 0.])?),
                (
                    z,
                    far[axis][1],
                    s.iso(Axis::U, 1.)?,
                    line([1., 0.], [1., 1.])?,
                ),
                (
                    far[axis][1],
                    far[axis][0],
                    s.iso(Axis::V, 1.)?.reverse()?,
                    line([1., 1.], [0., 1.])?,
                ),
                (
                    far[axis][0],
                    a,
                    s.iso(Axis::U, 0.)?.reverse()?,
                    line([0., 1.], [0., 0.])?,
                ),
            ],
        )?;
    }
    let uv = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    let mut cap_faces = [0; 3];
    for f in 0..3 {
        let path = net.cap_nodes[f].map(|i| core[i]);
        let mut seg = Vec::new();
        for e in 0..4 {
            seg.push((
                path[e],
                path[(e + 1) % 4],
                side(&net.caps[f], e)?,
                line(uv[e], uv[(e + 1) % 4])?,
            ));
        }
        cap_faces[f] = b.face(net.caps[f].clone(), seg)?;
    }
    let mut stop_faces = [0; 3];
    for axis in 0..3 {
        let r = net.strips[axis].control_points[0][5][(axis + 1) % 3];
        let mut cp = vec![vec![vec![0.; 3]; 2]; 2];
        for i in 0..2 {
            for j in 0..2 {
                cp[i][j][axis] = stop;
                cp[i][j][(axis + 1) % 3] = i as f64 * r;
                cp[i][j][(axis + 2) % 3] = j as f64 * r;
            }
        }
        let s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: cp,
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let mut curves = BTreeMap::new();
        curves.insert(
            (far[axis][0], far[axis][1]),
            net.strips[axis].iso(Axis::V, 1.)?,
        );
        stop_faces[axis] = b.plane_face(s, &[stops[axis], far[axis][0], far[axis][1]], &curves)?;
    }
    let mut incidence = BTreeMap::<usize, Vec<(usize, bool)>>::new();
    for (f, face) in b.m.faces.iter().enumerate() {
        for c in &b.m.loops[face.outer].coedges {
            incidence.entry(c.edge).or_default().push((f, c.reversed));
        }
    }
    check(
        incidence.len() == b.m.edges.len() && incidence.values().all(|v| v.len() == 2),
        "Corner boundary is not a two-use manifold",
    )?;
    let mut orientation = vec![None; b.m.faces.len()];
    for use_ in &source.shells[0].faces {
        orientation[use_.face] = Some(use_.reversed);
    }
    for _ in 0..b.m.faces.len() {
        for v in incidence.values() {
            let (a, ra) = v[0];
            let (z, rz) = v[1];
            let flip = ra == rz;
            match (orientation[a], orientation[z]) {
                (Some(x), Some(y)) => check(x ^ y == flip, "Contradictory shell orientation")?,
                (Some(x), None) => orientation[z] = Some(x ^ flip),
                (None, Some(y)) => orientation[a] = Some(y ^ flip),
                _ => {}
            }
        }
    }
    check(
        orientation.iter().all(Option::is_some),
        "Disconnected corner shell",
    )?;
    let smooth_edges = incidence
        .iter()
        .filter_map(|(&e, v)| {
            let f = [v[0].0, v[1].0];
            let cap = |x: usize| cap_faces.contains(&x);
            let strip = |x: usize| strip_faces.contains(&x);
            ((cap(f[0]) && cap(f[1]))
                || (cap(f[0]) && strip(f[1]))
                || (cap(f[1]) && strip(f[0]))
                || (strip(f[0]) && f[1] < 6)
                || (strip(f[1]) && f[0] < 6))
                .then_some(e)
        })
        .collect::<Vec<_>>();
    check(smooth_edges.len() == 15, "Wrong smooth-edge scope")?;
    b.m.shells = vec![Shell {
        closed: true,
        faces: orientation
            .into_iter()
            .enumerate()
            .map(|(face, r)| FaceUse {
                face,
                reversed: r.unwrap(),
            })
            .collect(),
    }];
    b.m.bodies = vec![Body {
        outer_shell: 0,
        inner_shells: vec![],
    }];
    b.m.inherit_topology_ids(&[source]);
    b.m.refresh_change_set(&[source]);
    let closure = crate::watertight::inspect_with_exact(&b.m, 1_000_000, 1_000_000)?;
    if closure.closure.status != crate::watertight::Closure::WithinTolerance {
        let mut failures = Vec::new();
        for u in &closure.closure.agreement.uses {
            if u.status != nurbs_core::curve_surface_agreement::Status::WithinTolerance {
                let p = &b.m.loops[u.wire].coedges[u.coedge].pcurve;
                let hull = (0..2)
                    .map(|k| {
                        [
                            p.control_points
                                .iter()
                                .map(|p| p[k])
                                .fold(f64::INFINITY, f64::min),
                            p.control_points
                                .iter()
                                .map(|p| p[k])
                                .fold(f64::NEG_INFINITY, f64::max),
                        ]
                    })
                    .collect::<Vec<_>>();
                failures.push(format!(
                    "face {}, coedge {}, {:?}, UV {:?}",
                    u.face, u.coedge, u.status, hull
                ));
            }
        }
        return Err(Error::new(
            "BREP_CORNER_PATCH_BODY",
            format!(
                "Corner closure {:?}, cells {}, uses {:?}",
                closure.closure.status, closure.closure.agreement.cells, failures
            ),
        ));
    }
    Ok(CornerPatchBody {
        model: b.m,
        cap_faces,
        strip_faces,
        stop_faces,
        smooth_edges,
    })
}
