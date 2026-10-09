use nurbs_core::{continuity, surface::Surface};
fn panel(lo: f64) -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![2., 2., 2., 5., 5., 5.],
        knots_v: vec![-4., -4., 2., 2.],
        control_points: (0..3)
            .map(|i| {
                (0..2)
                    .map(|j| vec![lo + i as f64 / 2., j as f64, 0.])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn adjacent_planes_have_certified_first_and_second_order_jets() {
    let a = panel(0.);
    let b = panel(1.);
    for order in [1, 2] {
        let r = continuity::inspect_surface_jets_checked_report(
            &a, &b, "uMax", "uMin", order, 1., 1e-8,
        )
        .unwrap();
        assert!(r.regularity_certified());
        assert!(r.decision.unwrap().accepted);
    }
    assert_eq!(a, panel(0.));
    assert_eq!(b, panel(1.));
}
#[test]
fn quadratic_departure_preserves_first_order_but_breaks_second_order() {
    let a = panel(0.);
    let mut b = panel(1.);
    // b(u,v)=[1+u,v,u^2/4], so first derivatives agree at u=0,
    // while the second transverse derivative has Z=1/2.
    for p in &mut b.control_points[2] {
        p[2] = 0.25;
    }
    let first =
        continuity::inspect_surface_jets_checked_report(&a, &b, "uMax", "uMin", 1, 1., 1e-8)
            .unwrap();
    assert!(first.decision.unwrap().accepted);
    let second =
        continuity::inspect_surface_jets_checked_report(&a, &b, "uMax", "uMin", 2, 1., 1e-8)
            .unwrap();
    assert!(!second.decision.unwrap().accepted);
    assert!(second.error_bounds.second_derivative_upper.unwrap() >= 0.5);
}
#[test]
fn position_gap_and_invalid_options_are_not_accepted() {
    let a = panel(0.);
    let b = panel(1.1);
    assert!(
        continuity::inspect_surface_jets_checked_report(&a, &b, "uMax", "uMin", 1, 1., 1e-8)
            .is_err()
    );
    assert!(
        continuity::inspect_surface_jets_checked_report(&a, &b, "uMax", "uMin", 0, 1., 1e-8)
            .is_err()
    );
    assert!(
        continuity::inspect_surface_jets_checked_report(&a, &b, "uMax", "uMin", 1, 0., 1e-8)
            .is_err()
    );
}
