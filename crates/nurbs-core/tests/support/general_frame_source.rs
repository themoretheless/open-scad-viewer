//! Original authored tilting frame and geometric-clock spatial source.
use super::nurbs;
use nurbs::{curve::Curve, surface::Surface};
#[path = "arc_bishop_source.rs"]
mod arc;
pub fn path() -> Curve {
    arc::path(true)
}
pub fn laws() -> (Curve, Curve, Curve) {
    let p = arc::path(false);
    // These are original authored frame coefficients, not a claim that the
    // field equals the independently arc-clocked path tangent. Native dot and
    // angular covers must establish transversality over the complete source.
    let axis = Curve {
        degree: 8,
        knots: [vec![0.; 9], vec![1.; 9]].concat(),
        control_points: p
            .control_points
            .windows(2)
            .map(|pair| {
                (0..3)
                    .map(|k| 1.125 * (pair[1][k] - pair[0][k]) + if k == 2 { 0.25 } else { 0. })
                    .collect()
            })
            .collect(),
        weights: vec![1.; 9],
        periodic: false,
    };
    let normal = nurbs::progressive_sweep::constant_vector_law([0., 0., 1.]).unwrap();
    let twist = Curve {
        degree: 4,
        knots: [vec![17.; 5], vec![29.; 5]].concat(),
        control_points: [0., 0., 0.03125, 0., 0.]
            .into_iter()
            .map(|v| vec![v, 0., 0.])
            .collect(),
        weights: vec![1.; 5],
        periodic: false,
    };
    (axis, normal, twist)
}
pub fn material() -> (Vec<Curve>, Surface, Vec<Vec<Curve>>) {
    let r = 1. / 1024.;
    let xyz = |a: f64, b: f64| vec![-2. * r * a, -r * b, 9. * r * a];
    let surface = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-3., -3., 5., 5.],
        knots_v: vec![7., 7., 11., 11.],
        control_points: vec![
            vec![xyz(-1., -1.), xyz(-1., 1.)],
            vec![xyz(1., -1.), xyz(1., 1.)],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let signs = [
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
    let profiles = (0..4)
        .map(|i| Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: signs[2 * i..=2 * i + 2]
                .iter()
                .map(|p| xyz(p[0], p[1]))
                .collect(),
            weights: vec![1., 0.5, 1.],
            periodic: false,
        })
        .collect();
    let trims = (0..4)
        .map(|i| Curve {
            degree: 2,
            knots: vec![13., 13., 13., 19., 19., 19.],
            control_points: signs[2 * i..=2 * i + 2]
                .iter()
                .map(|p| vec![1. + 4. * p[0], 9. + 2. * p[1]])
                .collect(),
            weights: vec![1., 0.5, 1.],
            periodic: false,
        })
        .collect();
    (profiles, surface, vec![trims])
}
