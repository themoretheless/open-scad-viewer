use nurbs_core::{curve::Curve,biarc::fit_biares_report};
#[test]
fn repeated_circle_travel_does_not_accept_only_a_short_supporting_arc() {
    // Five exact rational quarter circles: 450 degrees of forward travel.
    let points=[[1.,0.],[1.,1.],[0.,1.],[-1.,1.],[-1.,0.],[-1.,-1.],[0.,-1.],[1.,-1.],[1.,0.],[1.,1.],[0.,1.]];
    let c=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,2.,2.,3.,3.,4.,4.,5.,5.,5.],
        control_points:points.iter().map(|p|p.to_vec()).collect(),weights:(0..11).map(|i|if i%2==0 {1.} else {std::f64::consts::FRAC_1_SQRT_2}).collect(),periodic:false};
    let r=fit_biares_report(&c,1e-5,256).unwrap();
    for q in [[-1.,0.],[0.,-1.]] {
        let mut nearest=f64::INFINITY;
        for a in &r.as_nurbs {
            let [lo,hi]=a.domain();
            for i in 0..=2048 {
                let p=a.evaluate(lo+(hi-lo)*i as f64/2048.).unwrap().point;
                nearest=nearest.min((p[0]-q[0]).hypot(p[1]-q[1]));
            }
        }
        assert!(nearest<0.002,"fitted finite arcs omit source point {q:?}: gap={nearest}, pairs={}",r.arcs.len());
    }
}
#[test]
fn collinear_overshoot_is_not_hidden_by_an_infinite_support_line() {
    let c=Curve {degree:3,knots:vec![0.,0.,0.,0.,1.,1.,1.,1.],control_points:vec![vec![0.,0.],vec![4.,0.],vec![-3.,0.],vec![1.,0.]],weights:vec![1.;4],periodic:false};
    assert!(fit_biares_report(&c,1e-6,2).is_err());
}
#[test]
fn exact_semicircle_has_correct_radius_endpoints_and_g1_joints() {
    let c=nurbs_core::primitives::circle_arc([0.;3],[0.,0.,1.],2.,0.,180.).unwrap();
    let r=fit_biares_report(&c,1e-4,256).unwrap();
    assert!(r.max_error<=1e-4);assert!(r.as_nurbs.len()<=256);
    for a in &r.as_nurbs {
        let [lo,hi]=a.domain();
        for i in 0..=20 {
            let p=a.evaluate(lo+(hi-lo)*i as f64/20.).unwrap().point;
            assert!((p[0].hypot(p[1])-2.).abs()<1e-9 && p[2].abs()<1e-12);
        }
    }
    for pair in r.as_nurbs.windows(2) {
        let a=pair[0].evaluate(pair[0].domain()[1]).unwrap();
        let b=pair[1].evaluate(pair[1].domain()[0]).unwrap();
        assert!(a.point.iter().zip(&b.point).all(|(a,b)|(a-b).abs()<1e-10));
        let a=a.d1.unwrap();let b=b.d1.unwrap();
        let dot:f64=a.iter().zip(&b).map(|(a,b)|a*b).sum();
        let norm=|v:&[f64]|v.iter().map(|x|x*x).sum::<f64>().sqrt();
        assert!(dot/(norm(&a)*norm(&b))>1.-1e-10);
    }
}
#[test]
fn straight_finite_segments_preserve_the_analytic_line() {
    let c=Curve::from_polyline(vec![vec![1.,2.],vec![4.,6.]]).unwrap();
    let r=fit_biares_report(&c,1e-6,2).unwrap();
    assert_eq!(r.as_nurbs.len(),2);
    for a in r.as_nurbs {
        for i in 0..=10 {
            let p=a.evaluate(i as f64/10.).unwrap().point;
            assert!((4.*(p[0]-1.)-3.*(p[1]-2.)).abs()<1e-12);
            assert!(p[0]>=1.-1e-12 && p[0]<=4.+1e-12);
        }
    }
}
#[test]
fn nonplanar_and_invalid_requests_are_refused() {
    let line=Curve::from_polyline(vec![vec![0.,0.],vec![1.,0.]]).unwrap();
    for t in [0.,-1.,f64::NAN,f64::INFINITY] {assert!(fit_biares_report(&line,t,256).is_err());}
    for count in [0,257,usize::MAX] {assert!(fit_biares_report(&line,1e-3,count).is_err());}
    let c=Curve {degree:3,knots:vec![0.,0.,0.,0.,1.,1.,1.,1.],control_points:vec![vec![0.,0.,0.],vec![1.,0.,1.],vec![1.,1.,-1.],vec![2.,2.,0.]],weights:vec![1.;4],periodic:false};
    assert!(fit_biares_report(&c,1e-5,256).is_err());
}
