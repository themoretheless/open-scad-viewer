use nurbs_core::{surface::Surface, surface_linear_monotonicity};
fn graph() -> Surface {
    Surface {
        degree_u: 5,
        degree_v: 5,
        knots_u: [vec![0.; 6], vec![1.; 6]].concat(),
        knots_v: [vec![0.; 6], vec![1.; 6]].concat(),
        weights: vec![vec![1.; 6]; 6],
        periodic_u: false,
        periodic_v: false,
        control_points: (0..6)
            .map(|i| {
                (0..6)
                    .map(|j| {
                        let u = i as f64 / 5.;
                        let v = j as f64 / 5.;
                        let h = if (i + j) % 2 == 0 {
                            1e6 + 0.123456789
                        } else {
                            -1e6 + 0.987654321
                        };
                        vec![u + v + h, -u + v + h, -2. * v + h]
                    })
                    .collect()
            })
            .collect(),
    }
}
#[test]
fn projected_polynomial_hulls_prove_a_large_height_graph_in_one_cell() {
    let s = graph();
    let before = s.clone();
    let p = [[1, -1, 0], [1, 1, -2]];
    let result = surface_linear_monotonicity::inspect_strong_metric(&s, p, [1., 1.], 1).unwrap();
    assert!(result.certified, "{:?}", result.reason);
    assert_eq!(result.cells, 1);
    assert_eq!(s, before);
    assert!(
        !surface_linear_monotonicity::inspect_strong_metric(&s, p, [1., 1.], 0)
            .unwrap()
            .certified
    );
}
#[test]
fn a_fold_is_not_a_monotone_graph() {
    let mut s = graph();
    for i in 0..6 {
        for j in 0..6 {
            s.control_points[i][j] =
                vec![if i < 3 { i as f64 } else { (5 - i) as f64 }, j as f64, 0.];
        }
    }
    assert!(
        !surface_linear_monotonicity::inspect_strong_metric(
            &s,
            [[1, 0, 0], [0, 1, 0]],
            [1., 1.],
            1
        )
        .unwrap()
        .certified
    );
}
#[test]
fn discontinuous_translation_cannot_get_global_injectivity_from_local_slopes() {
    let s = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 0.5, 0.5, 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        weights: vec![vec![1.; 2]; 4],
        periodic_u: false,
        periodic_v: false,
        control_points: [0., 0.8, 0.2, 1.]
            .iter()
            .map(|&u| vec![vec![u, 0., 0.], vec![u, 1., 0.]])
            .collect(),
    };
    let result =
        surface_linear_monotonicity::inspect_strong_metric(&s, [[1, 0, 0], [0, 1, 0]], [1., 1.], 1);
    assert!(result.map(|r| !r.certified).unwrap_or(true));
}
