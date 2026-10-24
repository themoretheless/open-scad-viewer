//! Shared original native source for Rust/JSON/WASM/STEP qualification.
use super::nurbs;
use nurbs::curve::Curve;
pub fn path(reclock: bool) -> nurbs::curve::Curve {
    let mut p = nurbs::curve::Curve {
        degree: 9,
        knots: [vec![-3.; 10], vec![5.; 10]].concat(),
        control_points: vec![
            vec![0., 0., 0.],
            vec![1., 0., 0.],
            vec![2., 1., 0.],
            vec![3., 3., 1.],
            vec![2., 4., 6.],
            vec![-2., 4., -4.],
            vec![-3., 3., -1.],
            vec![-2., 1., 0.],
            vec![-1., 0., 0.],
            vec![0., 0., 0.],
        ],
        weights: vec![1., 1., 1., 1., 1.25, 1.125, 1., 1., 1., 1.],
        periodic: false,
    };
    // Exact rational Mobius time change phi(t)=2t/(1+t), preserving every
    // original geometric point. Endpoint raw speeds differ by factor four.
    if reclock {
        for (i, w) in p.weights.iter_mut().enumerate() {
            *w *= 2_f64.powi(i as i32);
        }
    }
    p
}

pub fn material() -> (Vec<Curve>, nurbs::surface::Surface, Vec<Vec<Curve>>) {
    let r = 1. / 128.;
    let s = nurbs::surface::Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-1., -1., 3., 3.],
        knots_v: vec![2., 2., 6., 6.],
        control_points: vec![
            vec![vec![0., -r, -r], vec![0., -r, r]],
            vec![vec![0., r, -r], vec![0., r, r]],
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
    let curves = (0..4)
        .map(|i| nurbs::curve::Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: signs[2 * i..=2 * i + 2]
                .iter()
                .map(|p| vec![0., r * p[0], r * p[1]])
                .collect(),
            weights: vec![1., 0.5, 1.],
            periodic: false,
        })
        .collect::<Vec<_>>();
    let trims = (0..4)
        .map(|i| nurbs::curve::Curve {
            degree: 2,
            knots: vec![7., 7., 7., 11., 11., 11.],
            control_points: signs[2 * i..=2 * i + 2]
                .iter()
                .map(|p| vec![1. + 2. * p[0], 4. + 2. * p[1]])
                .collect(),
            weights: vec![1., 0.5, 1.],
            periodic: false,
        })
        .collect();
    (curves, s, vec![trims])
}
