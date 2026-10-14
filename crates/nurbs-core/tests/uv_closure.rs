use nurbs_core::{curve::Curve, trim_domain::exact_loop_joins};
fn edge(a: [f64; 2], b: [f64; 2]) -> Curve {
    nurbs_core::paths::bezier(vec![a.to_vec(), b.to_vec()], Some(vec![1., 3.])).unwrap()
}
#[test]
fn exact_authored_joins_are_independent_of_weights_and_domain() {
    let a = edge([0., 0.], [2., 0.]);
    let b = edge([2., 0.], [0., 2.]);
    let c = edge([0., 2.], [0., 0.]);
    let loop_ = [a.clone(), b, c];
    assert_eq!(exact_loop_joins(&loop_).unwrap(), Some(true));
    let mapped = loop_
        .iter()
        .map(|c| {
            nurbs_core::foundation::reparameterize_curve_report(c, [2., 7.], None)
                .unwrap()
                .curve
        })
        .collect::<Vec<_>>();
    assert_eq!(exact_loop_joins(&mapped).unwrap(), Some(true));
    assert_eq!(exact_loop_joins(&[a]).unwrap(), Some(false));
    let mut broken = loop_.to_vec();
    broken[1].control_points[0][0] += 1e-12;
    assert_eq!(exact_loop_joins(&broken).unwrap(), Some(false));
}
#[test]
fn closed_rational_curve_is_not_a_simplicity_proof() {
    let c = nurbs_core::paths::bezier(
        vec![vec![0., 0.], vec![1., 2.], vec![0., 0.]],
        Some(vec![1., 4., 2.]),
    )
    .unwrap();
    assert_eq!(exact_loop_joins(&[c]).unwrap(), Some(true));
}
#[test]
fn periodic_and_unclamped_endpoints_remain_unproven() {
    let c = Curve {
        degree: 2,
        knots: (0..11).map(|i| i as f64).collect(),
        control_points: vec![
            vec![1., 0.],
            vec![0.5, 1.],
            vec![-0.5, 1.],
            vec![-1., 0.],
            vec![-0.5, -1.],
            vec![0.5, -1.],
            vec![1., 0.],
            vec![0.5, 1.],
        ],
        weights: vec![1.; 8],
        periodic: true,
    };
    assert_eq!(exact_loop_joins(&[c.clone()]).unwrap(), None);
    let mut unclamped = c;
    unclamped.periodic = false;
    assert_eq!(exact_loop_joins(&[unclamped]).unwrap(), None);
}
#[test]
fn wrong_dimensions_invalid_geometry_and_budget_are_refused() {
    assert!(exact_loop_joins(&[]).is_err());
    let c = edge([0., 0.], [0., 0.]);
    assert_eq!(exact_loop_joins(&vec![c.clone(); 256]).unwrap(), Some(true));
    assert!(exact_loop_joins(&vec![c.clone(); 257]).is_err());
    let mut bad = c;
    bad.weights[0] = 0.;
    assert!(exact_loop_joins(&[bad]).is_err());
    let three = nurbs_core::primitives::line([0.; 3], [1., 0., 0.]).unwrap();
    assert!(exact_loop_joins(&[three]).is_err());
}
