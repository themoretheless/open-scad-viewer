use nurbs_core::{
    curve::Curve,
    editing::rational_factor::{Refusal, cancel},
};
fn scalar(x: Vec<f64>, weights: Vec<f64>) -> Curve {
    let p = x.len() - 1;
    Curve {
        degree: p,
        knots: [vec![-2.; p + 1], vec![6.; p + 1]].concat(),
        control_points: x.into_iter().map(|x| vec![x, 0.]).collect(),
        weights,
        periodic: false,
    }
}
fn simple() -> Curve {
    scalar(vec![0., 0.5, 1.], vec![1., 2., 4.])
}
fn dump(name: &str, source: &Curve, output: &Curve) {
    println!(
        "factor-oracle={{\"name\":\"{}\",\"source\":{{\"degree\":{},\"knots\":{:?},\"controlPoints\":{:?},\"weights\":{:?}}},\"output\":{{\"degree\":{},\"knots\":{:?},\"controlPoints\":{:?},\"weights\":{:?}}}}}",
        name,
        source.degree,
        source.knots,
        source.control_points,
        source.weights,
        output.degree,
        output.knots,
        output.control_points,
        output.weights
    );
}
#[test]
fn nonconstant_denominator_factor_is_removed_without_changing_the_clock() {
    let source = simple();
    assert!(
        !nurbs_core::editing::exact_degree_reduction::reduce(&source, 1_000_000)
            .unwrap()
            .reduced
    );
    let r = cancel(&source, 1_000_000).unwrap();
    assert!(r.simplified, "{:?}", r.refusal);
    assert_eq!(r.factor_degree, 1);
    assert_eq!(r.curve.degree, 1);
    assert_eq!(r.curve.control_points, vec![vec![0., 0.], vec![1., 0.]]);
    assert_eq!(r.curve.weights, vec![0.5, 1.]);
    assert_eq!(r.curve.domain(), [-2., 6.]);
    assert!(r.work_used <= 1_000_000);
    for i in 0..=32 {
        let t = i as f64 / 32.;
        assert!(
            (r.curve.evaluate(-2. + 8. * t).unwrap().point[0] - 2. * t / (1. + t)).abs() < 1e-12
        );
    }
    assert_eq!(source, simple());
    dump("rational-clock", &source, &r.curve);
}
#[test]
fn three_dimensional_affine_embedding_retains_all_coordinate_laws() {
    let mut source = simple();
    source.control_points = vec![vec![0., 2., -1.], vec![0.5, 3., 1.], vec![1., 4., 3.]];
    let r = cancel(&source, 1_000_000).unwrap();
    assert!(r.simplified, "{:?}", r.refusal);
    assert_eq!(
        r.curve.control_points,
        vec![vec![0., 2., -1.], vec![1., 4., 3.]]
    );
    dump("3d-clock", &source, &r.curve);
}
#[test]
fn negative_monic_factor_uses_one_global_sign() {
    let source = scalar(vec![0., 0.5, 1.], vec![4., 2., 1.]);
    let r = cancel(&source, 1_000_000).unwrap();
    assert!(r.simplified, "{:?}", r.refusal);
    assert_eq!(r.curve.weights, vec![1., 0.5]);
    assert_eq!(r.curve.degree, 1);
    dump("negative-factor", &source, &r.curve);
}
#[test]
fn a_primitive_can_be_elevated_to_a_representable_positive_basis() {
    let source = scalar(vec![0., 0., 2., 4.5, 3.], vec![6., 3.75, 1.5, 3., 12.]);
    assert!(
        !nurbs_core::editing::exact_degree_reduction::reduce(&source, 1_000_000)
            .unwrap()
            .reduced
    );
    let r = cancel(&source, 1_000_000).unwrap();
    assert!(r.simplified, "{:?}", r.refusal);
    assert_eq!(r.factor_degree, 1);
    assert_eq!(r.curve.degree, 3);
    assert_eq!(r.curve.weights, vec![1.5, 0.25, 0.25, 1.5]);
    assert_eq!(
        r.curve.control_points,
        vec![vec![0., 0.], vec![0., 0.], vec![6., 0.], vec![3., 0.]]
    );
    dump("elevated-primitive", &source, &r.curve);
}
#[test]
fn nonrepresentable_cartesian_controls_never_become_an_approximate_edit() {
    let source = scalar(vec![0., 0.25, 0.5, 1.], vec![3., 8., 16., 24.]);
    let r = cancel(&source, 1_000_000).unwrap();
    assert!(!r.simplified);
    assert_eq!(r.refusal, Some(Refusal::NotRepresentable));
    assert_eq!(r.curve, source);
    assert_eq!(r.factor_degree, 1);
}
#[test]
fn mixed_sign_primitive_weights_are_not_replaced_by_individual_absolute_values() {
    let source = scalar(vec![0., 0.25, 5.5, 1.], vec![12., 4., 1., 18.]);
    source.validate().unwrap();
    let r = cancel(&source, 1_000_000).unwrap();
    assert!(!r.simplified);
    assert_eq!(r.refusal, Some(Refusal::NonPositiveWeights));
    assert_eq!(r.curve, source);
}
#[test]
fn one_ulp_perturbation_destroys_the_common_factor() {
    let mut source = simple();
    source.control_points[1][0] = f64::from_bits(0.5f64.to_bits() + 1);
    let r = cancel(&source, 1_000_000).unwrap();
    assert!(!r.simplified);
    assert_eq!(r.refusal, Some(Refusal::NoCommonFactor));
    assert_eq!(r.curve, source);
}
#[test]
fn constant_primitives_use_valid_degree_one_storage() {
    let source = scalar(vec![3., 3.], vec![1., 2.]);
    let r = cancel(&source, 1_000_000).unwrap();
    assert!(r.simplified, "{:?}", r.refusal);
    assert_eq!(r.curve.degree, 1);
    assert_eq!(r.curve.control_points, source.control_points);
    assert_eq!(r.curve.weights, vec![1., 1.]);
    dump("constant", &source, &r.curve);
}
#[test]
fn existing_maximum_degree_and_weight_limits_are_not_raised() {
    let source = scalar(vec![3.; 26], (0..26).map(|i| 2f64.powi(i)).collect());
    source.validate().unwrap();
    let r = cancel(&source, 1_000_000).unwrap();
    assert!(
        r.simplified,
        "{:?}; factor={}, work={}",
        r.refusal, r.factor_degree, r.work_used
    );
    assert_eq!(r.factor_degree, 25);
    assert_eq!(r.curve.degree, 1);
    assert!(r.work_used <= 1_000_000);
    dump("degree25", &source, &r.curve);
    let mut invalid = source.clone();
    invalid.weights[25] = 1e13;
    assert!(cancel(&invalid, 1_000_000).is_err());
}
#[test]
fn resource_refusal_is_atomic_and_the_existing_work_ceiling_is_retained() {
    let source = simple();
    for limit in [1, 2, 8, 32] {
        let r = cancel(&source, limit).unwrap();
        assert!(!r.simplified);
        assert_eq!(r.refusal, Some(Refusal::ResourceLimit));
        assert_eq!(r.curve, source);
        assert!(r.work_used <= limit);
    }
    assert!(cancel(&source, 0).is_err());
    assert!(cancel(&source, 1_000_001).is_err());
}
#[test]
fn invalid_tail_input_is_checked_before_any_factor_search() {
    for value in [0., -1., f64::NAN, f64::INFINITY] {
        let mut source = simple();
        source.weights[2] = value;
        assert!(cancel(&source, 1_000_000).is_err());
        assert_eq!(source.weights[2].to_bits(), value.to_bits());
    }
    let mut source = simple();
    source.control_points[2][0] = f64::NAN;
    assert!(cancel(&source, 1_000_000).is_err());
}
#[test]
fn valid_periodic_and_multispan_sources_are_explicitly_outside_this_operation() {
    let periodic = Curve {
        degree: 2,
        knots: vec![-2., -1., 0., 1., 2., 3., 4., 5.],
        control_points: vec![
            vec![0., 0.],
            vec![1., 0.],
            vec![0., 1.],
            vec![0., 0.],
            vec![1., 0.],
        ],
        weights: vec![1.; 5],
        periodic: true,
    };
    periodic.validate().unwrap();
    assert!(cancel(&periodic, 1_000_000).is_err());
    let multi = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 2., 2., 2.],
        control_points: (0..5).map(|i| vec![i as f64, 0.]).collect(),
        weights: vec![1.; 5],
        periodic: false,
    };
    multi.validate().unwrap();
    assert!(cancel(&multi, 1_000_000).is_err());
}
#[test]
fn a_late_identity_budget_refusal_never_publishes_the_finished_proposal() {
    let source = simple();
    let full = cancel(&source, 1_000_000).unwrap();
    assert!(full.simplified);
    assert!(full.work_used > 1);
    let limited = cancel(&source, full.work_used - 1).unwrap();
    assert!(!limited.simplified);
    assert_eq!(limited.factor_degree, 1);
    assert_eq!(limited.refusal, Some(Refusal::ResourceLimit));
    assert_eq!(limited.curve, source);
    assert!(limited.work_used <= full.work_used - 1);
}
#[test]
fn bounded_coefficient_growth_refuses_a_valid_extreme_binary_scale_source() {
    let source = scalar(
        (0..26)
            .map(|i| {
                if i % 3 == 0 {
                    f64::from_bits(1)
                } else {
                    i as f64 / 32.
                }
            })
            .collect(),
        (0..26).map(|i| 1. + ((i * i) % 11) as f64).collect(),
    );
    source.validate().unwrap();
    let r = cancel(&source, 1_000_000).unwrap();
    assert!(!r.simplified, "{:?}", r);
    assert_eq!(r.refusal, Some(Refusal::ResourceLimit));
    assert_eq!(r.factor_degree, 0);
    assert!(r.work_used + 257 <= 1_000_000);
    println!("coefficient-growth-work={}", r.work_used);
    assert_eq!(r.curve, source);
}
