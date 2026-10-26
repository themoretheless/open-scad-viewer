//! Topology assembly shared by the curved Boolean families: the result of
//! imprinting an intersection network onto two operands.
//!
//! A family discovers the network (special vertices on original edges, new
//! network edges between them) and the per-face UV pieces; the assembler
//! owns the identity bookkeeping (original vertices and edges, split pieces
//! of original edges, network edges), arranges each face chart through
//! `uv_regions`, classifies regions through a caller-supplied predicate,
//! trims kept surfaces to their region and hands the faces to
//! `imprint_pipeline::assemble_imprint_components`.
use crate::imprint_pipeline;
use crate::uv_regions::{self, Arrangement, Piece, normalized};
use crate::{Coedge, Edge, Face, FaceUse, Loop, Model, Vertex};
use brep_topology::CoedgeTrim;
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};
use std::collections::BTreeMap;

pub(crate) fn unsupported(message: impl Into<String>) -> Error {
    Error::new("BREP_UNSUPPORTED_OPERATION", message)
}

/// Original shell orientation of one face of a single-shell model.
pub(crate) fn face_reversed(model: &Model, face: usize) -> bool {
    model
        .shells
        .iter()
        .flat_map(|shell| shell.faces.iter())
        .find(|use_| use_.face == face)
        .map(|use_| use_.reversed)
        .unwrap_or(false)
}

/// A special vertex of the result: a hit on an original edge, a seam
/// crossing, a refinement point or a traced intersection point, with every
/// original edge it lies on as (operand, edge, edge parameter).
pub(crate) struct VRec {
    pub(crate) point: [f64; 3],
    pub(crate) on_edges: Vec<(usize, usize, f64)>,
    /// The vertex coincides with an original vertex (operand, index).
    pub(crate) orig: Option<(usize, usize)>,
}

/// A new edge of the result between two network vertices.
pub(crate) struct NetEdge {
    pub(crate) ends: [VKey; 2],
    /// Single-span (or multi-span) curve over [0, 1] from `ends[0]` to
    /// `ends[1]`.
    pub(crate) curve: Curve,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) enum VKey {
    Orig(usize, usize),
    Special(usize),
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) enum EKey {
    /// Network edge `k`.
    Net(usize),
    Orig(usize, usize),
    /// Piece `j` of split edge `(operand, edge)`.
    Piece(usize, usize, usize),
}

/// One boundary piece of an original face loop: (from, to, forward pcurve
/// over [0, 1], 3D edge key, forward runs against the 3D edge).
pub(crate) type BoundaryPiece = (VKey, VKey, Curve, EKey, bool);

/// Chart vertex table: topology key -> chart index, with its UV.
#[derive(Default)]
pub(crate) struct ChartVertices {
    keys: Vec<VKey>,
    pub(crate) uvs: Vec<[f64; 2]>,
}
impl ChartVertices {
    pub(crate) fn index(&mut self, key: VKey, uv: [f64; 2]) -> usize {
        if let Some(i) = self.keys.iter().position(|k| *k == key) {
            i
        } else {
            self.keys.push(key);
            self.uvs.push(uv);
            self.keys.len() - 1
        }
    }
    pub(crate) fn len(&self) -> usize {
        self.uvs.len()
    }
}

/// Full-use metadata for geometry that has already reached its final domain.
/// Lifts describe the authored pcurve endpoints; UV is deliberately not wrapped.
pub(crate) fn derive_full_trim(
    edge: &Edge,
    coedge: &Coedge,
    surface: &Surface,
) -> Result<CoedgeTrim> {
    let p = coedge.pcurve.domain();
    let uv = [
        coedge.pcurve.evaluate(p[0])?.point,
        coedge.pcurve.evaluate(p[1])?.point,
    ];
    let domains = [
        [
            surface.knots_u[surface.degree_u],
            surface.knots_u[surface.control_points.len()],
        ],
        [
            surface.knots_v[surface.degree_v],
            surface.knots_v[surface.control_points[0].len()],
        ],
    ];
    let mut periodic_lift = [[0; 2]; 2];
    for end in 0..2 {
        for axis in 0..2 {
            if [surface.periodic_u, surface.periodic_v][axis] {
                let d = domains[axis];
                periodic_lift[end][axis] =
                    ((uv[end][axis] - d[0]) / (d[1] - d[0])).floor() as i32;
            }
        }
    }
    Ok(CoedgeTrim {
        curve_parameter: edge.curve.domain(),
        pcurve_parameter: p,
        periodic_lift,
    })
}

