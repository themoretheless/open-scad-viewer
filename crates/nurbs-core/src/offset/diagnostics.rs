//! Outward interval diagnostics of the represented offset chord chain.
//! These predicates concern the returned binary64 line definitions, without
//! promoting construction coordinates into an authored source certificate.
use crate::{Result, check, curve_offset::Segment, distance_bounds::Interval};
#[cfg(feature = "codec")]
mod serialization;

pub struct Diagnostics {
    pub crossings: Vec<[usize; 2]>,
    pub contacts: Vec<[usize; 2]>,
    pub uncertain: Vec<[usize; 2]>,
    pub degenerate: Vec<usize>,
    pub complete: bool,
    pub checks: usize,
    pub total_pairs: usize,
}
impl Diagnostics {
    pub fn enumeration_complete(&self) -> bool {
        self.checks == self.total_pairs
    }
    pub fn predicates_complete(&self) -> bool {
        self.uncertain.is_empty()
    }
    pub fn is_simple(&self) -> bool {
        self.complete
            && self.crossings.is_empty()
            && self.contacts.is_empty()
            && self.degenerate.is_empty()
    }
}

/// Inspect the current represented straight edges, independent of offset provenance.
/// Station indices refer to retained control-polygon edges across ordered chunks.

pub fn inspect_curves(curves: &[crate::curve::Curve], max_pairs: usize) -> Result<Diagnostics> {
    check(
        !curves.is_empty() && curves.len() <= 128,
        "Select an ordered chord chain.",
    )?;
    let mut edges = Vec::new();
    let mut z = None;
    for curve in curves {
        curve.validate()?;
        check(
            curve.degree == 1 && !curve.periodic,
            "Chain diagnostics require retained degree-one curves.",
        )?;
        check(
            curve.knots[0] == curve.knots[1]
                && curve.knots[curve.knots.len() - 1] == curve.knots[curve.knots.len() - 2]
                && (0..curve.control_points.len() - 1)
                    .all(|i| curve.knots[i + 1] < curve.knots[i + 2]),
            "Use a continuous clamped chord chain.",
        )?;
        for point in &curve.control_points {
            check(
                point.len() == 2 || point.len() == 3,
                "Use an XY planar chain.",
            )?;
            let height = if point.len() == 3 { point[2] } else { 0. };
            if let Some(previous) = z {
                check(previous == height, "Use a constant Z chain.")?;
            } else {
                z = Some(height);
            }
        }
        for pair in curve.control_points.windows(2) {
            check(edges.len() < 65536, "Chain exceeds 65536 edges.")?;
            let station = edges.len() as f64;
            edges.push(Segment {
                domain: [station, station + 1.],
                points: [[pair[0][0], pair[0][1]], [pair[1][0], pair[1][1]]],
                error_upper_mm: 0.,
            });
        }
    }
    check(!edges.is_empty(), "Chain has no edges.")?;
    let closed = edges[0].points[0] == edges.last().unwrap().points[1];
    inspect_chain(&edges, closed, max_pairs)
}

