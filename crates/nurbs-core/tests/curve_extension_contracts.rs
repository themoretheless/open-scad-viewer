use nurbs_core::{curve::Curve, curve_extension::{extend_curve, End, Continuity}};
fn source() -> Curve {
    Curve { degree: 3, knots: vec![0.,0.,0.,0.,0.4,1.,1.,1.,1.],
        control_points: vec![vec![0.,0.,0.],vec![1.,2.,0.3],vec![2.5,-1.,0.1],vec![3.7,1.4,-0.5],vec![5.,0.2,0.]],
        weights: vec![1.,0.8,1.3,1.,1.], periodic: false }
}
fn near(a: &[f64], b: &[f64], tolerance: f64) {
    assert_eq!(a.len(),b.len());
    for (a,b) in a.iter().zip(b) { assert!((a-b).abs()<tolerance,"{a} vs {b}"); }
}
#[test]
fn multispan_extension_preserves_original_geometry_at_both_ends() {
    let c=source(); c.validate().unwrap();
    for end in [End::Stop,End::Start] {
        let r=extend_curve(&c,end,0.1,Continuity::C2).unwrap();
        for i in 0..=100 {
            let t=i as f64/100.;
            near(&c.evaluate(t).unwrap().point,&r.curve.evaluate(t).unwrap().point,2e-12);
        }
        let joint=if end==End::Start {0.} else {1.};
        let a=c.evaluate(joint).unwrap(); let b=r.curve.evaluate(joint).unwrap();
        near(&a.d1.unwrap(),&b.d1.unwrap(),2e-10);
        near(&a.d2.unwrap(),&b.d2.unwrap(),2e-8);
    }
}

#[test]
fn line_extension_retains_the_original_parameter_values() {
    let c=Curve::from_polyline(vec![vec![0.,0.],vec![2.,0.]]).unwrap();
    for end in [End::Start,End::Stop] {
        let r=extend_curve(&c,end,0.5,Continuity::C2).unwrap();
        let [lo,hi]=r.curve.domain();
        assert!(lo<=0. && hi>=1.);
        let t=if end==End::Start {lo} else {hi};
        near(&r.curve.evaluate(t).unwrap().point,&[2.*t,0.],1e-12);
        near(&r.curve.evaluate(0.5).unwrap().point,&[1.,0.],1e-12);
        assert_eq!(r.achieved_continuity,Continuity::C2);
    }
}

#[test]
fn multiple_terminal_knots_are_not_inserted_again() {
    let c=source().insert(0.4,2).unwrap();
    let r=extend_curve(&c,End::Stop,0.05,Continuity::G2).unwrap();
    assert_eq!(c.control_points.len(),r.curve.control_points.len());
    for i in 0..=50 {
        let t=i as f64/50.;
        near(&c.evaluate(t).unwrap().point,&r.curve.evaluate(t).unwrap().point,2e-12);
    }
}

#[test]
fn invalid_nonclamped_and_unresolvable_steps_are_refused() {
    let c=source();
    for length in [0.,-1.,f64::NAN,f64::INFINITY,1e300,f64::MIN_POSITIVE] {
        assert!(extend_curve(&c,End::Stop,length,Continuity::G1).is_err());
    }
    let mut unclamped=c.clone();
    unclamped.knots=vec![-3.,-2.,-1.,0.,0.4,1.,2.,3.,4.];
    unclamped.validate().unwrap();
    assert!(extend_curve(&unclamped,End::Stop,0.1,Continuity::G1).is_err());
    let mut invalid=c.clone(); invalid.weights[0]=f64::NAN;
    assert!(extend_curve(&invalid,End::Stop,0.1,Continuity::G1).is_err());
}

#[test]
fn terminal_extraction_respects_the_existing_control_budget() {
    let n=256;
    let c=Curve { degree:3,knots:[vec![0.;4],(1..253).map(|i| i as f64/253.).collect(),vec![1.;4]].concat(),
        control_points:(0..n).map(|i| vec![i as f64,(i as f64).sin()]).collect(),weights:vec![1.;n],periodic:false };
    c.validate().unwrap();
    assert!(extend_curve(&c,End::Stop,0.01,Continuity::G1).is_err());
}

#[test]
fn rational_circle_continuation_retains_the_analytic_circle() {
    let c=nurbs_core::primitives::circle_arc([0.;3],[0.,0.,1.],2.,0.,60.).unwrap();
    for end in [End::Start,End::Stop] {
        let r=extend_curve(&c,end,0.2,Continuity::G2).unwrap();
        let [lo,hi]=r.curve.domain();
        for i in 0..=100 {
            let p=r.curve.evaluate(lo+(hi-lo)*i as f64/100.).unwrap().point;
            assert!((p[0].hypot(p[1])-2.).abs()<2e-12);
            assert_eq!(p[2],0.);
        }
    }
}

#[test]
fn high_degree_growth_is_reported_without_a_false_stability_claim() {
    let c=Curve {degree:5,knots:[vec![0.;6],vec![1.;6]].concat(),
        control_points:(0..=5).map(|i| vec![i as f64/5.,if i%2==0 {0.5} else {-0.5},0.]).collect(),weights:vec![1.;6],periodic:false};
    let r=extend_curve(&c,End::Stop,20.,Continuity::C2).unwrap();
    assert!(!r.stable);
    assert!(r.extrapolation_growth>nurbs_core::curve_extension::STABILITY_GROWTH_LIMIT);
}
