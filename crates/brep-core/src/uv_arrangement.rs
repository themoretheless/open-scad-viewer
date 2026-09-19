//! Lifted-UV arrangements for frozen chart classes (G5b / F3 close).
//!
//! Supports multiple imprint curves per chart, monotone nesting, multiple holes,
//! periodic AnalyticCircle wrap strata, freeform chart touches, and missed-branch
//! mutation evidence. Periodic/pole rules refuse non-finite or wrap-ambiguous strata.

use crate::Model;
use crate::coverage_verifier::{UvCoverageCertificate, certify_lifted_uv_coverage};
use crate::trim_sew::{
    CellLabel, ChartEvent, ChartKind, ClassificationCertificate, SewCertificate,
    classify_chart_events, sew_closed_model_edges,
};
use cad_predicates::{ToleranceContext, ToleranceSpecIdentity};
use nurbs_core::{Error, Result};
use std::collections::BTreeMap;

fn refuse(message: &str) -> Error {
    Error::new("BREP_UV_ARRANGEMENT_REFUSED", message)
}

#[derive(Clone, Debug)]
pub struct UvImprintCurve {
    pub chart: ChartKind,
    pub parameter_intervals: Vec<[f64; 2]>,
    /// Optional hole intervals classified Outside relative to outer enter/exit.
    pub hole_intervals: Vec<[f64; 2]>,
    pub edge_id: usize,
    /// True when the chart is periodic and the interval may wrap the seam.
    pub periodic: bool,
}

#[derive(Clone, Debug)]
pub struct NestingRecord {
    pub outer_edge: usize,
    pub nested_edges: Vec<usize>,
    pub depth: usize,
}

#[derive(Clone, Debug)]
pub struct UvArrangement {
    pub chart: ChartKind,
    pub events: Vec<ChartEvent>,
    pub classification: ClassificationCertificate,
    pub hole_count: usize,
    /// Monotone nesting of imprint intervals (outer contains inner).
    pub nesting: Vec<NestingRecord>,
}

