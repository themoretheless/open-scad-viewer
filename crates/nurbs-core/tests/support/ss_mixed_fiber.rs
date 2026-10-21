use nurbs_core::{
    ss_intersection::{FiberAxis, FiberLimits, FiberSources, QualifiedFiber},
    surface::Surface,
};
pub fn sources() -> FiberSources {
    let a = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: [0.25, 0.375, 0.5]
            .into_iter()
            .enumerate()
            .map(|(i, x)| {
                (0..3)
                    .map(|j| {
                        vec![
                            x,
                            j as f64 / 2.,
                            (if j == 2 { 1. } else { 0. }) + (if i == 2 { 0.0625 } else { 0. }),
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    let b = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.25], vec![0., 1., 0.25]],
            vec![vec![1., 0., 0.25], vec![1., 1., 0.25]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    FiberSources::new(&a, &b).unwrap()
}
#[allow(dead_code)]
pub fn family() -> Vec<QualifiedFiber> {
    let src = sources();
    [
        (
            [[0., 0.375], [0.375, 0.625], [0.2, 0.55], [0.3, 0.7]],
            FiberAxis::FirstU,
        ),
        (
            [[0.625, 1.], [0.375, 0.625], [0.2, 0.55], [0.3, 0.7]],
            FiberAxis::FirstU,
        ),
        (
            [[0.125, 0.875], [0.375, 0.625], [0.3125, 0.4375], [0.3, 0.7]],
            FiberAxis::SecondU,
        ),
    ]
    .into_iter()
    .map(|(d, a)| {
        src.certify(d, a, FiberLimits::default())
            .unwrap()
            .into_fiber()
            .expect("uniform original fiber")
    })
    .collect()
}
