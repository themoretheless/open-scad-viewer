use nurbs_core::{surface::Surface, surface_regularity::inspect};
fn panel() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![2., 2., 7., 7.],
        knots_v: vec![-3., -3., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 4., 0.]],
            vec![vec![4., 0., 0.], vec![4., 4., 0.]],
        ],
        weights: vec![vec![1., 2.], vec![3., 6.]],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn weighted_plane_and_polynomial_graph_are_regular() {
    let p = inspect(&panel(), 1000).unwrap();
    assert!(p.spanwise_regular, "{p:?}");
    assert!(p.interior_basis_c1);
    let g = nurbs_core::polynomial::graph(
        [0., 1., 0., 1.],
        &[vec![0., 0., 1.], vec![0., 1., 0.], vec![1., 0., 0.]],
    )
    .unwrap();
    assert!(inspect(&g, 1000).unwrap().spanwise_regular);
}
#[test]
fn collapsed_and_folded_patches_are_never_certified() {
    let mut p = panel();
    for row in &mut p.control_points {
        for point in row {
            point[0] = 0.;
        }
    }
    let r = inspect(&p, 31).unwrap();
    assert!(!r.spanwise_regular);
    assert!(!r.unresolved.is_empty());
    assert!(r.cells <= 31);
    let folded = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            vec![vec![-1., 0., 0.], vec![-1., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    assert!(!inspect(&folded, 63).unwrap().spanwise_regular);
}
#[test]
fn rejects_invalid_source_and_budget() {
    for b in [0, 100001] {
        assert!(inspect(&panel(), b).is_err());
    }
    let mut p = panel();
    p.weights[0][0] = 0.;
    assert!(inspect(&p, 100).is_err());
}

#[test]
fn spanwise_regular_c0_seam_does_not_claim_c1() {
    let mut s = panel();
    s.knots_u = vec![0., 0., 0.5, 1., 1.];
    s.control_points = vec![
        vec![vec![0., 0., 0.], vec![0., 4., 0.]],
        vec![vec![2., 0., 0.], vec![2., 4., 0.]],
        vec![vec![4., 0., 1.], vec![4., 4., 1.]],
    ];
    s.weights = vec![vec![1.; 2]; 3];
    assert!(inspect(&s, 1).is_err());
    let r = inspect(&s, 100).unwrap();
    assert!(r.spanwise_regular);
    assert!(!r.interior_basis_c1);
}
