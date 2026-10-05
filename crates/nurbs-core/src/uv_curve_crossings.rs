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
/// Search the complete original span product. Every unvisited cell is retained.
/// Only strict Krawczyk inclusion plus contraction admits a unique crossing.
pub fn isolate(a: &Curve, b: &Curve, max_cells: usize) -> Result<Report> {
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
    };
    while let Some((sa, sb, d, depth)) = queue.pop_front() {
        let (state, root) = if out.visited == max_cells {
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
        out.complete &= state != State::Unresolved;
        out.cells.push(Cell {
            parameters: d,
            root,
            state,
        });
    }
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
}