pub(crate) fn attach_full_trim(
    edge: &Edge,
    coedge: &mut Coedge,
    surface: &Surface,
) -> Result<()> {
    let trim = derive_full_trim(edge, coedge, surface)?;
    *coedge = coedge.clone().with_trim(trim)?;
    Ok(())
}

pub(crate) struct Assembler<'m> {
    pub(crate) src: [&'m Model; 2],
    /// Per operand (call order): keep the regions inside the other operand.
    pub(crate) want_inside: [bool; 2],
    /// Per operand (call order): invert the kept faces' orientation.
    pub(crate) flip: [bool; 2],
    pub(crate) specials: Vec<VRec>,
    /// (operand, edge) -> sorted (t, special vertex).
    pub(crate) splits: BTreeMap<(usize, usize), Vec<(f64, usize)>>,
    pub(crate) net: Vec<NetEdge>,
    vertices: Vec<Vertex>,
    edges: Vec<Edge>,
    loops: Vec<Loop>,
    faces: Vec<Face>,
    shell: Vec<FaceUse>,
    vmap: BTreeMap<VKey, usize>,
    emap: BTreeMap<EKey, usize>,
}

impl<'m> Assembler<'m> {
    pub(crate) fn new(a: &'m Model, b: &'m Model, operation: &str) -> Result<Self> {
        let mut out = Self {
            src: [a, b],
            want_inside: [
                operation == "intersection",
                matches!(operation, "intersection" | "difference"),
            ],
            flip: [false, operation == "difference"],
            specials: Vec::new(),
            splits: BTreeMap::new(),
            net: Vec::new(),
            vertices: Vec::new(),
            edges: Vec::new(),
            loops: Vec::new(),
            faces: Vec::new(),
            shell: Vec::new(),
            vmap: BTreeMap::new(),
            emap: BTreeMap::new(),
        };
        // Authored interior trim ends are network events even when no
        // intersection was traced through them. Full 3D uses (including uses
        // with only a partial pcurve) do not split the original edge.
        for o in 0..2 {
            let model = out.src[o];
            for face in &model.faces {
                for wire in std::iter::once(&face.outer).chain(&face.holes) {
                    for coedge in &model.loops[*wire].coedges {
                        let Some(trim) = coedge.trim() else {
                            continue;
                        };
                        trim.validate()?;
                        let edge = &model.edges[coedge.edge];
                        let domain = edge.curve.domain();
                        let c = trim.curve_parameter;
                        if c[0].min(c[1]) == domain[0] && c[0].max(c[1]) == domain[1] {
                            continue;
                        }
                        for &t in &c {
                            if t <= domain[0] || t >= domain[1] {
                                continue;
                            }
                            if out.split_at(o, coedge.edge, t, 1e-12).is_some() {
                                continue;
                            }
                            let point = edge.curve.evaluate(t)?.point;
                            let id = out.special([point[0], point[1], point[2]]);
                            out.attach_edge(id, o, coedge.edge, t);
                        }
                    }
                }
            }
        }
        Ok(out)
    }

    pub(crate) fn special(&mut self, point: [f64; 3]) -> usize {
        let id = self.specials.len();
        self.specials.push(VRec {
            point,
            on_edges: Vec::new(),
            orig: None,
        });
        id
    }

    pub(crate) fn attach_edge(&mut self, id: usize, o: usize, e: usize, t: f64) {
        self.specials[id].on_edges.push((o, e, t));
        let table = self.splits.entry((o, e)).or_default();
        table.push((t, id));
        table.sort_by(|a, b| a.0.total_cmp(&b.0));
    }

    /// The special vertex already registered at (operand, edge, t), if any.
    pub(crate) fn split_at(&self, o: usize, e: usize, t: f64, tolerance: f64) -> Option<usize> {
        self.splits
            .get(&(o, e))
            .and_then(|table| table.iter().find(|(et, _)| (et - t).abs() <= tolerance))
            .map(|(_, id)| *id)
    }

