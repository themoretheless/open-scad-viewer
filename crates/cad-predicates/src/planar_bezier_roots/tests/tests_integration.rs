    use super::*;
    use crate::{Limits, SourceArena, ToleranceContext};
    fn arbitrary_pair(a: &[[f64; 3]], b: &[[f64; 3]], limits: Limits) -> PlanarScalarOutcome {
        let source = SourceArena::authored(
            "planar-projected-root-test",
            1,
            a.iter()
                .chain(b)
                .flatten()
                .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .unwrap();
        let refs = |start: usize, n: usize| {
            (start..start + n)
                .map(|i| std::array::from_fn(|k| source.leaf(i * 3 + k).unwrap()))
                .collect::<Vec<_>>()
        };
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&source, &tolerance, limits, None);
        planar_bezier_scalar_roots(&mut ctx, &refs(0, a.len()), &refs(a.len(), b.len()), 1e-12)
            .unwrap()
            .outcome
    }
    #[test]
    fn projected_coordinate_retains_multiple_contacts_and_nearby_bifurcations() {
        let ya = [0., 0., 1., 3.];
        let yb = [1., -1., 1., 7.];
        let frame = |i: usize, y: f64| [17. + 10. * i as f64 - y, -9. + 10. * i as f64 + y, 1.];
        let a = std::array::from_fn::<_, 4, _>(|i| frame(i, ya[i]));
        for (delta, count, multiplicity) in [
            (0., 1, 2),
            (2_f64.powi(-30), 0, 1),
            (-2_f64.powi(-30), 2, 1),
        ] {
            let b = std::array::from_fn::<_, 4, _>(|i| frame(i, yb[i] + delta));
            let PlanarScalarOutcome::Roots { reduction, roots } =
                arbitrary_pair(&a, &b, Limits::default())
            else {
                panic!("Missing projected reduction")
            };
            assert_eq!(reduction, PlanarReduction::CommonProjection);
            assert_eq!(roots.len(), count);
            assert!(roots.iter().all(|r| r.multiplicity == multiplicity));
            if delta == 0. {
                assert!(roots[0].interval[0] <= 1. / 3. && roots[0].interval[1] >= 1. / 3.)
            }
            if count == 2 {
                assert!(roots[0].interval[1] < roots[1].interval[0])
            }
        }
        let b = std::array::from_fn::<_, 4, _>(|i| frame(i, yb[i]));
        let mut limits = Limits::default();
        limits.max_work = 0;
        assert!(matches!(
            arbitrary_pair(&a, &b, limits),
            PlanarScalarOutcome::Indeterminate(Reason::ResourceLimit)
        ));
    }
    #[test]
    fn common_coordinate_uses_actual_monotonicity_and_original_weights() {
        let x = [0., 1., 0.9, 1.9];
        let ya = [0., 0., 1., 3.];
        let yb = [1., -1., 1., 7.];
        let mut a = std::array::from_fn::<_, 4, _>(|i| [x[i], ya[i], 1.]);
        let mut b = std::array::from_fn::<_, 4, _>(|i| [x[i], yb[i], 1.]);
        let PlanarScalarOutcome::Roots { reduction, roots } =
            arbitrary_pair(&a, &b, Limits::default())
        else {
            panic!("Missing actual monotonicity proof")
        };
        assert_eq!(reduction, PlanarReduction::CommonCoordinate(0));
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].multiplicity, 2);
        a[1][2] = 100.;
        a[2][2] = 100.;
        b[1][2] = 100.;
        b[2][2] = 100.;
        assert!(matches!(
            arbitrary_pair(&a, &b, Limits::default()),
            PlanarScalarOutcome::NotReducible | PlanarScalarOutcome::Indeterminate(_)
        ));
    }
    fn pair(y: &[f64; 4], limits: Limits) -> PlanarScalarOutcome {
        let a = (0..4).map(|i| [i as f64, 0., 1.]).collect::<Vec<_>>();
        let b = (0..4).map(|i| [i as f64, y[i], 1.]).collect::<Vec<_>>();
        let source = SourceArena::authored(
            "planar-scalar-test",
            1,
            a.iter()
                .chain(&b)
                .flatten()
                .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .unwrap();
        let refs = |start: usize| {
            (start..start + 4)
                .map(|i| std::array::from_fn(|k| source.leaf(i * 3 + k).unwrap()))
                .collect::<Vec<_>>()
        };
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&source, &tolerance, limits, None);
        planar_bezier_scalar_roots(&mut ctx, &refs(0), &refs(4), 1e-12)
            .unwrap()
            .outcome
    }
    #[test]
    fn isolates_non_dyadic_multiple_root_and_distinguishes_both_nearby_perturbations() {
        let PlanarScalarOutcome::Roots { roots, .. } = pair(&[1., -1., 0., 4.], Limits::default())
        else {
            panic!("Missing exact reduction")
        };
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].multiplicity, 2);
        assert!(roots[0].interval[0] <= 1. / 3. && roots[0].interval[1] >= 1. / 3.);
        let delta = 2_f64.powi(-40);
        let PlanarScalarOutcome::Roots { roots, .. } = pair(
            &[1. + delta, -1. + delta, delta, 4. + delta],
            Limits::default(),
        ) else {
            panic!("Missing exclusion")
        };
        assert!(roots.is_empty());
        let PlanarScalarOutcome::Roots { roots, .. } = pair(
            &[1. - delta, -1. - delta, -delta, 4. - delta],
            Limits::default(),
        ) else {
            panic!("Missing separated roots")
        };
        assert_eq!(roots.len(), 2);
        assert!(roots.iter().all(|r| r.multiplicity == 1));
        assert!(roots[0].interval[1] < roots[1].interval[0]);
    }
    #[test]
    fn fraction_free_sturm_preserves_multiplicity_and_endpoint_ownership() {
        let source = SourceArena::authored(
            "sturm-test",
            1,
            vec![AuthoredScalar::Binary64Bits(0_f64.to_bits())],
        )
        .unwrap();
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
        for (coefficients, expected) in [
            (vec![-0.1875, 1., -1.75, 1.], vec![(0.5, 2), (0.75, 1)]),
            (vec![0., 0., 0., -1., 3., -3., 1.], vec![(0., 3), (1., 3)]),
            (vec![1., -12., 60., -160., 240., -192., 64.], vec![(0.5, 6)]),
        ] {
            let polynomial = coefficients.into_iter().map(Expansion::scalar).collect();
            let roots = isolate(&polynomial, 1e-12, &mut ctx).unwrap();
            assert_eq!(roots.len(), expected.len());
            for (root, (t, m)) in roots.iter().zip(expected) {
                assert_eq!(root.interval, [t, t]);
                assert_eq!(root.multiplicity, m);
            }
        }
    }
    #[test]
    fn exhausted_predicate_work_never_becomes_a_disjoint_or_coincident_report() {
        assert!(matches!(
            pair(
                &[0.; 4],
                Limits {
                    max_work: 0,
                    ..Limits::default()
                }
            ),
            PlanarScalarOutcome::Indeterminate(Reason::ResourceLimit)
        ));
    }
