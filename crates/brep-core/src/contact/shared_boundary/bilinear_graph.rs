//! A nonplanar graph carrier and a strictly sided wall meet only on their
//! exactly owned boundary. All signs use original control and weight leaves.
use super::*;
#[derive(Clone, Debug)]
pub struct Certificate {
    pub edge: usize,
    pub graph_face: usize,
    pub sided_face: usize,
    pub polynomial_work: u64,
}
fn chart(s: &Surface) -> bool {
    s.validate().is_ok()
        && !s.periodic_u
        && !s.periodic_v
        && (2..=5).contains(&s.control_points.len())
        && (2..=5).contains(&s.control_points[0].len())
        && s.control_points.len() == s.degree_u + 1
        && s.control_points[0].len() == s.degree_v + 1
        && clamped(&s.knots_u, s.degree_u, s.control_points.len())
        && clamped(&s.knots_v, s.degree_v, s.control_points[0].len())
        && s.control_points.iter().flatten().all(|p| p.len() == 3)
}
/// Every coordinate permutation tried as `(graph u, graph v, height)` axes.
const AXIS_ORDERS: [[usize; 3]; 6] = [
    [0, 1, 2],
    [1, 0, 2],
    [0, 2, 1],
    [2, 0, 1],
    [1, 2, 0],
    [2, 1, 0],
];
/// Leaves per authored rational control point: three coordinates and a weight.
const POINT_LEAVES: usize = 4;
/// Leaves of the bilinear graph's 2x2 rational control net.
const GRAPH_LEAVES: usize = 4 * POINT_LEAVES;
/// Leaves per authored corner point.
const CORNER_LEAVES: usize = 3;

/// The graph's corners project onto an axis-aligned rectangle in `axes`.
fn rectangular(corners: &[&Vec<f64>; 4], axes: [usize; 3]) -> bool {
    corners[0][axes[0]] == corners[2][axes[0]]
        && corners[1][axes[0]] == corners[3][axes[0]]
        && corners[0][axes[1]] == corners[1][axes[1]]
        && corners[2][axes[1]] == corners[3][axes[1]]
}

/// Original control, weight and corner leaves of both surfaces.
fn source_arena(
    graph: &Surface,
    wall: &Surface,
    corners: &[&Vec<f64>; 4],
) -> Option<(SourceArena, usize)> {
    let mut values = Vec::new();
    for s in [graph, wall] {
        for (row, weights) in s.control_points.iter().zip(&s.weights) {
            for (p, w) in row.iter().zip(weights) {
                values.extend(p.iter().copied());
                values.push(*w);
            }
        }
    }
    let at = values.len();
    values.extend(corners.iter().flat_map(|p| p.iter().copied()));
    let arena = SourceArena::authored(
        "bilinear-graph-shared-boundary",
        1,
        values
            .into_iter()
            .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
            .collect(),
    )
    .ok()?;
    Some((arena, at))
}

fn net_refs(
    arena: &SourceArena,
    s: &Surface,
    offset: usize,
) -> Option<Vec<Vec<[cad_predicates::LeafRef; 4]>>> {
    let mut net = Vec::with_capacity(s.control_points.len());
    for (i, row) in s.control_points.iter().enumerate() {
        let mut refs = Vec::with_capacity(row.len());
        for j in 0..row.len() {
            let base = offset + POINT_LEAVES * (i * row.len() + j);
            let mut point = [arena.leaf(base).ok()?; 4];
            for (k, slot) in point.iter_mut().enumerate() {
                *slot = arena.leaf(base + k).ok()?;
            }
            refs.push(point);
        }
        net.push(refs);
    }
    Some(net)
}

fn corner_refs(arena: &SourceArena, at: usize) -> Option<[[cad_predicates::LeafRef; 3]; 4]> {
    let first = arena.leaf(at).ok()?;
    let mut corners = [[first; 3]; 4];
    for (i, corner) in corners.iter_mut().enumerate() {
        for (k, slot) in corner.iter_mut().enumerate() {
            *slot = arena.leaf(at + CORNER_LEAVES * i + k).ok()?;
        }
    }
    Some(corners)
}

