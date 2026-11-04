use num_bigint::BigInt;
use num_rational::BigRational as Q;
use num_traits::{One, ToPrimitive};
use nurbs_core::ss_intersection as ss;
use std::sync::Arc;
#[path = "support/ss_root_circle.rs"]
mod fixture;
fn q(n: i64, d: i64) -> Q {
    Q::new(n.into(), d.into())
}
fn clock(p: &[f64], d: [f64; 2]) -> Arc<ss::QualifiedSsAlgebraicClock> {
    Arc::new(ss::QualifiedSsAlgebraicClock::from_binary(p, d, Default::default()).unwrap())
}
#[test]
fn exact_unique_root_isolation_refines_without_a_float_definition() {
    let c = clock(&[-1., 0., 8.], [0.34375, 0.375]);
    c.reverify().unwrap();
    let r = c.enclose(64, Default::default()).unwrap();
    assert!(&r.bounds[0] * &r.bounds[0] * q(8, 1) <= Q::one());
    assert!(&r.bounds[1] * &r.bounds[1] * q(8, 1) >= Q::one());
    assert!(&r.bounds[1] - &r.bounds[0] < Q::new(1.into(), BigInt::one() << 60));
    assert!(r.operations <= ss::SsAlgebraicClockLimits::default().max_operations);
    let repeated = clock(&[1., -16., 96., -256., 256.], [0.2, 0.3]);
    assert_eq!(
        repeated.enclose(32, Default::default()).unwrap().bounds,
        [q(1, 4), q(1, 4)]
    );
    repeated.reverify().unwrap();
    let irrational_multiple = clock(&[1., 0., -16., 0., 64.], [0.34375, 0.375]);
    let r = irrational_multiple.enclose(64, Default::default()).unwrap();
    assert!(&r.bounds[0] * &r.bounds[0] * q(8, 1) <= Q::one());
    assert!(&r.bounds[1] * &r.bounds[1] * q(8, 1) >= Q::one());
    assert!(&r.bounds[1] - &r.bounds[0] < Q::new(1.into(), BigInt::one() << 60));
    irrational_multiple.reverify().unwrap();

    let padded = clock(&[-1., 0., 8., 0., 0.], [0.34375, 0.375]);
    padded.reverify().unwrap();
    assert_eq!(padded.polynomial().len(), 3);
}
#[test]
fn absent_ambiguous_endpoint_and_exhausted_proofs_cannot_own_a_clock() {
    let make = |p: &[f64], d| ss::QualifiedSsAlgebraicClock::from_binary(p, d, Default::default());
    assert!(make(&[-1., 0., 8.], [-1., 1.]).is_err());
    assert!(make(&[-1., 0., 8.], [0.5, 0.6]).is_err());
    assert!(make(&[0., 0.], [0., 1.]).is_err());
    assert!(make(&[1.], [0., 1.]).is_err());
    assert!(make(&[f64::NAN, 1.], [0., 1.]).is_err());
    assert!(ss::QualifiedSsAlgebraicClock::qualify(
        vec![Q::new_raw(1.into(), 0.into()), Q::one()],
        [q(0, 1), q(1, 1)],
        Default::default()
    )
    .is_err());
    assert!(ss::QualifiedSsAlgebraicClock::qualify(
        vec![Q::new_raw(2.into(), 2.into()), Q::one()],
        [q(-2, 1), q(0, 1)],
        Default::default()
    )
    .is_err());
    assert!(ss::QualifiedSsAlgebraicClock::qualify(
        vec![q(-1, 2), Q::one(), Q::new_raw(0.into(), 0.into())],
        [q(0, 1), q(1, 1)],
        Default::default()
    )
    .is_err());

    assert!(make(&[-0.5, 1.], [0.5, 1.]).is_err());
    let exact = make(&[-0.5, 1.], [0.5, 0.5]).unwrap();
    exact.reverify().unwrap();
    assert_eq!(
        exact.enclose(0, Default::default()).unwrap().bounds,
        [q(1, 2), q(1, 2)]
    );
    assert!(ss::QualifiedSsAlgebraicClock::from_binary(
        &[-1., 0., 8.],
        [0.3, 0.4],
        ss::SsAlgebraicClockLimits {
            max_operations: 1,
            ..Default::default()
        }
    )
    .is_err());
    let c = make(&[-1., 0., 8.], [0.3, 0.4]).unwrap();
    assert!(c
        .enclose(
            64,
            ss::SsAlgebraicClockLimits {
                max_operations: 1,
                ..Default::default()
            }
        )
        .is_err());
    let huge = Q::from_integer(BigInt::one() << 80);
    assert!(ss::QualifiedSsAlgebraicClock::qualify(
        vec![-huge.clone(), Q::one()],
        [&huge - q(1, 1), &huge + q(1, 1)],
        ss::SsAlgebraicClockLimits {
            max_coefficient_bits: 64,
            ..Default::default()
        }
    )
    .is_err());
}
#[test]
fn algebraic_points_keep_original_world_and_both_uv_images() {
    let wire = fixture::wire();
    let curve = wire.edges()[0].curve().clone();
    let c = clock(&[-1., 0., 8.], [0.34375, 0.375]);
    let p = ss::OriginalSsAlgebraicPoint::new(curve.clone(), c.clone()).unwrap();
    p.reverify().unwrap();
    let b = p.enclose(Default::default()).unwrap();
    let x = (0.125_f64).sqrt();
    let y = 0.5 + (0.3125_f64.powi(2) - (x - 0.5).powi(2)).sqrt();
    for (i, v) in [x, y, 0.].into_iter().enumerate() {
        assert!(b.geometry.world[i][0] <= v && v <= b.geometry.world[i][1]);
    }
    for i in [0, 2] {
        assert!(b.geometry.uv[i][0] <= x && x <= b.geometry.uv[i][1]);
    }
    assert!(p.same_selector(&p.clone()));
    let duplicate =
        ss::OriginalSsAlgebraicPoint::new(curve.clone(), clock(&[-1., 0., 8.], [0.34375, 0.375]))
            .unwrap();
    assert!(
        !p.same_selector(&duplicate),
        "Unshared clock ownership is not a join proof"
    );
    assert!(ss::OriginalSsAlgebraicPoint::new(curve, clock(&[-1., 0., 8.], [0., 0.5])).is_err());
    assert!(p
        .enclose(ss::SsAlgebraicPointLimits {
            clock: ss::SsAlgebraicClockLimits {
                max_operations: 1,
                ..Default::default()
            },
            ..Default::default()
        })
        .is_err());
}
#[test]
fn algebraic_ports_prove_mixed_axis_membership_and_reject_other_branches() {
    let wire = fixture::wire();
    let p = Arc::new(
        ss::OriginalSsAlgebraicPoint::new(
            wire.edges()[1].curve().clone(),
            clock(&[-257., 0., 4096.], [0.25, 0.251]),
        )
        .unwrap(),
    );
    let port = ss::QualifiedSsAlgebraicPort::bind(
        p.clone(),
        wire.edges()[2].curve().clone(),
        [0; 4],
        Default::default(),
    )
    .unwrap();
    assert!(Arc::ptr_eq(port.point(), &p));
    let x = (257_f64).sqrt() / 64.;
    let y = 0.5 + (0.3125_f64.powi(2) - (x - 0.5).powi(2)).sqrt();
    let d = port.parameter_bounds();
    assert!(d[0].to_f64().unwrap() <= y && y <= d[1].to_f64().unwrap());
    port.reverify().unwrap();
    assert!(ss::QualifiedSsAlgebraicPort::bind(
        p.clone(),
        wire.edges()[4].curve().clone(),
        [0; 4],
        Default::default()
    )
    .is_err());
    assert!(ss::QualifiedSsAlgebraicPort::bind(
        p,
        wire.edges()[2].curve().clone(),
        [1, 0, 0, 0],
        Default::default()
    )
    .is_err());
}
#[test]
fn exact_close_clocks_order_an_edge_when_binary64_cannot_separate_them() {
    let wire = fixture::wire();
    let curve = wire.edges()[0].curve().clone();
    let a = q(1, 2);
    let b = &a + Q::new(1.into(), BigInt::one() << 80);
    let eps = Q::new(1.into(), BigInt::one() << 90);
    assert_eq!(a.to_f64(), b.to_f64());
    let polynomial = vec![&a * &b, -(&a + &b), Q::one()];
    assert!(ss::QualifiedSsAlgebraicClock::qualify(
        polynomial.clone(),
        [&a - &eps, &b + &eps],
        Default::default()
    )
    .is_err());
    let ca = Arc::new(
        ss::QualifiedSsAlgebraicClock::qualify(
            polynomial.clone(),
            [&a - &eps, &a + &eps],
            Default::default(),
        )
        .unwrap(),
    );
    let cb = Arc::new(
        ss::QualifiedSsAlgebraicClock::qualify(
            polynomial,
            [&b - &eps, &b + &eps],
            Default::default(),
        )
        .unwrap(),
    );
    let pa = Arc::new(ss::OriginalSsAlgebraicPoint::new(curve.clone(), ca).unwrap());
    let pb = Arc::new(ss::OriginalSsAlgebraicPoint::new(curve.clone(), cb).unwrap());
    let start =
        ss::QualifiedSsAlgebraicPort::bind(pa.clone(), curve.clone(), [0; 4], Default::default())
            .unwrap();
    let end =
        ss::QualifiedSsAlgebraicPort::bind(pb, curve.clone(), [0; 4], Default::default()).unwrap();
    let edge = ss::OriginalSsAlgebraicEdge::new(start.clone(), end.clone()).unwrap();
    assert!(edge.forward());
    assert_eq!(edge.parameter_bounds(), [a, b]);
    edge.enclose(Default::default()).unwrap();
    edge.reverify().unwrap();
    assert!(!ss::OriginalSsAlgebraicEdge::new(end, start.clone())
        .unwrap()
        .forward());
    assert!(ss::OriginalSsAlgebraicEdge::new(start.clone(), start).is_err());
}
#[test]
fn overlapping_clock_boxes_do_not_authorize_endpoint_order() {
    let wire = fixture::wire();
    let curve = wire.edges()[0].curve().clone();
    let a = Arc::new(
        ss::OriginalSsAlgebraicPoint::new(curve.clone(), clock(&[-1., 0., 8.], [0.34, 0.37]))
            .unwrap(),
    );
    let b = Arc::new(
        ss::OriginalSsAlgebraicPoint::new(curve.clone(), clock(&[-5., 0., 32.], [0.35, 0.41]))
            .unwrap(),
    );
    let coarse = ss::SsAlgebraicPointLimits {
        bisections: 0,
        ..Default::default()
    };
    let x = ss::QualifiedSsAlgebraicPort::bind(a.clone(), curve.clone(), [0; 4], coarse).unwrap();
    let y = ss::QualifiedSsAlgebraicPort::bind(b.clone(), curve.clone(), [0; 4], coarse).unwrap();
    assert!(ss::OriginalSsAlgebraicEdge::new(x, y).is_err());
    let x =
        ss::QualifiedSsAlgebraicPort::bind(a, curve.clone(), [0; 4], Default::default()).unwrap();
    let y = ss::QualifiedSsAlgebraicPort::bind(b, curve, [0; 4], Default::default()).unwrap();
    assert!(ss::OriginalSsAlgebraicEdge::new(x, y).unwrap().forward());
}
#[test]
fn common_selector_binds_algebraic_clock_to_a_different_axis() {
    let wire = fixture::wire();
    let source = wire.edges()[1].curve().clone();
    let point = ss::OriginalSsAlgebraicPoint::new(source, clock(&[-257.,0.,4096.],[0.25,0.251])).unwrap();
    assert!(ss::OriginalSsRootPoint::from_algebraic(point.clone(), ss::SsAlgebraicPointLimits {
        clock:ss::SsAlgebraicClockLimits {max_operations:1,..Default::default()},
        ..Default::default()
    }).is_err());
    let shared = Arc::new(ss::OriginalSsRootPoint::from_algebraic(point.clone(),Default::default()).unwrap());
    assert!(shared.binary_parameter().is_none());
    shared.validate_selector().unwrap();
    let original = point.enclose(Default::default()).unwrap();
    let bounds = shared.enclose(Default::default()).unwrap();
    assert_eq!(bounds.uv,original.geometry.uv);
    assert_eq!(bounds.world,original.geometry.world);
    assert_eq!(bounds.projection_work,original.geometry.projection_work+original.clock_operations);
    let port = ss::bind_original_ss_port(shared.clone(),wire.edges()[2].curve().clone(),[0;4],Default::default()).unwrap();
    assert!(Arc::ptr_eq(port.point(),&shared));
    assert_eq!(port.kind(),ss::OriginalSsPortKind::OriginalParameterRoot);
    port.reverify().unwrap();
    assert!(ss::bind_original_ss_port(shared.clone(),wire.edges()[4].curve().clone(),[0;4],Default::default()).is_err());
    assert!(ss::bind_original_ss_port(shared,wire.edges()[2].curve().clone(),[1,0,0,0],Default::default()).is_err());
}
