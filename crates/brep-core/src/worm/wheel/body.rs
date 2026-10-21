//! Exact actual-control sewing of packed side charts and two planar caps.
use super::WheelCaps;
use crate::{
    Body, Coedge, Edge, Error, Face, FaceUse, Loop, Model, Result, Shell, TopologyIds, Vertex,
};
use nurbs_core::{curve::Curve, surface::Surface};
use std::collections::{BTreeMap, BTreeSet};
fn refuse(s: &str) -> Error {
    Error::new("BREP_WHEEL_BODY_UNPROVEN", s)
}
fn reverse(c: &Curve) -> Curve {
    let mut r = c.clone();
    r.control_points.reverse();
    r.weights.reverse();
    r.knots = c.knots.iter().rev().map(|x| -x).collect();
    r
}
fn same(a: &Curve, b: &Curve) -> bool {
    a.degree == b.degree
        && a.knots == b.knots
        && a.control_points == b.control_points
        && a.weights == b.weights
        && !a.periodic
        && !b.periodic
}
fn boundary(s: &Surface, side: usize) -> (Curve, Curve) {
    let nu = s.control_points.len();
    let nv = s.control_points[0].len();
    let u = [s.knots_u[s.degree_u], s.knots_u[nu]];
    let v = [s.knots_v[s.degree_v], s.knots_v[nv]];
    let mut c = if side == 0 || side == 2 {
        let j = if side == 0 { 0 } else { nv - 1 };
        Curve {
            degree: s.degree_u,
            knots: s.knots_u.clone(),
            periodic: false,
            control_points: s.control_points.iter().map(|r| r[j].clone()).collect(),
            weights: s.weights.iter().map(|r| r[j]).collect(),
        }
    } else {
        let i = if side == 1 { nu - 1 } else { 0 };
        Curve {
            degree: s.degree_v,
            knots: s.knots_v.clone(),
            periodic: false,
            control_points: s.control_points[i].clone(),
            weights: s.weights[i].clone(),
        }
    };
    if side >= 2 {
        c = reverse(&c);
    }
    let uv = [[u[0], v[0]], [u[1], v[0]], [u[1], v[1]], [u[0], v[1]]];
    (
        c,
        crate::line(uv[side].to_vec(), uv[(side + 1) % 4].to_vec()),
    )
}
struct Ledger {
    vertices: Vec<Vertex>,
    vertex_map: BTreeMap<[u64; 3], usize>,
    edges: Vec<Edge>,
    candidates: BTreeMap<[usize; 2], Vec<usize>>,
    uses: Vec<Vec<(usize, bool)>>,
    loops: Vec<Loop>,
    faces: Vec<Face>,
    reversed: Vec<bool>,
}
impl Ledger {
    fn vertex(&mut self, p: &[f64]) -> usize {
        let key = std::array::from_fn(|i| if p[i] == 0. { 0 } else { p[i].to_bits() });
        let n = self.vertices.len();
        *self.vertex_map.entry(key).or_insert_with(|| {
            self.vertices.push(Vertex {
                point: [p[0], p[1], p[2]],
            });
            n
        })
    }
    fn face(
        &mut self,
        surface: Surface,
        curves: Vec<Curve>,
        pcurves: Vec<Curve>,
        reversed: bool,
    ) -> Result<()> {
        surface.validate()?;
        if curves.len() != pcurves.len() || curves.len() < 2 {
            return Err(refuse("Incomplete face boundary ledger"));
        }
        let face = self.faces.len();
        let mut coedges = Vec::new();
        for (curve, pcurve) in curves.into_iter().zip(pcurves) {
            curve.validate()?;
            pcurve.validate()?;
            let vs = [
                self.vertex(&curve.control_points[0]),
                self.vertex(curve.control_points.last().unwrap()),
            ];
            if vs[0] == vs[1] {
                return Err(refuse("Collapsed wheel boundary"));
            }
            let pair = if vs[0] < vs[1] { vs } else { [vs[1], vs[0]] };
            let mut found = None;
            for &id in self.candidates.get(&pair).into_iter().flatten() {
                let rev = self.edges[id].vertices != vs;
                let candidate = if rev { reverse(&curve) } else { curve.clone() };
                if same(&self.edges[id].curve, &candidate) {
                    if self.uses[id].len() != 1 {
                        return Err(refuse("Nonmanifold wheel edge"));
                    }
                    let (other, other_rev) = self.uses[id][0];
                    if (other_rev ^ self.reversed[other]) == (rev ^ reversed) {
                        return Err(refuse("Inconsistent wheel face orientation"));
                    }
                    found = Some((id, rev));
                    break;
                }
            }
            let (edge, rev) = found.unwrap_or_else(|| {
                let id = self.edges.len();
                self.edges.push(Edge {
                    vertices: vs,
                    curve,
                    degenerate: false,
                });
                self.uses.push(Vec::new());
                self.candidates.entry(pair).or_default().push(id);
                (id, false)
            });
            self.uses[edge].push((face, rev));
            coedges.push(Coedge {
                edge,
                reversed: rev,
                pcurve,
            });
        }
        let outer = self.loops.len();
        self.loops.push(Loop { coedges });
        self.faces.push(Face {
            surface,
            outer,
            holes: vec![],
        });
        self.reversed.push(reversed);
        Ok(())
    }
    fn closed(&self) -> Result<()> {
        if self.uses.iter().any(|u| u.len() != 2) {
            return Err(refuse("Unpaired complete wheel boundaries"));
        }
        let mut stars = vec![Vec::new(); self.vertices.len()];
        for (e, edge) in self.edges.iter().enumerate() {
            for &v in &edge.vertices {
                stars[v].push(e);
            }
        }
        for star in stars {
            let mut link = BTreeMap::<usize, Vec<usize>>::new();
            for edge in star {
                let (a, b) = (self.uses[edge][0].0, self.uses[edge][1].0);
                link.entry(a).or_default().push(b);
                link.entry(b).or_default().push(a);
            }
            let mut visited = BTreeSet::new();
            let mut pending = link.keys().next().copied().into_iter().collect::<Vec<_>>();
            while let Some(f) = pending.pop() {
                if visited.insert(f) {
                    pending.extend(link[&f].iter().copied());
                }
            }
            if link.len() < 2 || link.values().any(|n| n.len() != 2) || visited.len() != link.len()
            {
                return Err(refuse("Nonmanifold wheel vertex link"));
            }
        }
        let mut neighbors = vec![Vec::new(); self.faces.len()];
        for u in &self.uses {
            let (a, b) = (u[0].0, u[1].0);
            neighbors[a].push(b);
            neighbors[b].push(a);
        }
        let mut visited = BTreeSet::new();
        let mut pending = vec![0];
        while let Some(f) = pending.pop() {
            if visited.insert(f) {
                pending.extend(neighbors[f].iter().copied());
            }
        }
        if visited.len() != self.faces.len() {
            return Err(refuse("Disconnected wheel shell"));
        }
        Ok(())
    }
}
/// Complete physical planar-capped topology with original side and cap owners.
/// Combinatorial closure is distinct from a geometric-solid qualification.
pub struct WheelBodyCandidate {
    source: WheelCaps,
    model: Model,
    closure: crate::watertight::QualifiedReport,
}
impl WheelBodyCandidate {
    pub fn source(&self) -> &WheelCaps {
        &self.source
    }
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn solid_geometry_certified(&self) -> bool {
        false
    }
    /// Original-to-actual side error, not a complete ideal finite-body bound.
    pub fn wall_error_upper(&self) -> f64 {
        self.source
            .sides
            .grids()
            .iter()
            .map(|g| g.certificate().error_upper)
            .fold(0., f64::max)
    }
    pub fn ideal_finite_body_error_upper(&self) -> Option<f64> {
        None
    }
    pub fn closure(&self) -> &crate::watertight::QualifiedReport {
        &self.closure
    }
    pub fn boundary_agreement(
        &self,
        max_cells: usize,
    ) -> Result<crate::boundary_agreement::Report> {
        crate::boundary_agreement::verify(&self.model, max_cells)
    }
}
pub fn wheel_body_candidate(source: WheelCaps) -> Result<WheelBodyCandidate> {
    let count = source
        .sides
        .grids()
        .iter()
        .map(|g| g.packed_surfaces().len())
        .sum::<usize>()
        + 2;
    if count > crate::MAX_FACES {
        return Err(refuse("Packed wheel exceeds topology face limit"));
    }
    let mut ledger = Ledger {
        vertices: vec![],
        vertex_map: BTreeMap::new(),
        edges: vec![],
        candidates: BTreeMap::new(),
        uses: vec![],
        loops: vec![],
        faces: vec![],
        reversed: vec![],
    };
    for (i, grid) in source.sides.grids().iter().enumerate() {
        for s in grid.packed_surfaces() {
            let mut s = s.clone();
            for p in s.control_points.iter_mut().flatten() {
                let z = p[2];
                p[2] = p[1];
                p[1] = -z;
            }
            let (curves, pcurves): (Vec<_>, Vec<_>) = (0..4).map(|side| boundary(&s, side)).unzip();
            ledger.face(s, curves, pcurves, i % 6 == 3 || i % 6 == 4)?;
        }
    }
    for (end, cap) in source.caps.iter().enumerate() {
        if cap.boundaries.len() != 1 || cap.pcurves.len() != 1 {
            return Err(refuse("Wheel caps need one complete outer contour"));
        }
        ledger.face(
            cap.surface.clone(),
            cap.boundaries[0].clone(),
            cap.pcurves[0].clone(),
            end == 0,
        )?;
    }
    ledger.closed()?;
    let faces = (0..ledger.faces.len())
        .map(|face| FaceUse {
            face,
            reversed: ledger.reversed[face],
        })
        .collect();
    let mut model = Model(
        brep_topology::Model {
            vertices: ledger.vertices,
            edges: ledger.edges,
            loops: ledger.loops,
            faces: ledger.faces,
            shells: vec![Shell {
                faces,
                closed: true,
            }],
            bodies: vec![Body {
                outer_shell: 0,
                inner_shells: vec![],
            }],
            tolerance_mm: source.tolerance,
        },
        TopologyIds::default(),
    );
    model.inherit_topology_ids(&[]);
    model.validate()?;
    let closure = crate::watertight::inspect_with_exact(&model, 1000000, 1000000)?;
    if closure.closure.status != crate::watertight::Closure::WithinTolerance {
        return Err(refuse("Complete wheel boundary agreement remains unproven"));
    }
    Ok(WheelBodyCandidate {
        source,
        model,
        closure,
    })
}