    pub(crate) fn push_net(&mut self, ends: [VKey; 2], curve: Curve) -> usize {
        self.net.push(NetEdge { ends, curve });
        self.net.len() - 1
    }

    /// 3D point of a vertex key.
    pub(crate) fn point_of(&self, key: VKey) -> [f64; 3] {
        match key {
            VKey::Special(id) => self.specials[id].point,
            VKey::Orig(o, v) => self.src[o].vertices[v].point,
        }
    }

    /// Chart identity of a vertex key: specials that coincide with an
    /// original vertex share its chart slot.
    pub(crate) fn chart_key(&self, key: VKey) -> VKey {
        match key {
            VKey::Special(id) => match self.specials[id].orig {
                Some((o, v)) => VKey::Orig(o, v),
                None => key,
            },
            other => other,
        }
    }

    fn vertex(&mut self, key: VKey) -> usize {
        let key = self.chart_key(key);
        if let Some(&id) = self.vmap.get(&key) {
            return id;
        }
        let point = self.point_of(key);
        let id = self.vertices.len();
        self.vertices.push(Vertex { point });
        self.vmap.insert(key, id);
        id
    }

    /// Vertex key at one original edge parameter of operand `o`.
    pub(crate) fn vkey_at(&self, o: usize, edge: usize, t: f64) -> Result<VKey> {
        let src = &self.src[o].edges[edge];
        let domain = src.curve.domain();
        if t <= domain[0] {
            return Ok(VKey::Orig(o, src.vertices[0]));
        }
        if t >= domain[1] {
            return Ok(VKey::Orig(o, src.vertices[1]));
        }
        let table = self
            .splits
            .get(&(o, edge))
            .ok_or_else(|| unsupported("Imprint: split edge has no event table"))?;
        for &(et, id) in table {
            if (et - t).abs() <= 1e-12 {
                return Ok(VKey::Special(id));
            }
        }
        Err(unsupported(
            "Imprint: split parameter does not match an event",
        ))
    }

    pub(crate) fn edge(&mut self, key: EKey) -> Result<usize> {
        if let Some(&id) = self.emap.get(&key) {
            return Ok(id);
        }
        let edge = match key {
            EKey::Net(k) => {
                let v0 = self.vertex(self.net[k].ends[0]);
                let v1 = self.vertex(self.net[k].ends[1]);
                Edge {
                    degenerate: false,
                    vertices: [v0, v1],
                    curve: self.net[k].curve.clone(),
                }
            }
            EKey::Orig(o, e) => {
                let src = &self.src[o].edges[e];
                let v0 = self.vertex(VKey::Orig(o, src.vertices[0]));
                let v1 = self.vertex(VKey::Orig(o, src.vertices[1]));
                Edge {
                    degenerate: false,
                    vertices: [v0, v1],
                    curve: src.curve.clone(),
                }
            }
            EKey::Piece(o, e, j) => {
                let table = self
                    .splits
                    .get(&(o, e))
                    .cloned()
                    .ok_or_else(|| unsupported("Imprint: piece of an unsplit edge"))?;
                let domain = self.src[o].edges[e].curve.domain();
                let t_lo = if j == 0 { domain[0] } else { table[j - 1].0 };
                let t_hi = if j == table.len() { domain[1] } else { table[j].0 };
                let v0 = self.vertex(self.vkey_at(o, e, t_lo)?);
                let v1 = self.vertex(self.vkey_at(o, e, t_hi)?);
                Edge {
                    degenerate: false,
                    vertices: [v0, v1],
                    curve: self.src[o].edges[e].curve.trim(t_lo, t_hi)?,
                }
            }
        };
        let id = self.edges.len();
        self.edges.push(edge);
        self.emap.insert(key, id);
        Ok(id)
    }

