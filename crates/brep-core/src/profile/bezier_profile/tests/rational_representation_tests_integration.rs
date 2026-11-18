    use super::*;
    fn retained_cubic()->Curve {
        // Exact binary64 source: H(t)=3(1+t)*(10t,5t²,1).
        Curve{degree:3,knots:vec![0.,0.,0.,0.,1.,1.,1.,1.],
            control_points:vec![vec![0.,0.],vec![2.5,0.],vec![6.,1.],vec![10.,5.]],
            weights:vec![3.,4.,5.,6.],periodic:false}
    }
    fn region(c:Curve)->Vec<Curve> {
        vec![Curve::from_polyline(vec![vec![0.,0.],vec![10.,0.]]).unwrap(),
            Curve::from_polyline(vec![vec![10.,0.],vec![10.,5.]]).unwrap(),c.reverse().unwrap()]
    }
    #[test]
    fn rational_cubic_common_factor_keeps_source_and_extrudes() {
        let c=retained_cubic();let wire=region(c.clone());let before=wire.clone();
        assert!((validate_single_wire(&wire,1e-7).unwrap()-50./3.).abs()<1e-6);
        let certificate=crate::profile_certificate::certify(&[wire.clone()],1e-7).unwrap();
        crate::profile_certificate::verify(&[wire.clone()],1e-7,&certificate).unwrap();
        let solid=crate::prism::extrude(&[wire.clone()],0.,3.).unwrap();
        solid.validate().unwrap();
        assert!(solid.edges.iter().any(|e|e.curve.degree==3&&e.curve.weights.iter().any(|w|*w!=e.curve.weights[0])));
        assert_eq!(wire,before);
        let result=boolean(&[wire.clone()],&[wire.clone()],"intersection",1e-7).unwrap();
        assert!((validate_profile(&result,1e-7).unwrap()-50./3.).abs()<1e-6);
        assert!(boolean(&[wire.clone()],&[wire],"difference",1e-7).unwrap().is_empty());
    }
    #[test]
    fn polynomial_quadratic_profile_has_exact_green_integral() {
        let c=Curve{degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:vec![vec![0.,0.],vec![5.,0.],vec![10.,5.]],
            weights:vec![1.;3],periodic:false};
        let wire=region(c.clone());
        assert!(requires_general(&c,1e-7));
        assert!((validate_single_wire(&wire,1e-7).unwrap()-50./3.).abs()<1e-12);
        crate::prism::extrude(&[wire.clone()],0.,3.).unwrap().validate().unwrap();
        // Different degrees and a nonconstant common homogeneous factor must
        // still describe the same retained boundary in Boolean operations.
        let rational=region(retained_cubic());
        assert!(boolean(&[wire.clone()],&[rational.clone()],"difference",1e-7).unwrap().is_empty());
        let union=boolean(&[wire],&[rational],"union",1e-7).unwrap();
        assert!((validate_profile(&union,1e-7).unwrap()-50./3.).abs()<1e-6);
    }
    #[test]
    fn rational_green_bounds_enclose_an_independent_polynomial_integral() {
        let c=retained_cubic();
        let bounds=area_ops::integral(&c,[0.,0.],1e-8).unwrap();
        // Open parabola contribution: 1/2 integral (10t*10t-5t²*10)dt=25/3.
        assert!(bounds.lo<=25./3.&&25./3.<=bounds.hi);
        assert!(bounds.hi-bounds.lo<2.1e-8);
    }
