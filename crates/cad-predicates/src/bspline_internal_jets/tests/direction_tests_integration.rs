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

#[test]
fn third_arc_direction_jet_owns_reclocked_curvature_variation_and_exact_budget() {
    let xyz=[[0.,0.,0.],[1.,0.,0.],[2.,1.,0.],[3.,3.,1.],[4.,6.,4.],
        [2.,7.,2.],[-2.,7.,-2.],[-4.,6.,-4.],[-3.,3.,-1.],[-2.,1.,0.],[-1.,0.,0.],[0.,0.,0.]];
    let p=xyz.iter().enumerate().map(|(i,q)|[q[0],q[1],q[2]/64.,(if i==5 || i==6 {1.125}else{1.})*2f64.powi(i as i32)]).collect::<Vec<_>>();
    let knots=[vec![-3.;12],vec![5.;12]].concat();let before=p.clone();
    let q=audit(&p,&knots,11,3,true,true,1000000);assert_eq!(q.outcome,BezierIdentity::Equal,"{q:?}");
    assert_eq!(audit(&p,&knots,11,3,true,true,q.work_used).outcome,BezierIdentity::Equal);
    for budget in [0,q.work_used-1] {let r=audit(&p,&knots,11,3,true,true,budget);assert_ne!(r.outcome,BezierIdentity::Equal);assert!(r.work_used<=budget);}
    assert_ne!(audit(&p,&knots,11,3,true,false,1000000).outcome,BezierIdentity::Equal);
    let mut wrong=p.clone();wrong[7][2]=wrong[7][2].next_up();
    assert_eq!(audit(&wrong,&knots,11,2,true,true,1000000).outcome,BezierIdentity::Equal);
    assert_eq!(audit(&wrong,&knots,11,3,true,true,1000000).outcome,BezierIdentity::Different);
    for x in [0.,1.] {let mut wrong=p.clone();wrong[10][0]=x;assert_ne!(audit(&wrong,&knots,11,3,true,true,1000000).outcome,BezierIdentity::Equal);}
    // Nonaxial initial tangents have irrational speed (sqrt(14) times a
    // rational). The root-free comparison must remain exact for both signs.
    for sign in [-1.,1.] {
        let oblique=p.iter().map(|q|[sign*(q[0]+q[2]),2.*q[0]+q[1],3.*q[0]-q[1]+q[2],q[3]]).collect::<Vec<_>>();
        assert_eq!(audit(&oblique,&knots,11,3,true,true,1000000).outcome,BezierIdentity::Equal);
    }
    // Two original Bezier spans share position but change raw speed at their
    // common knot. Inspect every internal join, not only the outer endpoints.
    let mut joined=p.iter().enumerate().map(|(i,q)|[q[0],q[1],q[2],q[3]/2f64.powi(i as i32)]).collect::<Vec<_>>();
    joined.extend(p.iter().skip(1).copied());let joined_knots=[vec![2.;12],vec![3.;11],vec![4.;12]].concat();
    assert_eq!(audit(&joined,&joined_knots,11,3,false,true,1000000).outcome,BezierIdentity::Equal);
    assert_ne!(audit(&joined,&joined_knots,11,3,false,false,1000000).outcome,BezierIdentity::Equal);
    joined[15][2]=joined[15][2].next_up();
    assert_eq!(audit(&joined,&joined_knots,11,2,false,true,1000000).outcome,BezierIdentity::Equal);
    assert_eq!(audit(&joined,&joined_knots,11,3,false,true,1000000).outcome,BezierIdentity::Different);
    assert_eq!(p,before);eprintln!("third geometric direction jet exact work={}",q.work_used);
}
