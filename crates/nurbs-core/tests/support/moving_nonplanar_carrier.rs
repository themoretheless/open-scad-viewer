use super::{RationalPiece, Surface};
pub fn fixture() -> (Surface, Vec<RationalPiece>) {
    let carrier = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                [-2., 2.]
                    .into_iter()
                    .map(|z| {
                        let h = if i == 1 { 1024. } else { 0. };
                        vec![1. + 2. * i as f64 - 8. * h, z, h]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    let xy = [
        [1., 0.],
        [1., 1.],
        [0., 1.],
        [-1., 1.],
        [-1., 0.],
        [-1., -1.],
        [0., -1.],
        [1., -1.],
        [1., 0.],
    ];
    let epsilon = 1. / 65536.;
    let motion = (0..4)
        .map(|i| RationalPiece {
            weights: vec![1., 0.5, 1.],
            controls: xy[2 * i..=2 * i + 2]
                .iter()
                .map(|&[x, y]| {
                    [
                        [x, y, 0.],
                        [0., 0., 1.],
                        [8. * x - epsilon * y, 8. * y + epsilon * x, 0.],
                        [0.; 3],
                    ]
                })
                .collect(),
        })
        .collect();
    (carrier, motion)
}
