use nurbs_core::{
    curve::Curve,
    surface::{self, Surface},
};
pub fn source(laps: usize) -> Surface {
    let p = [
        [1., 0.],
        [1., 1.],
        [0., 1.],
        [-1., 1.],
        [-1., 0.],
        [-1., -1.],
        [0., -1.],
        [1., -1.],
    ];
    let n = 8 * laps + 2;
    let mut knots = vec![-1., 0., 0.];
    for k in 1..=4 * laps + 1 {
        knots.extend([k as f64; 2]);
    }
    let c = Curve {
        degree: 2,
        knots,
        control_points: (0..n).map(|i| vec![p[i % 8][0], p[i % 8][1], 0.]).collect(),
        weights: (0..n)
            .map(|i| {
                if i % 2 == 0 {
                    1.
                } else {
                    std::f64::consts::FRAC_1_SQRT_2
                }
            })
            .collect(),
        periodic: true,
    };
    surface::extrude(&c, [0., 0., 2.]).unwrap()
}