    /// Boundary pieces of every loop of one original face of operand `o`
    /// (outer loop first), each in loop order and split at the events on
    /// its edges.
    pub(crate) fn boundary_loops(&self, o: usize, face: usize) -> Result<Vec<Vec<BoundaryPiece>>> {
        let model = self.src[o];
        let face = &model.faces[face];
        let mut out = Vec::new();
        for &wire in std::iter::once(&face.outer).chain(&face.holes) {
            let mut pieces = Vec::new();
            for coedge in &model.loops[wire].coedges {
                let e = coedge.edge;
                let src = &model.edges[e];
                let domain = src.curve.domain();
                let table = self.splits.get(&(o, e)).cloned().unwrap_or_default();
                let (c, p) = coedge
                    .trim()
                    .map_or((domain, coedge.pcurve.domain()), |trim| {
                        (trim.curve_parameter, trim.pcurve_parameter)
                    });
                let lo = c[0].min(c[1]);
                let hi = c[0].max(c[1]);
                let full_3d = lo == domain[0] && hi == domain[1];
                let mut bounds = vec![lo];
                bounds.extend(table.iter().map(|(t, _)| *t).filter(|t| *t > lo && *t < hi));
                bounds.push(hi);
                let p_at = |t: f64| {
                    let s = if coedge.reversed {
                        (c[1] - t) / (c[1] - c[0])
                    } else {
                        (t - c[0]) / (c[1] - c[0])
                    };
                    p[0] + s * (p[1] - p[0])
                };
                let trim_pcurve = |a: f64, b: f64| -> Result<Curve> {
                    if a < b {
                        Ok(normalized(coedge.pcurve.trim(a, b)?))
                    } else {
                        Ok(normalized(coedge.pcurve.trim(b, a)?.reverse()?))
                    }
                };
                let traversal_reversed = coedge.reversed ^ (c[1] < c[0]);
                if full_3d && bounds.len() == 2 {
                    let (ta, tb) = if traversal_reversed { (hi, lo) } else { (lo, hi) };
                    let (va, vb) = if traversal_reversed {
                        (src.vertices[1], src.vertices[0])
                    } else {
                        (src.vertices[0], src.vertices[1])
                    };
                    pieces.push((
                        VKey::Orig(o, va),
                        VKey::Orig(o, vb),
                        trim_pcurve(p_at(ta), p_at(tb))?,
                        EKey::Orig(o, e),
                        traversal_reversed,
                    ));
                    continue;
                }
                let order: Vec<usize> = if traversal_reversed {
                    (0..bounds.len() - 1).rev().collect()
                } else {
                    (0..bounds.len() - 1).collect()
                };
                for piece in order {
                    let t_lo = bounds[piece];
                    let t_hi = bounds[piece + 1];
                    let (ta, tb) = if traversal_reversed {
                        (t_hi, t_lo)
                    } else {
                        (t_lo, t_hi)
                    };
                    let ka = self.vkey_at(o, e, ta)?;
                    let kb = self.vkey_at(o, e, tb)?;
                    let j = table.partition_point(|(t, _)| *t <= t_lo);
                    pieces.push((
                        ka,
                        kb,
                        trim_pcurve(p_at(ta), p_at(tb))?,
                        EKey::Piece(o, e, j),
                        traversal_reversed,
                    ));
                }
            }
            out.push(pieces);
        }
        Ok(out)
    }

    /// Outer-loop boundary pieces only (faces without holes).
    pub(crate) fn boundary_pieces(&self, o: usize, face: usize) -> Result<Vec<BoundaryPiece>> {
        let mut loops = self.boundary_loops(o, face)?;
        Ok(loops.swap_remove(0))
    }

    pub(crate) fn emit_face(
        &mut self,
        o: usize,
        face: usize,
        reversed: bool,
        mut outer: Vec<Coedge>,
        mut holes: Vec<Vec<Coedge>>,
        surface: Option<Surface>,
    ) -> Result<()> {
        let surface = surface.unwrap_or_else(|| self.src[o].faces[face].surface.clone());
        for coedge in outer.iter_mut().chain(holes.iter_mut().flatten()) {
            attach_full_trim(&self.edges[coedge.edge], coedge, &surface)?;
        }
        let outer_id = self.loops.len();
        self.loops.push(Loop { coedges: outer });
        let mut hole_ids = Vec::with_capacity(holes.len());
        for hole in holes {
            hole_ids.push(self.loops.len());
            self.loops.push(Loop { coedges: hole });
        }
        let id = self.faces.len();
        self.faces.push(Face {
            surface,
            outer: outer_id,
            holes: hole_ids,
        });
        self.shell.push(FaceUse {
            face: id,
            reversed: reversed ^ self.flip[o],
        });
        Ok(())
    }

