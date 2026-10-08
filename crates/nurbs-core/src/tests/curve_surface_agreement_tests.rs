    use super::*;
    #[test]
    fn multispan_natural_boundaries_preserve_basis_senses_and_work_limits() {
        use cad_predicates::BezierIdentity;
        let s=Surface{degree_u:1,degree_v:2,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,0.,0.25,1.,1.,1.],
            control_points:(0..2).map(|u|[0.,1.,3.,5.].iter().map(|z|vec![u as f64,0.,*z]).collect()).collect(),
            weights:vec![vec![1.,0.75,1.5,1.];2],periodic_u:false,periodic_v:false};
        let transposed=Surface{degree_u:2,degree_v:1,knots_u:s.knots_v.clone(),knots_v:s.knots_u.clone(),
            control_points:(0..4).map(|v|(0..2).map(|u|s.control_points[u][v].clone()).collect()).collect(),
            weights:(0..4).map(|v|(0..2).map(|u|s.weights[u][v]).collect()).collect(),periodic_u:false,periodic_v:false};
        for (s,fixed) in [(s,0),(transposed,1)] {for end in [0,1] {for backward in [false,true] {for reversed in [false,true] {
            let mut a=vec![0.,0.];let mut b=a.clone();a[fixed]=end as f64;b[fixed]=a[fixed];
            a[1-fixed]=if backward{1.}else{0.};b[1-fixed]=1.-a[1-fixed];
            let p=Curve::from_polyline(vec![a,b]).unwrap();
            let mut c=Curve{degree:2,knots:if fixed==0{s.knots_v.clone()}else{s.knots_u.clone()},
                control_points:(0..4).map(|i|if fixed==0{s.control_points[end][i].clone()}else{s.control_points[i][end].clone()}).collect(),
                weights:(0..4).map(|i|if fixed==0{s.weights[end][i]}else{s.weights[i][end]}).collect(),periodic:false};
            if backward!=reversed{c=c.reverse().unwrap();}
            assert_eq!(verify_exact(&c,&p,&s,reversed,1_000_000).unwrap().unwrap().outcome,BezierIdentity::Equal);
            let limited=verify_exact(&c,&p,&s,reversed,1).unwrap().unwrap();
            assert!(matches!(limited.outcome,BezierIdentity::Indeterminate(_)));assert!(limited.work_used<=1);
            let mut bad=c.clone();bad.knots[3]=bad.knots[3].next_up();
            assert!(verify_exact(&bad,&p,&s,reversed,1_000_000).unwrap().is_none());
            let mut bad=c.clone();bad.control_points[1][2]+=1e-12;
            assert!(verify_exact(&bad,&p,&s,reversed,1_000_000).unwrap().is_none());
        }}}}
    }
    #[test]
    fn rounded_complement_is_not_an_exact_nurbs_basis_reversal() {
        let s=Surface{degree_u:1,degree_v:2,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,0.,0.1,1.,1.,1.],
            control_points:(0..2).map(|u|[0.,1.,3.,5.].iter().map(|z|vec![u as f64,0.,*z]).collect()).collect(),
            weights:vec![vec![1.,0.75,1.5,1.];2],periodic_u:false,periodic_v:false};
        let p=Curve::from_polyline(vec![vec![0.,0.],vec![0.,1.]]).unwrap();
        let c=Curve{degree:2,knots:s.knots_v.clone(),control_points:s.control_points[0].clone(),weights:s.weights[0].clone(),periodic:false}.reverse().unwrap();
        assert_eq!(c.knots[3]+s.knots_v[3],1.);
        assert!(verify_exact(&c,&p,&s,true,1_000_000).unwrap().is_none());
    }
    #[test]
    fn formal_composition_does_not_claim_full_chart_membership() {
        use cad_predicates::BezierIdentity;
        let s=Surface {degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
            control_points:vec![vec![vec![0.,0.,0.],vec![0.,1.,0.]],vec![vec![1.,0.,0.],vec![1.,1.,1.]]],
            weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false};
        let mut p=Curve::from_polyline(vec![vec![-0.25,0.5],vec![1.25,0.5]]).unwrap();
        p.weights[1]=0.75;
        let mut c=Curve::from_polyline(vec![vec![-0.25,0.5,-0.125],vec![1.25,0.5,0.625]]).unwrap();
        c.weights=p.weights.clone();
        assert!(verify_exact(&c,&p,&s,false,1_000_000).unwrap().is_none());
        assert_eq!(verify_exact_algebraic(&c,&p,&s,false,1_000_000).unwrap().unwrap().outcome,BezierIdentity::Equal);
        c.control_points.reverse();c.weights.reverse();
        assert_eq!(verify_exact_algebraic(&c,&p,&s,true,1_000_000).unwrap().unwrap().outcome,BezierIdentity::Equal);
        c.control_points[0][2]+=1e-12;
        assert_eq!(verify_exact_algebraic(&c,&p,&s,true,1_000_000).unwrap().unwrap().outcome,BezierIdentity::Different);
        assert!(matches!(verify_exact_algebraic(&c,&p,&s,true,0).unwrap().unwrap().outcome,BezierIdentity::Indeterminate(_)));
    }
    #[test]
    fn tensor_boundary_identity_preserves_domains_reversal_weights_and_limits() {
        use cad_predicates::BezierIdentity;
        let surface=Surface {
            degree_u:2,degree_v:1,knots_u:vec![2.,2.,2.,5.,5.,5.],knots_v:vec![-7.,-7.,11.,11.],
            control_points:(0..3).map(|i|(0..2).map(|j|vec![i as f64,j as f64,(10+i*i+j*3) as f64]).collect()).collect(),
            weights:vec![vec![1.,3.],vec![2.,5.],vec![4.,7.]],periodic_u:false,periodic_v:false,
        };
        for axis in 0..2 {for side in 0..2 {for backward in [false,true] {for reversed in [false,true] {
            let domain=[[2.,5.],[-7.,11.]];let fixed=1-axis;
            let mut a=[0.;2];let mut b=[0.;2];a[fixed]=domain[fixed][side];b[fixed]=a[fixed];
            a[axis]=domain[axis][usize::from(backward)];b[axis]=domain[axis][usize::from(!backward)];
            let p=Curve {degree:1,knots:vec![-9.,-9.,-3.,-3.],control_points:vec![a.to_vec(),b.to_vec()],weights:vec![2.,2.],periodic:false};
            let indices=if axis==0 {(0..3).map(|i|(i,side)).collect::<Vec<_>>()}else{(0..2).map(|j|(side*2,j)).collect()};
            let degree=indices.len()-1;
            let mut c=Curve {degree,knots:vec![5.;degree+1].into_iter().chain(vec![11.;degree+1]).collect(),
                control_points:indices.iter().map(|&(i,j)|surface.control_points[i][j].clone()).collect(),
                weights:indices.iter().map(|&(i,j)|surface.weights[i][j]).collect(),periodic:false};
            if backward!=reversed {c.control_points.reverse();c.weights.reverse();}
            let r=verify_exact(&c,&p,&surface,reversed,256).unwrap().unwrap();
            assert_eq!(r.outcome,BezierIdentity::Equal);assert!(r.work_used>0 && r.work_used<256);
            assert_eq!(verify_exact(&c,&p,&surface,reversed,r.work_used).unwrap().unwrap().outcome,BezierIdentity::Equal);
            assert!(matches!(verify_exact(&c,&p,&surface,reversed,r.work_used-1).unwrap().unwrap().outcome,BezierIdentity::Indeterminate(_)));
            let mut changed=c.clone();changed.control_points[0][2]=changed.control_points[0][2].next_up();
            assert_eq!(verify_exact(&changed,&p,&surface,reversed,1000000).unwrap().unwrap().outcome,BezierIdentity::Different);
            let mut scaled=c;for w in &mut scaled.weights {*w*=2.;}
            assert_eq!(verify_exact(&scaled,&p,&surface,reversed,1000000).unwrap().unwrap().outcome,BezierIdentity::Equal);
        }}}}
        let c=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],control_points:surface.control_points.iter().map(|row|row[0].clone()).collect(),weights:vec![1.,2.,4.],periodic:false};
        let p=Curve {degree:1,knots:vec![0.,0.,1.,1.],control_points:vec![vec![2.,-7.],vec![5.,-7.]],weights:vec![1.,1.],periodic:false};
        let mut subnormal=c.clone();subnormal.control_points[0][0]=f64::from_bits(1);
        assert_ne!(verify_exact(&subnormal,&p,&surface,false,1000000).unwrap().unwrap().outcome,BezierIdentity::Equal);
        let mut nonlinear=p.clone();nonlinear.weights[1]=2.;
        assert_eq!(verify_exact(&c,&nonlinear,&surface,false,1000000).unwrap().unwrap().outcome,BezierIdentity::Different);
        let mut interior=p.clone();for q in &mut interior.control_points {q[1]=(-7_f64).next_up();}
        assert_eq!(verify_exact(&c,&interior,&surface,false,1000000).unwrap().unwrap().outcome,BezierIdentity::Different);
        let mut partial=p;partial.control_points[0][0]=2.5;
        assert_eq!(verify_exact(&c,&partial,&surface,false,1000000).unwrap().unwrap().outcome,BezierIdentity::Different);
    }
    #[test]
    fn affine_chart_identity_uses_exact_poles_and_bounded_work() {
        use cad_predicates::BezierIdentity;
        let s=Surface{degree_u:1,degree_v:1,knots_u:vec![2.,2.,4.,4.],knots_v:vec![3.,3.,7.,7.],
            control_points:vec![vec![vec![0.,0.,10.],vec![0.,4.,8.]],vec![vec![2.,0.,10.],vec![2.,4.,8.]]],
            weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false};
        let p=Curve{degree:2,knots:vec![0.,0.,0.,1.,1.,1.],control_points:vec![vec![2.,3.],vec![3.,5.],vec![4.,7.]],weights:vec![1.,0.5,1.],periodic:false};
        let c=Curve{control_points:vec![vec![0.,0.,10.],vec![1.,2.,9.],vec![2.,4.,8.]],..p.clone()};
        let report=verify_exact(&c,&p,&s,false,1000000).unwrap().unwrap();
        assert_eq!(report.outcome,BezierIdentity::Equal);
        assert_eq!(verify_exact(&c,&p,&s,false,report.work_used).unwrap().unwrap().outcome,BezierIdentity::Equal);
        assert!(matches!(verify_exact(&c,&p,&s,false,0).unwrap().unwrap().outcome,BezierIdentity::Indeterminate(_)));
        let mut changed=c;changed.control_points[1][2]=9_f64.next_up();
        assert_eq!(verify_exact(&changed,&p,&s,false,1000000).unwrap().unwrap().outcome,BezierIdentity::Different);
    }
    #[test]
    fn constant_coordinates_are_exact_with_independent_positive_rational_weights() {
        use cad_predicates::BezierIdentity;
        let c=Curve{degree:1,knots:vec![0.,0.,1.,1.],control_points:vec![vec![10.,20.,30.];2],weights:vec![1.,2.],periodic:false};
        let p=Curve{degree:1,knots:vec![0.,0.,1.,1.],control_points:vec![vec![0.,0.],vec![1.,1.]],weights:vec![1.,3.],periodic:false};
        let mut s=plane();s.control_points=vec![vec![vec![10.,20.,30.];2];2];s.weights=vec![vec![1.,2.],vec![3.,4.]];
        assert_eq!(verify_exact(&c,&p,&s,false,1000000).unwrap().unwrap().outcome,BezierIdentity::Equal);
        let mut changed=c.clone();changed.control_points[0][0]=10_f64.next_up();
        assert_eq!(verify_exact(&changed,&p,&s,false,1000000).unwrap().unwrap().outcome,BezierIdentity::Different);
        s.control_points[0][0][0]=10_f64.next_up();
        assert_eq!(verify_exact(&c,&p,&s,false,1000000).unwrap().unwrap().outcome,BezierIdentity::Different);
        assert!(matches!(verify_exact(&c,&p,&s,false,0).unwrap().unwrap().outcome,BezierIdentity::Indeterminate(_)));
    }
    #[cfg(feature = "transport")]
    #[test]
    fn sweep_coedge_transport_retains_zero_budget_and_mismatch() {
        let world=Curve{degree:1,knots:vec![0.,0.,1.,1.],control_points:vec![vec![0.,0.,0.],vec![1.,0.,0.]],weights:vec![1.;2],periodic:false};
        let uv=Curve{degree:1,knots:vec![0.,0.,1.,1.],control_points:vec![vec![0.,0.],vec![1.,0.]],weights:vec![1.;2],periodic:false};
        let mut request=value_codec::json!({"op":"sweep_coedge_agreement_audit","world":world,"uv":uv,"surface":plane(),"reversed":false,"tolerance":1e-9,"maxCells":100});
        let mut exact=request.clone();
        exact["op"]=value_codec::json!("sweep_coedge_exact_audit");
        exact["maxWork"]=value_codec::json!(100000);
        assert_eq!(crate::transport::dispatch(exact.clone()).unwrap()["status"],"equal");
        exact["maxWork"]=value_codec::json!(0);
        assert_eq!(crate::transport::dispatch(exact.clone()).unwrap()["status"],"unresolved");
        exact["maxWork"]=value_codec::json!(100000);
        exact["world"]["controlPoints"][0][2]=value_codec::json!(1e-12);
        assert_eq!(crate::transport::dispatch(exact).unwrap()["status"],"different");
        let positive=crate::transport::dispatch(request.clone()).unwrap();
        assert_eq!(positive["withinTolerance"],true);
        assert_eq!(positive["exactIdentityCertified"],false);
        request["maxCells"]=value_codec::json!(0);
        let zero=crate::transport::dispatch(request.clone()).unwrap();
        assert_eq!(zero["status"],"unresolved");assert_eq!(zero["cells"],0);
        request["maxCells"]=value_codec::json!(100);
        request["world"]["controlPoints"][0][2]=value_codec::json!(1.);
        let mismatch=crate::transport::dispatch(request).unwrap();
        assert_eq!(mismatch["status"],"mismatch");
        assert!(!mismatch["witnessDistance"].is_null());
    }
    fn plane() -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }
    fn line(points: Vec<Vec<f64>>) -> Curve {
        Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            weights: vec![1.; 2],
            control_points: points,
            periodic: false,
        }
    }
    #[test]
    fn exact_identity_leaves_multispan_definitions_unproven() {
        let c=Curve {degree:1,knots:vec![0.,0.,0.5,1.,1.],
            control_points:vec![vec![0.,0.,0.],vec![0.5,0.5,0.],vec![1.,1.,0.]],
            weights:vec![1.;3],periodic:false};
        let p=line(vec![vec![0.,0.],vec![1.,1.]]);
        assert!(verify_exact(&c,&p,&plane(),false,1000000).unwrap().is_none());
        assert_eq!(verify(&c,&p,&plane(),false,1e-6,10000).unwrap().status,Status::WithinTolerance);
    }
    #[test]
    fn exact_identity_is_distinct_from_tolerance_and_preserves_reversal() {
        use cad_predicates::BezierIdentity;
        let p=line(vec![vec![0.,0.],vec![1.,1.]]);
        let mut c=line(vec![vec![0.,0.,0.],vec![1.,1.,0.]]);
        assert_eq!(verify_exact(&c,&p,&plane(),false,1000000).unwrap().unwrap().outcome,BezierIdentity::Equal);
        c.control_points.reverse();
        assert_eq!(verify_exact(&c,&p,&plane(),true,1000000).unwrap().unwrap().outcome,BezierIdentity::Equal);
        c.control_points[0][2]=1e-12;
        assert_eq!(verify(&c,&p,&plane(),true,1e-6,1000).unwrap().status,Status::WithinTolerance);
        assert_eq!(verify_exact(&c,&p,&plane(),true,1000000).unwrap().unwrap().outcome,BezierIdentity::Different);
        assert!(matches!(verify_exact(&c,&p,&plane(),true,0).unwrap().unwrap().outcome,BezierIdentity::Indeterminate(_)));
        let outside=line(vec![vec![-1.,0.],vec![1.,1.]]);
        assert!(verify_exact(&c,&outside,&plane(),false,1000000).unwrap().is_none());
    }
    #[test]
    fn natural_rational_boundary_keeps_direction_and_parameterization() {
        use cad_predicates::BezierIdentity;
        let edge=Curve{degree:2,knots:vec![0.,0.,0.,1.,1.,1.],control_points:vec![vec![0.,0.,0.],vec![1.,2.,0.],vec![3.,0.,0.]],weights:vec![1.,0.75,1.],periodic:false};
        let s=Surface{degree_u:1,degree_v:2,knots_u:vec![2.,2.,8.,8.],knots_v:vec![-2.,-2.,-2.,3.,3.,3.],control_points:vec![edge.control_points.clone(),edge.control_points.iter().map(|p|vec![p[0],p[1],1.]).collect()],weights:vec![edge.weights.clone(),edge.weights.clone()],periodic_u:false,periodic_v:false};
        let p=line(vec![vec![2.,-2.],vec![2.,3.]]);
        for (c,p,reversed) in [(edge.clone(),p.clone(),false),(edge.reverse().unwrap(),p.clone(),true),(edge.reverse().unwrap(),p.reverse().unwrap(),false)] {
            assert_eq!(verify_exact(&c,&p,&s,reversed,10000).unwrap().unwrap().outcome,BezierIdentity::Equal);
        }
        let mut nonlinear=p.clone();nonlinear.weights[1]=2.;
        assert_eq!(verify_exact(&edge,&nonlinear,&s,false,1_000_000).unwrap().unwrap().outcome,BezierIdentity::Different);
        let mut shifted=edge.clone();shifted.control_points[1][2]=1e-12;
        assert_eq!(verify_exact(&shifted,&p,&s,false,10000).unwrap().unwrap().outcome,BezierIdentity::Different);
    }
    #[test]
    fn identity_plane_preserves_rational_boundaries_on_nonunit_domains() {
        use cad_predicates::BezierIdentity;
        let mut s=plane();s.knots_u=vec![-20.,-20.,20.,20.];s.knots_v=s.knots_u.clone();
        s.control_points=vec![vec![vec![-20.,-20.,6.],vec![-20.,20.,6.]],vec![vec![20.,-20.,6.],vec![20.,20.,6.]]];
        let p=Curve{degree:2,knots:vec![0.,0.,0.,1.,1.,1.],control_points:vec![vec![-10.,0.],vec![0.,10.],vec![10.,0.]],weights:vec![1.,0.75,1.],periodic:false};
        let c=Curve{control_points:p.control_points.iter().map(|p|vec![p[0],p[1],6.]).collect(),..p.clone()};
        let r=verify_exact(&c,&p,&s,false,10000).unwrap().unwrap();assert_eq!(r.outcome,BezierIdentity::Equal);assert!(r.work_used<10000);
        assert_eq!(verify_exact(&c.reverse().unwrap(),&p,&s,true,10000).unwrap().unwrap().outcome,BezierIdentity::Equal);
        let mut changed=c.clone();changed.control_points[1][2]+=1e-12;
        assert_eq!(verify_exact(&changed,&p,&s,false,10000).unwrap().unwrap().outcome,BezierIdentity::Different);
        let mut changed_surface=s.clone();changed_surface.control_points[1][1][2]+=1e-12;
        assert_eq!(verify_exact(&c,&p,&changed_surface,false,1_000_000).unwrap().unwrap().outcome,BezierIdentity::Different);
    }
    #[test]
    fn matching_reversed_and_exhausted() {
        let c = line(vec![vec![0., 0., 0.], vec![1., 0., 0.]]);
        let p = line(vec![vec![0., 0.], vec![1., 0.]]);
        assert_eq!(
            verify(&c, &p, &plane(), false, 0.01, 1024).unwrap().status,
            Status::WithinTolerance
        );
        assert_eq!(
            verify(&c, &p, &plane(), false, 1e-9, 1).unwrap().status,
            Status::WithinTolerance
        );
        let mut reversed = c.clone();
        reversed.control_points.reverse();
        assert_eq!(
            verify(&reversed, &p, &plane(), true, 0.01, 1024)
                .unwrap()
                .status,
            Status::WithinTolerance
        );
        assert_eq!(
            verify(&reversed, &p, &plane(), false, 0.01, 1024)
                .unwrap()
                .status,
            Status::Mismatch
        );
    }
    #[test]
    fn rational_multispan_and_nonbinary_domains() {
        let c = Curve {
            degree: 1,
            knots: vec![0.1, 0.1, 0.37, 0.9, 0.9],
            weights: vec![1., 2., 0.5],
            control_points: vec![vec![0.1, 0.2, 0.], vec![0.4, 0.7, 0.], vec![0.8, 0.3, 0.]],
            periodic: false,
        };
        let mut p = c.clone();
        for cp in &mut p.control_points {
            cp.pop();
        }
        assert_eq!(
            verify(&c, &p, &plane(), false, 1e-9, 1).unwrap().status,
            Status::Unresolved
        );
        assert_eq!(
            verify(&c, &p, &plane(), false, 1e-9, 4096).unwrap().status,
            Status::WithinTolerance
        );
    }
    #[test]
    fn pcurve_crosses_surface_knots_at_strict_tolerance() {
        let mut surface = plane();
        surface.knots_u = vec![0., 0., 0.3, 1., 1.];
        surface.control_points = vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![0.3, 0., 0.], vec![0.3, 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ];
        surface.weights = vec![vec![1.; 2]; 3];
        let c = Curve {
            degree: 1,
            knots: vec![0.1, 0.1, 0.37, 0.9, 0.9],
            weights: vec![1., 2., 0.5],
            control_points: vec![vec![0.1, 0.2, 0.], vec![0.4, 0.7, 0.], vec![0.8, 0.3, 0.]],
            periodic: false,
        };
        let mut p = c.clone();
        for cp in &mut p.control_points {
            cp.pop();
        }
        let report = verify(&c, &p, &surface, false, 1e-9, 4096).unwrap();
        assert_eq!(report.status, Status::WithinTolerance);
        assert!(report.cells < 4096);
    }

    #[test]
    fn rational_curved_support_is_verified_without_planar_assumption() {
        let weights = vec![1., 0.5_f64.sqrt(), 1.];
        let c = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![1., 0., 0.5], vec![1., 1., 0.5], vec![0., 1., 0.5]],
            weights: weights.clone(),
            periodic: false,
        };
        let s = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: c.knots.clone(),
            knots_v: vec![0., 0., 1., 1.],
            control_points: c
                .control_points
                .iter()
                .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 1.]])
                .collect(),
            weights: weights.into_iter().map(|w| vec![w, w]).collect(),
            periodic_u: false,
            periodic_v: false,
        };
        let p = line(vec![vec![0., 0.5], vec![1., 0.5]]);
        assert_eq!(
            verify(&c, &p, &s, false, 1e-9, 1).unwrap().status,
            Status::WithinTolerance
        );
    }

    #[test]
    fn rational_pcurve_composition_preserves_the_shared_parameter() {
        let mut s = plane();
        s.control_points[1][1][2] = 1.; // S(u,v)=(u,v,uv)
        let mut p = line(vec![vec![0.25, 0.25], vec![0.75, 0.75]]);
        p.weights = vec![1., 2.];
        // Homogeneous Bernstein product: U*W, U*W, U*U, W*W.
        let mut c = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            periodic: false,
            control_points: vec![
                vec![0.25, 0.25, 0.0625],
                vec![0.5, 0.5, 0.1875],
                vec![0.75, 0.75, 0.5625],
            ],
            weights: vec![1., 2., 4.],
        };
        let report = verify(&c, &p, &s, false, 1e-9, 1).unwrap();
        assert_eq!(report.status, Status::WithinTolerance);
        assert_eq!(report.cells, 1);
        c.control_points.reverse();
        c.weights.reverse();
        assert_eq!(
            verify(&c, &p, &s, true, 1e-9, 1).unwrap().status,
            Status::WithinTolerance
        );
        c.control_points[1][2] += 0.01;
        assert_eq!(
            verify(&c, &p, &s, true, 1e-9, 100).unwrap().status,
            Status::Mismatch
        );
    }

    #[test]
    fn periodic_lift_crosses_seam_and_negative_periods() {
        let surface = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![-1., 0., 1., 2., 3., 4.],
            knots_v: vec![0., 0., 1., 1.],
            weights: vec![vec![1.; 2]; 4],
            control_points: [[0., 0.], [1., 0.], [0., 1.], [0., 0.]]
                .into_iter()
                .map(|xy| vec![vec![xy[0], xy[1], 0.], vec![xy[0], xy[1], 1.]])
                .collect(),
            periodic_u: true,
            periodic_v: false,
        };
        let mut c = Curve {
            degree: 1,
            knots: vec![0., 0., 0.5, 1., 1.],
            weights: vec![1.; 3],
            control_points: vec![vec![0., 0.5, 0.5], vec![0., 0., 0.5], vec![0.5, 0., 0.5]],
            periodic: false,
        };
        for shift in [-6., 0., 6.] {
            let p = line(vec![vec![2.5 + shift, 0.5], vec![3.5 + shift, 0.5]]);
            let report = verify(&c, &p, &surface, false, 1e-9, 4096).unwrap();
            assert_eq!(
                report.status,
                Status::WithinTolerance,
                "shift={shift}, cells={}",
                report.cells
            );
        }
        let p = line(vec![vec![2.5, 0.5], vec![3.5, 0.5]]);
        c.control_points[1][2] += 0.01;
        let report = verify(&c, &p, &surface, false, 1e-9, 4096).unwrap();
        assert_eq!(report.status, Status::Mismatch);
        assert!(report.witness_distance.unwrap()[0] > 1e-9);
    }

    #[test]
    fn both_periodic_axes_cross_seams_together() {
        let f = [0., 1., 0.5, 0.];
        let surface = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![-1., 0., 1., 2., 3., 4.],
            knots_v: vec![-1., 0., 1., 2., 3., 4.],
            weights: vec![vec![1.; 4]; 4],
            control_points: f
                .iter()
                .map(|&x| f.iter().map(|&y| vec![x, y, x * y]).collect())
                .collect(),
            periodic_u: true,
            periodic_v: true,
        };
        let c = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
            weights: vec![1.; 5],
            periodic: false,
            control_points: vec![
                vec![0.25, 0.25, 0.0625],
                vec![0.125, 0.125, 0.],
                vec![0., 0., 0.],
                vec![0.25, 0.25, 0.],
                vec![0.5, 0.5, 0.25],
            ],
        };
        let p = line(vec![vec![-0.5, 5.5], vec![0.5, 6.5]]);
        assert_eq!(
            verify(&c, &p, &surface, false, 1e-9, 4096).unwrap().status,
            Status::WithinTolerance
        );
    }

    #[test]
    fn rational_quadratic_periodic_surface_seam() {
        let xy = [[1., 0.], [0., 1.], [-1., 0.], [0., -1.], [1., 0.], [0., 1.]];
        let weights = [1., 2., 1., 2., 1., 2.];
        let surface = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![-2., -1., 0., 1., 2., 3., 4., 5., 6.],
            knots_v: vec![0., 0., 1., 1.],
            weights: weights.iter().map(|&w| vec![w, w]).collect(),
            control_points: xy
                .iter()
                .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 1.]])
                .collect(),
            periodic_u: true,
            periodic_v: false,
        };
        fn average(a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
            std::array::from_fn(|k| (a[k] + b[k]) * 0.5)
        }
        let h: Vec<[f64; 4]> = xy
            .iter()
            .zip(weights)
            .map(|(p, w)| [p[0] * w, p[1] * w, 0.5 * w, w])
            .collect();
        let left = [average(h[3], h[4]), h[4], average(h[4], h[5])];
        let right = [average(h[0], h[1]), h[1], average(h[1], h[2])];
        let controls = [
            average(average(left[0], left[1]), average(left[1], left[2])),
            average(left[1], left[2]),
            left[2],
            average(right[0], right[1]),
            average(average(right[0], right[1]), average(right[1], right[2])),
        ];
        let c = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
            weights: controls.iter().map(|p| p[3]).collect(),
            control_points: controls
                .iter()
                .map(|p| (0..3).map(|k| p[k] / p[3]).collect())
                .collect(),
            periodic: false,
        };
        let p = line(vec![vec![3.5, 0.5], vec![4.5, 0.5]]);
        assert_eq!(
            verify(&c, &p, &surface, false, 1e-9, 4096).unwrap().status,
            Status::WithinTolerance
        );
    }

    #[test]
    fn departure_has_enclosed_witness_and_invalid_uv_is_unresolved() {
        let c = line(vec![vec![0., 0., 0.1], vec![1., 0., 0.1]]);
        let p = line(vec![vec![0., 0.], vec![1., 0.]]);
        let r = verify(&c, &p, &plane(), false, 1e-6, 100).unwrap();
        assert_eq!(r.status, Status::Mismatch);
        let [lo, hi] = r.witness_distance.unwrap();
        assert!(lo <= 0.1 && hi >= 0.1 && lo > 1e-6);
        let outside = line(vec![vec![-1., 0.], vec![-0.5, 0.]]);
        assert_eq!(
            verify(&c, &outside, &plane(), false, 1e-6, 100)
                .unwrap()
                .status,
            Status::Unresolved
        );
        assert!(verify(&c, &p, &plane(), false, 0., 100).is_err());
    }
    #[test]
    fn unchanged_original_pcurve_interval_preserves_nonunit_and_reversed_mapping() {
        let mut pc=line(vec![vec![0.,0.4],vec![1.,0.4]]);
        pc.knots=vec![2.,2.,8.,8.];
        let before=pc.clone();
        let mut world=line(vec![vec![0.75,0.4,0.],vec![0.25,0.4,0.]]);
        world.knots=vec![10.,10.,20.,20.];
        assert_eq!(verify_on(&world,&pc,&plane(),true,[3.5,6.5],1e-9,128).unwrap().status,Status::WithinTolerance);
        assert_eq!(verify(&world,&pc,&plane(),true,1e-9,128).unwrap().status,Status::Mismatch);
        assert!(verify_on(&world,&pc,&plane(),true,[1.,6.5],1e-9,128).is_err());
        assert_eq!(pc,before);
    }
    #[test]
    fn rational_subinterval_world_proposal_is_checked_against_original_definition() {
        let pc=Curve {degree:2,knots:vec![2.,2.,2.,8.,8.,8.],
            control_points:vec![vec![0.,0.25],vec![0.5,0.4],vec![1.,0.7]],weights:vec![1.,0.75,1.],periodic:false};
        let mut world=pc.clone();for p in &mut world.control_points{p.push(0.);}
        let piece=world.trim(3.5,6.5).unwrap();
        assert_eq!(verify_on(&piece,&pc,&plane(),false,[3.5,6.5],1e-9,128).unwrap().status,Status::WithinTolerance);
    }

    #[test]
    fn degree_five_rational_plane_composition_is_bounded_and_detects_damage() {
        let p=Curve {degree:5,periodic:false,knots:[vec![0.;6],vec![1.;6]].concat(),
            control_points:vec![vec![0.,0.],vec![0.125,0.0625],vec![0.25,0.125],vec![0.5,0.75],vec![0.875,0.9375],vec![1.,1.]],
            weights:vec![1.,1.5,0.5,2.,0.75,1.]};
        let mut c=p.clone();c.control_points=p.control_points.iter().map(|q|vec![q[0],q[1],0.]).collect();
        let report=verify(&c,&p,&plane(),false,1e-7,1024).unwrap();
        assert_eq!(report.status,Status::WithinTolerance);assert_eq!(report.cells,1);
        c.control_points[2][2]=0.1;
        assert_eq!(verify(&c,&p,&plane(),false,1e-7,1024).unwrap().status,Status::Mismatch);
    }
