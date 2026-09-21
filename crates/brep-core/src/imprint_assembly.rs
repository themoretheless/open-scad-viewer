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
    pub(crate) fn new(a: &'m Model, b: &'m Model, operation: &str) -> Self {
        Self {
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
        }
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
        if t <= 0. {
            return Ok(VKey::Orig(o, src.vertices[0]));
        }
        if t >= 1. {
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
                let t_lo = if j == 0 { 0. } else { table[j - 1].0 };
                let t_hi = if j == table.len() { 1. } else { table[j].0 };
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
                let table = self.splits.get(&(o, e)).cloned().unwrap_or_default();
                if table.is_empty() {
                    let src = &model.edges[e];
                    let (va, vb) = if coedge.reversed {
                        (src.vertices[1], src.vertices[0])
                    } else {
                        (src.vertices[0], src.vertices[1])
                    };
                    pieces.push((
                        VKey::Orig(o, va),
                        VKey::Orig(o, vb),
                        normalized(coedge.pcurve.clone()),
                        EKey::Orig(o, e),
                        coedge.reversed,
                    ));
                    continue;
                }
                let count = table.len() + 1;
                let order: Vec<usize> = if coedge.reversed {
                    (0..count).rev().collect()
                } else {
                    (0..count).collect()
                };
                for j in order {
                    let t_lo = if j == 0 { 0. } else { table[j - 1].0 };
                    let t_hi = if j == count - 1 { 1. } else { table[j].0 };
                    let (tau_a, tau_b) = if coedge.reversed {
                        (1. - t_hi, 1. - t_lo)
                    } else {
                        (t_lo, t_hi)
                    };
                    let (ka, kb) = if coedge.reversed {
                        (self.vkey_at(o, e, t_hi)?, self.vkey_at(o, e, t_lo)?)
                    } else {
                        (self.vkey_at(o, e, t_lo)?, self.vkey_at(o, e, t_hi)?)
                    };
                    pieces.push((
                        ka,
                        kb,
                        normalized(coedge.pcurve.trim(tau_a, tau_b)?),
                        EKey::Piece(o, e, j),
                        coedge.reversed,
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
        outer: Vec<Coedge>,
        holes: Vec<Vec<Coedge>>,
        surface: Option<Surface>,
    ) {
        let outer_id = self.loops.len();
        self.loops.push(Loop { coedges: outer });
        let mut hole_ids = Vec::with_capacity(holes.len());
        for hole in holes {
            hole_ids.push(self.loops.len());
            self.loops.push(Loop { coedges: hole });
        }
        let id = self.faces.len();
        self.faces.push(Face {
            surface: surface.unwrap_or_else(|| self.src[o].faces[face].surface.clone()),
            outer: outer_id,
            holes: hole_ids,
        });
        self.shell.push(FaceUse {
            face: id,
            reversed: reversed ^ self.flip[o],
        });
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
                out.push(Coedge {
                    edge,
                    reversed: rev,
                    pcurve,
                });
            }
            built.push(out);
        }
        let outer = built.remove(0);
        self.emit_face(o, face, reversed, outer, built, None);
        Ok(())
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
            self.emit_face(o, face, reversed, outer, holes, Some(surface));
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
            out.push(Coedge {
                edge,
                reversed,
                pcurve,
            });
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
