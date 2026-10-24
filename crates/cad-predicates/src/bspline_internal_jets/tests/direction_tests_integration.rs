    use super::*;
    use crate::{Limits, SourceArena, ToleranceContext};
    fn audit(
        p: &[[f64; 4]],
        u: &[f64],
        degree: usize,
        order: usize,
        endpoints: bool,
        arc: bool,
        work: u64,
    ) -> BezierIdentityDecision {
        let mut v = p.iter().flatten().copied().collect::<Vec<_>>();
        let start = v.len();
        v.extend_from_slice(u);
        let arena = SourceArena::authored(
            "original-direction-jets",
            1,
            v.into_iter()
                .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
                .collect(),
        )
        .unwrap();
        let controls = (0..p.len())
            .map(|i| std::array::from_fn(|j| arena.leaf(i * 4 + j).unwrap()))
            .collect::<Vec<_>>();
        let knots = (0..u.len())
            .map(|i| arena.leaf(start + i).unwrap())
            .collect::<Vec<_>>();
        let tol = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(
            &arena,
            &tol,
            Limits {
                max_work: work,
                ..Limits::default()
            },
            None,
        );
        rational_bspline_direction_jet_identity(
            &mut ctx, &controls, &knots, degree, order, endpoints, arc,
        )
        .unwrap()
    }
    #[test]
    fn tangent_identity_owns_position_speed_jumps_and_exact_refusals() {
        let p=[[0.,0.,0.,1.],[1.,0.,0.,1.],[4.,0.,0.,1.]];
        let knots=[2.,2.,3.,5.,5.];
        let proof=audit(&p,&knots,1,0,false,false,1000000);
        assert_eq!(proof.outcome,BezierIdentity::Equal);
        assert!(matches!(audit(&p,&knots,1,0,false,false,proof.work_used-1).outcome,BezierIdentity::Indeterminate(_)));
        assert_ne!(audit(&p,&knots,1,0,true,false,1000000).outcome,BezierIdentity::Equal);
        for endpoint in [[0.,0.,0.,1.],[1.,0.,0.,1.],[4.,f64::from_bits(1),0.,1.]] {
            let mut wrong=p;wrong[2]=endpoint;
            assert_ne!(audit(&wrong,&knots,1,0,false,false,1000000).outcome,BezierIdentity::Equal);
        }
        // A repeated knot can break position even with parallel velocities.
        let disconnected=[[0.,0.,0.,1.],[1.,0.,0.,1.],[2.,0.,0.,1.],[3.,0.,0.,1.]];
        assert_ne!(audit(&disconnected,&[0.,0.,0.5,0.5,1.,1.],1,0,false,false,1000000).outcome,BezierIdentity::Equal);
    }
    #[test]
    fn direction_jets_keep_speed_changes_separate_and_refuse_reversal_and_partial_work() {
        let p = [[0., 0., 0., 1.], [1., 0., 0., 1.], [4., 0., 0., 1.]];
        let knots = [2., 2., 3., 5., 5.];
        for arc in [false, true] {
            for endpoints in [false, true] {
                let r = audit(&p, &knots, 1, 2, endpoints, arc, 1000000);
                assert_eq!(r.outcome, BezierIdentity::Equal);
                assert!(matches!(
                    audit(&p, &knots, 1, 2, endpoints, arc, r.work_used - 1).outcome,
                    BezierIdentity::Indeterminate(_)
                ));
                let mut bad = p;
                bad[2][0] = 0.;
                assert_ne!(
                    audit(&bad, &knots, 1, 2, endpoints, arc, 1000000).outcome,
                    BezierIdentity::Equal
                );
                let mut stopped = p;
                stopped[2][0] = 1.;
                assert_ne!(
                    audit(&stopped, &knots, 1, 2, endpoints, arc, 1000000).outcome,
                    BezierIdentity::Equal
                );
            }
        }
    }
    #[test]
    fn rational_conic_direction_curvature_does_not_infer_second_arc_jet() {
        let p = [
            [1., 0., 0., 1.],
            [1., 1., 0., 0.5],
            [0., 1., 0., 1.],
            [-1., 1., 0., 0.5],
            [-1., 0., 0., 1.],
            [-1., -1., 0., 0.5],
            [0., -1., 0., 1.],
            [1., -1., 0., 0.5],
            [1., 0., 0., 1.],
        ];
        let u = [2., 2., 2., 2.75, 2.75, 3.5, 3.5, 4.25, 4.25, 5., 5., 5.];
        for endpoints in [false, true] {
            assert_eq!(
                audit(&p, &u, 2, 1, endpoints, true, 1000000).outcome,
                BezierIdentity::Equal
            );
            assert_eq!(
                audit(&p, &u, 2, 2, endpoints, true, 1000000).outcome,
                BezierIdentity::Different
            );
        }
    }

#[test]
fn zero_direction_order_checks_positive_orientation_but_supplies_no_position_credit() {
    let u = [0., 0., 0., 0.5, 0.5, 0.5, 1., 1., 1.];
    let mut p = [
        [0., 0., 0., 1.],
        [0.5, 0., 0., 2.],
        [1., 0., 0., 4.],
        [1_f64.next_up(), 0., 0., 0.3],
        [1.5, 0., 0., 0.7],
        [2., 0., 0., 1.],
    ];
    let full = audit(&p, &u, 2, 0, false, true, 1000000);
    assert_eq!(full.outcome, BezierIdentity::Equal);
    for budget in [0, full.work_used - 1] {
        assert_ne!(
            audit(&p, &u, 2, 0, false, true, budget).outcome,
            BezierIdentity::Equal
        );
    }
    p[4][0] = 0.5;
    p[5][0] = 0.;
    assert_ne!(
        audit(&p, &u, 2, 0, false, true, 1000000).outcome,
        BezierIdentity::Equal
    );
}