    /// Whole original face (all loops), edges remapped and split where the
    /// network divided them.
    pub(crate) fn emit_whole(&mut self, o: usize, face: usize, reversed: bool) -> Result<()> {
        let loops = self.boundary_loops(o, face)?;
        let mut built = Vec::with_capacity(loops.len());
        for pieces in loops {
            let mut out = Vec::with_capacity(pieces.len());
            for (_, _, pcurve, key, rev) in pieces {
                let edge = self.edge(key)?;
                out.push(Coedge::new(
                    edge,
                    rev,
                    pcurve,
                ));
            }
            built.push(out);
        }
        let outer = built.remove(0);
        self.emit_face(o, face, reversed, outer, built, None)
    }

    /// The face surface trimmed to the UV bounding box of a region and
    /// re-parameterized to the unit square, with the region pcurves mapped by
    /// the same affine change (exact for every rational curve). The control
    /// hull of the result becomes tight: the solid audit proves body
    /// separation from surface control points, and downstream consumers keep
    /// their unit-domain assumption.
    pub(crate) fn tight_surface(
        &self,
        o: usize,
        face: usize,
        outer: &mut [Coedge],
        holes: &mut [Vec<Coedge>],
    ) -> Result<Surface> {
        let source = &self.src[o].faces[face].surface;
        let du = [
            source.knots_u[source.degree_u],
            source.knots_u[source.knots_u.len() - 1 - source.degree_u],
        ];
        let dv = [
            source.knots_v[source.degree_v],
            source.knots_v[source.knots_v.len() - 1 - source.degree_v],
        ];
        let mut lo = [f64::INFINITY; 2];
        let mut hi = [f64::NEG_INFINITY; 2];
        for coedge in outer.iter().chain(holes.iter().flatten()) {
            for p in &coedge.pcurve.control_points {
                for k in 0..2 {
                    lo[k] = lo[k].min(p[k]);
                    hi[k] = hi[k].max(p[k]);
                }
            }
        }
        let margin = 1e-12;
        let u0 = (lo[0] - margin).max(du[0]);
        let u1 = (hi[0] + margin).min(du[1]);
        let v0 = (lo[1] - margin).max(dv[0]);
        let v1 = (hi[1] + margin).min(dv[1]);
        if !((u1 - u0).is_finite() && u1 - u0 > 1e-9) || !((v1 - v0).is_finite() && v1 - v0 > 1e-9)
        {
            return Err(unsupported("Imprint: face region collapses in UV"));
        }
        if u0 == du[0] && u1 == du[1] && v0 == dv[0] && v1 == dv[1] {
            return Ok(source.clone());
        }
        let mut surface = source.trim([u0, u1, v0, v1])?;
        surface.knots_u = surface
            .knots_u
            .iter()
            .map(|k| (k - u0) / (u1 - u0))
            .collect();
        surface.knots_v = surface
            .knots_v
            .iter()
            .map(|k| (k - v0) / (v1 - v0))
            .collect();
        for coedge in outer.iter_mut().chain(holes.iter_mut().flatten()) {
            for p in &mut coedge.pcurve.control_points {
                p[0] = ((p[0] - u0) / (u1 - u0)).clamp(0., 1.);
                p[1] = ((p[1] - v0) / (v1 - v0)).clamp(0., 1.);
            }
        }
        Ok(surface)
    }

    /// Arrange one chart and emit the regions whose sample point `keep`
    /// accepts (the sample is a surface point of the face).
    pub(crate) fn emit_chart(
        &mut self,
        o: usize,
        face: usize,
        reversed: bool,
        pieces: Vec<Piece<EKey>>,
        vertex_count: usize,
        keep: &dyn Fn([f64; 3]) -> Result<bool>,
    ) -> Result<()> {
        let arrangement = uv_regions::arrange(pieces, vertex_count)?;
        for region in &arrangement.regions {
            let sample = uv_regions::region_sample(&arrangement, region)?;
            let p = self.src[o].faces[face]
                .surface
                .evaluate(sample[0], sample[1])?
                .point;
            if !keep([p[0], p[1], p[2]])? {
                continue;
            }
            let mut outer = self.cycle_coedges(&arrangement, region.outer)?;
            let mut holes = Vec::with_capacity(region.holes.len());
            for &hole in &region.holes {
                holes.push(self.cycle_coedges(&arrangement, hole)?);
            }
            let surface = self.tight_surface(o, face, &mut outer, &mut holes)?;
            self.emit_face(o, face, reversed, outer, holes, Some(surface))?;
        }
        Ok(())
    }

