    use super::*;
    #[test]
    fn oblique_graph_cap_preserves_exact_edges_in_both_senses() {
        for reverse in [false,true] {
            let mut points=vec![[-0.5,-0.5],[0.5,-0.5],[0.5,0.5],[-0.5,0.5]];
            if reverse {points.reverse();}
            let world=|p:[f64;2]|vec![p[0],10.-0.5*p[1],p[1]];
            let loops=vec![(0..4).map(|i|Curve{degree:1,knots:vec![0.,0.,1.,1.],
                control_points:vec![world(points[i]),world(points[(i+1)%4])],weights:vec![1.;2],periodic:false}).collect()];
            let cap=coordinate_cap(&loops,1e-9,100000).unwrap().expect("exact oblique graph");
            for (world,uv) in loops[0].iter().zip(&cap.loops[0]) {
                assert_eq!(nurbs_core::curve_surface_agreement::verify_exact(world,uv,&cap.surface,false,1000000).unwrap().unwrap().outcome,cad_predicates::BezierIdentity::Equal);
            }
        }
    }
    #[test]
    fn coordinate_caps_retain_exact_natural_uv_in_all_axes_and_senses() {
        for axis in 0..3 {for reverse in [false,true] {
            let axes:Vec<_>=(0..3).filter(|k|*k!=axis).collect();
            let mut points=vec![[-0.5,-0.5],[0.5,-0.5],[0.5,0.5],[-0.5,0.5]];
            if reverse {points.reverse();}
            let world=|p:[f64;2]|{let mut q=vec![0.;3];q[axis]=3.;q[axes[0]]=p[0];q[axes[1]]=p[1];q};
            let loops=vec![(0..4).map(|i|Curve{degree:1,knots:vec![0.,0.,1.,1.],
                control_points:vec![world(points[i]),world(points[(i+1)%4])],weights:vec![1.;2],periodic:false}).collect()];
            let cap=cap(&loops).unwrap();
            assert_eq!(cap.surface.knots_u,vec![-0.5,-0.5,0.5,0.5]);
            for (world,uv) in loops[0].iter().zip(&cap.loops[0]) {
                assert_eq!(nurbs_core::curve_surface_agreement::verify_exact(world,uv,&cap.surface,false,1000000).unwrap().unwrap().outcome,cad_predicates::BezierIdentity::Equal);
            }
        }}
    }
    #[test]
    fn translated_hollow_coordinate_cap_fits_shared_exact_budget() {
        let outer=nurbs_core::primitives::circle([0.,0.,10.],[0.,0.,1.],0.5).unwrap();
        let inner=nurbs_core::primitives::circle([0.,0.,10.],[0.,0.,1.],0.2).unwrap().reverse().unwrap();
        let loops=vec![retained_bezier_pieces(&outer).unwrap(),retained_bezier_pieces(&inner).unwrap()];
        let result=coordinate_cap(&loops,1e-9,100000).unwrap().expect("translated cap must fit unchanged shared budget");
        assert!(result.surface.knots_u[0]<0.);
        let mut work=0;
        for (worlds,uvs) in loops.iter().zip(&result.loops) {for (world,uv) in worlds.iter().zip(uvs) {
            let proof=nurbs_core::curve_surface_agreement::verify_exact(world,uv,&result.surface,false,1000000-work).unwrap().unwrap();
            work+=proof.work_used;
            assert_eq!(proof.outcome,cad_predicates::BezierIdentity::Equal);
        }}
        assert!(work<1000000);
        let mut changed=loops.clone();changed[0][0].control_points[1][2]=10_f64.next_up();
        assert!(coordinate_cap(&changed,1e-9,100000).unwrap().is_none());
    }
    #[test]
    fn stored_cap_refuses_zero_product_budget_and_unproved_zero_tolerance() {
        let points=[[0.,0.,0.],[1.,0.,0.],[1.,1.,0.],[0.,1.,0.]];
        let loops=vec![(0..4).map(|i|Curve {degree:1,knots:vec![0.,0.,1.,1.],
            control_points:vec![points[i].to_vec(),points[(i+1)%4].to_vec()],
            weights:vec![1.,1.],periodic:false}).collect()];
        let stored = cap_with_boundary_budget(&loops,1e-9,100000).unwrap();
        assert!(certify_cap_chart(&stored.surface, 1000, 1).is_ok());
        assert!(certify_cap_chart(&stored.surface, 0, 1).is_err());
        assert!(certify_cap_chart(&stored.surface, 1000, 0).is_err());
        let mut collapsed = stored.surface.clone();
        collapsed.control_points[1] = collapsed.control_points[0].clone();
        assert!(certify_cap_chart(&collapsed, 100, 1).is_err());
        for (tolerance,budget) in [(1e-9,0),(0.,100000)] {
            let error=cap_with_boundary_budget(&loops,tolerance,budget).err().unwrap();
            assert!(error.message.contains("Cap boundary composition unproved"));
        }
    }
