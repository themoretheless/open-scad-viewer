//! Exact index-only U/V exchange for an original clamped U boundary. This is
//! a proof view, never an edited source surface or a rounded clock reflection.
use super::*;
pub(super) fn inspect(
    c: &Curve,
    p: &Curve,
    s: &Surface,
    reversed: bool,
    tolerance: f64,
    max_cells: usize,
) -> Result<Option<Report>> {
    if max_cells == 0
        || p.degree != 1
        || p.control_points.len() != 2
        || p.weights[0] != p.weights[1]
        || p.control_points[0].len() != 2
        || s.periodic_u
        || s.periodic_v
        || s.control_points
            .len()
            .saturating_mul(s.control_points[0].len())
            > 4096
        || p.control_points[0][0] != p.control_points[1][0]
    {
        return Ok(None);
    }
    let u = p.control_points[0][0];
    let n = s.control_points.len();
    if u != s.knots_u[s.degree_u] && u != s.knots_u[n] {
        return Ok(None);
    }
    let transposed = Surface {
        degree_u: s.degree_v,
        degree_v: s.degree_u,
        knots_u: s.knots_v.clone(),
        knots_v: s.knots_u.clone(),
        control_points: (0..s.control_points[0].len())
            .map(|j| s.control_points.iter().map(|r| r[j].clone()).collect())
            .collect(),
        weights: (0..s.weights[0].len())
            .map(|j| s.weights.iter().map(|r| r[j]).collect())
            .collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let mut pc = p.clone();
    for point in &mut pc.control_points {
        point.swap(0, 1);
    }
    boundary_coefficients::inspect(c, &pc, &transposed, reversed, tolerance, max_cells)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_vertical_root_keeps_independent_clocks_and_gauges() {
        let s = Surface {
            degree_u: 10,
            degree_v: 9,
            knots_u: [vec![-2.; 11], vec![3.; 11]].concat(),
            knots_v: [vec![7.; 10], vec![11.; 10]].concat(),
            control_points: (0..11)
                .map(|i| {
                    (0..10)
                        .map(|j| vec![i as f64 / 10., j as f64 / 9., 0.])
                        .collect()
                })
                .collect(),
            weights: vec![vec![2.; 10]; 11],
            periodic_u: false,
            periodic_v: false,
        };
        let mut c = Curve {
            degree: 9,
            knots: s.knots_v.clone(),
            control_points: s.control_points[0].clone(),
            weights: vec![1.; 10],
            periodic: false,
        };
        let p = Curve::from_polyline(vec![vec![-2., 7.], vec![-2., 11.]]).unwrap();
        assert_eq!(
            inspect(&c, &p, &s, false, 1e-7, 1).unwrap().unwrap().status,
            Status::WithinTolerance
        );
        assert_eq!(
            verify(&c, &p, &s, false, 1e-7, 1).unwrap().status,
            Status::WithinTolerance
        );
        c.control_points[4][2] = 0.001;
        assert_ne!(
            inspect(&c, &p, &s, false, 1e-7, 100)
                .unwrap()
                .unwrap()
                .status,
            Status::WithinTolerance
        );
    }
}
