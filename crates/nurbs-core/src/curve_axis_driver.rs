//! Strict interval monotonicity of one original clamped pcurve coordinate.
//! Cell derivatives are normalized to each cell; only their signs are combined.
use crate::{check, curve::Curve, Result};
pub struct Cell {
    pub parameter: [f64; 2],
    pub normalized_derivative: Option<[f64; 2]>,
    pub admitted: bool,
}
pub struct Report {
    pub cells: Vec<Cell>,
    pub visited: usize,
    pub axis_extent: [f64; 2],
    pub increasing: bool,
    pub monotonic_proven: bool,
}
pub fn certify(curve: &Curve, axis: usize, max_cells: usize) -> Result<Report> {
    curve.validate()?;
    check(
        axis < curve.control_points[0].len()
            && !curve.periodic
            && (1..=100000).contains(&max_cells),
        "Choose an original nonperiodic coordinate and bounded driver work",
    )?;
    let d = curve.domain();
    check(
        curve.knots[..=curve.degree].iter().all(|&x| x == d[0])
            && curve.knots[curve.control_points.len()..]
                .iter()
                .all(|&x| x == d[1]),
        "Original driver endpoints must be clamped",
    )?;
    for &k in &curve.knots {
        if k > d[0] && k < d[1] {
            check(
                curve.knots.iter().filter(|&&x| x == k).count() <= curve.degree,
                "Driver must be continuous at every original knot",
            )?;
        }
    }
    let start = curve.control_points[0][axis];
    let end = curve.control_points.last().unwrap()[axis];
    let increasing = end > start;
    let mut out = Report {
        cells: vec![],
        visited: 0,
        axis_extent: [start.min(end), start.max(end)],
        increasing,
        monotonic_proven: false,
    };
    let mut queue = vec![];
    for span in (curve.degree..curve.control_points.len()).rev() {
        let interval = [curve.knots[span], curve.knots[span + 1]];
        if interval[0] < interval[1] {
            queue.push((span, interval, 0usize));
        }
    }
    while let Some((span, parameter, depth)) = queue.pop() {
        if out.visited == max_cells {
            out.cells.push(Cell {
                parameter,
                normalized_derivative: None,
                admitted: false,
            });
            continue;
        }
        out.visited += 1;
        let derivative = crate::curve_jets::enclose(curve, span, parameter)?[1][axis];
        let admitted = if increasing {
            derivative.lo > 0.
        } else {
            end < start && derivative.hi < 0.
        };
        let mid = parameter[0] * 0.5 + parameter[1] * 0.5;
        if !admitted
            && out.visited < max_cells
            && depth < 32
            && mid > parameter[0]
            && mid < parameter[1]
        {
            queue.push((span, [mid, parameter[1]], depth + 1));
            queue.push((span, [parameter[0], mid], depth + 1));
        } else {
            out.cells.push(Cell {
                parameter,
                normalized_derivative: Some([derivative.lo, derivative.hi]),
                admitted,
            });
        }
    }
    out.cells
        .sort_by(|a, b| a.parameter[0].total_cmp(&b.parameter[0]));
    out.monotonic_proven = out.cells.iter().all(|c| c.admitted);
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nonunit_reversed_and_interior_turning_coordinates() {
        let mut c = Curve::from_polyline(vec![vec![1., 0.], vec![0., 1.]]).unwrap();
        c.knots = vec![2., 2., 8., 8.];
        let r = certify(&c, 0, 8).unwrap();
        assert!(r.monotonic_proven && !r.increasing);
        assert_eq!(r.axis_extent, [0., 1.]);
        c.degree = 3;
        c.knots = vec![2., 2., 2., 2., 8., 8., 8., 8.];
        c.control_points = vec![vec![0., 0.], vec![2., 0.], vec![-1., 0.], vec![1., 0.]];
        c.weights = vec![1.; 4];
        let r = certify(&c, 0, 31).unwrap();
        assert!(!r.monotonic_proven);
        assert_eq!(r.cells.first().unwrap().parameter[0], 2.);
        assert_eq!(r.cells.last().unwrap().parameter[1], 8.);
        for p in r.cells.windows(2) {
            assert_eq!(p[0].parameter[1], p[1].parameter[0]);
        }
        assert!(r.visited <= 31);
    }
}
