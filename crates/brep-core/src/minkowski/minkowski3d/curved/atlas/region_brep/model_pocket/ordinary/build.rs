use super::*;
use crate::TopologyIds;
use brep_topology::{Body, Coedge, Edge, Face, FaceUse, Loop, Shell, Vertex};
use poly::Poly;
#[derive(Clone, Copy)]
enum Carrier {
    Source(usize),
    Floor,
    Wall(u32),
}
struct Builder<'a> {
    pocket: &'a AlgebraicModelPocket,
    arcs: &'a BTreeMap<u32, arcs::ArcData>,
    uv: &'a BTreeMap<u32, [Q; 2]>,
    model: Model,
    vertices: BTreeMap<PocketVertex, usize>,
    edges: BTreeMap<PocketEdge, usize>,
    source_slots: BTreeSet<usize>,
    groups: Vec<Vec<FaceUse>>,
    work: Work,
}
fn reverse(mut c: crate::Curve) -> crate::Curve {
    let [a, b] = c.domain();
    c.knots = c.knots.iter().rev().map(|t| a + b - t).collect();
    c.control_points.reverse();
    c.weights.reverse();
    c
}
fn line(a: Vec<f64>, b: Vec<f64>) -> crate::Curve {
    crate::Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![a, b],
        weights: vec![1.; 2],
        periodic: false,
    }
}
impl Builder<'_> {
    fn vertex(&mut self, key: PocketVertex) -> Result<usize> {
        if let Some(id) = self.vertices.get(&key) {
            return Ok(*id);
        }
        let point = match key {
            PocketVertex::Source(v) => self.pocket.original.vertices[v].point,
            PocketVertex::Top(v) | PocketVertex::Bottom(v) => {
                let uv = self.uv.get(&v).ok_or_else(|| {
                    fail("ordinary Model vertex has no exact rational arc witness")
                })?;
                let p = self.pocket.point(
                    usize::from(matches!(key, PocketVertex::Bottom(_))),
                    &uv[0],
                    &uv[1],
                )?;
                [
                    poly::exact(&p[0])?,
                    poly::exact(&p[1])?,
                    poly::exact(&p[2])?,
                ]
            }
        };
        let id = self.model.vertices.len();
        self.model.vertices.push(Vertex { point });
        self.vertices.insert(key, id);
        Ok(id)
    }
    fn edge(&mut self, key: PocketEdge) -> Result<usize> {
        if let Some(id) = self.edges.get(&key) {
            return Ok(*id);
        }
        let vertices = edge_vertices(
            &self.pocket.original,
            &self.pocket.certificate.partition,
            &key,
        )?;
        let vertices = [
            self.vertex(vertices[0].clone())?,
            self.vertex(vertices[1].clone())?,
        ];
        let (curve, degenerate) = match key {
            PocketEdge::Source(id) => {
                let e = &self.pocket.original.edges[id];
                (e.curve.clone(), e.degenerate)
            }
            PocketEdge::Top(id) | PocketEdge::Bottom(id) => {
                let a = &self.arcs[&id];
                (
                    poly::curve(
                        &a.lifted[usize::from(matches!(key, PocketEdge::Bottom(_)))],
                        &a.breaks,
                    )?,
                    false,
                )
            }
            PocketEdge::Vertical(_) => (
                line(
                    self.model.vertices[vertices[0]].point.to_vec(),
                    self.model.vertices[vertices[1]].point.to_vec(),
                ),
                false,
            ),
        };
        let id = self.model.edges.len();
        self.model.edges.push(Edge {
            vertices,
            curve,
            degenerate,
        });
        self.edges.insert(key, id);
        Ok(id)
    }
    fn source_loop(&mut self, id: usize) -> Result<usize> {
        let mut c = self.pocket.original.loops[id].clone();
        for use_ in &mut c.coedges {
            use_.edge = self.edge(PocketEdge::Source(use_.edge))?;
        }
        let id = self.model.loops.len();
        self.model.loops.push(c);
        Ok(id)
    }
    fn mapped_pcurve(&self, id: u32, carrier: usize) -> Result<crate::Curve> {
        let mut c = self.arcs[&id].curve.clone();
        let s = if carrier == 0 {
            &self.pocket.original.faces[self.pocket.certificate.source_face].surface
        } else {
            &self.pocket.floor
        };
        let maps = &self.pocket.certificate.normalized_parameter_maps[carrier];
        let ranges = [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ];
        for cp in &mut c.control_points {
            let uv = [Q::from_float(cp[0]).unwrap(), Q::from_float(cp[1]).unwrap()];
            for axis in 0..2 {
                let map = maps[axis].each_ref().map(roots::decode);
                let normalized =
                    map[0].clone()? + map[1].clone()? * &uv[0] + map[2].clone()? * &uv[1];
                cp[axis] = poly::exact(
                    &(Q::from_float(ranges[axis][0]).unwrap()
                        + normalized
                            * (Q::from_float(ranges[axis][1]).unwrap()
                                - Q::from_float(ranges[axis][0]).unwrap())),
                )?;
            }
        }
        Ok(c)
    }
    fn pcurve(&self, key: &PocketEdge, carrier: Carrier) -> Result<crate::Curve> {
        match (carrier, key) {
            (Carrier::Source(_), PocketEdge::Top(id)) => self.mapped_pcurve(*id, 0),
            (Carrier::Floor, PocketEdge::Bottom(id)) => self.mapped_pcurve(*id, 1),
            (Carrier::Wall(w), PocketEdge::Top(id)) if w == *id => {
                let a = &self.arcs[id];
                Ok(line(
                    vec![a.breaks[0], 0.],
                    vec![*a.breaks.last().unwrap(), 0.],
                ))
            }
            (Carrier::Wall(w), PocketEdge::Bottom(id)) if w == *id => {
                let a = &self.arcs[id];
                Ok(line(
                    vec![a.breaks[0], 1.],
                    vec![*a.breaks.last().unwrap(), 1.],
                ))
            }
            (Carrier::Wall(w), PocketEdge::Vertical(v)) => {
                let a = &self.arcs[&w];
                let e = &self.pocket.certificate.partition.edges[w as usize];
                let u = if *v == e.endpoints[0] {
                    a.breaks[0]
                } else if *v == e.endpoints[1] {
                    *a.breaks.last().unwrap()
                } else {
                    return Err(fail("ordinary Model wall vertical edge is not incident"));
                };
                Ok(line(vec![u, 0.], vec![u, 1.]))
            }
            _ => Err(fail("ordinary Model face has an incompatible edge carrier")),
        }
    }
    fn native_loop(
        &mut self,
        cycle: &[PocketEdgeUse],
        carrier: Carrier,
        reversed: bool,
    ) -> Result<usize> {
        let mut uses = cycle.to_vec();
        if reversed {
            uses.reverse();
            for u in &mut uses {
                u.positive ^= true;
            }
        }
        let mut coedges = vec![];
        for u in uses {
            let mut pc = self.pcurve(&u.edge, carrier)?;
            if !u.positive {
                pc = reverse(pc);
            }
            coedges.push(Coedge::new(self.edge(u.edge)?, !u.positive, pc));
        }
        let id = self.model.loops.len();
        self.model.loops.push(Loop { coedges });
        Ok(id)
    }
    fn wall_surface(&mut self, id: u32) -> Result<Surface> {
        let a = &self.arcs[&id];
        let mut pieces = vec![];
        for k in 0..a.uv.len() {
            let top = &a.lifted[0][k];
            let bottom = &a.lifted[1][k];
            let mut c = vec![];
            for axis in 0..3 {
                c.push(poly::mul(&top[axis], &bottom[3], &mut self.work)?);
            }
            for axis in 0..3 {
                c.push(poly::mul(&bottom[axis], &top[3], &mut self.work)?);
            }
            c.push(poly::mul(&top[3], &bottom[3], &mut self.work)?);
            pieces.push(poly::reduced(c)?);
        }
        let degree = pieces
            .iter()
            .flatten()
            .map(|p| p.len().saturating_sub(1))
            .max()
            .unwrap()
            .max(1);
        let mut cp: Vec<Vec<Vec<f64>>> = vec![];
        let mut weights: Vec<Vec<f64>> = vec![];
        let mut knots = vec![a.breaks[0]; degree + 1];
        for (k, p) in pieces.iter().enumerate() {
            let (points, w) = poly::float_net(p, degree)?;
            let mut rows = points
                .into_iter()
                .map(|p| vec![p[..3].to_vec(), p[3..6].to_vec()])
                .collect::<Vec<_>>();
            let mut row_weights = w.into_iter().map(|w| vec![w, w]).collect::<Vec<_>>();
            if k > 0 {
                if cp.last() != rows.first() {
                    return Err(fail(
                        "ordinary Model ruled wall spans have different endpoints",
                    ));
                }
                let gauge = Q::from_float(weights.last().unwrap()[0]).unwrap()
                    / Q::from_float(row_weights[0][0]).unwrap();
                for row in &mut row_weights {
                    for w in row {
                        *w = poly::exact(&(Q::from_float(*w).unwrap() * &gauge))?;
                    }
                }
                rows.remove(0);
                row_weights.remove(0);
                knots.extend(vec![a.breaks[k]; degree]);
            }
            cp.extend(rows);
            weights.extend(row_weights);
        }
        knots.extend(vec![*a.breaks.last().unwrap(); degree + 1]);
        Ok(Surface {
            degree_u: degree,
            degree_v: 1,
            knots_u: knots,
            knots_v: vec![0., 0., 1., 1.],
            control_points: cp,
            weights,
            periodic_u: false,
            periodic_v: false,
        })
    }
    fn face(
        &mut self,
        carrier: Carrier,
        reversed: bool,
        cycles: &[Vec<PocketEdgeUse>],
        source_copy: Option<usize>,
        original_slot: Option<usize>,
        group: usize,
    ) -> Result<usize> {
        let surface = match carrier {
            Carrier::Source(f) => self.pocket.original.faces[f].surface.clone(),
            Carrier::Floor => self.pocket.floor.clone(),
            Carrier::Wall(id) => self.wall_surface(id)?,
        };
        let mut loops = vec![];
        let mut prefix = 0;
        if let Some(f) = source_copy {
            let face = self.pocket.original.faces[f].clone();
            for l in std::iter::once(face.outer).chain(face.holes) {
                loops.push(self.source_loop(l)?);
            }
            prefix = loops.len();
        }
        for cycle in &cycles[prefix..] {
            loops.push(self.native_loop(cycle, carrier, reversed)?);
        }
        let (outer, holes) = if source_copy.is_some() && cycles.len() == prefix {
            (loops[0], loops[1..].to_vec())
        } else {
            let wires = loops
                .iter()
                .map(|l| {
                    self.model.loops[*l]
                        .coedges
                        .iter()
                        .map(|c| c.pcurve.clone())
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            let (orientation, depth) =
                crate::profile_certificate::topology_prisms::original_wires(&wires)?;
            let outer = orientation
                .iter()
                .zip(&depth)
                .enumerate()
                .filter(|(_, (o, d))| **o == 1 && **d == 0)
                .map(|(i, _)| i)
                .collect::<Vec<_>>();
            if outer.len() != 1
                || orientation
                    .iter()
                    .zip(&depth)
                    .enumerate()
                    .any(|(i, (o, d))| i != outer[0] && (*o != -1 || *d != 1))
            {
                return Err(fail(
                    "ordinary Model output face has invalid exact UV nesting/orientation",
                ));
            }
            (
                loops[outer[0]],
                loops
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| *i != outer[0])
                    .map(|(_, l)| *l)
                    .collect(),
            )
        };
        let face = Face {
            surface,
            outer,
            holes,
        };
        let id = if let Some(id) = original_slot {
            if !self.source_slots.insert(id) {
                return Err(fail("ordinary Model source face was assigned twice"));
            }
            self.model.faces[id] = face;
            id
        } else {
            let id = self.model.faces.len();
            self.model.faces.push(face);
            id
        };
        self.groups[group].push(FaceUse { face: id, reversed });
        Ok(id)
    }
}
pub(super) fn build(
    pocket: &AlgebraicModelPocket,
    arcs: &BTreeMap<u32, arcs::ArcData>,
    uv: &BTreeMap<u32, [Q; 2]>,
    removed: bool,
) -> Result<Model> {
    let mut model = Model::empty(pocket.original.tolerance_mm)?;
    if !removed {
        model.faces = pocket.original.faces.clone();
    }
    let mut b = Builder {
        pocket,
        arcs,
        uv,
        model,
        vertices: BTreeMap::new(),
        edges: BTreeMap::new(),
        source_slots: BTreeSet::new(),
        groups: vec![
            vec![];
            if removed {
                pocket.certificate.components.len()
            } else {
                1
            }
        ],
        work: Work::default(),
    };
    if !removed {
        for f in &pocket.certificate.faces {
            match &f.kind {
                PocketFaceKind::Source { face, reversed } => {
                    b.face(
                        Carrier::Source(*face),
                        *reversed,
                        &f.loops,
                        Some(*face),
                        Some(*face),
                        0,
                    )?;
                }
                PocketFaceKind::Remainder {
                    source_face,
                    outside_triangle,
                    reversed,
                    ..
                } => {
                    b.face(
                        Carrier::Source(*source_face),
                        *reversed,
                        &f.loops,
                        if *outside_triangle {
                            Some(*source_face)
                        } else {
                            None
                        },
                        if *outside_triangle {
                            Some(*source_face)
                        } else {
                            None
                        },
                        0,
                    )?;
                }
                PocketFaceKind::Floor { reversed, .. } => {
                    b.face(Carrier::Floor, *reversed, &f.loops, None, None, 0)?;
                }
                PocketFaceKind::Wall { domain_edge } => {
                    let positive = f.loops[0][0].positive;
                    b.face(
                        Carrier::Wall(*domain_edge),
                        !positive,
                        &f.loops,
                        None,
                        None,
                        0,
                    )?;
                }
            }
        }
        if b.source_slots.len() != pocket.original.faces.len() {
            return Err(fail(
                "ordinary Model conversion did not preserve every original face slot",
            ));
        }
    } else {
        let mut owner = BTreeMap::new();
        for f in &pocket.certificate.faces {
            if let PocketFaceKind::Floor {
                component,
                reversed,
                ..
            } = &f.kind
            {
                let top = f
                    .loops
                    .iter()
                    .map(|c| {
                        c.iter()
                            .map(|e| {
                                let mut e = e.clone();
                                if let PocketEdge::Bottom(id) = e.edge {
                                    owner.insert(id, *component as usize);
                                    e.edge = PocketEdge::Top(id);
                                }
                                e
                            })
                            .collect()
                    })
                    .collect::<Vec<_>>();
                let source_use = pocket.original.shells[0]
                    .faces
                    .iter()
                    .find(|u| u.face == pocket.certificate.source_face)
                    .unwrap();
                b.face(
                    Carrier::Source(pocket.certificate.source_face),
                    source_use.reversed,
                    &top,
                    None,
                    None,
                    *component as usize,
                )?;
                let bottom = f
                    .loops
                    .iter()
                    .map(|c| {
                        c.iter()
                            .rev()
                            .map(|e| {
                                let mut e = e.clone();
                                e.positive ^= true;
                                e
                            })
                            .collect()
                    })
                    .collect::<Vec<_>>();
                b.face(
                    Carrier::Floor,
                    !*reversed,
                    &bottom,
                    None,
                    None,
                    *component as usize,
                )?;
            }
        }
        for f in &pocket.certificate.faces {
            if let PocketFaceKind::Wall { domain_edge } = f.kind {
                let cycles: Vec<Vec<PocketEdgeUse>> = f
                    .loops
                    .iter()
                    .map(|c| {
                        c.iter()
                            .rev()
                            .map(|e| {
                                let mut e = e.clone();
                                e.positive ^= true;
                                e
                            })
                            .collect()
                    })
                    .collect::<Vec<_>>();
                let positive = cycles[0]
                    .iter()
                    .find(|e| matches!(e.edge, PocketEdge::Top(_)))
                    .unwrap()
                    .positive;
                b.face(
                    Carrier::Wall(domain_edge),
                    !positive,
                    &cycles,
                    None,
                    None,
                    owner[&domain_edge],
                )?;
            }
        }
    }
    for faces in b.groups {
        let s = b.model.shells.len();
        b.model.shells.push(Shell {
            faces,
            closed: true,
        });
        b.model.bodies.push(Body {
            outer_shell: s,
            inner_shells: vec![],
        });
    }
    b.model.rebuild_topology_ids();
    b.model.inherit_topology_ids(&[&pocket.original]);
    b.model.validate()?;
    Ok(b.model)
}
