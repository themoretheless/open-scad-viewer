    use super::*;
    use crate::{Limits, SourceArena, ToleranceContext};
    fn cusp(axis: usize, limits: Limits) -> PlanarStationaryOutcome {
        let points = [[1., -1., 1.], [-1., 2., 1.], [0., -4., 1.], [4., 8., 1.]];
        let source = SourceArena::authored(
            "stationary-cusp-test",
            1,
            points
                .iter()
                .flatten()
                .map(|v: &f64| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .unwrap();
        let refs = (0..4)
            .map(|i| std::array::from_fn(|k| source.leaf(i * 3 + k).unwrap()))
            .collect::<Vec<_>>();
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&source, &tolerance, limits, None);
        planar_bezier_stationary_roots(&mut ctx, &refs, axis, 1e-14)
            .unwrap()
            .outcome
    }
    #[test]
    fn stationary_cusp_preserves_distinct_derivative_multiplicities_and_uncertainty() {
        for (axis, multiplicity) in [(0, 1), (1, 2)] {
            let PlanarStationaryOutcome::Roots(roots) = cusp(axis, Limits::default()) else {
                panic!("stationary proof absent")
            };
            assert_eq!(roots.len(), 1);
            assert_eq!(roots[0].multiplicity, multiplicity);
            assert!(roots[0].interval[0] <= 1. / 3. && roots[0].interval[1] >= 1. / 3.);
        }
        let mut limits = Limits::default();
        limits.max_work = 0;
        assert!(matches!(
            cusp(0, limits),
            PlanarStationaryOutcome::Indeterminate(Reason::ResourceLimit)
        ));
    }
