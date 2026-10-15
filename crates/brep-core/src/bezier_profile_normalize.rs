//! Even-odd arrangement of authored spans. Native intersections split original
//! carriers; winding probes separated from every control hull choose material.
use super::{Result, controls, injective, point_inside, refused};
use crate::Curve;
use nurbs_core::intersection::{CurveCurveComponentKind, intersect_curve_curve_report};
/// Remove only roundoff-sized endpoint jet noise from a represented trim.
/// Positive weights bound the complete curve displacement by the maximum
/// Euclidean control displacement, independently of sampling or tangent size.
fn conform_endpoint_jets(c: &mut Curve, tolerance: f64) {
    let original = c.control_points.clone();
    let scale = original
        .iter()
        .flatten()
        .map(|v| v.abs())
        .fold(1., f64::max);
    let limit = (512. * f64::EPSILON * scale).min(tolerance * 0.001);
    for endpoint in [original[0].clone(), original.last().unwrap().clone()] {
        for i in 1..original.len() - 1 {
            for axis in 0..2 {
                if (c.control_points[i][axis] - endpoint[axis]).abs().next_up() > limit * 0.5 {
                    continue;
                }
                let mut candidate = c.control_points[i].clone();
                candidate[axis] = endpoint[axis];
                let displacement = ((candidate[0] - original[i][0]).abs().next_up()
                    + (candidate[1] - original[i][1]).abs().next_up())
                .next_up();
                if displacement <= limit {
                    c.control_points[i] = candidate;
                }
            }
        }
    }
}