pub(crate) fn orientation(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> Result<Option<i8>> {
    if (a[0] == b[0] && b[0] == c[0]) || (a[1] == b[1] && b[1] == c[1]) {
        return Ok(Some(0));
    }
    let x = |p: [f64; 2], axis| Interval::point(p[axis]).sub(Interval::point(a[axis]));
    let cross = x(b, 0)?.mul(x(c, 1)?)?.sub(x(b, 1)?.mul(x(c, 0)?)?)?;
    if cross.lo > 0. {
        return Ok(Some(1));
    }
    if cross.hi < 0. {
        return Ok(Some(-1));
    }
    // Resolve the represented binary64 inputs, never an earlier unrounded
    // construction. Indeterminate exact evaluation remains indeterminate.
    use cad_predicates::{
        AuthoredScalar, Limits, Outcome, PredicateContext, Sign, SourceArena, ToleranceContext,
        orient2d,
    };
    let arena = SourceArena::authored(
        "represented-chord-orientation",
        1,
        [a, b, c]
            .into_iter()
            .flatten()
            .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
            .collect(),
    )
    .map_err(|e| crate::input(format!("Invalid chord predicate source: {e}")))?;
    let tolerance = ToleranceContext::default_valid();
    let mut context = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
    let pair = |i| [arena.leaf(i).unwrap(), arena.leaf(i + 1).unwrap()];
    let result = orient2d(&mut context, pair(0), pair(2), pair(4))
        .map_err(|e| crate::input(format!("Chord orientation failed: {e}")))?;
    Ok(match result.outcome {
        Outcome::Sign(Sign::Positive) => Some(1),
        Outcome::Sign(Sign::Negative) => Some(-1),
        Outcome::Sign(Sign::Zero) => Some(0),
        Outcome::Indeterminate(_) => None,
    })
}
pub(crate) fn apart(a: &Segment, b: &Segment) -> bool {
    (0..2).any(|k| {
        a.points[0][k].max(a.points[1][k]) < b.points[0][k].min(b.points[1][k])
            || b.points[0][k].max(b.points[1][k]) < a.points[0][k].min(a.points[1][k])
    })
}
pub(crate) fn shared_vertex_only(a: &Segment, b: &Segment) -> Result<bool> {
    for i in 0..2 {
        for j in 0..2 {
            if a.points[i] != b.points[j] {
                continue;
            }
            let common = a.points[i];
            let left = a.points[1 - i];
            let right = b.points[1 - j];
            if (0..2).any(|k| {
                (left[k] < common[k] && right[k] > common[k])
                    || (right[k] < common[k] && left[k] > common[k])
            }) {
                return Ok(true);
            }
            if let Some(sign) = orientation(common, left, right)? {
                if sign != 0 {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

pub fn inspect_chain(segments: &[Segment], closed: bool, max_pairs: usize) -> Result<Diagnostics> {
    check(
        !segments.is_empty() && segments.len() <= 65536 && (1..=1_000_000).contains(&max_pairs),
        "Chain diagnostics require nonempty bounded segments and 1..1000000 pairs",
    )?;
    check(
        segments.iter().all(|s| {
            s.points
                .iter()
                .flatten()
                .all(|x| x.is_finite() && x.abs() <= 1e9)
        }),
        "Chain diagnostic points exceed coordinate bounds",
    )?;
    check(
        segments.iter().all(|s| {
            s.domain[0].is_finite() && s.domain[1].is_finite() && s.domain[0] < s.domain[1]
        }) && segments
            .windows(2)
            .all(|p| p[0].domain[1] == p[1].domain[0] && p[0].points[1] == p[1].points[0]),
        "Chain diagnostics require connected, ordered parameter cells",
    )?;
    check(
        !closed || segments[0].points[0] == segments[segments.len() - 1].points[1],
        "Closed chain diagnostic requires an exact closing point",
    )?;
    let n = segments.len();
    let mut report = Diagnostics {
        crossings: vec![],
        contacts: vec![],
        uncertain: vec![],
        degenerate: segments
            .iter()
            .enumerate()
            .filter(|(_, s)| s.points[0] == s.points[1])
            .map(|(i, _)| i)
            .collect(),
        complete: true,
        checks: 0,
        total_pairs: n * (n - 1) / 2,
    };
    for i in 0..n {
        for j in i + 1..n {
            if report.checks == max_pairs {
                report.complete = false;
                return Ok(report);
            }
            report.checks += 1;
            let (a, b) = (&segments[i], &segments[j]);
            if apart(a, b) {
                continue;
            }
            let adjacent = j == i + 1 || (closed && i == 0 && j == n - 1);
            if adjacent && shared_vertex_only(a, b)? {
                continue;
            }
            let shared = a.points.iter().any(|p| b.points.contains(p));
            if shared && !adjacent {
                report.contacts.push([i, j]);
                continue;
            }
            let signs = [
                orientation(a.points[0], a.points[1], b.points[0])?,
                orientation(a.points[0], a.points[1], b.points[1])?,
                orientation(b.points[0], b.points[1], a.points[0])?,
                orientation(b.points[0], b.points[1], a.points[1])?,
            ];
            if let [Some(x), Some(y), Some(z), Some(w)] = signs {
                if x * y < 0 && z * w < 0 {
                    report.crossings.push([i, j]);
                    continue;
                }
                if x * y > 0 || z * w > 0 {
                    continue;
                }
                let inside = |p: [f64; 2], line: [[f64; 2]; 2]| {
                    (0..2).all(|k| {
                        p[k] >= line[0][k].min(line[1][k]) && p[k] <= line[0][k].max(line[1][k])
                    })
                };
                if (x == 0 && inside(b.points[0], a.points))
                    || (y == 0 && inside(b.points[1], a.points))
                    || (z == 0 && inside(a.points[0], b.points))
                    || (w == 0 && inside(a.points[1], b.points))
                {
                    report.contacts.push([i, j]);
                    continue;
                }
            }
            // An unresolved touching/overlap predicate cannot prove simplicity.
            report.uncertain.push([i, j]);
            report.complete = false;
        }
    }
    Ok(report)
}

#[cfg(test)]
#[path = "diagnostics/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "diagnostics/current_chain_tests.rs"]
mod current_chain_tests;

#[cfg(test)]
#[path = "diagnostics/exact_orientation_tests.rs"]
mod exact_orientation_tests;
