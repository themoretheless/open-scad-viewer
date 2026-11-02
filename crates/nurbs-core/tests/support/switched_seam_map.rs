use super::Surface;
pub fn fixture() -> (Surface, Surface) {
    let make = |degree, knots: Vec<f64>, xy: &[[f64; 2]], weights: Vec<f64>, z: f64| Surface {
        degree_u: degree,
        degree_v: 2,
        knots_u: knots,
        knots_v: vec![-1., -1., -1., 2., 2., 2.],
        control_points: xy
            .iter()
            .map(|p| {
                (0..3)
                    .map(|j| vec![p[0], p[1], z + 0.5 * j as f64])
                    .collect()
            })
            .collect(),
        weights: weights.iter().map(|w| vec![*w; 3]).collect(),
        periodic_u: false,
        periodic_v: false,
    };
    // Original parameter s uses q=s before 1/2 and q=1/(3-2s) after.
    // The actual surface is geometrically G2; source parameter speeds differ.
    let a = make(
        2,
        vec![10., 10., 10., 12., 12., 14., 14., 14.],
        &[[0., 0.], [0.75, 0.], [1.5, 0.75], [2.25, 1.5], [3., 3.]],
        vec![4., 4., 4., 2., 1.],
        0.,
    );
    // One target span, q=(t+t^2)/2. Its inverse-map cut is irrational.
    let b = make(
        4,
        [vec![-2.; 5], vec![6.; 5]].concat(),
        &[[0., 0.], [0.375, 0.], [1., 0.125], [1.875, 0.75], [3., 3.]],
        vec![1.; 5],
        1.,
    );
    (a, b)
}

pub fn interior_pole_fixture() -> (Surface, Surface) {
    let (mut a, b) = fixture();
    for (i, row) in a.weights.iter_mut().enumerate() {
        *row = vec![
            if i < 3 {
                9.
            } else if i == 3 {
                3.
            } else {
                1.
            };
            3
        ];
    }
    (a, b)
}
