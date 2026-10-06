//! Original-definition planar crossing isolation. No tolerance root admission,
//! endpoint welding, tangency classification or partition parameter rounding.
use crate::{check, curve::Curve, curve_jets, distance_bounds::Interval as I, Result};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Excluded,
    Unique,
    Unresolved,
}
pub struct Cell {
    pub parameters: [[f64; 2]; 2],
    pub root: Option<[[f64; 2]; 2]>,
    pub state: State,
}
pub struct Report {
    pub cells: Vec<Cell>,
    pub visited: usize,
    pub complete: bool,
    /// Requested widths use each original curve parameter, not normalized cells.
    pub target_width: Option<[f64; 2]>,
    pub precision_proven: bool,
}
fn point(c: &Curve, span: usize, t: f64) -> Result<Vec<I>> {
    let net = crate::curve_distance::restricted_controls(c, span, I::point(t))?;
    (0..2)
        .map(|k| {
            net[0][k]
                .div(net[0][2])?
                .add(I::point(c.control_points[span - c.degree][k]))
        })
        .collect()
}
fn classify(
    a: &Curve,
    b: &Curve,
    sa: usize,
    sb: usize,
    d: [[f64; 2]; 2],
) -> Result<(State, Option<[[f64; 2]; 2]>)> {
    let ja = curve_jets::enclose(a, sa, d[0])?;
    let jb = curve_jets::enclose(b, sb, d[1])?;
    if (0..2).any(|k| ja[0][k].hi < jb[0][k].lo || jb[0][k].hi < ja[0][k].lo) {
        return Ok((State::Excluded, None));
    }
    let j = [
        [ja[1][0], I::new(-jb[1][0].hi, -jb[1][0].lo)?],
        [ja[1][1], I::new(-jb[1][1].hi, -jb[1][1].lo)?],
    ];
    let m = j.map(|row| row.map(|x| x.lo * 0.5 + x.hi * 0.5));
    let det = m[0][0] * m[1][1] - m[0][1] * m[1][0];
    if !det.is_finite() || det == 0. {
        return Ok((State::Unresolved, None));
    }
    let y = [
        [m[1][1] / det, -m[0][1] / det],
        [-m[1][0] / det, m[0][0] / det],
    ];
    if y.iter().flatten().any(|x| !x.is_finite()) {
        return Ok((State::Unresolved, None));
    }
    let t = d.map(|x| x[0] * 0.5 + x[1] * 0.5);
    let pa = point(a, sa, t[0])?;
    let pb = point(b, sb, t[1])?;
    let f = [pa[0].sub(pb[0])?, pa[1].sub(pb[1])?];
    let widths = [
        I::point(d[0][1]).sub(I::point(d[0][0]))?,
        I::point(d[1][1]).sub(I::point(d[1][0]))?,
    ];
    let x = [
        I::point(t[0]).sub(I::point(d[0][0]))?.div(widths[0])?,
        I::point(t[1]).sub(I::point(d[1][0]))?.div(widths[1])?,
    ];
    let mut k = [I::point(0.); 2];
    for i in 0..2 {
        k[i] = x[i];
        let mut norm = I::point(0.);
        for r in 0..2 {
            k[i] = k[i].sub(I::point(y[i][r]).mul(f[r])?)?;
        }
        for col in 0..2 {
            let mut entry = I::point(if i == col { 1. } else { 0. });
            for r in 0..2 {
                entry = entry.sub(I::point(y[i][r]).mul(j[r][col])?)?;
            }
            norm = norm.add(I::point(entry.lo.abs().max(entry.hi.abs())))?;
            k[i] = k[i].add(entry.mul(I::new(0., 1.)?.sub(x[col])?)?)?;
        }
        if norm.hi >= 1. || k[i].lo <= 0. || k[i].hi >= 1. {
            return Ok((State::Unresolved, None));
        }
    }
    let mut root = [[0.; 2]; 2];
    for i in 0..2 {
        let r = I::point(d[i][0]).add(widths[i].mul(k[i])?)?;
        root[i] = [r.lo.max(d[i][0]), r.hi.min(d[i][1])];
    }
    Ok((State::Unique, Some(root)))
}
/// Freshly certify one original span box. Supplied state/enclosures are ignored.
pub fn certify_box(a: &Curve, b: &Curve, parameters: [[f64; 2]; 2]) -> Result<Cell> {
    let mut spans = [0; 2];
    for (i, c) in [a, b].iter().enumerate() {
        c.validate()?;
        check(
            c.control_points[0].len() == 2 && !c.periodic,
            "Crossing certification needs original nonperiodic UV curves",
        )?;
        check(
            parameters[i].iter().all(|x| x.is_finite()) && parameters[i][0] < parameters[i][1],
            "Crossing box must have positive finite widths",
        )?;
        spans[i] = (c.degree..c.control_points.len())
            .find(|&j| {
                c.knots[j] < c.knots[j + 1]
                    && parameters[i][0] >= c.knots[j]
                    && parameters[i][1] <= c.knots[j + 1]
            })
            .ok_or_else(|| {
                crate::Error::new(
                    "NURBS_CROSSING_BOX",
                    "Crossing box must lie in one original knot span",
                )
            })?;
    }
    let (state, root) = classify(a, b, spans[0], spans[1], parameters)?;
    Ok(Cell {
        parameters,
        root,
        state,
    })
}
/// Search the complete original span product. Every unvisited cell is retained.
/// Only strict Krawczyk inclusion plus contraction admits a unique crossing.
pub fn isolate(a: &Curve, b: &Curve, max_cells: usize) -> Result<Report> {
    isolate_impl(a, b, max_cells, None)
}
/// Tighten isolated roots under the same global work budget. Stopping refinement
/// preserves its last valid root enclosure and marks unmet precision explicitly.
pub fn isolate_refined(
    a: &Curve,
    b: &Curve,
    max_cells: usize,
    target_width: [f64; 2],
) -> Result<Report> {
    check(
        target_width.iter().all(|x| x.is_finite() && *x > 0.),
        "Requested root widths must be finite and positive",
    )?;
    isolate_impl(a, b, max_cells, Some(target_width))
}
fn isolate_impl(
    a: &Curve,
    b: &Curve,
    max_cells: usize,
    target_width: Option<[f64; 2]>,
) -> Result<Report> {
    for c in [a, b] {
        c.validate()?;
        check(
            c.control_points[0].len() == 2 && !c.periodic,
            "Crossing isolation needs original nonperiodic UV curves",
        )?;
    }
    check(
        (1..=100000).contains(&max_cells),
        "Crossing work must be bounded",
    )?;
    let spans = |c: &Curve| {
        (c.degree..c.control_points.len())
            .filter(|&s| c.knots[s] < c.knots[s + 1])
            .collect::<Vec<_>>()
    };
    let aa = spans(a);
    let bb = spans(b);
    check(
        aa.len().checked_mul(bb.len()).is_some_and(|n| n <= 100000),
        "Crossing span product exceeds resource bound",
    )?;
    let mut queue = std::collections::VecDeque::new();
    for &sa in &aa {
        for &sb in &bb {
            queue.push_back((
                sa,
                sb,
                [
                    [a.knots[sa], a.knots[sa + 1]],
                    [b.knots[sb], b.knots[sb + 1]],
                ],
                0usize,
            ));
        }
    }
    let mut out = Report {
        cells: vec![],
        visited: 0,
        complete: true,
        target_width,
        precision_proven: true,
    };
    while let Some((sa, sb, d, depth)) = queue.pop_front() {
        let (state, mut root) = if out.visited == max_cells {
            (State::Unresolved, None)
        } else {
            out.visited += 1;
            classify(a, b, sa, sb, d)?
        };
        if state == State::Unresolved && out.visited < max_cells && depth < 32 {
            let axis = depth % 2;
            let mid = d[axis][0] * 0.5 + d[axis][1] * 0.5;
            if mid > d[axis][0] && mid < d[axis][1] {
                let mut left = d;
                left[axis][1] = mid;
                let mut right = d;
                right[axis][0] = mid;
                queue.push_back((sa, sb, left, depth + 1));
                queue.push_back((sa, sb, right, depth + 1));
                continue;
            }
        }
        if let (State::Unique, Some(target), Some(mut current)) = (state, target_width, root) {
            let mut owner = d;
            while (0..2).any(|i| (current[i][1] - current[i][0]).next_up() > target[i])
                && out.visited < max_cells
            {
                let mut candidate = current;
                for i in 0..2 {
                    let pad = (current[i][1] - current[i][0]) * 0.5;
                    candidate[i] = [
                        (current[i][0] - pad).next_down().max(owner[i][0]),
                        (current[i][1] + pad).next_up().min(owner[i][1]),
                    ];
                }
                if (0..2).any(|i| candidate[i][0] >= candidate[i][1]) || candidate == owner {
                    break;
                }
                out.visited += 1;
                let (next, enclosure) = classify(a, b, sa, sb, candidate)?;
                crate::numeric(
                    next != State::Excluded,
                    "Refinement excluded a previously certified crossing",
                )?;
                if let (State::Unique, Some(narrower)) = (next, enclosure) {
                    let mut intersection = narrower;
                    for i in 0..2 {
                        intersection[i] = [
                            current[i][0].max(narrower[i][0]),
                            current[i][1].min(narrower[i][1]),
                        ];
                        crate::numeric(
                            intersection[i][0] <= intersection[i][1],
                            "Certified crossing enclosures became disjoint",
                        )?;
                    }
                    if intersection == current {
                        break;
                    }
                    current = intersection;
                    owner = candidate;
                } else {
                    break;
                }
            }
            root = Some(current);
            out.precision_proven &=
                (0..2).all(|i| (current[i][1] - current[i][0]).next_up() <= target[i]);
        }
        out.complete &= state != State::Unresolved;
        out.cells.push(Cell {
            parameters: d,
            root,
            state,
        });
    }
    out.precision_proven &= out.complete;
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crossing_nonunit_domains_and_excluded_pair() {
        let mut a = Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.]]).unwrap();
        let mut b = Curve::from_polyline(vec![vec![0., 1.], vec![1., 0.]]).unwrap();
        a.knots = vec![2., 2., 8., 8.];
        b.knots = vec![10., 10., 14., 14.];
        let r = isolate(&a, &b, 64).unwrap();
        assert!(r.complete);
        let roots = r
            .cells
            .iter()
            .filter(|c| c.state == State::Unique)
            .collect::<Vec<_>>();
        assert_eq!(roots.len(), 1);
        let x = roots[0].root.unwrap();
        assert!(x[0][0] <= 5. && x[0][1] >= 5. && x[1][0] <= 12. && x[1][1] >= 12.);
        for p in &mut b.control_points {
            p[1] += 3.;
        }
        let r = isolate(&a, &b, 64).unwrap();
        assert!(r.complete && r.cells.iter().all(|c| c.state == State::Excluded));
    }
    #[test]
    fn coincidence_and_budget_stop_never_admit_crossings() {
        let a = Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.]]).unwrap();
        let r = isolate(&a, &a, 3).unwrap();
        assert!(!r.complete);
        assert_eq!(r.visited, 3);
        assert!(r.cells.iter().all(|c| c.state != State::Unique));
    }
    #[test]
    fn rational_arc_crossing_preserves_original_parameters() {
        let a = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![1., 0.], vec![1., 1.], vec![0., 1.]],
            weights: vec![1., 0.5f64.sqrt(), 1.],
            periodic: false,
        };
        let b = Curve::from_polyline(vec![vec![0., 0.3], vec![1., 0.3]]).unwrap();
        let r = isolate(&a, &b, 4096).unwrap();
        assert!(r.complete, "{} visited", r.visited);
        let roots = r
            .cells
            .iter()
            .filter(|c| c.state == State::Unique)
            .collect::<Vec<_>>();
        assert_eq!(roots.len(), 1);
        let x = roots[0].root.unwrap();
        let expected = (1_f64 - 0.3 * 0.3).sqrt();
        assert!(x[1][0] <= expected && expected <= x[1][1]);
        let mut near = b;
        for p in &mut near.control_points {
            p[1] = 1.000000000001;
        }
        let r = isolate(&a, &near, 4096).unwrap();
        assert!(r.cells.iter().all(|c| c.state != State::Unique));
    }
    #[test]
    fn refinement_preserves_certified_root_when_requested_precision_cannot_be_reached() {
        let a = Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.]]).unwrap();
        let b = Curve::from_polyline(vec![vec![0., 1.], vec![1., 0.]]).unwrap();
        let r = isolate_refined(&a, &b, 1, [1e-30, 1e-30]).unwrap();
        assert!(r.complete && !r.precision_proven);
        assert_eq!(r.visited, 1);
        assert_eq!(r.cells[0].state, State::Unique);
        assert!(r.cells[0]
            .root
            .unwrap()
            .iter()
            .all(|x| x[0] <= 0.5 && x[1] >= 0.5));
    }
    #[test]
    fn rational_crossing_refines_to_requested_original_parameter_widths() {
        let a = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![1., 0.], vec![1., 1.], vec![0., 1.]],
            weights: vec![1., 0.5f64.sqrt(), 1.],
            periodic: false,
        };
        let b = Curve::from_polyline(vec![vec![0., 0.3], vec![1., 0.3]]).unwrap();
        let r = isolate_refined(&a, &b, 4096, [1e-8, 1e-8]).unwrap();
        assert!(r.complete && r.precision_proven);
        let roots = r.cells.iter().filter_map(|c| c.root).collect::<Vec<_>>();
        assert_eq!(roots.len(), 1);
        assert!(roots[0].iter().all(|x| x[1] - x[0] <= 1e-8));
        let expected = (1_f64 - 0.3 * 0.3).sqrt();
        assert!(roots[0][1][0] <= expected && expected <= roots[0][1][1]);
    }
    #[test]
    fn supplied_boxes_are_rechecked_against_original_spans() {
        let a = Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.]]).unwrap();
        let b = Curve::from_polyline(vec![vec![0., 1.], vec![1., 0.]]).unwrap();
        assert_eq!(
            certify_box(&a, &b, [[0., 1.]; 2]).unwrap().state,
            State::Unique
        );
        assert_eq!(
            certify_box(&a, &b, [[0., 0.25]; 2]).unwrap().state,
            State::Excluded
        );
        assert!(certify_box(&a, &b, [[-0.1, 1.], [0., 1.]]).is_err());
        assert!(certify_box(&a, &b, [[0.5, 0.5], [0., 1.]]).is_err());
    }
}
