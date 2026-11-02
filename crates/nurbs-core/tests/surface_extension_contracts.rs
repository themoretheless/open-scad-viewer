use nurbs_core::{curve::Curve, curve_extension::{End,Continuity}, surface::{self,Axis}, editing::surface_extension::extend_surface};
fn near(a:[f64;3],b:[f64;3]) {
    for (a,b) in a.into_iter().zip(b) {assert!((a-b).abs()<2e-11,"{a} vs {b}");}
}
#[test]
fn plane_extensions_follow_the_analytic_parameterization_for_all_four_ends() {
    let s=nurbs_core::patches::plane([1.,2.,3.],[2.,0.,0.],[0.,3.,0.]).unwrap();
    for axis in [Axis::U,Axis::V] {for end in [End::Start,End::Stop] {
        let r=extend_surface(&s,axis,end,0.5,Continuity::C2).unwrap();
        let t=&r.surface;
        let u=[t.knots_u[t.degree_u],t.knots_u[t.control_points.len()]];
        let v=[t.knots_v[t.degree_v],t.knots_v[t.control_points[0].len()]];
        for i in 0..=10 { for j in 0..=10 {
            let x=u[0]+(u[1]-u[0])*i as f64/10.;let y=v[0]+(v[1]-v[0])*j as f64/10.;
            near(t.evaluate(x,y).unwrap().point,[1.+2.*x,2.+3.*y,3.]);
        }}
        assert!(r.stable);
        assert_eq!(r.achieved_continuity,Continuity::C2);
    }}
}
#[test]
fn multispan_rational_extrusion_keeps_the_old_parameter_rectangle() {
    let c=Curve {degree:3,knots:vec![0.,0.,0.,0.,0.4,1.,1.,1.,1.],
        control_points:vec![vec![0.,0.,0.],vec![1.,2.,0.],vec![2.5,-1.,0.],vec![3.7,1.4,0.],vec![5.,0.2,0.]],
        weights:vec![1.,0.8,1.3,1.,1.],periodic:false};
    let s=surface::extrude(&c,[0.,0.,2.]).unwrap();
    for end in [End::Start,End::Stop] {
        let r=extend_surface(&s,Axis::U,end,0.05,Continuity::G2).unwrap();
        for i in 0..=30 { for j in 0..=5 {
            let u=i as f64/30.;let v=j as f64/5.;
            near(s.evaluate(u,v).unwrap().point,r.surface.evaluate(u,v).unwrap().point);
        }}
    }
}
#[test]
fn inconsistent_line_domains_and_invalid_requests_are_refused() {
    let mut s=nurbs_core::patches::plane([0.;3],[1.,0.,0.],[0.,1.,0.]).unwrap();
    s.control_points[1][1][0]=2.;
    s.validate().unwrap();
    assert!(extend_surface(&s,Axis::U,End::Stop,0.5,Continuity::G1).is_err());
    let s=nurbs_core::patches::plane([0.;3],[1.,0.,0.],[0.,1.,0.]).unwrap();
    for length in [0.,-1.,f64::NAN,f64::INFINITY] {
        assert!(extend_surface(&s,Axis::U,End::Stop,length,Continuity::G1).is_err());
    }
    let mut bad=s;bad.weights.clear();
    assert!(extend_surface(&bad,Axis::V,End::Start,0.5,Continuity::G1).is_err());
}
