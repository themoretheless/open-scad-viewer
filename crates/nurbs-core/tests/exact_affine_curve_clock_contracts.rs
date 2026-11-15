use nurbs_core::{
    curve::Curve,
    editing::exact_projective::{Refusal, affine_curve_clock},
};
const WORK: u64 = 1_000_000;

fn source() -> Curve {
    Curve {
        degree: 1,
        knots: vec![-1., 0., 1., 3., 5.],
        control_points: vec![vec![2., 5.], vec![1., 3.], vec![5., -2.]],
        weights: vec![1., 2., 0.5],
        periodic: false,
    }
}

#[test]
fn maps_a_rational_unclamped_curve_and_its_exterior_knots() {
    let original = source();
    for (endpoints, expected) in [
        ([6., 12.], [4., 6., 8., 12., 16.]),
        ([12., 6.], [2., 6., 10., 12., 14.]),
    ] {
        let report = affine_curve_clock(&original, endpoints, WORK).unwrap();
        let out = report
            .geometry
            .unwrap_or_else(|| panic!("{:?}", report.refusal));
        assert_eq!(out.knots, expected);
        for t in [0., 0.25, 1., 2.5, 3.] {
            let map = endpoints[0] + (endpoints[1] - endpoints[0]) * t / 3.;
            let a = original.evaluate(t).unwrap().point;
            let b = out.evaluate(map).unwrap().point;
            assert!(a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-13));
        }
    }
}

#[test]
fn checks_the_periodic_seam_and_reversed_traversal() {
    let original = Curve {
        degree: 1,
        knots: vec![-1., 0., 1., 2., 3., 4.],
        control_points: vec![vec![0., 0.], vec![1., 0.], vec![0., 1.], vec![0., 0.]],
        weights: vec![1., 2., 4., 1.],
        periodic: true,
    };
    let report = affine_curve_clock(&original, [12., 6.], WORK).unwrap();
    let out = report
        .geometry
        .unwrap_or_else(|| panic!("{:?}", report.refusal));
    assert!(out.periodic);
    for t in [0., 0.5, 1., 1.5, 2.5, 3.] {
        let a = original.evaluate(t).unwrap().point;
        let b = out.evaluate(12. - 2. * t).unwrap().point;
        assert!(a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-13));
    }
}

#[test]
fn refuses_nonbinary_knots_and_budget_exhaustion() {
    let original = source();
    let report = affine_curve_clock(&original, [0., 1.], WORK).unwrap();
    assert!(report.geometry.is_none());
    assert_eq!(report.refusal, Some(Refusal::NotRepresentable));
    let limited = affine_curve_clock(&original, [0., 3.], 1).unwrap();
    assert!(limited.geometry.is_none());
    assert_eq!(limited.refusal, Some(Refusal::ResourceLimit));
    assert!(limited.work_used <= 1);
    assert!(affine_curve_clock(&original, [0., 0.], WORK).is_err());
}
