//! Convex interval homotopies from original UV arcs to a temporary polygon.
//! The polygon is a winding witness only and never replaces source geometry.
use super::*;
use num_traits::Zero;
#[derive(Clone, Copy, Debug)]
pub struct OriginalSsWindingLimits {
    pub max_segments: usize,
    /// Shared across all attempts and all source queries, not per segment.
    pub geometry: OriginalSsCurveLimits,
}
impl Default for OriginalSsWindingLimits {
    fn default() -> Self {
        Self {
            max_segments: 1024,
            geometry: Default::default(),
        }
    }
}
#[derive(Clone, Debug)]
pub struct QualifiedOriginalSsWinding {
    wire: OriginalSsRootWire,
    support: usize,
    query: [[f64; 2]; 2],
    winding: i32,
    segments: usize,
    limits: OriginalSsWindingLimits,
}
impl QualifiedOriginalSsWinding {
    pub fn winding(&self) -> i32 {
        self.winding
    }
    pub fn segments(&self) -> usize {
        self.segments
    }
    pub fn reverify(&self) -> Result<()> {
        self.wire.reverify()?;
        let fresh = qualify_box(&self.wire, self.support, self.query, self.limits)?;
        check(
            fresh.winding == self.winding && fresh.segments == self.segments,
            "Original UV winding witness changed",
        )
    }
}
pub(super) fn charge(
    remaining: &mut OriginalSsCurveLimits,
    used: &OriginalSsCurveBounds,
) -> Result<()> {
    remaining.native_rectangles = remaining
        .native_rectangles
        .checked_sub(used.native_rectangles)
        .ok_or_else(|| crate::resource("Original winding rectangle work exhausted"))?;
    remaining.interval_operations = remaining
        .interval_operations
        .checked_sub(used.interval_operations)
        .ok_or_else(|| crate::resource("Original winding interval work exhausted"))?;
    remaining.projection.max_work = remaining
        .projection
        .max_work
        .checked_sub(used.projection_work)
        .ok_or_else(|| crate::resource("Original winding projection work exhausted"))?;
    remaining.world_rectangles = remaining
        .world_rectangles
        .checked_sub(used.world_rectangles)
        .ok_or_else(|| crate::resource("Original winding world work exhausted"))?;
    Ok(())
}
pub(super) fn endpoint(
    point: &Arc<OriginalSsRootPoint>,
    cache: &mut Vec<(Arc<OriginalSsRootPoint>, OriginalSsCurveBounds)>,
    remaining: &mut OriginalSsCurveLimits,
) -> Result<OriginalSsCurveBounds> {
    if let Some((_, bounds)) = cache.iter().find(|(p, _)| p.same_selector(point)) {
        return Ok(bounds.clone());
    }
    let value = point.enclose(*remaining)?;
    charge(remaining, &value)?;
    cache.push((point.clone(), value.clone()));
    Ok(value)
}
fn sample(bounds: &OriginalSsCurveBounds, support: usize) -> [f64; 2] {
    std::array::from_fn(|k| {
        let d = bounds.uv[2 * support + k];
        (d[0] * 0.5 + d[1] * 0.5).max(d[0]).min(d[1])
    })
}
fn orientation(a: [f64; 2], b: [f64; 2], p: [f64; 2]) -> Q {
    let q = |x| Q::from_float(x).unwrap();
    (q(b[0]) - q(a[0])) * (q(p[1]) - q(a[1])) - (q(b[1]) - q(a[1])) * (q(p[0]) - q(a[0]))
}
/// Winding in one original unlifted UV plane. Periodic homology and pole
/// junctions require a quotient-domain arrangement, so they cannot enter this
/// planar classifier merely because their world incidence is closed.
pub fn qualify_original_ss_winding(
    wire: &OriginalSsRootWire,
    support: usize,
    point: [f64; 2],
    limits: OriginalSsWindingLimits,
) -> Result<QualifiedOriginalSsWinding> {
    qualify_box(wire, support, point.map(|x| [x; 2]), limits)
}
pub(super) fn qualify_box(
    wire: &OriginalSsRootWire,
    support: usize,
    query: [[f64; 2]; 2],
    limits: OriginalSsWindingLimits,
) -> Result<QualifiedOriginalSsWinding> {
    check(
        query
            .iter()
            .all(|d| d.iter().all(|x| x.is_finite()) && d[0] <= d[1]),
        "Invalid original UV winding query box",
    )?;
    let point = query.map(|d| (d[0] * 0.5 + d[1] * 0.5).max(d[0]).min(d[1]));
    check(
        support < 2
            && point.iter().all(|x| x.is_finite())
            && (1..=4096).contains(&limits.max_segments),
        "Invalid original UV winding query",
    )?;
    let source = wire.edges()[0].curve().source();
    for edge in wire.edges() {
        let s = edge.curve().source();
        check(
            s.first() == source.first()
                && s.second() == source.second()
                && edge.start().phase() == [0; 4]
                && edge.end().phase() == [0; 4]
                && edge.start().kind() != OriginalSsPortKind::OriginalSourceNode
                && edge.end().kind() != OriginalSsPortKind::OriginalSourceNode
                && edge.start().kind() != OriginalSsPortKind::CollapsedPole
                && edge.end().kind() != OriginalSsPortKind::CollapsedPole,
            "Original UV winding needs a common planar source chart without quotient junctions",
        )?;
    }
    let mut remaining = limits.geometry;
    let mut endpoint_cache = Vec::new();
    let mut segments_used = 0;
    let mut total = 0i32;
    for edge in wire.edges() {
        let start = endpoint(edge.start().point(), &mut endpoint_cache, &mut remaining)?;
        let end = endpoint(edge.end().point(), &mut endpoint_cache, &mut remaining)?;
        // A complete selector enclosure contained in the query proves actual
        // boundary contact. Refinement cannot establish whole-box separation.
        for endpoint in [&start, &end] {
            check(
                !(0..2).all(|k| {
                    let d = endpoint.uv[2 * support + k];
                    query[k][0] <= d[0] && d[1] <= query[k][1]
                }),
                "Original UV winding query contains a boundary selector",
            )?;
        }
        let a = edge.start().parameter_bounds();
        let b = edge.end().parameter_bounds();
        let (lo, hi) = if edge.forward() {
            (&a[1], &b[0])
        } else {
            (&b[1], &a[0])
        };
        let outer = edge.parameter_bounds();
        let domain = edge.curve().parameter_domain();
        let outward = |q: &Q, lower: bool| -> Result<f64> {
            let x = q
                .to_f64()
                .filter(|x| x.is_finite())
                .ok_or_else(|| crate::input("UV clock overflow"))?;
            let exact = Q::from_float(x).unwrap();
            Ok(if lower && exact > *q {
                x.next_down()
            } else if !lower && exact < *q {
                x.next_up()
            } else {
                x
            })
        };
        let lower = outward(&outer[0], true)?.max(domain[0]);
        let upper = outward(&outer[1], false)?.min(domain[1]);
        let (c0, c1) = if edge.forward() {
            (lower, upper)
        } else {
            (upper, lower)
        };
        // Accepted chords form one connected polygon in source traversal order.
        // Only arcs whose COMPLETE enclosure overlaps the query need splitting.
        let mut pending = vec![(c0, c1, start, end, lo.clone(), hi.clone())];
        while let Some((c0, c1, start, end, lo, hi)) = pending.pop() {
            check(
                segments_used < limits.max_segments,
                "Original UV winding subdivision exhausted",
            )?;
            segments_used += 1;
            let arc = edge.curve().enclose([c0.min(c1), c0.max(c1)], remaining)?;
            charge(&mut remaining, &arc)?;
            let box2: [[f64; 2]; 2] = std::array::from_fn(|k| {
                let j = 2 * support + k;
                [
                    arc.uv[j][0].min(start.uv[j][0]).min(end.uv[j][0]),
                    arc.uv[j][1].max(start.uv[j][1]).max(end.uv[j][1]),
                ]
            });
            if !(0..2).any(|k| query[k][1] < box2[k][0] || box2[k][1] < query[k][0]) {
                let q = (&lo + &hi) / Q::from_integer(2.into());
                let x = q
                    .to_f64()
                    .filter(|x| x.is_finite())
                    .ok_or_else(|| crate::input("UV winding clock overflow"))?;
                let xq = Q::from_float(x).unwrap();
                check(
                    lo < xq && xq < hi && c0.min(c1) < x && x < c0.max(c1),
                    "UV winding interior clock cannot be represented",
                )?;
                let mid = edge.curve().enclose([x; 2], remaining)?;
                charge(&mut remaining, &mid)?;
                let (left, right) = if edge.forward() {
                    ((lo, xq.clone()), (xq, hi))
                } else {
                    ((xq.clone(), hi), (lo, xq))
                };
                pending.push((x, c1, mid.clone(), end, right.0, right.1));
                pending.push((c0, x, start, mid, left.0, left.1));
                continue;
            }
            let a = sample(&start, support);
            let b = sample(&end, support);
            let sign = orientation(a, b, point);
            if a[1] <= point[1] && point[1] < b[1] && sign > Q::zero() {
                total += 1;
            }
            if b[1] <= point[1] && point[1] < a[1] && sign < Q::zero() {
                total -= 1;
            }
        }
    }
    Ok(QualifiedOriginalSsWinding {
        wire: wire.clone(),
        support,
        query,
        winding: total,
        segments: segments_used,
        limits,
    })
}
