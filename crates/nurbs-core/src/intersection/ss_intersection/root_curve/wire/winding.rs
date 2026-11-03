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
    point: [f64; 2],
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
        let fresh = qualify_original_ss_winding(&self.wire, self.support, self.point, self.limits)?;
        check(
            fresh.winding == self.winding && fresh.segments == self.segments,
            "Original UV winding witness changed",
        )
    }
}
pub(super) fn charge(remaining: &mut OriginalSsCurveLimits, used: &OriginalSsCurveBounds) -> Result<()> {
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
    let mut n = 1usize;
    loop {
        let count = n
            .checked_mul(wire.edges().len())
            .ok_or_else(|| crate::resource("UV winding segment overflow"))?;
        check(
            count <= limits.max_segments.saturating_sub(segments_used),
            "Original UV winding subdivision exhausted",
        )?;
        segments_used += count;
        let mut total = 0i32;
        let mut separated = true;
        for edge in wire.edges() {
            let start = endpoint(edge.start().point(), &mut endpoint_cache, &mut remaining)?;
            let end = endpoint(edge.end().point(), &mut endpoint_cache, &mut remaining)?;
            let a = edge.start().parameter_bounds();
            let b = edge.end().parameter_bounds();
            // Interior partition points must lie strictly between the two
            // uncertain endpoint clocks; the first/last arc boxes include the
            // complete endpoint intervals.
            let (lo, hi) = if edge.forward() {
                (&a[1], &b[0])
            } else {
                (&b[1], &a[0])
            };
            let mut points = Vec::with_capacity(n + 1);
            points.push(start);
            let mut clocks = vec![None];
            for i in 1..n {
                let q = lo + (hi - lo) * Q::from_integer(i.into()) / Q::from_integer(n.into());
                let x = q
                    .to_f64()
                    .filter(|x| x.is_finite())
                    .ok_or_else(|| crate::input("UV winding clock overflow"))?;
                let xq = Q::from_float(x).unwrap();
                check(
                    lo < &xq && &xq < hi,
                    "UV winding interior clock cannot be represented",
                )?;
                clocks.push(Some(x));
            }
            clocks.push(None);
            if !edge.forward() {
                clocks.reverse();
            }
            for clock in clocks.iter().take(n).skip(1) {
                let value = edge.curve().enclose([clock.unwrap(); 2], remaining)?;
                charge(&mut remaining, &value)?;
                points.push(value);
            }
            points.push(end);
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
            for i in 0..n {
                let c0 = clocks[i].unwrap_or(if edge.forward() { lower } else { upper });
                let c1 = clocks[i + 1].unwrap_or(if edge.forward() { upper } else { lower });
                let arc = edge.curve().enclose([c0.min(c1), c0.max(c1)], remaining)?;
                charge(&mut remaining, &arc)?;
                let box2: [[f64; 2]; 2] = std::array::from_fn(|k| {
                    let j = 2 * support + k;
                    [
                        arc.uv[j][0]
                            .min(points[i].uv[j][0])
                            .min(points[i + 1].uv[j][0]),
                        arc.uv[j][1]
                            .max(points[i].uv[j][1])
                            .max(points[i + 1].uv[j][1]),
                    ]
                });
                if !(0..2).any(|k| point[k] < box2[k][0] || box2[k][1] < point[k]) {
                    separated = false;
                    break;
                }
                let a = sample(&points[i], support);
                let b = sample(&points[i + 1], support);
                let sign = orientation(a, b, point);
                if a[1] <= point[1] && point[1] < b[1] && sign > Q::zero() {
                    total += 1;
                }
                if b[1] <= point[1] && point[1] < a[1] && sign < Q::zero() {
                    total -= 1;
                }
            }
            if !separated {
                break;
            }
        }
        if separated {
            return Ok(QualifiedOriginalSsWinding {
                wire: wire.clone(),
                support,
                point,
                winding: total,
                segments: segments_used,
                limits,
            });
        }
        n = n
            .checked_mul(2)
            .ok_or_else(|| crate::resource("UV winding subdivisions overflow"))?;
    }
}
