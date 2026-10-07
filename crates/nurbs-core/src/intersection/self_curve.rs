//! Conservative self-intersection audit: global monotonicity and exact chord witnesses.
use crate::foundation::guards::Budget;
use crate::{Result, check, chord_intersection, curve::Curve, curve_jets};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Absent,
    Present,
    Unresolved,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub status: Status,
    pub axis: Option<usize>,
    pub cells: usize,
    pub pairs: usize,
    pub witness_spans: Option<[usize; 2]>,
    pub witness_point: Option<[f64; 2]>,
}
/// Distinct interior intersections count; the normal closure endpoint of a
/// closed curve does not. Global coordinate monotonicity proves injectivity of
/// any admitted rational curve; closed planar degree-one loops additionally use
/// continuous trim simplicity. Proper crossings of planar degree-one spans
/// prove presence. Remaining tangencies/overlaps/curved nonmonotone cases stay
/// unresolved, never silently classified as simple.
pub fn inspect(c: &Curve, tolerance: f64, max_cells: usize, max_pairs: usize) -> Result<Report> {
    c.validate()?;
    check(
        (2..=3).contains(&c.control_points[0].len())
            && tolerance.is_finite()
            && tolerance > 0.
            && (1..=100000).contains(&max_cells)
            && (1..=100000).contains(&max_pairs),
        "Self audit needs 2D/3D curve, positive tolerance and bounded work",
    )?;
    let spans: Vec<_> = (c.degree..c.control_points.len())
        .filter(|&i| c.knots[i] < c.knots[i + 1])
        .collect();
    let mut out = Report {
        status: Status::Unresolved,
        axis: None,
        cells: 0,
        pairs: 0,
        witness_spans: None,
        witness_point: None,
    };
    // Reject C0 discontinuities from an injectivity conclusion based only on
    // derivatives. Nonperiodic interior knot multiplicity <=degree ensures C0.
    let [a, b] = c.domain();
    let continuous = !c.periodic
        && c.knots
            .iter()
            .filter(|&&t| t > a && t < b)
            .all(|t| c.knots.iter().filter(|x| *x == t).count() <= c.degree);
    let monotone_budget = (max_cells / 4).max(1);
    if continuous && c.control_points.first() != c.control_points.last() {
        let mut axes: Vec<_> = (0..c.control_points[0].len()).collect();
        axes.sort_by(|&x, &y| {
            (c.control_points.last().unwrap()[y] - c.control_points[0][y])
                .abs()
                .total_cmp(&(c.control_points.last().unwrap()[x] - c.control_points[0][x]).abs())
        });
        for axis in axes {
            let mut stack: Vec<_> = spans
                .iter()
                .map(|&i| (i, [c.knots[i], c.knots[i + 1]]))
                .collect();
            let mut sign = 0i8;
            let mut complete = true;
            // Unified guard backing the monotone sweep cell budget (1065).
            let mut guard =
                Budget::with_iterations(monotone_budget)?.guard("self_curve_monotone");
            while let Some((span, domain)) = stack.pop() {
                if out.cells == monotone_budget {
                    complete = false;
                    break;
                }
                guard.tick()?;
                out.cells += 1;
                let j = curve_jets::enclose(c, span, domain)?;
                let d = j[1][axis];
                let direction = if d.lo > 0. {
                    1
                } else if d.hi < 0. {
                    -1
                } else {
                    0
                };
                if direction != 0 {
                    if sign != 0 && direction != sign {
                        complete = false;
                        break;
                    }
                    sign = direction;
                } else {
                    let mid = domain[0] * 0.5 + domain[1] * 0.5;
                    if mid == domain[0] || mid == domain[1] {
                        complete = false;
                        break;
                    }
                    stack.push((span, [domain[0], mid]));
                    stack.push((span, [mid, domain[1]]));
                }
            }
            if complete && sign != 0 {
                out.status = Status::Absent;
                out.axis = Some(axis);
                return Ok(out);
            }
        }
    }
    if continuous
        && spans.iter().all(|&i| {
            [c.knots[i], c.knots[i + 1]]
                .iter()
                .all(|t| c.knots.iter().filter(|x| *x == t).count() >= c.degree)
                && (0..c.control_points[0].len()).any(|axis| {
                    let points = &c.control_points[i - c.degree..=i];
                    (points.windows(2).all(|w| w[0][axis] <= w[1][axis])
                        || points.windows(2).all(|w| w[0][axis] >= w[1][axis]))
                        && points.first().unwrap()[axis] != points.last().unwrap()[axis]
                })
        })
    {
        let mut complete = true;
        'pairs: for (index, &i) in spans.iter().enumerate() {
            for &j in &spans[index + 1..] {
                if out.pairs == max_pairs {
                    complete = false;
                    break 'pairs;
                }
                out.pairs += 1;
                let left = &c.control_points[i - c.degree..=i];
                let right = &c.control_points[j - c.degree..=j];
                for (x, tx) in [
                    (&left[0], c.knots[i]),
                    (left.last().unwrap(), c.knots[i + 1]),
                ] {
                    for (y, ty) in [
                        (&right[0], c.knots[j]),
                        (right.last().unwrap(), c.knots[j + 1]),
                    ] {
                        if x == y && tx != ty && !((tx == a && ty == b) || (tx == b && ty == a)) {
                            out.status = Status::Present;
                            out.witness_spans = Some([i, j]);
                            out.witness_point = Some([x[0], x[1]]);
                            return Ok(out);
                        }
                    }
                }
                let separated = (0..c.control_points[0].len()).any(|axis| {
                    let lmin = left.iter().map(|p| p[axis]).fold(f64::INFINITY, f64::min);
                    let lmax = left
                        .iter()
                        .map(|p| p[axis])
                        .fold(f64::NEG_INFINITY, f64::max);
                    let rmin = right.iter().map(|p| p[axis]).fold(f64::INFINITY, f64::min);
                    let rmax = right
                        .iter()
                        .map(|p| p[axis])
                        .fold(f64::NEG_INFINITY, f64::max);
                    lmax < rmin
                        || rmax < lmin
                        || (lmax == rmin && lmin < lmax && rmin < rmax)
                        || (rmax == lmin && rmin < rmax && lmin < lmax)
                });
                if separated {
                    continue;
                }
                let mut pending =
                    vec![([c.knots[i], c.knots[i + 1]], [c.knots[j], c.knots[j + 1]])];
                // Unified guard backing the pair-subdivision cell budget (1065).
                let mut guard =
                    Budget::with_iterations(max_cells)?.guard("self_curve_pair_cells");
                while let Some((l, r)) = pending.pop() {
                    if out.cells == max_cells {
                        complete = false;
                        break 'pairs;
                    }
                    guard.tick()?;
                    out.cells += 1;
                    let lb = crate::curve_distance::enclosure(
                        c,
                        i,
                        crate::distance_bounds::Interval::new(l[0], l[1])?,
                    )?;
                    let rb = crate::curve_distance::enclosure(
                        c,
                        j,
                        crate::distance_bounds::Interval::new(r[0], r[1])?,
                    )?;
                    if crate::distance_bounds::box_distance(&lb, &rb)?.0 > 0. {
                        continue;
                    }
                    let axis = usize::from(r[1] - r[0] > l[1] - l[0]);
                    let d = if axis == 0 { l } else { r };
                    let mid = d[0] * 0.5 + d[1] * 0.5;
                    if mid == d[0] || mid == d[1] {
                        complete = false;
                        break 'pairs;
                    }
                    if axis == 0 {
                        pending.push(([l[0], mid], r));
                        pending.push(([mid, l[1]], r));
                    } else {
                        pending.push((l, [r[0], mid]));
                        pending.push((l, [mid, r[1]]));
                    }
                }
            }
        }
        if complete {
            out.status = Status::Absent;
            return Ok(out);
        }
    }
    let planar = c.control_points[0].len() == 2
        || c.control_points
            .iter()
            .all(|p| p[2] == c.control_points[0][2]);
    if c.degree != 1 || !planar {
        return Ok(out);
    }
    let chords: Vec<_> = spans
        .iter()
        .map(|&i| {
            [
                [c.control_points[i - 1][0], c.control_points[i - 1][1]],
                [c.control_points[i][0], c.control_points[i][1]],
            ]
        })
        .collect();
    for i in 0..chords.len() {
        for j in i + 1..chords.len() {
            if out.pairs == max_pairs {
                return Ok(out);
            }
            out.pairs += 1;
            if let Ok(w) = chord_intersection::proper(chords[i], chords[j], tolerance) {
                out.status = Status::Present;
                out.witness_spans = Some([spans[i], spans[j]]);
                out.witness_point = Some(w.point);
                return Ok(out);
            }
        }
    }
    if chords.len() >= 3
        && chords.first().unwrap()[0] == chords.last().unwrap()[1]
        && out.cells < max_cells
        && out.pairs < max_pairs
    {
        let wire = chords
            .iter()
            .map(|p| Curve::from_polyline(p.iter().map(|p| p.to_vec()).collect()))
            .collect::<Result<Vec<_>>>()?;
        let proof = crate::trim_simplicity::inspect(
            &wire,
            tolerance,
            max_pairs - out.pairs,
            max_cells - out.cells,
        )?;
        out.cells += proof.cells;
        out.pairs += proof.pairs.len();
        if proof.proven_simple {
            out.status = Status::Absent;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod guard_tests {
    use super::*;

    fn polyline(points: &[[f64; 2]]) -> Curve {
        Curve::from_polyline(points.iter().map(|p| p.to_vec()).collect()).unwrap()
    }

    #[test]
    fn monotone_sweep_guard_preserves_the_absent_verdict() {
        // A globally x-monotone polyline proves simple; guards must not perturb it.
        let c = polyline(&[[0., 0.], [1., 1.], [2., 0.5], [3., 2.]]);
        let r = inspect(&c, 1e-9, 1000, 1000).unwrap();
        assert_eq!(r.status, Status::Absent);
    }

    #[test]
    fn crossing_polyline_still_witnesses_present() {
        let c = polyline(&[[0., 0.], [2., 2.], [2., 0.], [0., 2.]]);
        let r = inspect(&c, 1e-9, 1000, 1000).unwrap();
        assert_eq!(r.status, Status::Present);
    }

    #[test]
    fn cell_guard_matches_the_configured_budget() {
        let mut guard = Budget::with_iterations(5).unwrap().guard("self_curve_pair_cells");
        for _ in 0..5 {
            guard.tick().unwrap();
        }
        let err = guard.tick().unwrap_err();
        assert_eq!(err.code, crate::RESOURCE_LIMIT, "{err:?}");
        assert!(err.contains("self_curve_pair_cells"));
    }
}
