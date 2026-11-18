use nurbs_core::surface::Surface;
pub fn plane(z: f64) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., z], vec![0., 1., z]],
            vec![vec![1., 0., z], vec![1., 1., z]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
pub fn tangent(isolated: bool) -> Surface {
    let q = [0.25, -0.25, 0.25];
    Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        vec![
                            i as f64 / 2.,
                            j as f64 / 2.,
                            q[i] + if isolated { q[j] } else { 0. },
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}
pub fn cone() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![-1., 1., 2.]],
            vec![vec![0., 0., 0.], vec![1., 1., 2.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
pub fn pole_plane(vertical: bool) -> Surface {
    let p = if vertical {
        vec![
            vec![vec![0., -1., -1.], vec![0., -1., 3.]],
            vec![vec![0., 1., -1.], vec![0., 1., 3.]],
        ]
    } else {
        vec![
            vec![vec![-2., -2., 0.], vec![-2., 2., 0.]],
            vec![vec![2., -2., 0.], vec![2., 2., 0.]],
        ]
    };
    Surface {
        control_points: p,
        ..cone()
    }
}
pub fn curved_periodic_cutter() -> Surface {
    let q = [0.25, -0.25, 0.25];
    let y = [-0.5, 0., 0.5];
    Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        vec![
                            0.5 + 0.5 * i as f64,
                            y[j],
                            1. + 0.125 * y[j] + 0.0625 * (q[i] + q[j]),
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}
