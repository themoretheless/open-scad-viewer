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
    /// At original vertex `vertex` of `operand`, and optionally on an edge
    /// `(other operand, edge, curve parameter)` of the other operand.
    Vertex {
        operand: usize,
        vertex: usize,
        on: Option<(usize, usize, f64)>,
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

/// Assemble union, difference, intersection or XOR of two single-shell
/// solids from a caller-supplied intersection network. XOR preserves separate
/// topology for the two differences at shared intersection curves.
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
    if !matches!(operation, "union" | "difference" | "intersection" | "xor") {
        return Err(Error::new(
            "BREP_UNSUPPORTED_OPERATION",
            "Network Boolean admits union, difference, intersection or xor",
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
        for end in piece.ends {
            if let NetworkEnd::Vertex { operand, vertex, .. } = end
                && (operand > 1 || vertex >= operands[operand].vertices.len())
            {
                return Err(Error::new("BREP_INVALID_INPUT", "Network vertex out of range"));
            }
        }
    }
    if pieces.is_empty() {
        if let Some(result) = crate::boolean_support::simplify(a, b, operation)? {
            return Ok(result);
        }
        let relation = if inside(0, a.vertices[0].point)? {
            SpatialRelation::BContainsA
        } else if inside(1, b.vertices[0].point)? {
            SpatialRelation::AContainsB
        } else {
            SpatialRelation::Disjoint
        };
        return imprint_pipeline::regularized_empty_algebra(a, b, operation, relation);
    }
    let tolerance = tolerance.max(a.tolerance_mm).max(b.tolerance_mm);
    if operation == "xor" {
        let mut second_difference = Assembler::new(a, b, "difference")?;
        second_difference.want_inside.reverse();
        second_difference.flip.reverse();
        let first = assemble_retained_network(
            operands, Assembler::new(a, b, "difference")?, pieces, inside, tolerance,
        )?;
        let second = assemble_retained_network(
            operands, second_difference, pieces, inside, tolerance,
        )?;
        return crate::boolean_support::concatenate_bodies(&first, &second);
    }
    assemble_retained_network(
        operands, Assembler::new(a, b, operation)?, pieces, inside, tolerance,
    )
}

fn assemble_retained_network(
    operands: [&Model; 2],
    mut asm: Assembler<'_>,
    pieces: &[NetworkPiece],
    inside: &Inside<'_>,
    tolerance: f64,
) -> Result<Model> {
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
            NetworkEnd::Vertex { operand, vertex, on, point } => {
                // A special that shares the original vertex's chart slot.
                let existing = asm.specials.iter().position(|v| v.orig == Some((operand, vertex)));
                let id = existing.unwrap_or_else(|| {
                    let id = asm.special(point);
                    asm.specials[id].orig = Some((operand, vertex));
                    id
                });
                if let Some((o, e, t)) = on
                    && asm.split_at(o, e, t, 1e-7) != Some(id)
                {
                    asm.attach_edge(id, o, e, t);
                }
                id
            }
            NetworkEnd::Free { id, point } => *free.entry(id).or_insert_with(|| asm.special(point)),
        }
    };
    let mut per_face: [Vec<Vec<(usize, Curve)>>; 2] =
        operands.map(|model| vec![Vec::new(); model.faces.len()]);
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
    asm.finish(tolerance)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_xor_closes_both_differences_at_a_shared_rim() {
        let a = crate::cuboid([0.; 3], [2.; 3]).unwrap();
        let b = crate::cuboid([1.; 3], [3.; 3]).unwrap();
        let solids = [&a, &b];
        let rim = [
            [2., 1., 1.], [2., 2., 1.], [1., 2., 1.],
            [1., 2., 2.], [1., 1., 2.], [2., 1., 2.],
        ];
        let end = |point: [f64; 3]| {
            let on = std::array::from_fn(|side| {
                let model = solids[side];
                model.edges.iter().enumerate().find_map(|(edge, curve)| {
                    let [a, b] = curve.vertices.map(|v| model.vertices[v].point);
                    let axis = (0..3).find(|&axis| a[axis] != b[axis])?;
                    let ratio = (point[axis] - a[axis]) / (b[axis] - a[axis]);
                    if !(0. < ratio && ratio < 1.)
                        || (0..3).any(|i| i != axis && point[i] != a[i])
                    {
                        return None;
                    }
                    let [lo, hi] = curve.curve.domain();
                    Some((side, edge, lo + ratio * (hi - lo)))
                })
            });
            assert_eq!(on.iter().flatten().count(), 1);
            NetworkEnd::Edges { on, point }
        };
        let pieces = (0..rim.len()).map(|index| {
            let [start, finish] = [rim[index], rim[(index + 1) % rim.len()]];
            let faces = std::array::from_fn(|side| {
                let boundary = if side == 0 { 2. } else { 1. };
                let axis = (0..3).find(|&i| start[i] == boundary && finish[i] == boundary).unwrap();
                solids[side].faces.iter().position(|face| {
                    face.surface.control_points.iter().flatten().all(|p| p[axis] == boundary)
                }).unwrap()
            });
            let pcurves = std::array::from_fn(|side| {
                let surface = &solids[side].faces[faces[side]].surface;
                let origin = &surface.control_points[0][0];
                let axes = [&surface.control_points[1][0], &surface.control_points[0][1]];
                let knots = [&surface.knots_u, &surface.knots_v];
                let uv = |point: [f64; 3]| {
                    (0..2).map(|i| {
                        let axis = (0..3).find(|&k| axes[i][k] != origin[k]).unwrap();
                        let unit = (point[axis] - origin[axis]) / (axes[i][axis] - origin[axis]);
                        knots[i][1] + unit * (knots[i][2] - knots[i][1])
                    }).collect::<Vec<_>>()
                };
                Curve::from_polyline(vec![uv(start), uv(finish)]).unwrap()
            });
            NetworkPiece {
                faces,
                curve: Curve::from_polyline(vec![start.to_vec(), finish.to_vec()]).unwrap(),
                pcurves,
                ends: [end(start), end(finish)],
            }
        }).collect::<Vec<_>>();
        let inside = |side: usize, point: [f64; 3]| {
            let lo = if side == 0 { 1. } else { 0. };
            Ok(point.into_iter().all(|x| lo < x && x < lo + 2.))
        };
        let result = network_boolean(&a, &b, "xor", &pieces, &inside, 1e-7).unwrap();
        assert_eq!(result.bodies.len(), 2);
        assert!(result.shells.iter().all(|shell| shell.closed));
        assert!(brep_topology::explorer::boundary_edges(&result.0).is_empty());
        let mass = crate::analysis::mass_properties(&result, 1e-6, 200000).unwrap();
        assert!((mass.signed_volume_mm3 - 14.).abs() < 1e-6);
    }
}
