use nurbs_core::{construct::curves::airfoil::naca5_airfoil, curve::Curve};
fn y_at_x(curve:&Curve,x:f64,upper:bool)->f64 {
    let (mut lo,mut hi)=if upper {(0.,0.5)}else{(0.5,1.)};
    for _ in 0..64 {
        let t=(lo+hi)*0.5;let p=curve.evaluate(t).unwrap().point;
        if (p[0]>x)==upper {lo=t;}else{hi=t;}
    }
    curve.evaluate((lo+hi)*0.5).unwrap().point[1]
}
#[test]
fn encoded_naca23012_has_the_published_mean_line_not_two_percent_camber() {
    // NACA Report 537, Table I and the nose mean-line equation (pp.521-522).
    // These constants describe the nominal published shape, not aerodynamic qualification.
    let curve=naca5_airfoil(23012,1.,true,128,3,32).unwrap();
    let x=0.15;let m=0.2025;let k1=15.957;
    let expected=k1/6.*(x*x*x-3.*m*x*x+m*m*(3.-m)*x);
    let actual=(y_at_x(&curve,x,true)+y_at_x(&curve,x,false))*0.5;
    assert!((actual-expected).abs()<2e-4,"encoded mean line {actual} vs published nominal {expected}");
}
#[test]
fn all_published_nonreflex_mean_lines_and_lift_scalings_match_their_equations() {
    use nurbs_core::construct::curves::airfoil::naca5_section_at;
    let table=[(1,0.058,361.4),(2,0.126,51.64),(3,0.2025,15.957),(4,0.29,6.643),(5,0.391,3.230)];
    for (p,m,k1) in table {for lift in [1,2,4] {for x in [0.,0.02,0.15,0.4,0.8,1.] {
        let k1=k1*lift as f64/2.;
        let (yc,slope)=if x<m {(k1/6.*(x*x*x-3.*m*x*x+m*m*(3.-m)*x),k1/6.*(3.*x*x-6.*m*x+m*m*(3.-m)))}else{(k1*m*m*m/6.*(1.-x),-k1*m*m*m/6.)};
        let yt=0.6*(0.2969*x.sqrt()-0.126*x-0.3516*x*x+0.2843*x*x*x-0.1036*x*x*x*x);
        let angle=slope.atan();let chord=2.5;
        let expected=[[chord*(x-yt*angle.sin()),chord*(yc+yt*angle.cos()),0.],[chord*(x+yt*angle.sin()),chord*(yc-yt*angle.cos()),0.]];
        let code=lift*10000+p*1000+12;
        let (upper,lower)=naca5_section_at(code,x,chord,true).unwrap();
        for (a,b) in upper.into_iter().chain(lower).zip(expected.into_iter().flatten()) {assert!((a-b).abs()<2e-14);}
    }}}
}
#[test]
fn unsupported_codes_and_invalid_station_chord_and_fit_limits_are_refused() {
    use nurbs_core::construct::curves::airfoil::naca5_section_at;
    for code in [0,9999,100000,23112,26012,23000,23099] {
        assert!(naca5_section_at(code,0.4,1.,true).is_err());
        assert!(naca5_airfoil(code,1.,true,64,3,16).is_err());
    }
    for x in [f64::NAN,f64::INFINITY,-0.1,1.1] {assert!(naca5_section_at(23012,x,1.,true).is_err());}
    for chord in [f64::NAN,f64::INFINITY,0.,-1.,1e7] {assert!(naca5_section_at(23012,0.4,chord,true).is_err());}
    for (samples,degree,controls) in [(7,3,16),(2048,3,16),(64,0,16),(64,26,32),(64,3,65)] {
        assert!(naca5_airfoil(23012,1.,true,samples,degree,controls).is_err());
    }
}
#[test]
fn custom_max_camber_solver_remains_available_and_distinct_from_standard_code() {
    use nurbs_core::construct::curves::airfoil::{naca5,naca5_camber,naca5_camber_line,naca5_section_at};
    let custom=naca5_camber(0.15,0.02).unwrap();
    assert!((naca5_camber_line(0.15,&custom).0-0.02).abs()<1e-12);
    naca5(0.15,0.02,0.12,1.,true,64,3,16).unwrap();
    let (a,b)=naca5_section_at(23012,0.15,1.,true).unwrap();
    assert!(((a[1]+b[1])*0.5-0.02).abs()>0.001);
}
#[test]
fn open_and_closed_trailing_edges_preserve_the_declared_thickness_convention() {
    use nurbs_core::construct::curves::airfoil::naca5_section_at;
    let (u,l)=naca5_section_at(23012,1.,2.5,false).unwrap();
    let gap=(u[0]-l[0]).hypot(u[1]-l[1]);
    assert!((gap-2.5*0.00252).abs()<1e-14);
    let (u,l)=naca5_section_at(23012,1.,2.5,true).unwrap();
    assert!((u[0]-l[0]).hypot(u[1]-l[1])<1e-14);
}
