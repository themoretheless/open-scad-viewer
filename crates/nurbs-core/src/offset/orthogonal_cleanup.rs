//! Exact original-source miter offset and fill cleanup on orthogonal line wires.
use crate::{
    Result, check,
    chord_arrangement::{Arrangement, Edge, Vertex},
    chord_winding::FillRule,
    curve::Curve,
    exact_curve_segments::exact_add,
};
use std::collections::BTreeMap;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CleanupStop {
    Complete,
    UnsupportedSource,
    PrecisionLimit,
    CollapsedOffset,
    WorkLimit,
    FillUnresolved,
}
#[derive(Clone, Copy, Debug)]
pub struct CleanupLimits {
    pub pairs: usize,
    pub edges: usize,
    pub witness_checks: usize,
}
impl Default for CleanupLimits {
    fn default() -> Self {
        Self {
            pairs: 1000000,
            edges: 65536,
            witness_checks: 1000000,
        }
    }
}
#[derive(Clone, Debug)]
pub struct OrthogonalCleanup {
    pub curves: Option<Vec<Vec<Curve>>>,
    pub offset_vertices: Vec<[f64; 2]>,
    pub stop: CleanupStop,
    pub pairs: usize,
    pub source_to_arrangement_error_upper: Option<f64>,
}
fn axis(a: [f64; 2], b: [f64; 2]) -> Option<usize> {
    if a[0] != b[0] && a[1] == b[1] {
        Some(0)
    } else if a[0] == b[0] && a[1] != b[1] {
        Some(1)
    } else {
        None
    }
}
fn inside(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> bool {
    (0..2).all(|k| p[k] >= a[k].min(b[k]) && p[k] <= a[k].max(b[k]))
}
fn vertex(vertices: &mut Vec<Vertex>, ids: &mut BTreeMap<[u64; 2], usize>, p: [f64; 2]) -> usize {
    let p = p.map(|x| if x == 0. { 0. } else { x });
    *ids.entry(p.map(f64::to_bits)).or_insert_with(|| {
        let i = vertices.len();
        vertices.push(Vertex {
            point: p,
            error_upper_mm: 0.,
        });
        i
    })
}
/// Construct the exact nominal miter-offset chain of a closed original
/// orthogonal NURBS line wire, split every crossing/contact/overlap exactly,
/// and return its NonZero or EvenOdd fill boundary with material on the left.
/// Offset distance is signed LEFT of source traversal. This cleans the defined
/// miter chain; it does not certify Minkowski dilation/erosion, tool clearance,
/// or arbitrary smooth-offset topology. All coordinate shifts must be exactly
/// representable. Source/miter/arrangement image identity therefore has ZERO
/// approximation and rounding error. Winding and graph admission remain
/// independently mandatory; unresolved fill never returns geometry.
/// Accepts 4..256 clamped, constant-weight degree-one two-pole curves in XY
/// (2D, or common stored Z), exact joins and perpendicular consecutive edges.
/// Finite source/generated coordinates and |distance| are bounded by 1e6.
pub fn cleanup_orthogonal_miter_offset(
    source: &[Curve],
    distance: f64,
    rule: FillRule,
    limits: CleanupLimits,
) -> Result<OrthogonalCleanup> {
    check(
        (4..=256).contains(&source.len())
            && distance.is_finite()
            && distance.abs() <= 1e6
            && (1..=1000000).contains(&limits.pairs)
            && (1..=65536).contains(&limits.edges)
            && (1..=1000000).contains(&limits.witness_checks),
        "Orthogonal cleanup needs a bounded wire, finite distance and positive work limits",
    )?;
    let mut out = OrthogonalCleanup {
        curves: None,
        offset_vertices: vec![],
        stop: CleanupStop::UnsupportedSource,
        pairs: 0,
        source_to_arrangement_error_upper: None,
    };
    for curve in source {
        curve.validate()?;
    }
    let dim = source[0].control_points.first().map(Vec::len).unwrap_or(0);
    let mut original = Vec::new();
    let mut directions = Vec::new();
    let mut z = None;
    for (i, c) in source.iter().enumerate() {
        if !matches!(dim, 2 | 3)
            || c.control_points[0].len() != dim
            || c.periodic
            || c.degree != 1
            || c.control_points.len() != 2
            || c.weights[0] != c.weights[1]
            || c.knots[0] != c.knots[1]
            || c.knots[2] != c.knots[3]
        {
            return Ok(out);
        }
        check(
            c.control_points.iter().flatten().all(|x| x.abs() <= 1e6),
            "Orthogonal source coordinate exceeds bound",
        )?;
        check(
            c.control_points[1] == source[(i + 1) % source.len()].control_points[0],
            "Orthogonal source wire must have exact closed joins",
        )?;
        if dim == 3 {
            for p in &c.control_points {
                if z.is_some_and(|value| value != p[2]) {
                    return Ok(out);
                }
                z = Some(p[2]);
            }
        }
        let a = [c.control_points[0][0], c.control_points[0][1]];
        let b = [c.control_points[1][0], c.control_points[1][1]];
        let Some(k) = axis(a, b) else { return Ok(out) };
        original.push(a);
        directions.push((k, if b[k] > a[k] { 1. } else { -1. }));
    }
    let n = original.len();
    for i in 0..n {
        let incoming = directions[(i + n - 1) % n];
        let outgoing = directions[i];
        if incoming.0 == outgoing.0 {
            return Ok(out);
        }
        let mut p = original[i];
        for (k, sign) in [incoming, outgoing] {
            let normal_axis = 1 - k;
            let shift = if k == 0 {
                distance * sign
            } else {
                -distance * sign
            };
            let Some(value) = exact_add(p[normal_axis], shift) else {
                out.stop = CleanupStop::PrecisionLimit;
                return Ok(out);
            };
            p[normal_axis] = value;
        }
        if p.iter().any(|x| !x.is_finite() || x.abs() > 1e6) {
            out.stop = CleanupStop::PrecisionLimit;
            return Ok(out);
        }
        out.offset_vertices
            .push(p.map(|x| if x == 0. { 0. } else { x }));
    }
    let points = &out.offset_vertices;
    let mut axes = Vec::new();
    let mut cuts = Vec::new();
    for i in 0..n {
        let Some(k) = axis(points[i], points[(i + 1) % n]) else {
            out.stop = CleanupStop::CollapsedOffset;
            return Ok(out);
        };
        axes.push(k);
        cuts.push(vec![points[i], points[(i + 1) % n]]);
    }
    for i in 0..n {
        for j in i + 1..n {
            if out.pairs == limits.pairs {
                out.stop = CleanupStop::WorkLimit;
                return Ok(out);
            }
            out.pairs += 1;
            let [a, b] = [points[i], points[(i + 1) % n]];
            let [c, d] = [points[j], points[(j + 1) % n]];
            if axes[i] != axes[j] {
                let mut p = [0.; 2];
                p[1 - axes[i]] = a[1 - axes[i]];
                p[1 - axes[j]] = c[1 - axes[j]];
                if inside(p, a, b) && inside(p, c, d) {
                    cuts[i].push(p);
                    cuts[j].push(p);
                }
            } else if a[1 - axes[i]] == c[1 - axes[i]] {
                for p in [a, b] {
                    if inside(p, c, d) {
                        cuts[j].push(p);
                    }
                }
                for p in [c, d] {
                    if inside(p, a, b) {
                        cuts[i].push(p);
                    }
                }
            }
        }
    }
    let mut vertices = Vec::new();
    let mut ids = BTreeMap::new();
    let mut edges = Vec::new();
    let mut geometry = BTreeMap::new();
    let mut occurrences = Vec::new();
    for i in 0..n {
        cuts[i].sort_by(|a, b| {
            let order = a[axes[i]].total_cmp(&b[axes[i]]);
            if points[i][axes[i]] < points[(i + 1) % n][axes[i]] {
                order
            } else {
                order.reverse()
            }
        });
        cuts[i].dedup();
        for pair in cuts[i].windows(2) {
            if occurrences.len() == limits.edges {
                out.stop = CleanupStop::WorkLimit;
                return Ok(out);
            }
            let vs = [
                vertex(&mut vertices, &mut ids, pair[0]),
                vertex(&mut vertices, &mut ids, pair[1]),
            ];
            // Metadata conservatively encloses the complete parent miter
            // segment. Exact topology follows the stored physical cuts, not
            // rounded source parameter fractions.
            let edge = Edge {
                vertices: vs,
                source_edge: i,
                source_domain: [i as f64, (i + 1) as f64],
                source_parameters: [[0., 1.]; 2],
                error_upper_mm: 0.,
            };
            let key = if vs[0] < vs[1] { vs } else { [vs[1], vs[0]] };
            if !geometry.contains_key(&key) {
                geometry.insert(key, edges.len());
                edges.push(edge.clone());
            }
            occurrences.push(edge);
        }
    }
    let graph = Arrangement {
        vertices,
        edges,
        source_edges: Some(occurrences),
    };
    out.source_to_arrangement_error_upper = Some(0.);
    match crate::chord_fill_selection::boundary_curves(&graph, rule, limits.witness_checks) {
        Ok(mut curves) => {
            if let Some(z) = z {
                for c in curves.iter_mut().flatten() {
                    for p in &mut c.control_points {
                        p.push(z);
                    }
                    c.validate()?;
                }
            }
            out.curves = Some(curves);
            out.stop = CleanupStop::Complete;
        }
        Err(_) => out.stop = CleanupStop::FillUnresolved,
    }
    Ok(out)
}
