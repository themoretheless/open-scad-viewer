use nurbs_core::optical_shapes::{dome,lens};
#[test]
fn domes_follow_independent_sphere_equation_and_exact_pole_rim() {
    for h in [0.2,1.,2.,4.] {
        let center=[3.,-2.,5.];let s=dome(center,[2.,3.],h).unwrap();
        let radius=(4.+h*h)/(2.*h);let zc=5.+h-radius;
        assert!(s.control_points[0].iter().all(|p|p==&vec![3.,-2.,5.+h]));
        assert!(s.control_points.last().unwrap().iter().all(|p|p[2]==5.));
        for i in 0..=19 {for j in 0..=23 {
            let u=i as f64/19.;let v=4.*j as f64/23.;let p=s.evaluate(u,v).unwrap().point;
            let residual=(p[0]-3.).powi(2)+((p[1]+2.)*2./3.).powi(2)+(p[2]-zc).powi(2)-radius*radius;
            assert!(residual.abs()<1e-11,"h={h} residual={residual}");
            if i==19 {assert!((p[2]-5.).abs()<1e-12);}
        }}
        assert!(s.control_points.iter().all(|r|r.first()==r.last()));
    }
}
#[test]
fn convex_lens_caps_have_exact_shared_rims_and_barrel_boundaries() {
    for (sags,t) in [([1.,0.5],2.),([1.,0.5],1.5),([0.,1.],2.),([0.,0.],2.)] {
        let l=lens([2.,-3.,7.],[3.,4.],sags,t).unwrap();
        assert_eq!(l.rim_thickness,t-sags[0]-sags[1]);
        let upper=l.upper.control_points.last().unwrap();let lower=l.lower.control_points.last().unwrap();
        if let Some(b)=&l.barrel {
            assert_eq!(b.control_points.first(),Some(lower));assert_eq!(b.control_points.last(),Some(upper));
            assert_eq!(b.weights[0],*l.lower.weights.last().unwrap());assert_eq!(b.weights[1],*l.upper.weights.last().unwrap());
        } else {assert_eq!(upper,lower);}
        for j in 0..=16 {
            let v=4.*j as f64/16.;
            assert!((l.upper.evaluate(0.,v).unwrap().point[2]-(7.+t/2.)).abs()<1e-12);
            assert!((l.lower.evaluate(0.,v).unwrap().point[2]-(7.-t/2.)).abs()<1e-12);
            for i in 0..=9 {
                let u=i as f64/9.;let a=l.upper.evaluate(u,v).unwrap().point;let b=l.lower.evaluate(u,v).unwrap().point;
                assert!(a[2]>=upper[0][2]-1e-12 && b[2]<=lower[0][2]+1e-12);
                assert!(a[2]>=b[2]-1e-12);
            }
        }
    }
}
#[test]
fn invalid_dimensions_and_intersecting_or_major_lens_caps_are_rejected() {
    for h in [0.,-1.,f64::NAN,f64::INFINITY] {assert!(dome([0.;3],[2.,3.],h).is_err());}
    for axes in [[0.,2.],[-1.,2.],[1.,f64::INFINITY]] {assert!(dome([0.;3],axes,1.).is_err());}
    assert!(dome([1e100,0.,0.],[2.,3.],1.).is_err());
    assert!(dome([0.,0.,1e100],[2.,3.],1.).is_err());
    for (sags,t) in [([1.,1.],1.),([-1.,1.],2.),([3.,0.],4.),([0.,0.],0.),([f64::NAN,0.],1.)] {
        assert!(lens([0.;3],[2.,3.],sags,t).is_err());
    }
    assert!(lens([0.,0.,1e100],[2.,3.],[0.,0.],1.).is_err());
}