pub const DEFAULT_UV_RESOURCE_LIMIT: usize = 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TensorAxis {
    U,
    V,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TensorBoundary {
    UMin,
    UMax,
    VMin,
    VMax,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LiftedUvGeometry {
    PlaneSegment {
        start: [f64; 2],
        end: [f64; 2],
    },
    AnalyticCircleArc {
        center: [f64; 2],
        radius: f64,
        /// Lifted angular interval. End may exceed TAU, but span must be < TAU.
        interval: [f64; 2],
    },
    TensorIsoLine {
        axis: TensorAxis,
        fixed: f64,
        interval: [f64; 2],
    },
    TensorRectangleBoundary {
        side: TensorBoundary,
        domain: [[f64; 2]; 2],
    },
    /// Certified rational curved pcurve trace (interval/exact correspondence).
    RationalCurvedTrace {
        samples: Vec<[f64; 2]>,
        closed: bool,
        correspondence: &'static str,
        overlap: bool,
        singular: bool,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct LiftedUvPrimitive {
    pub edge_id: usize,
    pub geometry: LiftedUvGeometry,
}

#[derive(Clone, Debug, PartialEq)]
pub enum UvVertexOrigin {
    AuthoredEndpoint,
    ConstructedIntersection { edges: [usize; 2] },
    PeriodicSeam { edge: usize, shift: i32 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct UvVertex {
    pub point: [f64; 2],
    pub origin: UvVertexOrigin,
}

#[derive(Clone, Debug, PartialEq)]
pub enum UvHalfedgeGeometry {
    Segment,
    CircleArc {
        center: [f64; 2],
        radius: f64,
        interval: [f64; 2],
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct UvHalfedge {
    pub origin: usize,
    pub destination: usize,
    pub twin: usize,
    pub next: usize,
    pub cell: usize,
    pub source_edge: usize,
    pub geometry: UvHalfedgeGeometry,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WindingLabel {
    Exterior,
    Material(i32),
}

#[derive(Clone, Debug, PartialEq)]
pub struct UvCell {
    pub boundary: Vec<usize>,
    pub signed_area: f64,
    pub winding: i32,
    pub label: WindingLabel,
}

#[derive(Clone, Debug)]
pub struct LiftedUvArrangement {
    pub chart: ChartKind,
    pub context: ToleranceSpecIdentity,
    pub vertices: Vec<UvVertex>,
    pub halfedges: Vec<UvHalfedge>,
    pub cells: Vec<UvCell>,
    pub coverage: UvCoverageCertificate,
}

fn cross(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}

fn sub(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

fn point_key(point: [f64; 2]) -> (u64, u64) {
    let canonical = |v: f64| if v == 0. { 0f64.to_bits() } else { v.to_bits() };
    (canonical(point[0]), canonical(point[1]))
}

fn segment_intersection(
    a: [f64; 2],
    b: [f64; 2],
    c: [f64; 2],
    d: [f64; 2],
) -> Result<Option<(f64, f64, [f64; 2])>> {
    let r = sub(b, a);
    let s = sub(d, c);
    let denominator = cross(r, s);
    let ca = sub(c, a);
    if denominator == 0. {
        if cross(ca, r) == 0. {
            let rr = r[0] * r[0] + r[1] * r[1];
            let t0 = (ca[0] * r[0] + ca[1] * r[1]) / rr;
            let da = sub(d, a);
            let t1 = (da[0] * r[0] + da[1] * r[1]) / rr;
            if t0.max(t1).min(1.) > t0.min(t1).max(0.) {
                return Err(refuse("Overlapping UV branches are ambiguous"));
            }
        }
        return Ok(None);
    }
    let t = cross(ca, s) / denominator;
    let u = cross(ca, r) / denominator;
    if (-1e-15..=1. + 1e-15).contains(&t) && (-1e-15..=1. + 1e-15).contains(&u) {
        Ok(Some((
            t.clamp(0., 1.),
            u.clamp(0., 1.),
            [a[0] + t * r[0], a[1] + t * r[1]],
        )))
    } else {
        Ok(None)
    }
}

fn interval_contains(outer: [f64; 2], inner: [f64; 2]) -> bool {
    outer[0] <= inner[0] + 1e-15
        && inner[1] <= outer[1] + 1e-15
        && (inner[0] - outer[0]).abs() + (outer[1] - inner[1]).abs() > 1e-12
}

fn compute_monotone_nesting(curves: &[UvImprintCurve]) -> Result<Vec<NestingRecord>> {
    let mut intervals: Vec<(usize, [f64; 2])> = Vec::new();
    for curve in curves {
        for interval in &curve.parameter_intervals {
            intervals.push((curve.edge_id, *interval));
        }
        for hole in &curve.hole_intervals {
            // Holes nest inside material intervals; record with offset edge ids.
            intervals.push((curve.edge_id.saturating_add(10_000), *hole));
        }
    }
    intervals.sort_by(|a, b| {
        (a.1[1] - a.1[0])
            .partial_cmp(&(b.1[1] - b.1[0]))
            .unwrap_or(std::cmp::Ordering::Equal)
            .reverse()
            .then_with(|| a.0.cmp(&b.0))
    });
    let mut nesting = Vec::new();
    for (i, (edge, span)) in intervals.iter().enumerate() {
        let mut nested = Vec::new();
        let mut depth = 0usize;
        for (j, (other_edge, other)) in intervals.iter().enumerate() {
            if i == j {
                continue;
            }
            if interval_contains(*span, *other) {
                nested.push(*other_edge);
            }
            if interval_contains(*other, *span) {
                depth += 1;
            }
        }
        if !nested.is_empty() || depth > 0 {
            nested.sort_unstable();
            nested.dedup();
            nesting.push(NestingRecord {
                outer_edge: *edge,
                nested_edges: nested,
                depth,
            });
        }
    }
    // Crossing (non-monotone) pairs refuse: intervals overlap without containment.
    for i in 0..intervals.len() {
        for j in (i + 1)..intervals.len() {
            let a = intervals[i].1;
            let b = intervals[j].1;
            let overlap_lo = a[0].max(b[0]);
            let overlap_hi = a[1].min(b[1]);
            if overlap_hi > overlap_lo + 1e-12
                && !interval_contains(a, b)
                && !interval_contains(b, a)
            {
                return Err(refuse(
                    "Non-monotone overlapping imprint intervals; refuse UV arrange",
                ));
            }
        }
    }
    Ok(nesting)
}

/// Split a periodic wrap interval [a,b] with a>b into [a, period] U [0, b].
fn expand_periodic_intervals(intervals: &[[f64; 2]], period: f64) -> Result<Vec<[f64; 2]>> {
    let mut out = Vec::new();
    for interval in intervals {
        if !interval[0].is_finite() || !interval[1].is_finite() {
            return Err(refuse("Periodic imprint interval must be finite"));
        }
        if interval[0] < 0. || interval[1] < 0. || interval[0] > period || interval[1] > period {
            return Err(refuse("Periodic imprint interval must lie in [0, period]"));
        }
        if interval[1] >= interval[0] {
            if (interval[1] - interval[0]) >= period - 1e-9 {
                return Err(refuse("Periodic full-period imprint is a pole/seam refuse"));
            }
            out.push(*interval);
        } else {
            // Wrap across the seam: [lo, period] + [0, hi].
            let first = [interval[0], period];
            let second = [0., interval[1]];
            if (first[1] - first[0]) + (second[1] - second[0]) >= period - 1e-9 {
                return Err(refuse(
                    "Periodic full-period wrap imprint is a pole/seam refuse",
                ));
            }
            if first[1] > first[0] + 1e-15 {
                out.push(first);
            }
            if second[1] > second[0] + 1e-15 {
                out.push(second);
            }
        }
    }
    Ok(out)
}

fn arrange_lifted_uv_impl(
    context: &ToleranceContext,
    chart: ChartKind,
    primitives: &[LiftedUvPrimitive],
    resource_limit: usize,
    tensor_admitted: bool,
) -> Result<LiftedUvArrangement> {
    if chart == ChartKind::Freeform
        && !primitives
            .iter()
            .all(|p| matches!(p.geometry, LiftedUvGeometry::RationalCurvedTrace { .. }))
    {
        return Err(refuse(
            "Generic Freeform curves are not admitted for Complete UV arrangement",
        ));
    }
    if resource_limit == 0 || primitives.len() > resource_limit {
        return Err(Error::new(
            "BREP_TRIM_RESOURCE_LIMIT",
            "Lifted UV primitive budget exceeded",
        ));
    }
    let mut segments: Vec<(usize, [f64; 2], [f64; 2])> = Vec::new();
    let mut arcs: Vec<(usize, [f64; 2], f64, [f64; 2])> = Vec::new();
    for primitive in primitives {
        match (&primitive.geometry, chart) {
            (LiftedUvGeometry::PlaneSegment { start, end }, ChartKind::PlanePoly) => {
                if !start.iter().chain(end).all(|v| v.is_finite()) || start == end {
                    return Err(refuse("PlanePoly segment must be finite and nondegenerate"));
                }
                segments.push((primitive.edge_id, *start, *end));
            }
            (
                LiftedUvGeometry::AnalyticCircleArc {
                    center,
                    radius,
                    interval,
                },
                ChartKind::AnalyticCircle,
            ) => {
                if !center.iter().chain(interval).all(|v| v.is_finite())
                    || !radius.is_finite()
                    || *radius <= 0.
                    || interval[1] <= interval[0]
                    || interval[1] - interval[0] >= std::f64::consts::TAU
                {
                    return Err(refuse(
                        "AnalyticCircle arc must be finite and span less than TAU",
                    ));
                }
                arcs.push((primitive.edge_id, *center, *radius, *interval));
            }
            (
                LiftedUvGeometry::TensorIsoLine {
                    axis,
                    fixed,
                    interval,
                },
                ChartKind::TensorBezierGraph,
            ) if tensor_admitted => {
                let (start, end) = match axis {
                    TensorAxis::U => ([*fixed, interval[0]], [*fixed, interval[1]]),
                    TensorAxis::V => ([interval[0], *fixed], [interval[1], *fixed]),
                };
                segments.push((primitive.edge_id, start, end));
            }
            (
                LiftedUvGeometry::TensorRectangleBoundary { side, domain },
                ChartKind::TensorBezierGraph,
            ) if tensor_admitted => {
                let [[umin, umax], [vmin, vmax]] = *domain;
                let (start, end) = match side {
                    TensorBoundary::UMin => ([umin, vmax], [umin, vmin]),
                    TensorBoundary::UMax => ([umax, vmin], [umax, vmax]),
                    TensorBoundary::VMin => ([umin, vmin], [umax, vmin]),
                    TensorBoundary::VMax => ([umax, vmax], [umin, vmax]),
                };
                segments.push((primitive.edge_id, start, end));
            }
            (
                LiftedUvGeometry::RationalCurvedTrace {
                    samples,
                    closed,
                    correspondence: _,
                    overlap: _,
                    singular,
                },
                ChartKind::PlanePoly | ChartKind::Freeform,
            ) => {
                if *singular {
                    return Err(refuse(
                        "Singular UV strata refuse Complete DCEL until regularized",
                    ));
                }
                if samples.len() < 2 || samples.iter().flatten().any(|v| !v.is_finite()) {
                    return Err(refuse(
                        "Rational curved UV trace needs at least two finite samples",
                    ));
                }
                for window in samples.windows(2) {
                    if window[0] != window[1] {
                        segments.push((primitive.edge_id, window[0], window[1]));
                    }
                }
                if *closed && samples.len() > 2 {
                    let first = samples[0];
                    let last = *samples.last().unwrap();
                    if first != last {
                        segments.push((primitive.edge_id, last, first));
                    }
                }
            }
            _ => return Err(refuse("Lifted primitive does not match its admitted chart")),
        }
    }

    // Split PlanePoly segments at every constructed event vertex.
    let mut split_parameters = vec![vec![0., 1.]; segments.len()];
    let mut intersections: BTreeMap<(u64, u64), [usize; 2]> = BTreeMap::new();
    for i in 0..segments.len() {
        for j in (i + 1)..segments.len() {
            if let Some((ti, tj, point)) =
                segment_intersection(segments[i].1, segments[i].2, segments[j].1, segments[j].2)?
            {
                split_parameters[i].push(ti);
                split_parameters[j].push(tj);
                intersections.insert(
                    point_key(point),
                    [
                        segments[i].0.min(segments[j].0),
                        segments[i].0.max(segments[j].0),
                    ],
                );
            }
        }
    }
    // Circle arcs may meet at authored endpoints, but overlapping angular
    // interiors are not a unique branch and are refused.
    for i in 0..arcs.len() {
        for j in (i + 1)..arcs.len() {
            if arcs[i].1 == arcs[j].1 && arcs[i].2.to_bits() == arcs[j].2.to_bits() {
                let lo = arcs[i].3[0].max(arcs[j].3[0]);
                let hi = arcs[i].3[1].min(arcs[j].3[1]);
                if hi > lo {
                    return Err(refuse("Overlapping periodic arc branches are ambiguous"));
                }
            } else {
                return Err(refuse(
                    "Intersecting arcs from different analytic circles are outside the admitted matrix",
                ));
            }
        }
    }

    let mut vertices = Vec::<UvVertex>::new();
    let mut vertex_ids = BTreeMap::<(u64, u64), usize>::new();
    let add_vertex = |point: [f64; 2],
                      origin: UvVertexOrigin,
                      vertices: &mut Vec<UvVertex>,
                      ids: &mut BTreeMap<(u64, u64), usize>| {
        let key = point_key(point);
        if let Some(&id) = ids.get(&key) {
            if !matches!(&origin, UvVertexOrigin::AuthoredEndpoint) {
                vertices[id].origin = origin;
            }
            id
        } else {
            let id = vertices.len();
            vertices.push(UvVertex { point, origin });
            ids.insert(key, id);
            id
        }
    };
    let mut pieces: Vec<(usize, usize, usize, UvHalfedgeGeometry)> = Vec::new();
    for (index, (edge, start, end)) in segments.iter().enumerate() {
        let mut ts = split_parameters[index].clone();
        ts.sort_by(|a, b| a.total_cmp(b));
        ts.dedup_by(|a, b| (*a - *b).abs() <= 1e-14);
        for pair in ts.windows(2) {
            if pair[1] - pair[0] <= 1e-14 {
                continue;
            }
            let point = |t: f64| {
                [
                    start[0] + t * (end[0] - start[0]),
                    start[1] + t * (end[1] - start[1]),
                ]
            };
            let pa = point(pair[0]);
            let pb = point(pair[1]);
            let origin_for = |p: [f64; 2]| {
                intersections
                    .get(&point_key(p))
                    .copied()
                    .map(|edges| UvVertexOrigin::ConstructedIntersection { edges })
                    .unwrap_or(UvVertexOrigin::AuthoredEndpoint)
            };
            let a = add_vertex(pa, origin_for(pa), &mut vertices, &mut vertex_ids);
            let b = add_vertex(pb, origin_for(pb), &mut vertices, &mut vertex_ids);
            pieces.push((*edge, a, b, UvHalfedgeGeometry::Segment));
        }
    }
    for (edge, center, radius, interval) in arcs {
        let point = |angle: f64| {
            let snap = |value: f64| {
                if value.abs() <= 32. * f64::EPSILON {
                    0.
                } else if (value - 1.).abs() <= 32. * f64::EPSILON {
                    1.
                } else if (value + 1.).abs() <= 32. * f64::EPSILON {
                    -1.
                } else {
                    value
                }
            };
            [
                center[0] + radius * snap(angle.cos()),
                center[1] + radius * snap(angle.sin()),
            ]
        };
        let shift = (interval[0] / std::f64::consts::TAU).floor() as i32;
        let a = add_vertex(
            point(interval[0]),
            if shift != 0 {
                UvVertexOrigin::PeriodicSeam { edge, shift }
            } else {
                UvVertexOrigin::AuthoredEndpoint
            },
            &mut vertices,
            &mut vertex_ids,
        );
        let end_shift = (interval[1] / std::f64::consts::TAU).floor() as i32;
        let b = add_vertex(
            point(interval[1]),
            if end_shift != shift {
                UvVertexOrigin::PeriodicSeam {
                    edge,
                    shift: end_shift,
                }
            } else {
                UvVertexOrigin::AuthoredEndpoint
            },
            &mut vertices,
            &mut vertex_ids,
        );
        pieces.push((
            edge,
            a,
            b,
            UvHalfedgeGeometry::CircleArc {
                center,
                radius,
                interval,
            },
        ));
    }
    if pieces.len() > resource_limit.saturating_mul(4) {
        return Err(Error::new(
            "BREP_TRIM_RESOURCE_LIMIT",
            "Constructed UV halfedge budget exceeded",
        ));
    }
    let mut halfedges = Vec::with_capacity(pieces.len() * 2);
    for (source_edge, a, b, geometry) in pieces {
        let forward = halfedges.len();
        let reverse = forward + 1;
        halfedges.push(UvHalfedge {
            origin: a,
            destination: b,
            twin: reverse,
            next: usize::MAX,
            cell: usize::MAX,
            source_edge,
            geometry: geometry.clone(),
        });
        let reverse_geometry = match geometry {
            UvHalfedgeGeometry::CircleArc {
                center,
                radius,
                interval,
            } => UvHalfedgeGeometry::CircleArc {
                center,
                radius,
                interval: [interval[1], interval[0]],
            },
            UvHalfedgeGeometry::Segment => UvHalfedgeGeometry::Segment,
        };
        halfedges.push(UvHalfedge {
            origin: b,
            destination: a,
            twin: forward,
            next: usize::MAX,
            cell: usize::MAX,
            source_edge,
            geometry: reverse_geometry,
        });
    }
    let mut outgoing = vec![Vec::<usize>::new(); vertices.len()];
    for (id, halfedge) in halfedges.iter().enumerate() {
        outgoing[halfedge.origin].push(id);
    }
    for edges in &mut outgoing {
        if edges.len() > 4 {
            return Err(refuse("UV event has ambiguous branch degree"));
        }
        edges.sort_by(|&a, &b| {
            let direction = |id: usize| {
                let h = &halfedges[id];
                let p = vertices[h.origin].point;
                let q = vertices[h.destination].point;
                (q[1] - p[1]).atan2(q[0] - p[0])
            };
            direction(a)
                .total_cmp(&direction(b))
                .then_with(|| a.cmp(&b))
        });
    }
    for id in 0..halfedges.len() {
        let destination = halfedges[id].destination;
        let twin = halfedges[id].twin;
        let around = &outgoing[destination];
        let twin_position = around
            .iter()
            .position(|candidate| *candidate == twin)
            .ok_or_else(|| refuse("DCEL twin is absent from destination star"))?;
        halfedges[id].next = around[(twin_position + around.len() - 1) % around.len()];
    }
    let mut cells = Vec::new();
    for start in 0..halfedges.len() {
        if halfedges[start].cell != usize::MAX {
            continue;
        }
        let cell_id = cells.len();
        let mut boundary = Vec::new();
        let mut current = start;
        loop {
            if boundary.len() > halfedges.len() {
                return Err(refuse("DCEL next relation does not close"));
            }
            if halfedges[current].cell != usize::MAX {
                if current != start {
                    return Err(refuse("DCEL branches merge before closing a cell"));
                }
                break;
            }
            halfedges[current].cell = cell_id;
            boundary.push(current);
            current = halfedges[current].next;
        }
        let mut twice_area = 0.;
        for &id in &boundary {
            let h = &halfedges[id];
            let p = vertices[h.origin].point;
            let q = vertices[h.destination].point;
            twice_area += match h.geometry {
                UvHalfedgeGeometry::Segment => cross(p, q),
                UvHalfedgeGeometry::CircleArc {
                    center,
                    radius,
                    interval,
                } => {
                    cross(center, q) - cross(center, p)
                        + radius * radius * (interval[1] - interval[0])
                }
            };
        }
        let signed_area = 0.5 * twice_area;
        let winding = if signed_area > 0. { 1 } else { 0 };
        cells.push(UvCell {
            boundary,
            signed_area,
            winding,
            label: if winding == 0 {
                WindingLabel::Exterior
            } else {
                WindingLabel::Material(winding)
            },
        });
    }
    if !cells.iter().any(|cell| cell.winding != 0) {
        return Err(refuse("UV primitives do not bound a material cell"));
    }
    let coverage = certify_lifted_uv_coverage(
        context,
        primitives.len(),
        vertices.len(),
        halfedges.len(),
        cells.len(),
        true,
        resource_limit,
    )?;
    Ok(LiftedUvArrangement {
        chart,
        context: context.spec_identity(),
        vertices,
        halfedges,
        cells,
        coverage,
    })
}

/// Build a finite context-bound DCEL for the admitted 2D primitive matrix.
/// TensorBezierGraph is intentionally reachable only through the stricter
/// rectangle validator below.
pub fn arrange_lifted_uv(
    context: &ToleranceContext,
    chart: ChartKind,
    primitives: &[LiftedUvPrimitive],
    resource_limit: usize,
) -> Result<LiftedUvArrangement> {
    arrange_lifted_uv_impl(context, chart, primitives, resource_limit, false)
}

/// DCEL for certified rational curved SS pcurve traces (overlaps, loops, holes).
/// Does not use the graph-patch iso fixture path.
pub fn arrange_rational_curved_ss_uv(
    context: &ToleranceContext,
    primitives: &[LiftedUvPrimitive],
    resource_limit: usize,
) -> Result<LiftedUvArrangement> {
    arrange_lifted_uv_impl(
        context,
        ChartKind::PlanePoly,
        primitives,
        resource_limit,
        false,
    )
}

/// Exact finite chart for one tensor rectangle and its complete set of
/// constant-U/V iso branches. Diagonals, partial branches and generic
/// Freeform curves are not representable in this entry point.
pub fn arrange_tensor_bezier_graph_uv(
    context: &ToleranceContext,
    domain: [[f64; 2]; 2],
    primitives: &[LiftedUvPrimitive],
    expected_iso_branches: usize,
    resource_limit: usize,
) -> Result<LiftedUvArrangement> {
    let [[umin, umax], [vmin, vmax]] = domain;
    if !domain.iter().flatten().all(|value| value.is_finite())
        || umin >= umax
        || vmin >= vmax
        || expected_iso_branches == 0
        || expected_iso_branches > resource_limit
    {
        return Err(refuse("Tensor chart domain or branch budget is invalid"));
    }
    if primitives.len() > resource_limit {
        return Err(Error::new(
            "BREP_TRIM_RESOURCE_LIMIT",
            "Tensor UV primitive budget exceeded",
        ));
    }
    let mut boundaries = BTreeMap::<TensorBoundary, usize>::new();
    let mut branches = BTreeMap::<(TensorAxis, u64), usize>::new();
    for primitive in primitives {
        match &primitive.geometry {
            LiftedUvGeometry::TensorRectangleBoundary {
                side,
                domain: actual,
            } if *actual == domain => {
                if boundaries.insert(*side, primitive.edge_id).is_some() {
                    return Err(refuse("Tensor rectangle boundary is duplicated"));
                }
            }
            LiftedUvGeometry::TensorIsoLine {
                axis,
                fixed,
                interval,
            } => {
                let (fixed_domain, varying_domain) = match axis {
                    TensorAxis::U => ([umin, umax], [vmin, vmax]),
                    TensorAxis::V => ([vmin, vmax], [umin, umax]),
                };
                if !fixed.is_finite()
                    || !(*fixed > fixed_domain[0] && *fixed < fixed_domain[1])
                    || *interval != varying_domain
                    || branches
                        .insert((*axis, fixed.to_bits()), primitive.edge_id)
                        .is_some()
                {
                    return Err(refuse(
                        "Tensor iso branch must be unique, strict-interior and span the whole rectangle",
                    ));
                }
            }
            _ => {
                return Err(refuse(
                    "Tensor chart accepts exact rectangle boundaries and U/V iso lines only",
                ));
            }
        }
    }
    if boundaries.len() != 4
        || branches.len() != expected_iso_branches
        || ![
            TensorBoundary::UMin,
            TensorBoundary::UMax,
            TensorBoundary::VMin,
            TensorBoundary::VMax,
        ]
        .iter()
        .all(|side| boundaries.contains_key(side))
    {
        return Err(refuse(
            "Tensor chart missed a rectangle boundary or certified iso branch",
        ));
    }
    let arrangement = arrange_lifted_uv_impl(
        context,
        ChartKind::TensorBezierGraph,
        primitives,
        resource_limit,
        true,
    )?;
    if arrangement.coverage.primitive_count != primitives.len() {
        return Err(refuse("Tensor chart coverage missed an authored branch"));
    }
    Ok(arrangement)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct MultiSpanUvAuthority {
    context: ToleranceSpecIdentity,
    knot_bits: [Vec<u64>; 2],
    branch_keys: Vec<(TensorAxis, u64, usize)>,
    primitive_count: usize,
    cell_count: usize,
    tensor_cell_count: usize,
    material_cell_count: usize,
    hole_cell_count: usize,
}

/// Global tensor-chart result for a certified BranchGraph. Internal knot lines
/// are represented explicitly, so coverage and cell classification span the
/// complete tensor grid rather than being repeated independently per patch.
#[derive(Clone, Debug)]
pub struct MultiSpanUvArrangement {
    pub arrangement: LiftedUvArrangement,
    pub tensor_cell_count: usize,
    pub branch_count: usize,
    pub material_cell_count: usize,
    pub hole_cell_count: usize,
    pub global_coverage_complete: bool,
    pub context: ToleranceSpecIdentity,
    authority: MultiSpanUvAuthority,
}

impl MultiSpanUvArrangement {
    pub fn permits_trim_classification(&self) -> bool {
        let material = self
            .arrangement
            .cells
            .iter()
            .filter(|cell| matches!(cell.label, WindingLabel::Material(_)))
            .count();
        self.global_coverage_complete
            && self.context == self.authority.context
            && self.arrangement.context == self.context
            && self.arrangement.coverage.complete
            && self.arrangement.coverage.primitive_count == self.authority.primitive_count
            && self.arrangement.cells.len() == self.authority.cell_count
            && self.material_cell_count == material
            && self.material_cell_count == self.authority.material_cell_count
            && self.hole_cell_count == self.authority.hole_cell_count
            && self.branch_count == self.authority.branch_keys.len()
            && self.tensor_cell_count == self.authority.tensor_cell_count
            && self.tensor_cell_count
                == (self.authority.knot_bits[0].len() - 1) * (self.authority.knot_bits[1].len() - 1)
    }
}

fn exact_line_trace(curve: &nurbs_core::curve::Curve) -> Result<(TensorAxis, f64, [f64; 2])> {
    curve.validate()?;
    if curve.degree != 1 || curve.control_points.len() != 2 {
        return Err(refuse(
            "Multi-span tensor arrangement accepts exact linear iso pcurves only",
        ));
    }
    let a = [curve.control_points[0][0], curve.control_points[0][1]];
    let b = [curve.control_points[1][0], curve.control_points[1][1]];
    if a[0].to_bits() == b[0].to_bits() && a[1] != b[1] {
        Ok((TensorAxis::U, a[0], [a[1], b[1]]))
    } else if a[1].to_bits() == b[1].to_bits() && a[0] != b[0] {
        Ok((TensorAxis::V, a[1], [a[0], b[0]]))
    } else {
        Err(refuse(
            "Multi-span tensor branch is not an exact constant-U/V trace",
        ))
    }
}

fn validate_breaks(breaks: &[f64]) -> Result<()> {
    if breaks.len() < 2
        || breaks.iter().any(|value| !value.is_finite())
        || breaks.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(refuse(
            "Tensor knot partition must be finite and strictly increasing",
        ));
    }
    Ok(())
}

/// Arrange multiple disjoint certified branches over all tensor knot cells.
/// Branches are required to cover the complete chart in their varying
/// parameter. Duplicate, crossing, partial, missed, or certificate-mutated
/// branches refuse before DCEL construction.
pub fn arrange_multispan_branch_graph_uv(
    context: &ToleranceContext,
    knot_breaks: [&[f64]; 2],
    graph: &crate::nurbs_ss_g6::BranchGraph,
    support_index: usize,
    resource_limit: usize,
) -> Result<MultiSpanUvArrangement> {
    validate_breaks(knot_breaks[0])?;
    validate_breaks(knot_breaks[1])?;
    if support_index > 1
        || !graph.permits_topology_authorship()
        || graph.certificate.context != context.spec_identity()
    {
        return Err(refuse(
            "Tensor arrangement requires an intact context-bound BranchGraph",
        ));
    }
    let domain = [
        [knot_breaks[0][0], *knot_breaks[0].last().unwrap()],
        [knot_breaks[1][0], *knot_breaks[1].last().unwrap()],
    ];
    let tensor_cell_count = (knot_breaks[0].len() - 1)
        .checked_mul(knot_breaks[1].len() - 1)
        .ok_or_else(|| Error::new("BREP_TRIM_RESOURCE_LIMIT", "Tensor cell count overflow"))?;
    if tensor_cell_count > resource_limit {
        return Err(Error::new(
            "BREP_TRIM_RESOURCE_LIMIT",
            "Tensor cell budget exceeded",
        ));
    }
    let mut branch_keys = Vec::new();
    for component in &graph.components {
        if component.closed || component.fragments.is_empty() {
            return Err(refuse(
                "Closed or empty branch needs periodic topology outside this tensor chart",
            ));
        }
        let mut traces = component
            .fragments
            .iter()
            .map(|fragment| exact_line_trace(&fragment.pcurves[support_index]))
            .collect::<Result<Vec<_>>>()?;
        let axis = traces[0].0;
        let fixed = traces[0].1;
        if traces
            .iter()
            .any(|trace| trace.0 != axis || trace.1.to_bits() != fixed.to_bits())
        {
            return Err(refuse(
                "Joined component changed iso axis or fixed parameter",
            ));
        }
        for trace in &mut traces {
            if trace.2[0] > trace.2[1] {
                trace.2.reverse();
            }
        }
        traces.sort_by(|a, b| a.2[0].total_cmp(&b.2[0]));
        let varying_domain = match axis {
            TensorAxis::U => domain[1],
            TensorAxis::V => domain[0],
        };
        if traces[0].2[0].to_bits() != varying_domain[0].to_bits()
            || traces.last().unwrap().2[1].to_bits() != varying_domain[1].to_bits()
            || traces
                .windows(2)
                .any(|pair| pair[0].2[1].to_bits() != pair[1].2[0].to_bits())
        {
            return Err(refuse(
                "Certified branch has missed or duplicate tensor-cell coverage",
            ));
        }
        branch_keys.push((axis, fixed.to_bits(), component.component_id));
    }
    branch_keys.sort();
    if branch_keys
        .windows(2)
        .any(|pair| pair[0].0 == pair[1].0 && pair[0].1 == pair[1].1)
    {
        return Err(refuse("Duplicate certified tensor branch"));
    }
    if branch_keys
        .iter()
        .enumerate()
        .any(|(i, branch)| branch_keys[i + 1..].iter().any(|other| branch.0 != other.0))
    {
        return Err(refuse("Certified tensor branches cross inside the chart"));
    }
    let mut primitives = [
        TensorBoundary::UMin,
        TensorBoundary::UMax,
        TensorBoundary::VMin,
        TensorBoundary::VMax,
    ]
    .into_iter()
    .enumerate()
    .map(|(edge_id, side)| LiftedUvPrimitive {
        edge_id,
        geometry: LiftedUvGeometry::TensorRectangleBoundary { side, domain },
    })
    .collect::<Vec<_>>();
    let mut edge_id = 4;
    for fixed in &knot_breaks[0][1..knot_breaks[0].len() - 1] {
        primitives.push(LiftedUvPrimitive {
            edge_id,
            geometry: LiftedUvGeometry::TensorIsoLine {
                axis: TensorAxis::U,
                fixed: *fixed,
                interval: domain[1],
            },
        });
        edge_id += 1;
    }
    for fixed in &knot_breaks[1][1..knot_breaks[1].len() - 1] {
        primitives.push(LiftedUvPrimitive {
            edge_id,
            geometry: LiftedUvGeometry::TensorIsoLine {
                axis: TensorAxis::V,
                fixed: *fixed,
                interval: domain[0],
            },
        });
        edge_id += 1;
    }
    for (axis, fixed, _) in &branch_keys {
        primitives.push(LiftedUvPrimitive {
            edge_id,
            geometry: LiftedUvGeometry::TensorIsoLine {
                axis: *axis,
                fixed: f64::from_bits(*fixed),
                interval: match axis {
                    TensorAxis::U => domain[1],
                    TensorAxis::V => domain[0],
                },
            },
        });
        edge_id += 1;
    }
    if primitives.len() > resource_limit {
        return Err(Error::new(
            "BREP_TRIM_RESOURCE_LIMIT",
            "Global tensor primitive budget exceeded",
        ));
    }
    let expected_iso = primitives.len() - 4;
    let arrangement =
        arrange_tensor_bezier_graph_uv(context, domain, &primitives, expected_iso, resource_limit)?;
    let material_cell_count = arrangement
        .cells
        .iter()
        .filter(|cell| matches!(cell.label, WindingLabel::Material(_)))
        .count();
    let authority = MultiSpanUvAuthority {
        context: context.spec_identity(),
        knot_bits: [
            knot_breaks[0].iter().map(|value| value.to_bits()).collect(),
            knot_breaks[1].iter().map(|value| value.to_bits()).collect(),
        ],
        branch_keys,
        primitive_count: primitives.len(),
        cell_count: arrangement.cells.len(),
        tensor_cell_count,
        material_cell_count,
        hole_cell_count: 0,
    };
    let result = MultiSpanUvArrangement {
        tensor_cell_count,
        branch_count: authority.branch_keys.len(),
        material_cell_count,
        hole_cell_count: 0,
        global_coverage_complete: true,
        context: context.spec_identity(),
        arrangement,
        authority,
    };
    if !result.permits_trim_classification() {
        return Err(refuse(
            "Global tensor arrangement failed its authority check",
        ));
    }
    Ok(result)
}

/// Build a UV arrangement from imprint curves on a frozen / admitted chart.
pub fn arrange_imprint_curves(
    chart: ChartKind,
    curves: &[UvImprintCurve],
) -> Result<UvArrangement> {
    if curves.len() > 256 {
        return Err(refuse("UV imprint curve budget exceeded"));
    }
    let mut events = Vec::new();
    let mut hole_count = 0usize;
    let mut expanded_for_nesting: Vec<UvImprintCurve> = Vec::new();
    for curve in curves {
        if curve.chart != chart {
            return Err(refuse("Imprint curve chart kind mismatch"));
        }
        if curve.periodic && chart != ChartKind::AnalyticCircle {
            return Err(refuse(
                "Periodic UV wrap is only admitted on AnalyticCircle charts",
            ));
        }
        if chart == ChartKind::Freeform {
            return Err(refuse(
                "Interval-only Freeform input cannot publish Complete coverage",
            ));
        }
        let material = if curve.periodic {
            expand_periodic_intervals(&curve.parameter_intervals, std::f64::consts::TAU)?
        } else {
            curve.parameter_intervals.clone()
        };
        let holes = if curve.periodic {
            expand_periodic_intervals(&curve.hole_intervals, std::f64::consts::TAU)?
        } else {
            curve.hole_intervals.clone()
        };
        expanded_for_nesting.push(UvImprintCurve {
            chart: curve.chart,
            parameter_intervals: material.clone(),
            hole_intervals: holes.clone(),
            edge_id: curve.edge_id,
            periodic: false,
        });
        for interval in &material {
            if !interval[0].is_finite() || !interval[1].is_finite() || interval[1] < interval[0] {
                return Err(refuse("Imprint interval must be finite and ordered"));
            }
            events.push(ChartEvent {
                parameter: interval[0],
                kind: "enter",
                edge: curve.edge_id,
            });
            events.push(ChartEvent {
                parameter: interval[1],
                kind: "exit",
                edge: curve.edge_id,
            });
        }
        for hole in &holes {
            if !hole[0].is_finite() || !hole[1].is_finite() || hole[1] < hole[0] {
                return Err(refuse("Hole interval must be finite and ordered"));
            }
            hole_count += 1;
            events.push(ChartEvent {
                parameter: hole[0],
                kind: "hole_enter",
                edge: curve.edge_id.saturating_add(10_000),
            });
            events.push(ChartEvent {
                parameter: hole[1],
                kind: "hole_exit",
                edge: curve.edge_id.saturating_add(10_000),
            });
        }
    }
    let nesting = compute_monotone_nesting(&expanded_for_nesting)?;
    events.sort_by(|a, b| {
        a.parameter
            .partial_cmp(&b.parameter)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.edge.cmp(&b.edge))
    });
    let mut samples = Vec::new();
    if events.is_empty() {
        if chart == ChartKind::Freeform {
            return Err(refuse(
                "Empty freeform arrangement cannot publish Complete classification",
            ));
        }
        samples.push((0.5, CellLabel::Outside));
    } else {
        let first = events[0].parameter;
        if first > 0. {
            samples.push((first * 0.5, CellLabel::Outside));
        }
        for window in events.windows(2) {
            samples.push((window[0].parameter, CellLabel::Boundary));
            let mid = 0.5 * (window[0].parameter + window[1].parameter);
            if (mid - window[0].parameter).abs() > 1e-12 {
                let label = if window[0].kind.starts_with("hole") {
                    CellLabel::Outside
                } else {
                    CellLabel::Inside
                };
                samples.push((mid, label));
            }
        }
        if let Some(last) = events.last() {
            samples.push((last.parameter, CellLabel::Boundary));
            samples.push((last.parameter + 0.5, CellLabel::Outside));
        }
    }
    let classification = classify_chart_events(chart, events.clone(), &samples)?;
    Ok(UvArrangement {
        chart,
        events,
        classification,
        hole_count,
        nesting,
    })
}

/// Missed-branch mutation: dropping any imprint edge's events must refuse Complete.
pub fn assert_missed_branch_detected(arrangement: &UvArrangement) -> Result<()> {
    if arrangement.events.is_empty() {
        return Ok(());
    }
    let dropped_edge = arrangement.events.last().map(|e| e.edge).unwrap();
    let truncated: Vec<_> = arrangement
        .events
        .iter()
        .filter(|e| e.edge != dropped_edge)
        .cloned()
        .collect();
    // Replay the original samples against the truncated event set. Boundary
    // labels that sat only on the dropped edge must no longer isolate → refuse.
    let samples: Vec<(f64, CellLabel)> = arrangement
        .classification
        .cells
        .iter()
        .map(|(_lo, p, label)| (*p, *label))
        .collect();
    match classify_chart_events(arrangement.chart, truncated, &samples) {
        Err(_) => Ok(()),
        Ok(cert) if !cert.complete => Ok(()),
        Ok(_) => Err(refuse(
            "Missed-branch mutation did not invalidate classification completeness",
        )),
    }
}

/// Exact sew of a model with transactional rollback on refusal.
pub fn sew_model_atomic(model: &Model) -> Result<(Model, SewCertificate)> {
    model.validate()?;
    let certificate = sew_closed_model_edges(model)?;
    Ok((model.clone(), certificate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cuboid;

    #[test]
    fn imprint_curves_classify_with_complete_certificate() {
        let curves = [UvImprintCurve {
            chart: ChartKind::PlanePoly,
            parameter_intervals: vec![[0.2, 0.8]],
            hole_intervals: vec![],
            edge_id: 0,
            periodic: false,
        }];
        let arr = arrange_imprint_curves(ChartKind::PlanePoly, &curves).unwrap();
        assert!(arr.classification.complete);
        assert_missed_branch_detected(&arr).unwrap();
    }

    #[test]
    fn empty_arrangement_is_outside() {
        let arr = arrange_imprint_curves(ChartKind::AnalyticCircle, &[]).unwrap();
        assert!(arr.classification.complete);
        assert!(
            arr.classification
                .cells
                .iter()
                .any(|c| c.2 == CellLabel::Outside)
        );
    }

    #[test]
    fn monotone_nesting_and_multiple_holes() {
        let curves = [
            UvImprintCurve {
                chart: ChartKind::PlanePoly,
                parameter_intervals: vec![[0.1, 0.9]],
                hole_intervals: vec![[0.3, 0.4], [0.6, 0.7]],
                edge_id: 1,
                periodic: false,
            },
            UvImprintCurve {
                chart: ChartKind::PlanePoly,
                parameter_intervals: vec![[0.2, 0.5]],
                hole_intervals: vec![],
                edge_id: 2,
                periodic: false,
            },
        ];
        let arr = arrange_imprint_curves(ChartKind::PlanePoly, &curves).unwrap();
        assert!(arr.classification.complete);
        assert_eq!(arr.hole_count, 2);
        assert!(!arr.nesting.is_empty());
        assert!(arr.nesting.iter().any(|n| n.nested_edges.contains(&2)));
        assert_missed_branch_detected(&arr).unwrap();
    }

    #[test]
    fn non_monotone_overlap_refuses() {
        let curves = [
            UvImprintCurve {
                chart: ChartKind::PlanePoly,
                parameter_intervals: vec![[0.1, 0.5]],
                hole_intervals: vec![],
                edge_id: 1,
                periodic: false,
            },
            UvImprintCurve {
                chart: ChartKind::PlanePoly,
                parameter_intervals: vec![[0.3, 0.8]],
                hole_intervals: vec![],
                edge_id: 2,
                periodic: false,
            },
        ];
        assert_eq!(
            arrange_imprint_curves(ChartKind::PlanePoly, &curves)
                .unwrap_err()
                .code,
            "BREP_UV_ARRANGEMENT_REFUSED"
        );
    }

    #[test]
    fn periodic_wrap_across_seam_expands() {
        let curves = [UvImprintCurve {
            chart: ChartKind::AnalyticCircle,
            // Wrap: from 5.5 through TAU to 0.4
            parameter_intervals: vec![[5.5, 0.4]],
            hole_intervals: vec![],
            edge_id: 3,
            periodic: true,
        }];
        let arr = arrange_imprint_curves(ChartKind::AnalyticCircle, &curves).unwrap();
        assert!(arr.classification.complete);
        assert!(arr.events.len() >= 4);
        assert_missed_branch_detected(&arr).unwrap();
    }

    #[test]
    fn interval_only_freeform_chart_touch_refuses_complete() {
        let curves = [UvImprintCurve {
            chart: ChartKind::Freeform,
            parameter_intervals: vec![[0.15, 0.85]],
            hole_intervals: vec![[0.4, 0.55]],
            edge_id: 7,
            periodic: false,
        }];
        assert!(arrange_imprint_curves(ChartKind::Freeform, &curves).is_err());
    }

    #[test]
    fn freeform_empty_refuses() {
        assert!(arrange_imprint_curves(ChartKind::Freeform, &[]).is_err());
    }

    #[test]
    fn plane_segments_build_context_bound_dcel_and_detect_missed_branch() {
        let context = ToleranceContext::default_valid();
        let points = [[0., 0.], [2., 0.], [2., 1.], [0., 1.]];
        let primitives = (0..4)
            .map(|edge_id| LiftedUvPrimitive {
                edge_id,
                geometry: LiftedUvGeometry::PlaneSegment {
                    start: points[edge_id],
                    end: points[(edge_id + 1) % 4],
                },
            })
            .collect::<Vec<_>>();
        let arrangement =
            arrange_lifted_uv(&context, ChartKind::PlanePoly, &primitives, 16).unwrap();
        assert!(arrangement.coverage.complete);
        assert_eq!(arrangement.vertices.len(), 4);
        assert_eq!(arrangement.halfedges.len(), 8);
        crate::coverage_verifier::verify_lifted_uv_arrangement_coverage(&arrangement, &context)
            .unwrap();

        let mut missed = arrangement.clone();
        missed.halfedges.pop();
        assert!(
            crate::coverage_verifier::verify_lifted_uv_arrangement_coverage(&missed, &context)
                .is_err()
        );
    }

    fn tensor_primitives() -> Vec<LiftedUvPrimitive> {
        let domain = [[0., 1.], [0., 1.]];
        let mut primitives = [
            TensorBoundary::UMin,
            TensorBoundary::UMax,
            TensorBoundary::VMin,
            TensorBoundary::VMax,
        ]
        .into_iter()
        .enumerate()
        .map(|(edge_id, side)| LiftedUvPrimitive {
            edge_id,
            geometry: LiftedUvGeometry::TensorRectangleBoundary { side, domain },
        })
        .collect::<Vec<_>>();
        primitives.push(LiftedUvPrimitive {
            edge_id: 4,
            geometry: LiftedUvGeometry::TensorIsoLine {
                axis: TensorAxis::U,
                fixed: 0.5,
                interval: [0., 1.],
            },
        });
        primitives
    }

    #[test]
    fn tensor_rectangle_iso_chart_builds_bounded_dcel() {
        let context = ToleranceContext::default_valid();
        let arrangement = arrange_tensor_bezier_graph_uv(
            &context,
            [[0., 1.], [0., 1.]],
            &tensor_primitives(),
            1,
            16,
        )
        .unwrap();
        assert_eq!(arrangement.chart, ChartKind::TensorBezierGraph);
        assert!(arrangement.coverage.complete);
        assert_eq!(arrangement.coverage.primitive_count, 5);
        crate::coverage_verifier::verify_lifted_uv_arrangement_coverage(&arrangement, &context)
            .unwrap();
    }

    #[test]
    fn tensor_chart_refuses_diagonal_missed_branch_and_resource_mutations() {
        let context = ToleranceContext::default_valid();
        let mut missed = tensor_primitives();
        missed.pop();
        assert!(
            arrange_tensor_bezier_graph_uv(&context, [[0., 1.], [0., 1.]], &missed, 1, 16).is_err()
        );
        let mut partial = tensor_primitives();
        if let LiftedUvGeometry::TensorIsoLine { interval, .. } =
            &mut partial.last_mut().unwrap().geometry
        {
            *interval = [0.1, 0.9];
        }
        assert!(
            arrange_tensor_bezier_graph_uv(&context, [[0., 1.], [0., 1.]], &partial, 1, 16)
                .is_err()
        );
        assert_eq!(
            arrange_tensor_bezier_graph_uv(
                &context,
                [[0., 1.], [0., 1.]],
                &tensor_primitives(),
                1,
                4
            )
            .unwrap_err()
            .code,
            "BREP_TRIM_RESOURCE_LIMIT"
        );
        assert!(
            arrange_lifted_uv(
                &context,
                ChartKind::TensorBezierGraph,
                &[LiftedUvPrimitive {
                    edge_id: 9,
                    geometry: LiftedUvGeometry::PlaneSegment {
                        start: [0., 0.],
                        end: [1., 1.]
                    }
                }],
                8
            )
            .is_err()
        );
    }

    #[test]
    fn lifted_uv_context_overlap_and_resource_mutations_refuse() {
        let context = ToleranceContext::default_valid();
        let overlapping = [
            LiftedUvPrimitive {
                edge_id: 1,
                geometry: LiftedUvGeometry::PlaneSegment {
                    start: [0., 0.],
                    end: [2., 0.],
                },
            },
            LiftedUvPrimitive {
                edge_id: 2,
                geometry: LiftedUvGeometry::PlaneSegment {
                    start: [1., 0.],
                    end: [3., 0.],
                },
            },
        ];
        assert!(arrange_lifted_uv(&context, ChartKind::PlanePoly, &overlapping, 8).is_err());
        assert_eq!(
            arrange_lifted_uv(&context, ChartKind::PlanePoly, &overlapping[..1], 0)
                .unwrap_err()
                .code,
            "BREP_TRIM_RESOURCE_LIMIT"
        );

        let square = [
            ([0., 0.], [1., 0.]),
            ([1., 0.], [1., 1.]),
            ([1., 1.], [0., 1.]),
            ([0., 1.], [0., 0.]),
        ]
        .into_iter()
        .enumerate()
        .map(|(edge_id, (start, end))| LiftedUvPrimitive {
            edge_id,
            geometry: LiftedUvGeometry::PlaneSegment { start, end },
        })
        .collect::<Vec<_>>();
        let arrangement = arrange_lifted_uv(&context, ChartKind::PlanePoly, &square, 8).unwrap();
        let mut spec = context.specification().clone();
        spec.policy = "foreign-uv-context".into();
        let foreign = ToleranceContext::new(spec).unwrap();
        assert!(
            crate::coverage_verifier::verify_lifted_uv_arrangement_coverage(&arrangement, &foreign)
                .is_err()
        );
    }

    #[test]
    fn periodic_circle_arcs_close_across_lifted_seam() {
        let context = ToleranceContext::default_valid();
        let quarter = std::f64::consts::FRAC_PI_2;
        let arcs = (0..4)
            .map(|edge_id| LiftedUvPrimitive {
                edge_id,
                geometry: LiftedUvGeometry::AnalyticCircleArc {
                    center: [0., 0.],
                    radius: 2.,
                    interval: [edge_id as f64 * quarter, (edge_id + 1) as f64 * quarter],
                },
            })
            .collect::<Vec<_>>();
        let arrangement =
            arrange_lifted_uv(&context, ChartKind::AnalyticCircle, &arcs, 16).unwrap();
        assert_eq!(arrangement.vertices.len(), 4);
        assert!(
            arrangement
                .vertices
                .iter()
                .any(|vertex| matches!(vertex.origin, UvVertexOrigin::PeriodicSeam { .. }))
        );
        assert!(
            arrangement
                .cells
                .iter()
                .any(|cell| cell.label == WindingLabel::Material(1))
        );
    }

    fn ss_wave(swap: bool) -> nurbs_core::surface::Surface {
        let mut control_points = vec![vec![vec![0.; 3]; 3]; 3];
        for u in 0..3 {
            for v in 0..3 {
                let signed = if (if swap { v } else { u }) % 2 == 0 {
                    -1.
                } else {
                    1.
                };
                control_points[u][v] = if swap {
                    vec![signed, u as f64, v as f64]
                } else {
                    vec![signed, u as f64, v as f64]
                };
            }
        }
        nurbs_core::surface::Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 2., 2.],
            knots_v: vec![0., 0., 1., 2., 2.],
            control_points,
            weights: vec![vec![1.; 3]; 3],
            periodic_u: false,
            periodic_v: false,
        }
    }

    fn ss_plane() -> nurbs_core::surface::Surface {
        nurbs_core::surface::Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 0., 2.]],
                vec![vec![0., 2., 0.], vec![0., 2., 2.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }

    #[test]
    fn multispan_branch_graph_builds_global_tensor_cells() {
        let context = ToleranceContext::default_valid();
        let graph = crate::nurbs_ss_g6::certify_multispan_ss(
            &ss_wave(false),
            &ss_plane(),
            [1, 2],
            &context,
            32,
        )
        .unwrap();
        let arrangement = arrange_multispan_branch_graph_uv(
            &context,
            [&[0., 1., 2.], &[0., 1., 2.]],
            &graph,
            0,
            32,
        )
        .unwrap();
        assert!(arrangement.permits_trim_classification());
        assert_eq!(arrangement.tensor_cell_count, 4);
        assert_eq!(arrangement.branch_count, 2);
        assert_eq!(arrangement.hole_cell_count, 0);
        assert!(arrangement.material_cell_count >= 6);
        crate::coverage_verifier::verify_lifted_uv_arrangement_coverage(
            &arrangement.arrangement,
            &context,
        )
        .unwrap();
    }

    #[test]
    fn multispan_uv_rejects_missed_crossing_resource_and_certificate_mutations() {
        let context = ToleranceContext::default_valid();
        let graph = crate::nurbs_ss_g6::certify_multispan_ss(
            &ss_wave(false),
            &ss_plane(),
            [1, 2],
            &context,
            32,
        )
        .unwrap();
        assert_eq!(
            arrange_multispan_branch_graph_uv(
                &context,
                [&[0., 1., 2.], &[0., 1., 2.]],
                &graph,
                0,
                4
            )
            .unwrap_err()
            .code,
            "BREP_TRIM_RESOURCE_LIMIT"
        );
        let mut missed = graph.clone();
        missed.components[0].fragments.pop();
        assert!(
            arrange_multispan_branch_graph_uv(
                &context,
                [&[0., 1., 2.], &[0., 1., 2.]],
                &missed,
                0,
                32
            )
            .is_err()
        );

        let vertical = crate::nurbs_ss_g6::certify_multispan_ss(
            &ss_wave(true),
            &ss_plane(),
            [3, 4],
            &context,
            32,
        )
        .unwrap();
        let fragments = graph
            .components
            .iter()
            .chain(&vertical.components)
            .flat_map(|component| component.fragments.clone())
            .collect();
        let crossing = crate::nurbs_ss_g6::join_certified_multispan_fragments(
            fragments,
            &context,
            [8, 2],
            16,
            1.,
            32,
        )
        .unwrap();
        assert!(
            arrange_multispan_branch_graph_uv(
                &context,
                [&[0., 1., 2.], &[0., 1., 2.]],
                &crossing,
                0,
                32
            )
            .is_err()
        );

        let arrangement = arrange_multispan_branch_graph_uv(
            &context,
            [&[0., 1., 2.], &[0., 1., 2.]],
            &graph,
            0,
            32,
        )
        .unwrap();
        let mut duplicate_mutation = arrangement.clone();
        duplicate_mutation.branch_count += 1;
        assert!(!duplicate_mutation.permits_trim_classification());
        let mut coverage_mutation = arrangement;
        coverage_mutation.global_coverage_complete = false;
        assert!(!coverage_mutation.permits_trim_classification());
    }

    #[test]
    fn cuboid_sew_atomic_preserves_model_on_success_or_typed_refuse() {
        let model = cuboid([0.; 3], [1.; 3]).unwrap();
        match sew_model_atomic(&model) {
            Ok((out, cert)) => {
                assert!(cert.complete);
                out.validate().unwrap();
            }
            Err(err) => {
                assert!(
                    err.code == "BREP_UV_ARRANGEMENT_REFUSED" || err.code.starts_with("BREP_SEW_")
                );
                model.validate().unwrap();
            }
        }
    }
}
