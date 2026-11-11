use super::*;
#[test]
fn aligned_corners_do_not_certify_an_interior_rank_loss() {
    let s = Surface {
        degree_u: 3,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [0., 1., 0., 1.]
            .iter()
            .map(|&x| vec![vec![x, 0., 0.], vec![x, 1., 0.]])
            .collect(),
        weights: vec![vec![1.; 2]; 4],
        periodic_u: false,
        periodic_v: false,
    };
    assert!(s.evaluate(0.5, 0.5).unwrap().unit_normal().is_none());
    assert_eq!(
        certify_surface_cell(&s, 3, 1).classification,
        SurfaceRegularityClass::Unresolved
    );
}
#[test]
fn flat_corner_normals_do_not_prove_a_patch_is_planar() {
    let s = Surface {
        degree_u: 3,
        degree_v: 3,
        knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points: (0..4)
            .map(|i| {
                (0..4)
                    .map(|j| {
                        vec![
                            i as f64,
                            j as f64,
                            if (1..=2).contains(&i) && (1..=2).contains(&j) {
                                1.
                            } else {
                                0.
                            },
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 4]; 4],
        periodic_u: false,
        periodic_v: false,
    };
    assert!(s.evaluate(0.5, 0.5).unwrap().point[2] > 0.);
    assert_eq!(
        certify_surface_cell(&s, 3, 3).classification,
        SurfaceRegularityClass::Regular
    );
    assert_eq!(
        classify_normal_box(&[[0., 0.], [0., 0.], [1., 2.]]),
        NormalBoxClass::Regular
    );
}
