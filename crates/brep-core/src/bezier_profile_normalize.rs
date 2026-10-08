//! Even-odd arrangement of authored spans. Native intersections split original
//! carriers; winding probes separated from every control hull choose material.
use super::{Result, controls, injective, point_inside, refused};
use crate::Curve;
use nurbs_core::intersection::{CurveCurveComponentKind, intersect_curve_curve_report};
pub fn normalize(wires: &[Vec<Curve>], tolerance: f64) -> Result<Vec<Vec<Curve>>> {
    if wires.is_empty() {
        return Ok(Vec::new());
    }
    if !tolerance.is_finite() || tolerance <= 0. || wires.len() > 32 {
        return Err(refused("Invalid Bézier normalization options"));
    }
    let mut curves = Vec::new();
    for wire in wires {
        if wire.is_empty() {
            return Err(refused("Empty authored boundary"));
        }
        for (i, c) in wire.iter().enumerate() {
            controls(c)?;
            if c.control_points.last() != wire[(i + 1) % wire.len()].control_points.first() {
                return Err(refused("Authored boundary is open"));
            }
            let mut cuts = c.domain().to_vec();
            for k in 0..2 {
                cuts.extend(nurbs_core::curve_extrema::axis_extrema(c, k)?);
            }
            cuts.sort_by(f64::total_cmp);
            cuts.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
            for interval in cuts.windows(2) {
                let piece = c.trim(interval[0], interval[1])?;
                if !injective(&controls(&piece)?, tolerance) {
                    return Err(refused("Authored cusp/injectivity remains unresolved"));
                }
                curves.push(piece);
                if curves.len() > 1022 {
                    return Err(refused("Bézier normalization exceeds 1022 spans"));
                }
            }
        }
    }
    let mut cuts: Vec<_> = curves.iter().map(|c| c.domain().to_vec()).collect();
    let mut work = 0;
    for i in 0..curves.len() {
        for j in i + 1..curves.len() {
            let a = controls(&curves[i])?;
            let b = controls(&curves[j])?;
            if (0..2).any(|k| {
                a.iter()
                    .all(|p| b.iter().all(|q| p[k] < q[k] - 8. * tolerance))
                    || a.iter()
                        .all(|p| b.iter().all(|q| p[k] > q[k] + 8. * tolerance))
            }) {
                continue;
            }
            let shared = [a[0], *a.last().unwrap()]
                .into_iter()
                .find(|p| *p == b[0] || *p == *b.last().unwrap());
            if shared.is_some_and(|p| super::shared_hulls_separate(&a, &b, p)) {
                continue;
            }
            work += 1;
            if work > 8192 {
                return Err(refused("Bézier normalization pair budget exceeded"));
            }
            let report = intersect_curve_curve_report(
                &super::boolean_ops::lift(&curves[i]),
                &super::boolean_ops::lift(&curves[j]),
                None,
            )?;
            if !report.report.unresolved.is_empty() {
                return Err(refused("Self-intersection normalization is unresolved"));
            }
            for event in report.report.components {
                if event.kind != CurveCurveComponentKind::Point || event.residual > tolerance {
                    return Err(refused(
                        "Self-overlap normalization requires a resolved point event",
                    ));
                }
                cuts[i].push(event.first);
                cuts[j].push(event.second);
            }
        }
    }
    let all: Vec<_> = wires.iter().flatten().cloned().collect();
    let mut pieces = Vec::new();
    for (i, source) in curves.iter().enumerate() {
        cuts[i].sort_by(f64::total_cmp);
        cuts[i].dedup_by(|a, b| (*a - *b).abs() < 1e-12);
        for interval in cuts[i].windows(2) {
            if interval[1] - interval[0] <= 1e-12 {
                continue;
            }
            let mut c = source.trim(interval[0], interval[1])?;
            let e = c.evaluate((interval[0] + interval[1]) * 0.5)?;
            let d =
                e.d1.ok_or_else(|| refused("Normalization tangent is unresolved"))?;
            let length = d[0].hypot(d[1]);
            if length <= tolerance {
                return Err(refused("Normalization cusp is unresolved"));
            }
            let n = [-d[1] / length, d[0] / length];
            let mut decision = None;
            // Two distances must agree. Each winding call admits its decision
            // only after complete control-hull separation from the probe.
            for distance in [32. * tolerance, 64. * tolerance] {
                // The source span crosses the normal probe exactly once if
                // its tangent-coordinate Bernstein controls are monotone.
                let source_hull = controls(source)?;
                let monotone = source_hull.windows(2).all(|p| {
                    if p[0] == p[1] {
                        return true;
                    }
                    let value = (p[1][0] - p[0][0]) * d[0] + (p[1][1] - p[0][1]) * d[1];
                    let round = 256.
                        * f64::EPSILON
                        * ((p[0][0].abs() + p[1][0].abs()) * d[0].abs()
                            + (p[0][1].abs() + p[1][1].abs()) * d[1].abs());
                    value > round
                });
                if !monotone {
                    return Err(refused(
                        "Normalization normal-probe uniqueness is unresolved",
                    ));
                }
                let endpoints = [
                    [e.point[0] - n[0] * distance, e.point[1] - n[1] * distance],
                    [e.point[0] + n[0] * distance, e.point[1] + n[1] * distance],
                ];
                for (other_id, other) in curves.iter().enumerate() {
                    if other_id == i {
                        continue;
                    }
                    let mut pending = vec![(other.clone(), 0usize)];
                    while let Some((part, depth)) = pending.pop() {
                        work += 1;
                        if work > 100000 || depth > 40 {
                            return Err(refused("Normalization probe separation budget exceeded"));
                        }
                        let hull = controls(&part)?;
                        let separated = [[1., 0.], [0., 1.], [d[0] / length, d[1] / length]]
                            .into_iter()
                            .any(|axis| {
                                let bounds = |points: &[[f64; 2]]| {
                                    points.iter().fold(
                                        [f64::INFINITY, f64::NEG_INFINITY],
                                        |r, p| {
                                            let v = p[0] * axis[0] + p[1] * axis[1];
                                            [r[0].min(v), r[1].max(v)]
                                        },
                                    )
                                };
                                let a = bounds(&hull);
                                let b = bounds(&endpoints);
                                let round = 512.
                                    * f64::EPSILON
                                    * hull
                                        .iter()
                                        .flatten()
                                        .chain(endpoints.iter().flatten())
                                        .map(|v| v.abs())
                                        .fold(1., f64::max);
                                a[1] + 8. * tolerance + round < b[0]
                                    || b[1] + 8. * tolerance + round < a[0]
                            });
                        if separated {
                            continue;
                        }
                        let [a, b] = part.domain();
                        let [left, right] = part.split((a + b) * 0.5)?;
                        pending.push((right, depth + 1));
                        pending.push((left, depth + 1));
                    }
                }
                let left = point_inside(
                    &all,
                    [e.point[0] + n[0] * distance, e.point[1] + n[1] * distance],
                    tolerance,
                )?;
                let right = point_inside(
                    &all,
                    [e.point[0] - n[0] * distance, e.point[1] - n[1] * distance],
                    tolerance,
                )?;
                if decision.is_some_and(|v| v != (left, right)) {
                    return Err(refused(
                        "Normalization material side is tolerance-ambiguous",
                    ));
                }
                decision = Some((left, right));
            }
            let (left, right) = decision.unwrap();
            if left == right {
                return Err(refused(
                    "Normalization side probes do not resolve the material boundary",
                ));
            }
            if !left {
                c = c.reverse()?;
            }
            pieces.push((c, i));
            if pieces.len() > 2048 {
                return Err(refused("Normalization fragment budget exceeded"));
            }
        }
    }
    super::boolean_ops::assemble(pieces, tolerance, true)
}
