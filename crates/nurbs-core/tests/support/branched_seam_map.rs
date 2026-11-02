use super::Surface;
/// Four original branches, no global monotone Cartesian coordinate.
/// Both clocks are improper: q_A=(s+s²)/2, q_B=(t+t³)/2.
pub fn fixture(laps: usize) -> (Surface, Surface) {
    let make = |degree, xy: &[[f64; 2]], laps, z| {
        let mut rows = Vec::new();
        for branch in 0..4 * laps {
            for (i, &[mut x, mut y]) in xy.iter().enumerate() {
                if branch > 0 && i == 0 {
                    continue;
                }
                for _ in 0..branch % 4 {
                    (x, y) = (-y, x);
                }
                rows.push((0..3).map(|j| vec![x, y, z + 0.5 * j as f64]).collect());
            }
        }
        let mut knots = vec![0.; degree + 1];
        for branch in 1..4 * laps {
            knots.extend(vec![branch as f64; degree]);
        }
        knots.extend(vec![(4 * laps) as f64; degree + 1]);
        Surface {
            degree_u: degree,
            degree_v: 2,
            knots_u: knots,
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            weights: vec![vec![1.; 3]; rows.len()],
            control_points: rows,
            periodic_u: false,
            periodic_v: false,
        }
    };
    (
        make(
            4,
            &[
                [60., 60.],
                [45., 75.],
                [20., 95.],
                [-15., 105.],
                [-60., 60.],
            ],
            1,
            0.,
        ),
        make(
            6,
            &[
                [60., 60.],
                [50., 70.],
                [40., 78.],
                [27., 87.],
                [8., 96.],
                [-20., 100.],
                [-60., 60.],
            ],
            laps,
            1.,
        ),
    )
}

/// Independent dyadic cuts of the existing polynomial Bezier branches. All
/// fixture coefficients and products below are exactly representable binary64.
/// This is fixture construction, not a proof-side knot insertion algorithm.
pub fn independently_cut() -> (Surface, Surface) {
    fn split(s: Surface, at: f64) -> Surface {
        let degree = s.degree_u;
        let mut rows = Vec::new();
        let mut knots = vec![0.; degree + 1];
        for branch in 0..4 {
            let mut work = s.control_points[branch * degree..=branch * degree + degree].to_vec();
            if branch > 0 {
                rows.extend(work.into_iter().skip(1));
                if branch < 3 {
                    knots.extend(vec![branch as f64 + 1.; degree]);
                }
                continue;
            }
            let mut left = vec![work[0].clone()];
            let mut right = vec![work[degree].clone()];
            for level in 1..=degree {
                for i in 0..=degree - level {
                    for j in 0..3 {
                        for k in 0..3 {
                            work[i][j][k] = (1. - at) * work[i][j][k] + at * work[i + 1][j][k];
                        }
                    }
                }
                left.push(work[0].clone());
                right.push(work[degree - level].clone());
            }
            right.reverse();
            if branch > 0 {
                left.remove(0);
            }
            rows.extend(left);
            rows.extend(right.into_iter().skip(1));
            knots.extend(vec![branch as f64 + at; degree]);
            if branch < 3 {
                knots.extend(vec![branch as f64 + 1.; degree]);
            }
        }
        knots.extend(vec![4.; degree + 1]);
        Surface {
            weights: vec![vec![1.; 3]; rows.len()],
            control_points: rows,
            knots_u: knots,
            ..s
        }
    }
    let (a, b) = fixture(1);
    (split(a, 0.25), split(b, 0.5))
}
