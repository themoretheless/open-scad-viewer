use num_rational::BigRational as Q;
use num_traits::{ToPrimitive, Zero};
use nurbs_core::{ss_intersection as ss, surface::Surface};
#[allow(dead_code)]
#[path = "support/ss_original_ideal.rs"]
mod fixture;
use fixture::{plane, tangent};
#[test]
fn original_rational_cox_equations_match_both_supports_without_a_shared_graph_frame() {
    let a = plane(0.);
    let mut b = tangent(false);
    b.weights = vec![vec![1., 2., 3.], vec![2., 1., 2.], vec![3., 2., 1.]];
    b.knots_u = vec![-2., -2., -2., 6., 6., 6.];
    b.knots_v = vec![3., 3., 3., 5., 5., 5.];
    let source = ss::FiberSources::new(&a, &b).unwrap();
    let ideal = ss::qualify_original_ss_ideal(&source, [1, 1, 2, 2], Default::default()).unwrap();
    ideal.reverify().unwrap();
    let p = [
        Q::new(1.into(), 3.into()),
        Q::new(2.into(), 5.into()),
        Q::new(3.into(), 7.into()),
        Q::new(4.into(), 9.into()),
    ];
    let h = ideal.homogeneous();
    let av = a
        .evaluate(p[0].to_f64().unwrap(), p[1].to_f64().unwrap())
        .unwrap();
    let bv = b
        .evaluate(
            -2. + 8. * p[2].to_f64().unwrap(),
            3. + 2. * p[3].to_f64().unwrap(),
        )
        .unwrap();
    for support in 0..2 {
        let expected = if support == 0 { &av.point } else { &bv.point };
        for k in 0..3 {
            let exact = h[support][k].evaluate(&p) / h[support][3].evaluate(&p);
            assert!((exact.to_f64().unwrap() - expected[k]).abs() < 1e-12);
        }
    }
    for k in 0..3 {
        let exact =
            ideal.equations()[k].evaluate(&p) / (h[0][3].evaluate(&p) * h[1][3].evaluate(&p));
        assert!((exact.to_f64().unwrap() - (av.point[k] - bv.point[k])).abs() < 1e-12);
    }
}
#[test]
fn general_elimination_preserves_tangent_multiplicity_and_source_equations() {
    let source = ss::FiberSources::new(&plane(0.), &tangent(false)).unwrap();
    let ideal = ss::qualify_original_ss_ideal(&source, [1, 1, 2, 2], Default::default()).unwrap();
    let basis = ss::qualify_ss_groebner_basis(&ideal, Default::default()).unwrap();
    basis.reverify().unwrap();
    assert_eq!(basis.complex_dimension(), Some(1));
    assert!(!basis.proves_disjoint_on_span());
    for equation in ideal.equations() {
        assert!(
            basis
                .remainder(equation, Default::default())
                .unwrap()
                .is_zero()
        );
    }
    let derivative = ideal.equations()[2].derivative(2).unwrap();
    assert!(
        !basis
            .remainder(&derivative, Default::default())
            .unwrap()
            .is_zero(),
        "ordinary elimination must not silently radicalize a double contact"
    );
    let p = [
        Q::new(1.into(), 2.into()),
        Q::new(1.into(), 3.into()),
        Q::new(1.into(), 2.into()),
        Q::new(1.into(), 3.into()),
    ];
    assert!(ideal.equations().iter().all(|f| f.evaluate(&p).is_zero()));
}
#[test]
fn empty_complex_ideal_and_overlap_dimension_have_distinct_proofs() {
    for (z, dimension, empty) in [(0., Some(2), false), (1., None, true)] {
        let source = ss::FiberSources::new(&plane(0.), &plane(z)).unwrap();
        let ideal =
            ss::qualify_original_ss_ideal(&source, [1, 1, 1, 1], Default::default()).unwrap();
        let basis = ss::qualify_ss_groebner_basis(&ideal, Default::default()).unwrap();
        assert_eq!(basis.complex_dimension(), dimension);
        assert_eq!(basis.proves_disjoint_on_span(), empty);
        basis.reverify().unwrap();
    }
    let source = ss::FiberSources::new(&plane(0.), &tangent(true)).unwrap();
    let ideal = ss::qualify_original_ss_ideal(&source, [1, 1, 2, 2], Default::default()).unwrap();
    let basis = ss::qualify_ss_groebner_basis(&ideal, Default::default()).unwrap();
    assert_eq!(
        basis.complex_dimension(),
        Some(1),
        "complex dimension must not be relabeled real/world branch dimension"
    );
}
#[test]
fn exact_original_elimination_never_publishes_a_partial_basis_on_exhaustion() {
    let source = ss::FiberSources::new(&plane(0.), &tangent(false)).unwrap();
    assert!(ss::qualify_original_ss_ideal(&source, [0, 1, 2, 2], Default::default()).is_err());
    assert!(
        ss::qualify_original_ss_ideal(
            &source,
            [1, 1, 2, 2],
            ss::ExactSsAlgebraLimits {
                max_work: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    let ideal = ss::qualify_original_ss_ideal(&source, [1, 1, 2, 2], Default::default()).unwrap();
    for limits in [
        ss::ExactSsGroebnerLimits {
            max_basis: 1,
            ..Default::default()
        },
        ss::ExactSsGroebnerLimits {
            variable_order: [0, 0, 2, 3],
            ..Default::default()
        },
        ss::ExactSsGroebnerLimits {
            algebra: ss::ExactSsAlgebraLimits {
                max_work: 1,
                ..Default::default()
            },
            ..Default::default()
        },
    ] {
        assert!(ss::qualify_ss_groebner_basis(&ideal, limits).is_err());
    }
}
#[test]
fn generic_deflation_removes_original_double_contact_without_changing_its_zero_set() {
    let source = ss::FiberSources::new(&plane(0.), &tangent(false)).unwrap();
    let ideal = ss::qualify_original_ss_ideal(&source, [1, 1, 2, 2], Default::default()).unwrap();
    let basis = ss::qualify_ss_groebner_basis(&ideal, Default::default()).unwrap();
    let deflated = ss::qualify_ss_deflated_system(&basis, Default::default()).unwrap();
    deflated.reverify().unwrap();
    assert_eq!(deflated.equations().len(), 3);
    assert!(
        deflated
            .proofs()
            .iter()
            .any(|p| p.original() != p.deflated())
    );
    for p in deflated.proofs() {
        let original_degree = p
            .original()
            .terms()
            .keys()
            .map(|e| e.iter().sum::<usize>())
            .max()
            .unwrap_or(0);
        let new_degree = p
            .deflated()
            .terms()
            .keys()
            .map(|e| e.iter().sum::<usize>())
            .max()
            .unwrap_or(0);
        assert!(new_degree <= original_degree);
    }
    let p = [
        Q::new(1.into(), 2.into()),
        Q::new(1.into(), 3.into()),
        Q::new(1.into(), 2.into()),
        Q::new(1.into(), 3.into()),
    ];
    assert!(
        deflated
            .equations()
            .iter()
            .all(|f| f.evaluate(&p).is_zero())
    );
    assert!(
        ss::qualify_ss_deflated_system(
            &basis,
            ss::ExactSsAlgebraLimits {
                max_work: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
}
#[test]
fn source_equivalent_deflation_qualifies_every_point_of_a_general_double_contact() {
    let source = ss::FiberSources::new(&plane(0.), &tangent(false)).unwrap();
    let ideal = ss::qualify_original_ss_ideal(&source, [1, 1, 2, 2], Default::default()).unwrap();
    let basis = ss::qualify_ss_groebner_basis(&ideal, Default::default()).unwrap();
    let equations = ss::qualify_ss_deflated_system(&basis, Default::default()).unwrap();
    let domain = [[0.4, 0.6], [0.1, 0.9], [0.4, 0.6], [0.25, 0.75]];
    assert!(
        source
            .certify(domain, ss::FiberAxis::SecondV, Default::default())
            .unwrap()
            .fiber()
            .is_none()
    );
    let result = ss::qualify_deflated_ss_fiber(
        &equations,
        domain,
        ss::FiberAxis::SecondV,
        Default::default(),
    )
    .unwrap();
    let report = result.report().clone();
    let fiber = result.into_fiber().unwrap_or_else(|| panic!("{report:?}"));
    fiber.reverify().unwrap();
    fiber
        .at(0.5, Default::default())
        .unwrap()
        .into_fiber()
        .unwrap()
        .reverify()
        .unwrap();
    assert!(fiber.report().contraction_upper.unwrap() < 1.);
    assert!(fiber.report().interval_operations <= 100000);
    for limits in [
        ss::DeflatedSsFiberLimits {
            max_interval_operations: 1,
            ..Default::default()
        },
        ss::DeflatedSsFiberLimits {
            preconditioner: Some([[0.; 3]; 3]),
            ..Default::default()
        },
    ] {
        assert!(
            ss::qualify_deflated_ss_fiber(&equations, domain, ss::FiberAxis::SecondV, limits)
                .unwrap()
                .fiber()
                .is_none()
        );
    }
    let mut empty = domain;
    empty[2] = [0.6, 0.8];
    assert_eq!(
        ss::qualify_deflated_ss_fiber(
            &equations,
            empty,
            ss::FiberAxis::SecondV,
            Default::default()
        )
        .unwrap()
        .report()
        .status,
        ss::FiberStatus::Excluded
    );
}

#[test]
fn disconnected_source_knots_are_rejected_before_a_source_equivalent_ss_owner() {
    let a = plane(0.);
    let mut b = plane(0.);
    b.knots_u = vec![0., 0., 0.5, 0.5, 1., 1.];
    b.control_points = vec![
        b.control_points[0].clone(),
        b.control_points[0].clone(),
        b.control_points[1].clone(),
        b.control_points[1].clone(),
    ];
    b.weights = vec![vec![1.; 2]; 4];
    assert!(ss::FiberSources::new(&a, &b).is_err());
}
