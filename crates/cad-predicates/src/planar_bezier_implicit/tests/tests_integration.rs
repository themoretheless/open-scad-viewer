    use super::*;
    use crate::{Limits, SourceArena, ToleranceContext};
    fn pair(a: &[[f64; 3]], b: &[[f64; 3]], limits: Limits) -> PlanarPairOutcome {
        let source = SourceArena::authored(
            "implicit-pair-regression",
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
                .map(|i| std::array::from_fn(|k| source.leaf(3 * i + k).unwrap()))
                .collect::<Vec<_>>()
        };
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&source, &tolerance, limits, None);
        planar_bezier_pair_roots(&mut ctx, &refs(0, a.len()), &refs(a.len(), b.len()), 1e-12)
            .unwrap()
            .outcome
    }
    #[test]
    fn tiny_finite_cap_crossing_is_not_admitted_and_bounded_inward_proposal_is_separated() {
        // Exact binary64 regression of the original source cap. Independent
        // rational arithmetic gives opposite conic signs at 1-2^-16 and
        // 1-2^-20, with the entire inverse range strictly inside (0,1).
        let a: [[f64;3];3]=[[0.26853768389484367,0.16625579515959196,0.8762538394705197],
            [0.18573459718751711,0.,0.7958957478207549],[0.,0.,1.]];
        let original: [[f64;2];6]=[[0.371469194535166,0.],[0.3030302000309807,0.],[0.2345912056748032,0.],
            [0.19004607787900915,0.052433411889112505],[0.23802662093647095,0.10499429840894568],
            [0.26853768389484367,0.16625579515959196]];
        for repaired in [false,true] {
            let mut points=original;
            if repaired {
                points[1]=[0.30303020013988224,1.5625e-10];
                points[2]=[0.23459120579439832,1.5625e-10];
                points[3]=[0.19004607800556444,0.05243341203716979];
                points[4]=[0.2380266210555293,0.10499429854879032];
            }
            let b=points.map(|p|[p[0],p[1],0.9999999999999999]);
            let source=SourceArena::authored("finite-cap-crossing-regression",1,a.iter().chain(&b).flatten().map(|x|AuthoredScalar::Binary64Bits(x.to_bits())).collect()).unwrap();
            let refs=|start:usize,n:usize|(start..start+n).map(|i|std::array::from_fn(|k|source.leaf(i*3+k).unwrap())).collect::<Vec<_>>();
            let tolerance=ToleranceContext::default_valid();
            let mut ctx=PredicateContext::new(&source,&tolerance,Limits{max_work:1000000,..Limits::default()},None);
            let proof=planar_bezier_endpoint_separation(&mut ctx,&refs(0,3),&refs(3,6)).unwrap();
            assert_eq!(proof.proven,repaired,"{:?}",proof.reason);
            assert!(proof.work_used<=1000000);
        }
    }
    #[test]
    fn direct_quadratic_bernstein_separation_keeps_exact_ends_and_refuses_crossings() {
        let a=[[0.,0.,1.],[2.5,0.,1.],[5.,25.,1.]];
        let b=[[0.,0.,1.],[1.,0.,1.],[2.,0.,1.],[3.,0.,1.],[4.,0.,1.],[5.,25.,1.]];
        let prove=|a:&[[f64;3]],b:&[[f64;3]],work:u64| {
            let source=SourceArena::authored("direct-conic-separation-test",1,a.iter().chain(b).flatten().map(|x|AuthoredScalar::Binary64Bits(x.to_bits())).collect()).unwrap();
            let refs=|start:usize,n:usize|(start..start+n).map(|i|std::array::from_fn(|k|source.leaf(i*3+k).unwrap())).collect::<Vec<_>>();
            let t=ToleranceContext::default_valid();let mut ctx=PredicateContext::new(&source,&t,Limits{max_work:work,..Limits::default()},None);
            planar_bezier_endpoint_separation(&mut ctx,&refs(0,a.len()),&refs(a.len(),b.len())).unwrap()
        };
        for transform in [false,true] {
            let frame=|p:[f64;3]|if transform {[17.+p[0]-p[1],-9.+p[0]+p[1],p[2]]}else{p};
            let aa=a.map(frame);let bb=b.map(frame);
            let good=prove(&aa,&bb,1000000);assert!(good.proven);assert!(good.work_used<=1000000);
            let mut crossing=b;crossing[1][1]=2.;
            assert!(!prove(&aa,&crossing.map(frame),1000000).proven);
        }
        assert!(!prove(&a,&a,1000000).proven);
        let empty=prove(&a,&b,1);assert!(!empty.proven&&empty.work_used<=1);
        assert_eq!(empty.reason,Some(Reason::ResourceLimit));
    }
    #[test]
    fn quadratic_carrier_accepts_quintic_query_without_losing_interior_events() {
        let a = [[0.,0.,1.], [2.5,0.,1.], [5.,25.,1.]];
        let b = [[0.,0.,1.],[1.,0.,1.],[2.,0.,1.],[3.,0.,1.],[4.,0.,1.],[5.,25.,1.]];
        let PlanarPairOutcome::Roots(roots) = pair(&a,&b,Limits::default()) else { panic!("Complete quintic census required"); };
        assert_eq!(roots.len(),2);
        assert!(roots.iter().any(|r|r.first==[0.,0.]&&r.second==[0.,0.]&&r.multiplicity==2));
        assert!(roots.iter().any(|r|r.first==[1.,1.]&&r.second==[1.,1.]));
        let mut crossing=b; crossing[1][1]=2.;
        let PlanarPairOutcome::Roots(roots) = pair(&a,&crossing,Limits::default()) else {panic!("Interior quintic census required");};
        assert!(roots.iter().any(|r|r.first[0]>0.&&r.first[1]<1.&&r.second[0]>0.&&r.second[1]<1.));
        assert!(matches!(pair(&a,&b,Limits{max_work:1,..Limits::default()}),PlanarPairOutcome::Indeterminate(_)));
    }
    #[test]
    fn nonlinear_parameter_maps_retain_third_order_contact_and_affine_covariance() {
        // A(t)=(3t,3t²), B(s)=(3s+3s²,3s²+3s³).
        // The implicit substitution is a nonzero multiple of s³(1+s).
        let a = [[0., 0., 1.], [1.5, 0., 1.], [3., 3., 1.]];
        let b = [[0., 0., 1.], [1., 0., 1.], [3., 1., 1.], [6., 6., 1.]];
        for transformed in [false, true] {
            let frame = |p: [f64; 3]| {
                if transformed {
                    [17. + p[0] - p[1], -9. + p[0] + p[1], p[2]]
                } else {
                    p
                }
            };
            let PlanarPairOutcome::Roots(roots) =
                pair(&a.map(frame), &b.map(frame), Limits::default())
            else {
                panic!("missing implicit census")
            };
            assert_eq!(roots.len(), 1);
            assert_eq!(roots[0].first, [0., 0.]);
            assert_eq!(roots[0].second, [0., 0.]);
            assert_eq!(roots[0].multiplicity, 3);
        }
    }
    #[test]
    fn inverse_filters_contacts_outside_the_finite_carrier_and_preserves_pair_parameters() {
        let a = [[0., 0., 1.], [1.5, 0., 1.], [3., 3., 1.]];
        // B(s)=(3s,3/4): A meets B at t=s=1/2.
        let b = [[0., 0.75, 1.], [3., 0.75, 1.]];
        let PlanarPairOutcome::Roots(roots) = pair(&a, &b, Limits::default()) else {
            panic!("missing roots")
        };
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].first, [0.5, 0.5]);
        assert_eq!(roots[0].second, [0.5, 0.5]);
        // Same infinite parabola, but contact x=6 is outside A.
        let b = [[6., 3., 1.], [6., 15., 1.]];
        assert!(
            matches!(pair(&a,&b,Limits::default()),PlanarPairOutcome::Roots(r) if r.is_empty())
        );
    }
    #[test]
    fn singular_carrier_returns_both_node_preimages_and_only_the_true_cusp_preimage() {
        // A(t)=(3(2t-1)²,3(2t-1)³): a cusp at t=1/2.
        let a = [[3., -3., 1.], [-1., 3., 1.], [-1., -3., 1.], [3., 3., 1.]];
        let b = [[-1., 0., 1.], [1., 0., 1.]];
        let PlanarPairOutcome::Roots(r) = pair(&a, &b, Limits::default()) else {
            panic!("missing cusp")
        };
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].first, [0.5, 0.5]);
        assert_eq!(r[0].second, [0.5, 0.5]);
        // Proper nodal cubic: x=3u²-3, y=3u³-3u, u=4t-2.
        let a = [
            [9., -18., 1.],
            [-7., 26., 1.],
            [-7., -26., 1.],
            [9., 18., 1.],
        ];
        let b = [[-1., 0., 1.], [1., 0., 1.]];
        let outcome = pair(&a, &b, Limits::default());
        let PlanarPairOutcome::Roots(r) = outcome else {
            panic!("missing node: {outcome:?}")
        };
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].first, [0.25, 0.25]);
        assert_eq!(r[1].first, [0.75, 0.75]);
        assert!(r.iter().all(|r| r.second == [0.5, 0.5]));
    }
    #[test]
    fn common_carrier_and_resource_exhaustion_do_not_claim_empty_intersections() {
        let a = [[0., 0., 1.], [1.5, 0., 1.], [3., 3., 1.]];
        assert!(matches!(
            pair(&a, &a, Limits::default()),
            PlanarPairOutcome::CommonCarrier
        ));
        let mut limits = Limits::default();
        limits.max_work = 0;
        assert!(matches!(
            pair(&a, &a, limits),
            PlanarPairOutcome::Indeterminate(Reason::ResourceLimit)
        ));
    }
    #[test]
    fn quartic_eliminant_keeps_distinct_pair_parameters_and_a_double_contact() {
        // A(u)=(192u,384u²), B(s)=(96s+96s²,Y(s)).
        // Y-X²/96=-96(s-1/4)²(s-3/4)(s+1).
        let a = [[0., 0., 1.], [96., 0., 1.], [192., 384., 1.]];
        let b = [
            [0., 4.5, 1.],
            [32., -8., 1.],
            [96., 37.5, 1.],
            [192., 357., 1.],
        ];
        let outcome = pair(&a, &b, Limits::default());
        let PlanarPairOutcome::Roots(roots) = outcome else {
            panic!("quartic census: {outcome:?}")
        };
        assert_eq!(roots.len(), 2);
        for (r, (u, s, m)) in roots.iter().zip([(0.15625, 0.25, 2), (0.65625, 0.75, 1)]) {
            assert_eq!(r.first, [u, u]);
            assert_eq!(r.second, [s, s]);
            assert_eq!(r.multiplicity, m);
        }
    }
    #[test]
    fn rational_conic_keeps_original_weights_and_irrational_observation_box() {
        let a = [[3., 0., 1.], [3., 3., 0.5], [0., 3., 1.]];
        let b = [[1., 1., 1.], [4., 4., 1.]];
        let outcome = pair(&a, &b, Limits::default());
        let PlanarPairOutcome::Roots(roots) = outcome else {
            panic!("rational census: {outcome:?}")
        };
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].first, [0.5, 0.5]);
        assert!(roots[0].second[0] <= 1. / 3. && roots[0].second[1] >= 1. / 3.);
        assert!(roots[0].second[1] - roots[0].second[0] <= 1e-12);
    }
    #[test]
    fn exceptional_inverse_fiber_censuses_self_nodes_but_does_not_double_count_a_cusp() {
        for (a, expected) in [
            (
                [
                    [9., -18., 1.],
                    [-7., 26., 1.],
                    [-7., -26., 1.],
                    [9., 18., 1.],
                ],
                1,
            ),
            (
                [[3., -3., 1.], [-1., 3., 1.], [-1., -3., 1.], [3., 3., 1.]],
                0,
            ),
        ] {
            let source = SourceArena::authored(
                "self-pair-regression",
                1,
                a.iter()
                    .flatten()
                    .map(|v: &f64| AuthoredScalar::Binary64Bits(v.to_bits()))
                    .collect(),
            )
            .unwrap();
            let refs = (0..a.len())
                .map(|i| std::array::from_fn(|k| source.leaf(3 * i + k).unwrap()))
                .collect::<Vec<_>>();
            let tolerance = ToleranceContext::default_valid();
            let mut ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
            let outcome = planar_bezier_self_roots(&mut ctx, &refs, 1e-12)
                .unwrap()
                .outcome;
            let PlanarPairOutcome::Roots(roots) = outcome else {
                panic!("self census: {outcome:?}")
            };
            assert_eq!(roots.len(), expected);
            if expected > 0 {
                assert_eq!(roots[0].first, [0.25, 0.25]);
                assert_eq!(roots[0].second, [0.75, 0.75]);
            }
        }
    }
