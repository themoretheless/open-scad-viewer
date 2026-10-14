use nurbs_core::{curve::Curve,primitives,profile_spring::{self,Request,Stop}};
fn profile()->Curve {let mut p=primitives::ellipse_arc([0.;3],[0.2,0.,0.],[0.,0.1,0.],0.,360.).unwrap();for c in &mut p.control_points {c.pop();}p}
fn request(turns:f64)->Request {Request {center:[2.,-3.,5.],radius:2.,height:3.,turns,phase_degrees:23.,tolerance:1e-4,max_spans:256}}
#[test]
fn rational_profile_transport_matches_independent_helix_normal_frame() {
    let p=profile();let before=p.clone();
    for (turns,height) in [(-2.5,3.),(2.5,3.),(2.5,-3.),(2.5,0.)] {
        let mut r=request(turns);r.height=height;
        let mut input=p.clone();if height==0. {for c in &mut input.control_points {c.push(0.);}}
        let report=profile_spring::approximate(&input,r).unwrap();assert_eq!(report.stop,Stop::EstimateReached);
        let w=std::f64::consts::TAU*turns;let speed=(r.radius*w).hypot(r.height);
        for patch in &report.patches {
            let lo=patch.knots_v[3];let hi=patch.knots_v[patch.control_points[0].len()];
            for i in 0..=7 {for j in 0..=9 {
                let u=i as f64/7.;let t=lo+(hi-lo)*j as f64/9.;let q=p.evaluate(u).unwrap().point;
                let (s,c)=(r.phase_degrees.to_radians()+w*t).sin_cos();
                let ideal=[2.+(2.+q[0])*c-r.height/speed*q[1]*s,-3.+(2.+q[0])*s+r.height/speed*q[1]*c,5.+r.height*t-2.*w/speed*q[1]];
                let actual=patch.evaluate(u,t).unwrap().point;
                let error=(actual[0]-ideal[0]).hypot(actual[1]-ideal[1]).hypot(actual[2]-ideal[2]);
                assert!(error<=report.real_arithmetic_error_estimate+1e-11,"{error}");
                if j==0 || j==9 {
                    let delta=[actual[0]-2.-2.*c,actual[1]+3.-2.*s,actual[2]-5.-r.height*t];
                    let tangent=[-2.*w*s/speed,2.*w*c/speed,r.height/speed];
                    let normal_component=(0..3).map(|k|delta[k]*tangent[k]).sum::<f64>();
                    assert!(normal_component.abs()<1e-10);
                    assert!((delta[0].hypot(delta[1]).hypot(delta[2])-q[0].hypot(q[1])).abs()<1e-10);
                }
            }}
        }
        for pair in report.patches.windows(2) {
            assert_eq!(pair[0].knots_v.last(),pair[1].knots_v.first());
            for (a,b) in pair[0].control_points.iter().zip(&pair[1].control_points) {assert_eq!(a.last(),b.first());}
        }
    }
    assert_eq!(p,before);
}
#[test]
fn budget_returns_all_turns_and_refinement_improves_error_estimate() {
    let p=profile();let mut r=request(4.);r.max_spans=1;
    let coarse=profile_spring::approximate(&p,r).unwrap();assert_eq!(coarse.stop,Stop::SpanBudget);assert_eq!(coarse.spans,1);
    assert_eq!(*coarse.patches[0].knots_v.last().unwrap(),1.);
    r.max_spans=23;let fine=profile_spring::approximate(&p,r).unwrap();assert_eq!(fine.spans,23);assert_eq!(fine.patches.len(),3);
    assert!(fine.real_arithmetic_error_estimate<coarse.real_arithmetic_error_estimate);
    assert_eq!(*fine.patches.last().unwrap().knots_v.last().unwrap(),1.);
}
#[test]
fn invalid_requests_and_nonplanar_local_profiles_are_rejected() {
    let p=profile();for radius in [0.,-1.,f64::NAN] {let mut r=request(1.);r.radius=radius;assert!(profile_spring::approximate(&p,r).is_err());}
    for turns in [0.,f64::INFINITY] {assert!(profile_spring::approximate(&p,request(turns)).is_err());}
    for budget in [0,1025] {let mut r=request(1.);r.max_spans=budget;assert!(profile_spring::approximate(&p,r).is_err());}
    let mut r=request(1.);r.tolerance=0.;assert!(profile_spring::approximate(&p,r).is_err());
    let mut bad=p;for c in &mut bad.control_points {c.push(0.1);}assert!(profile_spring::approximate(&bad,request(1.)).is_err());
}
