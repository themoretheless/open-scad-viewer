use nurbs_core::ss_intersection as ss;
use std::sync::Arc;
#[allow(dead_code)]
#[path = "support/ss_original_ideal.rs"]
mod fixture;
#[path = "common/periodic_extrusion.rs"]
mod periodic;
fn contact_curve(nonunit: bool) -> ss::OriginalSsRootCurve {
    let mut a = fixture::plane(0.);
    let mut b = fixture::tangent(false);
    if nonunit {
        a.knots_u = vec![-2., -2., 6., 6.];
        a.knots_v = vec![3., 3., 5., 5.];
        b.knots_u = vec![-5., -5., -5., 3., 3., 3.];
        b.knots_v = vec![10., 10., 10., 14., 14., 14.];
    }
    let source = ss::FiberSources::new(&a, &b).unwrap();
    let ideal = ss::qualify_original_ss_ideal(&source, [1, 1, 2, 2], Default::default()).unwrap();
    let basis = ss::qualify_ss_groebner_basis(&ideal, Default::default()).unwrap();
    let equations = ss::qualify_ss_deflated_system(&basis, Default::default()).unwrap();
    let f = ss::qualify_deflated_ss_fiber(
        &equations,
        [[0.4, 0.6], [0.1, 0.9], [0.4, 0.6], [0.25, 0.75]],
        ss::FiberAxis::SecondV,
        Default::default(),
    )
    .unwrap()
    .into_fiber()
    .unwrap();
    ss::OriginalSsRootCurve::from_deflated(f)
}
fn contains(bounds: &[[f64; 2]], p: &[f64]) {
    for (d, x) in bounds.iter().zip(p) {
        assert!(d[0] <= *x && *x <= d[1], "{d:?} misses {x}");
    }
}
#[test]
fn root_valued_world_and_uv_geometry_retains_original_nonunit_span_clocks() {
    let curve = contact_curve(true);
    curve.reverify().unwrap();
    let coarse = curve
        .enclose(
            [0.5; 2],
            ss::OriginalSsCurveLimits {
                max_refinements: 0,
                ..Default::default()
            },
        )
        .unwrap();
    let fine = curve.enclose([0.5; 2], Default::default()).unwrap();
    contains(&fine.world, &[0.5, 0.5, 0.]);
    contains(&fine.uv, &[2., 4., -1., 12.]);
    assert!(fine.world[1][1] - fine.world[1][0] < coarse.world[1][1] - coarse.world[1][0]);
    assert!(fine.world.iter().all(|d| d[1] - d[0] < 1e-10));
    assert!(fine.interval_operations <= 1000000);
    assert!(fine.projection_work <= 1000000);
    assert!(curve.enclose([-1., 0.], Default::default()).is_err());
}
#[test]
fn root_point_identity_is_shared_without_nearest_point_welding() {
    let curve = Arc::new(contact_curve(false));
    let a = ss::OriginalSsRootPoint::new(curve.clone(), 0.5).unwrap();
    let b = a.clone();
    assert!(a.same_selector(&b));
    let other = ss::OriginalSsRootPoint::new(Arc::new((*curve).clone()), 0.5).unwrap();
    assert!(!a.same_selector(&other));
    contains(
        &a.enclose(Default::default()).unwrap().world,
        &[0.5, 0.5, 0.],
    );
    assert!(ss::OriginalSsRootPoint::new(curve, 0.9).is_err());
}
#[test]
fn periodic_root_carrier_keeps_lifted_uv_and_encloses_one_world_seam_point() {
    let source =
        ss::FiberSources::new(&periodic::source(1), &fixture::curved_periodic_cutter()).unwrap();
    let f = source
        .certify_quotient(
            [[-0.03125, 0.03125], [0.4, 0.6], [0.4, 0.6], [0.35, 0.65]],
            ss::FiberAxis::FirstU,
            Default::default(),
        )
        .unwrap()
        .into_fiber()
        .unwrap();
    let curve = ss::OriginalSsRootCurve::from_quotient(f);
    let b = curve.enclose([0.; 2], Default::default()).unwrap();
    contains(&b.world, &[1., 0., 1.]);
    assert_eq!(b.uv[0], [0., 0.]);
    assert!(curve.source().first().periodic_u);
    assert!(b.world.iter().all(|d| d[1] - d[0] < 1e-10),"{b:?}");
}
#[test]
fn exact_pole_carrier_retains_a_nonartificial_world_endpoint() {
    let source = ss::FiberSources::new(&fixture::cone(), &fixture::pole_plane(true)).unwrap();
    let ideal = ss::qualify_original_ss_ideal(&source, [1; 4], Default::default()).unwrap();
    let pole = ss::qualify_ss_pole_stratum(&ideal, 0, 1, 0, Default::default()).unwrap();
    let basis = ss::qualify_ss_groebner_basis(&ideal, Default::default()).unwrap();
    let deflated = ss::qualify_ss_deflated_system(&basis, Default::default()).unwrap();
    let saturated = ss::qualify_ss_pole_saturation(&deflated, &pole, Default::default()).unwrap();
    let arc = ss::qualify_pole_ss_root_arc(
        &saturated,
        [[0.4, 0.6], [0., 0.25], [0.4, 0.8], [0.2, 0.5]],
        Default::default(),
    )
    .unwrap();
    let curve = ss::OriginalSsRootCurve::from_pole(arc);
    assert_eq!(curve.pole_endpoint().unwrap().0, 0.);
    let at = curve.enclose([0.; 2], Default::default()).unwrap();
    contains(&at.world, &[0.; 3]);
    let inside = curve.enclose([0.125; 2], Default::default()).unwrap();
    contains(&inside.world, &[0., 0.125, 0.25]);
}
