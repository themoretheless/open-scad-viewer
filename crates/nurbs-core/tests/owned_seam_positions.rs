use nurbs_core::{curve::Curve, surface::Surface, surface_g2_pcurves};
fn plane() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn public_report_mutation_cannot_change_owned_input_snapshots() {
    let s = plane();
    let c = Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.]]).unwrap();
    let p = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
    let mut result = surface_g2_pcurves::inspect_owned(
        &c,
        [&p, &p],
        [&s, &s],
        [false, false],
        1e-7,
        1e-5,
        1e-4,
        10000,
        50000,
    )
    .unwrap();
    let token = result
        .positions
        .as_ref()
        .expect("full-curve positions must qualify");
    assert!(token.matches(&c, &p, &s, false, 1e-7));
    result.report.positions.proven = false;
    result.report.positions.upper = Some(100.);
    token.reverify().unwrap();
    let mut changed = s.clone();
    changed.control_points[1][1][2] = 0.25;
    assert!(!token.matches(&c, &p, &changed, false, 1e-7));
    let mut changed = c.clone();
    changed.control_points[1][2] = 0.01;
    assert!(!token.matches(&changed, &p, &s, false, 1e-7));
    assert!(!token.matches(&c, &p, &s, true, 1e-7));
    assert!(!token.matches(&c, &p, &s, false, 1e-8));
}
#[test]
fn mismatching_curve_does_not_receive_positional_ownership() {
    let s = plane();
    let c = Curve::from_polyline(vec![vec![0., 0., 0.01], vec![1., 0., 0.01]]).unwrap();
    let p = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
    let result = surface_g2_pcurves::inspect_owned(
        &c,
        [&p, &p],
        [&s, &s],
        [false, false],
        1e-7,
        1e-5,
        1e-4,
        10000,
        50000,
    )
    .unwrap();
    assert!(!result.report.positions.proven);
    assert!(result.positions.is_none());
}
