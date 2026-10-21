use nurbs_core::{ss_intersection::FiberSources, surface::Surface};
pub fn sources(z: &[f64], level: f64) -> FiberSources {
    let degree = z.len() - 1;
    let mut knots = vec![0.; degree + 1];
    knots.extend(vec![1.; degree + 1]);
    let a = Surface {
        degree_u: 1,
        degree_v: degree,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: knots,
        control_points: [0.25, 0.5]
            .into_iter()
            .map(|x| {
                z.iter()
                    .enumerate()
                    .map(|(j, &z)| vec![x, j as f64 / degree as f64, z])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; z.len()]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let b = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., level], vec![0., 1., level]],
            vec![vec![1., 0., level], vec![1., 1., level]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    FiberSources::new(&a, &b).unwrap()
}
