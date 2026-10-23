    use super::*;
    #[test]
    fn polar_jacobian_bounds_contain_independent_rational_coordinate_derivatives() {
        // x=2, y=2u/(1+u), z=-3v/(1+2v), from separable weights.
        let s = Surface {
            degree_u: 1, degree_v: 1,
            knots_u: vec![0.,0.,1.,1.], knots_v: vec![0.,0.,1.,1.],
            control_points: (0..2).map(|i| (0..2).map(|j| vec![2.,i as f64,-(j as f64)]).collect()).collect(),
            weights: vec![vec![1.,3.],vec![2.,6.]], periodic_u: false, periodic_v: false,
        };
        let coordinates=[[1.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,1.,0.]];
        for i in 0..4 { for j in 0..4 {
            let domain=[[i as f64/4.,(i+1) as f64/4.],[j as f64/4.,(j+1) as f64/4.]];
            let bounds=polar_section_jacobian(&s,[1,1],domain,coordinates).unwrap().unwrap();
            for u in domain[0] { for v in domain[1] {
                let y=2.*u/(1.+u);let dy=2./(1.+u).powi(2);
                let expected=[[2.*dy/(4.+y*y),0.],[y*dy/(4.+y*y).sqrt(),3./(1.+2.*v).powi(2)]];
                for row in 0..2 { for axis in 0..2 {
                    let b=bounds[row][axis];let value=expected[row][axis];
                    assert!(b.lo<=value && value<=b.hi,"{row}/{axis}: {b:?} excludes {value}");
                }}
            }}
        }}
    }
    #[test]
    fn polar_projection_requires_global_coverage_continuity_and_two_independent_parameters() {
        let mut s = Surface {
            degree_u: 1, degree_v: 1,
            knots_u: vec![0.,0.,1.,1.], knots_v: vec![0.,0.,1.,1.],
            control_points: (0..2).map(|i| (0..2).map(|j| vec![2.,i as f64,-(j as f64)]).collect()).collect(),
            weights: vec![vec![1.;2];2], periodic_u: false, periodic_v: false,
        };
        let coordinates = [[1.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,1.,0.]];
        let before = format!("{s:?}");
        assert!(certify_polar_projection(&s,coordinates,4,16).unwrap().is_some());
        assert!(certify_polar_projection(&s,coordinates,4,15).unwrap().is_none());
        assert!(certify_polar_projection(&s,coordinates,0,16).is_err());
        assert!(certify_polar_projection(&s,coordinates,4,0).is_err());
        let mut invalid = coordinates; invalid[0][0] = f64::NAN;
        assert!(certify_polar_projection(&s,invalid,4,16).is_err());
        let mut wrong_branch = coordinates; wrong_branch[0][0] = -1.;
        assert!(certify_polar_projection(&s,wrong_branch,4,16).unwrap().is_none());
        assert_eq!(format!("{s:?}"),before);
        s.control_points[1] = s.control_points[0].clone();
        assert!(certify_polar_projection(&s,coordinates,4,16).unwrap().is_none());
        let row = |y| vec![vec![2.,y,0.],vec![2.,y,-1.]];
        s.control_points = vec![row(0.),row(1.),row(0.)];
        s.weights = vec![vec![1.;2];3]; s.knots_u = vec![0.,0.,0.5,1.,1.];
        // Locally invertible halves overlap. No collection of local proofs
        // may be promoted to a proof of this complete folded chart.
        assert!(certify_polar_projection(&s,coordinates,4,32).unwrap().is_none());
        s.control_points = vec![row(0.),row(1.),row(0.),row(1.)];
        s.weights = vec![vec![1.;2];4]; s.knots_u = vec![0.,0.,0.5,0.5,1.,1.];
        assert!(certify_polar_projection(&s,coordinates,4,32).is_err());
    }

    fn graph() -> Surface {
        Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: (0..3)
                .map(|i| {
                    (0..3)
                        .map(|j| vec![i as f64 / 2., j as f64 / 2., (i * j) as f64])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 3]; 3],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn perspective_projection_requires_a_nonzero_denominator_and_full_global_coverage(){
        let basis=[[1.,0.,0.,0.],[0.,1.,0.,0.]];
        let mut s=graph();let before=format!("{s:?}");
        assert!(certify_projective_projection(&s,basis,[0.,0.,0.,1.],16).unwrap().is_some());
        assert!(certify_projective_projection(&s,basis,[0.,0.,0.,1.],15).unwrap().is_none());
        for d in [[0.,0.,0.,0.],[0.,0.,0.,-1.],[1.,0.,0.,-0.5],[1.,0.,0.,0.]]{
            assert!(certify_projective_projection(&s,basis,d,16).unwrap().is_none());
        }
        assert_eq!(format!("{s:?}"),before);
        assert!(certify_projective_projection(&s,basis,[0.,0.,0.,1.],0).is_err());
        assert!(certify_projective_projection(&s,[[f64::NAN,0.,0.,0.],basis[1]],[0.,0.,0.,1.],16).is_err());
        s.degree_u=1;s.knots_u=vec![0.,0.,0.5,1.,1.];
        assert!(certify_projective_projection(&s,basis,[0.,0.,0.,1.],32).unwrap().is_some());
        assert!(certify_projective_projection(&s,basis,[0.,0.,0.,1.],31).unwrap().is_none());
        for p in &mut s.control_points[2]{p[0]=0.;}
        // Each side is locally invertible, but the complete folded chart isn't.
        assert!(certify_projective_projection(&s,basis,[0.,0.,0.,1.],32).unwrap().is_none());
        s.periodic_u=true;
        s.knots_u=vec![-1.,0.,1.,2.,3.];
        s.control_points[2]=s.control_points[0].clone();s.weights[2]=s.weights[0].clone();
        assert!(certify_projective_projection(&s,basis,[0.,0.,0.,1.],32).unwrap().is_none());
    }
    #[test]
    fn diagonal_projection_certifies_quarter_cylinder_and_refuses_partial_coverage(){
        let mut s=Surface{degree_u:2,degree_v:1,knots_u:vec![0.,0.,0.,1.,1.,1.],knots_v:vec![0.,0.,1.,1.],
            control_points:vec![vec![vec![2.,0.,0.],vec![2.,0.,4.]],vec![vec![2.,2.,0.],vec![2.,2.,4.]],vec![vec![0.,2.,0.],vec![0.,2.,4.]]],
            weights:vec![vec![1.;2],vec![std::f64::consts::FRAC_1_SQRT_2;2],vec![1.;2]],periodic_u:false,periodic_v:false};
        let basis=[[-1.,1.,0.],[0.,0.,1.]];
        assert!(!certify(&s,10).unwrap().proven);
        let q=certify_linear_projection(&s,basis,16).unwrap();assert!(q.is_some(),"{q:?}");
        assert!(certify_linear_projection(&s,basis,15).unwrap().is_none());
        for row in &mut s.control_points{for p in row{p[2]=0.;}}
        assert!(certify_linear_projection(&s,basis,16).unwrap().is_none());
    }
    #[test]
    fn normal_bounds_cover_graph_derivatives_and_do_not_collapse_at_points() {
        // S(u,v)=(u,v,4uv), hence (Su cross Sv).d = -4v*dx-4u*dy+dz.
        let s=graph();
        for domain in [[[0.,1.],[0.,1.]],[[0.2,0.4],[0.3,0.7]],[[0.25,0.25],[0.5,0.5]]] {
            for direction in [[0.,0.,1.],[0.3,-0.2,0.7],[0.,0.,-1.]] {
                let b=normal_direction_bounds(&s,domain,direction,10).unwrap().unwrap();
                for u in domain[0] {for v in domain[1] {
                    let value=-4.*v*direction[0]-4.*u*direction[1]+direction[2];
                    assert!(b[0]<=value && value<=b[1], "{b:?}: {value}");
                }}
                if direction[2]==1. {assert!(b[0]>0.);}
                if direction[2]==-1. {assert!(b[1]<0.);}
            }
        }
    }
    #[test]
    fn rational_normal_bounds_contain_an_independent_separable_formula() {
        let s=Surface {degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
            control_points:vec![vec![vec![0.,0.,0.],vec![0.,1.,0.]],vec![vec![1.,0.,0.],vec![1.,1.,0.]]],
            weights:vec![vec![1.,3.],vec![2.,6.]],periodic_u:false,periodic_v:false};
        // x=2u/(1+u), y=3v/(1+2v); normal Z is dx/du * dy/dv.
        for i in 0..10 {for j in 0..10 {
            let domain=[[i as f64/10.,(i+1) as f64/10.],[j as f64/10.,(j+1) as f64/10.]];
            let b=normal_direction_bounds(&s,domain,[0.,0.,1.],1).unwrap().unwrap();
            for u in domain[0] {for v in domain[1] {
                let z=6./((1.+u).powi(2)*(1.+2.*v).powi(2));
                assert!(b[0]<=z && z<=b[1]);
            }}
        }}
    }
    #[test]
    fn normal_bounds_keep_both_knot_sides_and_refuse_partial_coverage() {
        let mut s=graph();s.degree_u=1;s.knots_u=vec![0.,0.,0.5,1.,1.];
        for p in &mut s.control_points[2] {p[0]=0.;}
        let d=[[0.5,0.5],[0.3,0.7]];
        assert!(normal_direction_bounds(&s,d,[0.,0.,1.],1).unwrap().is_none());
        let b=normal_direction_bounds(&s,d,[0.,0.,1.],2).unwrap().unwrap();
        assert!(b[0]<0. && b[1]>0.);
        assert!(normal_direction_bounds(&s,[[-1.,0.],[0.,1.]],[0.,0.,1.],2).is_err());
    }
    #[test]
    fn nonlinear_graph_and_vertical_rotation_are_globally_injective() {
        let mut s = graph();
        let r = certify(&s, 10).unwrap();
        assert!(r.proven);
        assert_eq!(r.projection, Some([0, 1]));
        assert!(r.contraction_upper.unwrap() < 1e-10);
        for row in &mut s.control_points {
            for p in row {
                p.swap(1, 2);
            }
        }
        assert!(certify(&s, 10).unwrap().proven);
    }
    #[test]
    fn folded_and_collapsed_charts_never_claim_injectivity() {
        let mut s = graph();
        for (i, row) in s.control_points.iter_mut().enumerate() {
            for p in row {
                p[0] = [1., -1., 1.][i];
                p[2] = 0.;
            }
        }
        assert!(!certify(&s, 10).unwrap().proven);
        for row in &mut s.control_points {
            for p in row {
                p[0] = 0.;
            }
        }
        assert!(!certify(&s, 10).unwrap().proven);
    }
    #[test]
    fn clamped_pole_reports_missing_quotient_proof_without_spending_budget() {
        let mut s = graph();
        let pole = s.control_points[0][0].clone();
        for p in &mut s.control_points[0] { *p = pole.clone(); }
        for (j, w) in s.weights[0].iter_mut().enumerate() { *w = 1. + j as f64; }
        let result = certify(&s, 10).unwrap();
        assert!(!result.proven);
        assert_eq!(result.spans, 0);
        assert_eq!(result.reason, "collapsed-boundary-requires-quotient-proof");
        // Close but distinct controls cannot be classified as an exact pole.
        s.control_points[0][1][0] += 1e-10;
        assert_ne!(certify(&s, 10).unwrap().reason,
            "collapsed-boundary-requires-quotient-proof");
    }
    #[test]
    fn all_spans_must_pass_the_same_global_test() {
        let mut s = graph();
        s.degree_u = 1;
        s.knots_u = vec![0., 0., 0.3, 1., 1.];
        let partial = certify(&s, 1).unwrap();
        assert!(!partial.proven);
        assert_eq!(partial.reason, "work-limit");
        let r = certify(&s, 2).unwrap();
        assert!(r.proven);
        assert_eq!(r.spans, 2);
        // Both individual spans are regular; their union folds back on itself.
        for p in &mut s.control_points[2] {
            p[0] = 0.;
            p[2] = 0.;
        }
        for p in &mut s.control_points[1] {
            p[2] = 0.;
        }
        assert!(!certify(&s, 2).unwrap().proven);
    }