/// The owned wall row vanishes, every other sign agrees, and the complete
/// opposite row is strict. A full opposite row is strict, so every point off
/// the owned boundary, including free-parameter endpoints, has the same strict
/// graph sign.
fn strict_off_boundary(ws: &[Vec<Sign>], bound: (usize, usize)) -> Option<bool> {
    let end = [ws.len().checked_sub(1)?, ws.first()?.len().checked_sub(1)?][bound.0];
    let row = if bound.1 == 0 { 0 } else { end };
    let opposite = end - row;
    let mut sign = None;
    let mut valid = true;
    for (i, r) in ws.iter().enumerate() {
        for (j, &s) in r.iter().enumerate() {
            if [i, j][bound.0] == row {
                if s != Sign::Zero {
                    valid = false;
                }
            } else if s != Sign::Zero {
                if sign.is_some_and(|wanted| wanted != s) {
                    valid = false;
                }
                sign = Some(s);
            } else if [i, j][bound.0] == opposite {
                valid = false;
            }
        }
    }
    Some(valid && sign.is_some())
}

fn sides(graph: &Surface, wall: &Surface, bound: (usize, usize), max_work: u64) -> Option<u64> {
    if !chart(graph) || !chart(wall) || graph.degree_u != 1 || graph.degree_v != 1 {
        return None;
    }
    let corners = [
        &graph.control_points[0][0],
        &graph.control_points[1][0],
        &graph.control_points[0][1],
        &graph.control_points[1][1],
    ];
    for axes in AXIS_ORDERS {
        if !rectangular(&corners, axes) {
            continue;
        }
        let (arena, at) = source_arena(graph, wall, &corners)?;
        let gr = net_refs(&arena, graph, 0)?;
        let wr = net_refs(&arena, wall, GRAPH_LEAVES)?;
        let cr = corner_refs(&arena, at)?;
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(
            &arena,
            &tolerance,
            Limits {
                max_work,
                ..Limits::default()
            },
            None,
        );
        let gs = cad_predicates::rational_bilinear_graph_side(&mut ctx, &gr, cr, axes)
            .ok()?
            .signs?;
        if gs.iter().flatten().any(|s| *s != Sign::Zero) {
            continue;
        }
        let ws = cad_predicates::rational_bilinear_graph_side(&mut ctx, &wr, cr, axes)
            .ok()?
            .signs?;
        if strict_off_boundary(&ws, bound)? {
            return Some(ctx.work_used());
        }
    }
    None
}
/// Call only after structural validation. This does not prove injectivity.
pub(crate) fn certify(model: &Model, faces: [usize; 2], max_work: u64) -> Option<Certificate> {
    if max_work == 0 || max_work > cad_predicates::MAX_WORK {
        return None;
    }
    let uses = |f: usize| {
        std::iter::once(model.faces[f].outer)
            .chain(model.faces[f].holes.iter().copied())
            .flat_map(|l| model.loops[l].coedges.iter())
            .collect::<Vec<_>>()
    };
    for wall_id in faces {
        let graph_id = if wall_id == faces[0] {
            faces[1]
        } else {
            faces[0]
        };
        let wall = &model.faces[wall_id].surface;
        let graph = &model.faces[graph_id].surface;
        if graph.degree_u != 1 || graph.degree_v != 1 || !chart(graph) || !chart(wall) {
            continue;
        }
        for wu in uses(wall_id) {
            for gu in uses(graph_id) {
                if wu.edge != gu.edge {
                    continue;
                }
                let edge = &model.edges[wu.edge].curve;
                let Some(bound) = boundary(wall, &wu.pcurve, edge) else {
                    continue;
                };
                let Some(polynomial_work) = sides(graph, wall, bound, max_work) else {
                    continue;
                };
                let exact = nurbs_core::curve_surface_agreement::verify_exact(
                    edge,
                    &gu.pcurve,
                    graph,
                    gu.reversed,
                    32768,
                )
                .ok()
                .flatten()
                .is_some_and(|d| d.outcome == cad_predicates::BezierIdentity::Equal);
                if exact {
                    return Some(Certificate {
                        edge: wu.edge,
                        graph_face: graph_id,
                        sided_face: wall_id,
                        polynomial_work,
                    });
                }
            }
        }
    }
    None
}
