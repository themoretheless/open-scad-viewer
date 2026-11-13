//! Boolean assembly from an intersection network supplied by the caller.
//!
//! A curved family that lives outside this crate (for example a certified
//! tracer in `brep-intersect`) hands over the intersection as pieces, each
//! inside exactly one face of either operand, with its 3D curve and both
//! pcurves over `[0, 1]` on a common parameter, plus a material predicate
//! for each operand. This module does the rest exactly like the in-crate
//! families: piece ends on original edges become shared split vertices,
//! free ends are shared by id, every face chart is arranged through
//! `uv_regions`, regions are classified by the caller's predicate at their
//! interior sample and assembled through `imprint_pipeline`.
//!
//! The caller owns the meaning of the result: the network's accuracy sets
//! `tolerance_mm`, and an incomplete network or a wrong predicate gives a
//! wrong solid that validation may or may not catch. Callers state what
//! they certified.
use crate::imprint_assembly::{Assembler, ChartVertices, EKey, VKey, face_reversed};
use crate::imprint_pipeline::{self, SpatialRelation};
use crate::uv_regions::{Piece, pcurve_point};
use crate::Model;
use nurbs_core::{Error, Result, curve::Curve};
use std::collections::BTreeMap;

/// Where a network piece ends.
#[derive(Clone, Copy, Debug)]
pub enum NetworkEnd {
    /// On original edges, as `(operand, edge, curve parameter)` with
    /// operand 0 = first; one entry, or one per operand where seams of both
    /// operands meet at the point.
    Edges {
        on: [Option<(usize, usize, f64)>; 2],
        point: [f64; 3],
    },
    /// A free network vertex; ends with the same `id` are the same vertex.
    Free { id: usize, point: [f64; 3] },
}

/// One intersection piece inside face `faces[0]` of the first operand and
/// face `faces[1]` of the second.
#[derive(Clone, Debug)]
pub struct NetworkPiece {
    pub faces: [usize; 2],
    /// 3D curve over `[0, 1]`.
    pub curve: Curve,
    /// UV pcurves on the two faces over the same `[0, 1]` parameter.
    pub pcurves: [Curve; 2],
    pub ends: [NetworkEnd; 2],
}

/// Material predicate: `inside(o, p)` is true when point `p` of a face of
/// operand `o` lies inside the *other* operand. Ambiguity must be an error.
pub type Inside<'a> = dyn Fn(usize, [f64; 3]) -> Result<bool> + 'a;

/// Assemble `operation` ("union", "difference", "intersection") of two
/// single-shell solids from a caller-supplied intersection network.
pub fn network_boolean(
    a: &Model,
    b: &Model,
    operation: &str,
    pieces: &[NetworkPiece],
    inside: &Inside<'_>,
    tolerance: f64,
) -> Result<Model> {
    a.validate()?;
    b.validate()?;
    if !matches!(operation, "union" | "difference" | "intersection") {
        return Err(Error::new(
            "BREP_UNSUPPORTED_OPERATION",
            "Network Boolean admits union, difference or intersection",
        ));
    }
    if !(tolerance.is_finite() && tolerance > 0.) {
        return Err(Error::new("BREP_INVALID_INPUT", "Network tolerance must be positive"));
    }
    let operands = [a, b];
    for piece in pieces {
        for o in 0..2 {
            if piece.faces[o] >= operands[o].faces.len() {
                return Err(Error::new("BREP_INVALID_INPUT", "Network piece face out of range"));
            }
        }
    }
    if pieces.is_empty() {
        let relation = if inside(0, a.vertices[0].point)? {
            SpatialRelation::BContainsA
        } else if inside(1, b.vertices[0].point)? {
            SpatialRelation::AContainsB
        } else {
            SpatialRelation::Disjoint
        };
        return imprint_pipeline::regularized_empty_algebra(a, b, operation, relation);
    }
    let mut asm = Assembler::new(a, b, operation)?;
    let mut free: BTreeMap<usize, usize> = BTreeMap::new();
    let mut vertex = |asm: &mut Assembler, end: NetworkEnd| -> usize {
        match end {
            NetworkEnd::Edges { on, point } => {
                let on: Vec<(usize, usize, f64)> = on.into_iter().flatten().collect();
                let existing = on.iter().find_map(|&(o, e, t)| asm.split_at(o, e, t, 1e-7));
                let id = existing.unwrap_or_else(|| asm.special(point));
                for (o, e, t) in on {
                    if asm.split_at(o, e, t, 1e-7) != Some(id) {
                        asm.attach_edge(id, o, e, t);
                    }
                }
                id
            }
            NetworkEnd::Free { id, point } => *free.entry(id).or_insert_with(|| asm.special(point)),
        }
    };
    let mut per_face: [Vec<Vec<(usize, Curve)>>; 2] =
        [vec![Vec::new(); a.faces.len()], vec![Vec::new(); b.faces.len()]];
    for piece in pieces {
        let v0 = vertex(&mut asm, piece.ends[0]);
        let v1 = vertex(&mut asm, piece.ends[1]);
        let k = asm.push_net([VKey::Special(v0), VKey::Special(v1)], piece.curve.clone());
        for o in 0..2 {
            per_face[o][piece.faces[o]].push((k, piece.pcurves[o].clone()));
        }
    }
    for o in 0..2 {
        let model = operands[o];
        let want = asm.want_inside[o];
        for face in 0..model.faces.len() {
            let reversed = face_reversed(model, face);
            let mut chart = ChartVertices::default();
            let mut list: Vec<Piece<EKey>> = Vec::new();
            for loop_pieces in asm.boundary_loops(o, face)? {
                for (ka, kb, pcurve, key, rev) in loop_pieces {
                    let ia = chart.index(asm.chart_key(ka), pcurve_point(&pcurve, 0.)?);
                    let ib = chart.index(asm.chart_key(kb), pcurve_point(&pcurve, 1.)?);
                    list.push(Piece {
                        v: [ia, ib],
                        pcurve,
                        key,
                        reversed: rev,
                    });
                }
            }
            for (k, pcurve) in &per_face[o][face] {
                let mut pcurve = pcurve.clone();
                let ends = asm.net[*k].ends;
                let n = pcurve.control_points.len();
                let ia = chart.index(asm.chart_key(ends[0]), pcurve_point(&pcurve, 0.)?);
                let ib = chart.index(asm.chart_key(ends[1]), pcurve_point(&pcurve, 1.)?);
                pcurve.control_points[0] = chart.uvs[ia].to_vec();
                pcurve.control_points[n - 1] = chart.uvs[ib].to_vec();
                list.push(Piece {
                    v: [ia, ib],
                    pcurve,
                    key: EKey::Net(*k),
                    reversed: false,
                });
            }
            let keep = |p: [f64; 3]| -> Result<bool> { Ok(inside(o, p)? == want) };
            asm.emit_chart(o, face, reversed, list, chart.len(), &keep)?;
        }
    }
    asm.finish(tolerance.max(a.tolerance_mm).max(b.tolerance_mm))
}
