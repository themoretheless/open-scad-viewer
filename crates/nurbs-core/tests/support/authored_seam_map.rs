//! Authored exact quartic/octic seam, outside the previous inverse-chart family.
use super::{Curve, Surface};
pub fn fixture() -> (Surface, Surface, Curve) {
    let surface = |p, xy: &[[f64; 2]], domain: [f64; 2], z| Surface {
        degree_u: p,
        degree_v: 2,
        knots_u: [vec![domain[0]; p + 1], vec![domain[1]; p + 1]].concat(),
        knots_v: vec![-1., -1., -1., 2., 2., 2.],
        control_points: xy
            .iter()
            .map(|p| {
                (0..3)
                    .map(|j| vec![p[0], p[1], z + 0.5 * j as f64])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; xy.len()],
        periodic_u: false,
        periodic_v: false,
    };
    // A_xy(s)=840*(s+s^2+s^3,s^2+s^3+s^4), regular at both ends.
    // No affine Cartesian coordinate or constant projective inverse spans it.
    let a = surface(
        4,
        &[
            [0., 0.],
            [210., 0.],
            [560., 140.],
            [1260., 630.],
            [2520., 2520.],
        ],
        [10., 14.],
        0.,
    );
    // Original degree-eight coefficients of A((t+t^2)/2), all exact dyadics.
    let b = surface(
        8,
        &[
            [0., 0.],
            [52.5, 0.],
            [127.5, 7.5],
            [234.375, 31.875],
            [390., 90.75],
            [624.375, 219.375],
            [990., 495.],
            [1575., 1102.5],
            [2520., 2520.],
        ],
        [-2., 6.],
        1.,
    );
    let map = Curve {
        degree: 2,
        knots: vec![3., 3., 3., 5., 5., 7., 7., 7.],
        control_points: [0., 0.125, 0.375, 0.625, 1.]
            .iter()
            .map(|&v| vec![v, 0.])
            .collect(),
        weights: vec![1.; 5],
        periodic: false,
    };
    (a, b, map)
}