    fn cycle_coedges(
        &mut self,
        arrangement: &Arrangement<EKey>,
        cycle: usize,
    ) -> Result<Vec<Coedge>> {
        let mut out = Vec::new();
        for (piece, forward) in arrangement.cycle_pieces(cycle) {
            let edge = self.edge(piece.key)?;
            let (pcurve, reversed) = if forward {
                (piece.pcurve.clone(), piece.reversed)
            } else {
                (piece.pcurve.reverse()?, !piece.reversed)
            };
            out.push(Coedge::new(
                edge,
                reversed,
                pcurve,
            ));
        }
        Ok(out)
    }

    /// Close the result: several edge-connected components become several
    /// bodies. Validation failures are reported as refusals.
    pub(crate) fn finish(self, tolerance: f64) -> Result<Model> {
        match imprint_pipeline::assemble_imprint_components(
            self.vertices,
            self.edges,
            self.loops,
            self.faces,
            self.shell,
            tolerance,
            &[self.src[0], self.src[1]],
        ) {
            Ok(model) => Ok(model),
            Err(e) if e.code == "BREP_RESOURCE_LIMIT" => Err(e),
            Err(e) => Err(unsupported(format!(
                "Imprint result failed validation: {}",
                e.message
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Shell, TopologyIds, cuboid, cylinder};

    fn set_domain(curve: &mut Curve, domain: [f64; 2]) {
        let old = curve.domain();
        for knot in &mut curve.knots {
            *knot = domain[0] + (*knot - old[0]) / (old[1] - old[0]) * (domain[1] - domain[0]);
        }
    }

    fn coedge(model: &Model, face: usize, index: usize) -> &Coedge {
        let wire = model.faces[face].outer;
        &model.loops[wire].coedges[index]
    }

    fn coedge_mut(model: &mut Model, face: usize, index: usize) -> &mut Coedge {
        let wire = model.faces[face].outer;
        &mut model.loops[wire].coedges[index]
    }

    fn scale_use(model: &mut Model, face: usize, index: usize) {
        let edge = coedge_mut(model, face, index).edge;
        set_domain(&mut model.edges[edge].curve, [2., 6.]);
        set_domain(&mut coedge_mut(model, face, index).pcurve, [10., 14.]);
    }

    fn set_trim(model: &mut Model, face: usize, index: usize, c: [f64; 2], p: [f64; 2]) {
        let coedge = coedge_mut(model, face, index);
        let lifts = coedge.trim().map_or([[0; 2]; 2], |trim| trim.periodic_lift);
        *coedge = coedge
            .clone()
            .with_trim(CoedgeTrim {
                curve_parameter: c,
                pcurve_parameter: p,
                periodic_lift: lifts,
            })
            .unwrap();
    }

    fn partial_model(reversed: bool) -> (Model, usize) {
        let mut model = cuboid([0., 0., 0.], [2., 2., 2.]).unwrap();
        let index = (0..model.loops[model.faces[0].outer].coedges.len())
            .find(|&i| coedge_mut(&mut model, 0, i).reversed == reversed)
            .unwrap();
        scale_use(&mut model, 0, index);
        set_trim(&mut model, 0, index, [3., 5.], [11., 13.]);
        (model, index)
    }

    fn shrunk_face(mut model: Model, face: usize) -> Model {
        let count = model.loops[model.faces[face].outer].coedges.len();
        for i in 0..count {
            scale_use(&mut model, face, i);
            let c = if coedge(&model, face, i).reversed {
                [4., 6.]
            } else {
                [2., 4.]
            };
            set_trim(&mut model, face, i, c, [10., 12.]);
        }
        model
    }

    fn emit_open(model: &Model, face: usize) -> Model {
        let other = cuboid([10., 10., 10.], [11., 11., 11.]).unwrap();
        let mut asm = Assembler::new(model, &other, "union").unwrap();
        let mut chart = ChartVertices::default();
        let mut pieces = Vec::new();
        let mut links = Vec::new();
        for (ka, kb, pcurve, key, reversed) in asm.boundary_pieces(0, face).unwrap() {
            let ua = pcurve.evaluate(0.).unwrap().point;
            let ub = pcurve.evaluate(1.).unwrap().point;
            let ia = chart.index(asm.chart_key(ka), [ua[0], ua[1]]);
            let ib = chart.index(asm.chart_key(kb), [ub[0], ub[1]]);
            links.push((ka, kb, ia, ib));
            pieces.push(Piece {
                v: [ia, ib],
                pcurve,
                key,
                reversed,
            });
        }
        // Close the gaps left by authored partial uses with the exact remainder
        // of each source curve. This keeps the open-assembly agreement check
        // meaningful on curved as well as planar faces.
        let wire = model.faces[face].outer;
        for i in 0..links.len() {
            let next = (i + 1) % links.len();
            let (_, from, _, ia) = links[i];
            let (to, _, ib, _) = links[next];
            let source = &model.loops[wire].coedges[i];
            let interval = if source.reversed { [2., 4.] } else { [4., 6.] };
            let mut curve = normalized(
                model.edges[source.edge]
                    .curve
                    .trim(interval[0], interval[1])
                    .unwrap(),
            );
            if source.reversed {
                curve = curve.reverse().unwrap();
            }
            let pcurve = normalized(source.pcurve.trim(12., 14.).unwrap());
            let key = asm.push_net([from, to], curve);
            pieces.push(Piece {
                v: [ia, ib],
                pcurve,
                key: EKey::Net(key),
                reversed: false,
            });
        }
        asm.emit_chart(0, face, false, pieces, chart.len(), &|_| Ok(true))
            .unwrap();
        let mut output = Model(
            brep_topology::Model {
                vertices: asm.vertices,
                edges: asm.edges,
                loops: asm.loops,
                faces: asm.faces,
                shells: vec![Shell {
                    faces: asm.shell,
                    closed: false,
                }],
                bodies: Vec::new(),
                tolerance_mm: model.tolerance_mm,
            },
            TopologyIds::default(),
        );
        output.rebuild_topology_ids();
        output
    }

    #[test]
    fn partial_forward_and_reversed_use_native_authored_parameters() {
        for reversed in [false, true] {
            let (model, index) = partial_model(reversed);
            let other = cuboid([10., 10., 10.], [11., 11., 11.]).unwrap();
            let mut asm = Assembler::new(&model, &other, "union").unwrap();
            assert_eq!(asm.splits[&(0, coedge(&model, 0, index).edge)].len(), 2);
            let pieces = asm.boundary_pieces(0, 0).unwrap();
            let (_, _, ref pcurve, key, rev) = pieces[index];
            assert_eq!(rev, reversed);
            let EKey::Piece(_, edge, _) = key else {
                panic!("partial 3D trim must produce an edge piece")
            };
            assert_eq!(edge, coedge(&model, 0, index).edge);
            let output_edge = asm.edge(key).unwrap();
            assert_eq!(asm.edges[output_edge].curve.domain(), [3., 5.]);
            let original = &coedge(&model, 0, index).pcurve;
            for (t, q) in [(0., 11.), (1., 13.)] {
                let got = pcurve.evaluate(t).unwrap().point;
                let want = original.evaluate(q).unwrap().point;
                assert_eq!(got, want, "reversed={reversed}");
            }
        }
    }

    #[test]
    fn pcurve_only_partial_use_does_not_split_3d_edge() {
        let (mut model, index) = partial_model(false);
        set_trim(&mut model, 0, index, [2., 6.], [11., 13.]);
        let other = cuboid([10., 10., 10.], [11., 11., 11.]).unwrap();
        let asm = Assembler::new(&model, &other, "union").unwrap();
        assert!(asm.splits.is_empty());
        let pieces = asm.boundary_pieces(0, 0).unwrap();
        assert!(matches!(pieces[index].3, EKey::Orig(0, _)));
    }

    #[test]
    fn authored_original_endpoint_is_reused_and_external_splits_are_ignored() {
        let (mut model, index) = partial_model(false);
        set_trim(&mut model, 0, index, [2., 5.], [10., 13.]);
        let other = cuboid([10., 10., 10.], [11., 11., 11.]).unwrap();
        let asm = Assembler::new(&model, &other, "union").unwrap();
        let pieces = asm.boundary_pieces(0, 0).unwrap();
        assert!(matches!(pieces[index].0, VKey::Orig(0, _)));
        assert!(matches!(pieces[index].1, VKey::Special(_)));

        set_trim(&mut model, 0, index, [3., 5.], [11., 13.]);
        let edge = coedge(&model, 0, index).edge;
        let mut asm = Assembler::new(&model, &other, "union").unwrap();
        for t in [2.5, 4., 5.5] {
            let point = model.edges[edge].curve.evaluate(t).unwrap().point;
            let id = asm.special([point[0], point[1], point[2]]);
            asm.attach_edge(id, 0, edge, t);
        }
        let pieces = asm.boundary_pieces(0, 0).unwrap();
        let selected: Vec<_> = pieces
            .iter()
            .filter(|piece| matches!(piece.3, EKey::Piece(0, e, _) if e == edge))
            .collect();
        assert_eq!(selected.len(), 2);
        assert!(matches!(selected[0].3, EKey::Piece(0, _, 2)));
        assert!(matches!(selected[1].3, EKey::Piece(0, _, 3)));
    }

    #[test]
    fn overlapping_partial_uses_share_physical_edge_pieces() {
        let (mut model, index) = partial_model(false);
        let wire = model.faces[0].outer;
        let mut second = model.loops[wire].coedges[index].clone();
        second = second
            .with_trim(CoedgeTrim {
                curve_parameter: [4., 6.],
                pcurve_parameter: [12., 14.],
                periodic_lift: [[0; 2]; 2],
            })
            .unwrap();
        model.loops[wire].coedges.push(second);
        let other = cuboid([10., 10., 10.], [11., 11., 11.]).unwrap();
        let asm = Assembler::new(&model, &other, "union").unwrap();
        let pieces = asm.boundary_pieces(0, 0).unwrap();
        let keys: Vec<_> = pieces.iter().map(|piece| piece.3).collect();
        let common: Vec<_> = keys.iter().filter(|key| keys.iter().filter(|k| k == key).count() > 1).collect();
        assert_eq!(common.len(), 2);
        assert!(matches!(common[0], EKey::Piece(0, _, _)));
    }

    #[test]
    fn emit_chart_attaches_full_trims_and_open_output_validates_continuously() {
        let model = shrunk_face(cuboid([0., 0., 0.], [2., 2., 2.]).unwrap(), 0);
        let output = emit_open(&model, 0);
        for coedge in &output.loops[output.faces[0].outer].coedges {
            let trim = coedge.trim().unwrap();
            assert_eq!(trim.curve_parameter, output.edges[coedge.edge].curve.domain());
            assert_eq!(trim.pcurve_parameter, coedge.pcurve.domain());
        }
        output.validate_continuous(10000).unwrap();
    }

    #[test]
    fn same_chart_periodic_emit_and_lifts_do_not_wrap_or_follow_reversal() {
        let model = shrunk_face(cylinder(2., 4.).unwrap(), 0);
        let output = emit_open(&model, 0);
        output.validate_continuous(10000).unwrap();

        let source = cylinder(2., 4.).unwrap();
        let coedge = &source.loops[source.faces[0].outer].coedges[0];
        let mut pcurve = coedge.pcurve.clone();
        for point in &mut pcurve.control_points {
            point[0] += 1.25;
        }
        let forward = Coedge::new(coedge.edge, false, pcurve.clone());
        let reversed = Coedge::new(coedge.edge, true, pcurve);
        let a = derive_full_trim(&source.edges[coedge.edge], &forward, &source.faces[0].surface)
            .unwrap();
        let b = derive_full_trim(&source.edges[coedge.edge], &reversed, &source.faces[0].surface)
            .unwrap();
        assert_eq!(a.periodic_lift, b.periodic_lift);
        assert_eq!(forward.pcurve.control_points[0][0], coedge.pcurve.control_points[0][0] + 1.25);
    }
}
