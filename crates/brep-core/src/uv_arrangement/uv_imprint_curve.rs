use super::*;

pub(super) fn refuse(message: &str) -> Error {
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

pub(super) fn cross(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}

fn sub(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

pub(super) fn point_key(point: [f64; 2]) -> (u64, u64) {
    let canonical = |v: f64| if v == 0. { 0f64.to_bits() } else { v.to_bits() };
    (canonical(point[0]), canonical(point[1]))
}

pub(super) fn segment_intersection(
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

pub(super) fn compute_monotone_nesting(curves: &[UvImprintCurve]) -> Result<Vec<NestingRecord>> {
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
    // Laminar-family sweep: walk intervals by (lo asc, hi desc) with a stack of
    // still-open candidate containers instead of comparing every pair. A stack
    // entry is dropped only once it ends strictly before the current interval
    // starts, so it can no longer contain the current or any later interval.
    let n = intervals.len();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| {
        intervals[a]
            .1[0]
            .total_cmp(&intervals[b].1[0])
            .then_with(|| intervals[b].1[1].total_cmp(&intervals[a].1[1]))
            .then_with(|| a.cmp(&b))
    });
    let mut depth_of = vec![0usize; n];
    let mut nested_of: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut stack: Vec<usize> = Vec::new();
    for &i in &order {
        let span = intervals[i].1;
        while stack
            .last()
            .is_some_and(|&top| intervals[top].1[1] < span[0] - 1e-15)
        {
            stack.pop();
        }
        for &container in &stack {
            if interval_contains(intervals[container].1, span) {
                depth_of[i] += 1;
                nested_of[container].push(intervals[i].0);
            }
        }
        stack.push(i);
    }
    let mut nesting = Vec::new();
    for (i, (edge, _)) in intervals.iter().enumerate() {
        let mut nested = std::mem::take(&mut nested_of[i]);
        let depth = depth_of[i];
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
pub(super) fn expand_periodic_intervals(intervals: &[[f64; 2]], period: f64) -> Result<Vec<[f64; 2]>> {
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