pub fn normalize(wires: &[Vec<Curve>], tolerance: f64) -> Result<Vec<Vec<Curve>>> {
    if wires.is_empty() {
        return Ok(Vec::new());
    }
    if !tolerance.is_finite() || tolerance <= 0. || wires.len() > 32 {
        return Err(refused("Invalid Bézier normalization options"));
    }
    let mut curves = Vec::new();
    let mut parents = Vec::new();
    let mut parent_ids = Vec::new();
    let mut next_parent_id = 0usize;
    let mut work = 0;
    for wire in wires {
        if wire.is_empty() {
            return Err(refused("Empty authored boundary"));
        }
        for (i, c) in wire.iter().enumerate() {
            let parent_id = next_parent_id;
            next_parent_id += 1;
            controls(c)?;
            if c.control_points.last() != wire[(i + 1) % wire.len()].control_points.first() {
                return Err(refused("Authored boundary is open"));
            }
            if c.control_points.iter().all(|p| p == &c.control_points[0]) {
                continue;
            }
            let mut cuts = c.domain().to_vec();
            // One injective coordinate suffices. Splitting every coordinate
            // extremum rounds otherwise exact tangent/implicit identities.
            // Stationary roots on the chosen coordinate still split zero jets.
            let injective_axis = (0..2)
                .filter(|&axis| {
                    (c.control_points.last().unwrap()[axis] - c.control_points[0][axis]).abs()
                        > 8. * tolerance
                        && nurbs_core::intersection::planar_scalar::monotone(c, axis)
                })
                .max_by(|&a, &b| {
                    let delta = |k: usize| {
                        (c.control_points.last().unwrap()[k] - c.control_points[0][k]).abs()
                    };
                    delta(a).total_cmp(&delta(b))
                });
            let axes = injective_axis.map_or_else(|| vec![0, 1], |k| vec![k]);
            for k in axes {
                cuts.extend(
                    nurbs_core::intersection::planar_scalar::stationary(c, k)?.ok_or_else(
                        || refused("Authored stationary parameter completeness is unresolved"),
                    )?,
                );
            }
            super::coincidence_ops::coalesce(c, &mut cuts, tolerance, &mut work)?;
            for interval in cuts.windows(2) {
                let mut piece = c.trim(interval[0], interval[1])?;
                // A shared cut is evaluated once by the same source definition,
                // so independent trim roundoff cannot split a cusp into two
                // almost-coincident vertices. This is only endpoint conformance.
                let scale = c
                    .control_points
                    .iter()
                    .flatten()
                    .map(|v| v.abs())
                    .fold(1., f64::max);
                let limit = (512. * f64::EPSILON * scale).min(tolerance * 0.001);
                for (index, t) in [
                    (0, interval[0]),
                    (piece.control_points.len() - 1, interval[1]),
                ] {
                    let point = if t == c.domain()[0] {
                        c.control_points[0].clone()
                    } else if t == c.domain()[1] {
                        c.control_points.last().unwrap().clone()
                    } else {
                        c.evaluate(t)?.point
                    };
                    let p = &piece.control_points[index];
                    let displacement = ((p[0] - point[0]).abs().next_up()
                        + (p[1] - point[1]).abs().next_up())
                    .next_up();
                    if displacement > limit {
                        return Err(refused(
                            "Stationary trim endpoint conformance exceeds roundoff budget",
                        ));
                    }
                    piece.control_points[index] = point;
                }

                conform_endpoint_jets(&mut piece, tolerance);
                if !injective(&piece, tolerance) {
                    return Err(refused("Authored cusp/injectivity remains unresolved"));
                }
                parents.push(c.clone());
                parent_ids.push(parent_id);
                curves.push(piece);
                if curves.len() > 1022 {
                    return Err(refused("Bézier normalization exceeds 1022 spans"));
                }
            }
        }
    }
    let mut cuts: Vec<_> = curves.iter().map(|c| c.domain().to_vec()).collect();
    let mut aliases: Vec<_> = (0..curves.len()).collect();
    let mut linear = vec![false; curves.len()];
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
            use nurbs_core::intersection::planar_scalar::Relation;
            if parent_ids[i] == parent_ids[j]
                && let Some(events) = nurbs_core::intersection::planar_scalar::self_intersections(
                    &parents[i],
                    tolerance,
                )?
            {
                let a = curves[i].domain();
                let b = curves[j].domain();
                for point in events {
                    for (t, u) in [(point.first, point.second), (point.second, point.first)] {
                        if a[0] <= t && t <= a[1] && b[0] <= u && u <= b[1] {
                            cuts[i].push(t);
                            cuts[j].push(u);
                        }
                    }
                }
                continue;
            }
            // Stationary splitting retains the authored parent definition.
            // Count contacts on that source before rounding a trim control net;
            // otherwise a multiple root can spuriously bifurcate after a cut.
            if (parents[i].domain() != curves[i].domain()
                || parents[j].domain() != curves[j].domain())
                && let Some(Relation::Points(events)) =
                    nurbs_core::intersection::planar_scalar::intersect(
                        &parents[i],
                        &parents[j],
                        tolerance,
                    )?
            {
                let a = curves[i].domain();
                let b = curves[j].domain();
                for point in events {
                    if a[0] <= point.first
                        && point.first <= a[1]
                        && b[0] <= point.second
                        && point.second <= b[1]
                    {
                        cuts[i].push(point.first);
                        cuts[j].push(point.second);
                    }
                }
                continue;
            }
            if let Some(relation) = nurbs_core::intersection::planar_scalar::intersect(
                &curves[i], &curves[j], tolerance,
            )? {
                let (first, second, straight) = match relation {
                    Relation::Points(points) => {
                        for point in points {
                            cuts[i].push(point.first);
                            cuts[j].push(point.second);
                        }
                        continue;
                    }
                    Relation::Coincident { linear, .. } => {
                        (curves[i].domain(), curves[j].domain(), linear)
                    }
                    Relation::Overlap {
                        points,
                        first,
                        second,
                        linear,
                        ..
                    } => {
                        for point in points {
                            cuts[i].push(point[0]);
                            cuts[j].push(point[1]);
                        }
                        (first, second, linear)
                    }
                };
                cuts[i].extend(first);
                cuts[j].extend(second);
                let old = aliases[j];
                let new = aliases[i];
                linear[new] = straight || linear[new] || linear[old];
                for alias in &mut aliases {
                    if *alias == old {
                        *alias = new;
                    }
                }
                continue;
            }
            let report = intersect_curve_curve_report(
                &super::boolean_ops::lift(&curves[i]),
                &super::boolean_ops::lift(&curves[j]),
                None,
            )?;
            if !report.report.unresolved.is_empty() {
                return Err(refused(&format!(
                    "Self-intersection normalization is unresolved for spans {i} and {j}"
                )));
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
    // Share every proposed cut across admitted carriers before comparing
    // fragments. This also handles a third curve crossing a shared subspan.
    super::coincidence_ops::synchronize(&curves, &aliases, &mut cuts, tolerance, &mut work)?;
    let mut fragments: Vec<(Curve, usize, bool)> = Vec::new();
    for (i, source) in curves.iter().enumerate() {
        super::coincidence_ops::coalesce(source, &mut cuts[i], tolerance, &mut work)?;
        for range in cuts[i].windows(2) {
            if range[1] <= range[0] {
                continue;
            }
            let piece = source.trim(range[0], range[1])?;
            let mut duplicate = None;
            for (j, (other, alias, _)) in fragments.iter().enumerate() {
                if *alias != aliases[i] {
                    continue;
                }
                let scale = piece
                    .control_points
                    .iter()
                    .chain(&other.control_points)
                    .flatten()
                    .map(|v| v.abs())
                    .fold(1., f64::max);
                let limit = (4096. * f64::EPSILON * scale).min(tolerance * 0.001);
                if linear[*alias] {
                    for reversed in [false, true] {
                        let a = [
                            piece.control_points.first().unwrap(),
                            piece.control_points.last().unwrap(),
                        ];
                        let b = if reversed {
                            [
                                other.control_points.last().unwrap(),
                                other.control_points.first().unwrap(),
                            ]
                        } else {
                            [
                                other.control_points.first().unwrap(),
                                other.control_points.last().unwrap(),
                            ]
                        };
                        if a.iter().zip(b).all(|(p, q)| {
                            ((p[0] - q[0]).abs().next_up() + (p[1] - q[1]).abs().next_up())
                                .next_up()
                                <= limit
                        }) {
                            duplicate = Some(j);
                            break;
                        }
                    }
                    if duplicate.is_some() {
                        break;
                    }
                }
                for reversed in [false, true] {
                    let b = if reversed {
                        other.reverse()?
                    } else {
                        other.clone()
                    };
                    if nurbs_core::intersection::planar_coincidence::agreement(&piece, &b)?
                        .is_some_and(|e| e <= limit)
                    {
                        duplicate = Some(j);
                        break;
                    }
                }
                if duplicate.is_some() {
                    break;
                }
            }
            if let Some(j) = duplicate {
                fragments[j].2 = !fragments[j].2;
            } else {
                fragments.push((piece, aliases[i], true));
            }
            if fragments.len() > 2048 {
                return Err(refused("Normalization fragment budget exceeded"));
            }
        }
    }
    let curves: Vec<_> = fragments
        .into_iter()
        .filter_map(|(c, _, odd)| odd.then_some(c))
        .collect();
    let all = curves.clone();
    let mut pieces = Vec::new();
    for (i, source) in curves.iter().enumerate() {
        let mut c = source.clone();
        let interval = source.domain();
        let e = c.evaluate(interval[0] * 0.5 + interval[1] * 0.5)?;
        let raw =
            e.d1.ok_or_else(|| refused("Normalization tangent is unresolved"))?;
        let span = interval[1] - interval[0];
        let d = [raw[0] * span, raw[1] * span];
        let length = d[0].hypot(d[1]);
        if !length.is_finite() || length <= tolerance {
            return Err(refused("Normalization cusp is unresolved"));
        }
        let n = [-d[1] / length, d[0] / length];
        let mut decision = None;
        // Two distances must agree. Each winding call admits its decision
        // only after complete control-hull separation from the probe.
        for distance in [32. * tolerance, 64. * tolerance] {
            // Ordered tangent-coordinate controls give a fast uniqueness
            // proof. Unordered controls need not mean an ambiguous curve.
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
            let endpoints = [
                [e.point[0] - n[0] * distance, e.point[1] - n[1] * distance],
                [e.point[0] + n[0] * distance, e.point[1] + n[1] * distance],
            ];
            if !monotone {
                // Count intersections with the actual represented finite
                // probe. This uses original curve coordinates and weights,
                // not rounded rotated controls. An uncertain census cannot
                // become an empty or unique intersection report.
                let probe = Curve::from_polyline(endpoints.iter().map(|p| p.to_vec()).collect())?;
                let unique = matches!(
                    nurbs_core::intersection::planar_scalar::intersect(source, &probe, tolerance)?,
                    Some(nurbs_core::intersection::planar_scalar::Relation::Points(points))
                        if points.len() == 1 && points[0].multiplicity == 1
                );
                if !unique {
                    return Err(refused(
                        "Normalization normal-probe uniqueness is unresolved",
                    ));
                }
            }
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
                                points
                                    .iter()
                                    .fold([f64::INFINITY, f64::NEG_INFINITY], |r, p| {
                                        let v = p[0] * axis[0] + p[1] * axis[1];
                                        [r[0].min(v), r[1].max(v)]
                                    })
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
                    let [left, right] = part.split(a * 0.5 + b * 0.5)?;
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
    super::boolean_ops::assemble(pieces, tolerance, true)
}
