use nurbs_core::{bellows::{self,Request},surface::Surface};
fn request(periods:usize)->Request {Request {center:[2.,-3.,5.],mean_radius:4.,amplitude:0.6,height:12.,radial_thickness:0.4,periods}}
fn domain(s:&Surface)->[f64;2] {[s.knots_u[s.degree_u],s.knots_u[s.control_points.len()]]}
#[test]
fn corrugated_radii_and_axial_positions_match_independent_wave_formula() {
    for amplitude in [0.6,-0.6,0.] {
        let mut r=request(7);r.amplitude=amplitude;let shell=bellows::cubic(r).unwrap();assert_eq!(shell.outer.len(),2);
        for (outer,inner) in shell.outer.iter().zip(&shell.inner) {let d=domain(outer);
            for i in 0..=19 {let u=d[0]+(d[1]-d[0])*i as f64/19.;let scaled=u*(2*r.periods) as f64;let segment=(scaled.floor() as usize).min(2*r.periods-1);let t=scaled-segment as f64;
                let sign=if segment%2==0 {1.} else {-1.};let radius=r.mean_radius+sign*amplitude*(1.-6.*t*t+4.*t*t*t);
                for j in 0..=17 {let v=4.*j as f64/17.;let a=outer.evaluate(u,v).unwrap().point;let b=inner.evaluate(u,4.-v).unwrap().point;
                    assert!(((a[0]-2.).hypot(a[1]+3.)-radius).abs()<1e-10);
                    assert!(((b[0]-2.).hypot(b[1]+3.)-(radius-r.radial_thickness)).abs()<1e-10);
                    assert!((a[2]-(5.+12.*u)).abs()<1e-10 && (b[2]-a[2]).abs()<1e-10);
                }
            }
        }
    }
}
#[test]
fn all_skin_and_rim_boundaries_reuse_exact_control_data() {
    let s=bellows::cubic(request(12)).unwrap();
    for skin in [&s.outer,&s.inner] {
        for pair in skin.windows(2) {assert_eq!(pair[0].control_points.last(),pair[1].control_points.first());assert_eq!(pair[0].weights.last(),pair[1].weights.first());assert_eq!(pair[0].knots_u.last(),pair[1].knots_u.first());}
        for p in skin {for row in &p.control_points {assert_eq!(row.first(),row.last());}}
    }
    let rev=|r:&Vec<Vec<f64>>|r.iter().rev().cloned().collect::<Vec<_>>();
    assert_eq!(s.bottom.control_points[0],rev(&s.inner[0].control_points[0]));assert_eq!(s.bottom.control_points[1],s.outer[0].control_points[0]);
    assert_eq!(s.top.control_points[0],*s.inner.last().unwrap().control_points.last().unwrap());assert_eq!(s.top.control_points[1],rev(s.outer.last().unwrap().control_points.last().unwrap()));
    let reverse_weights=|w:&Vec<f64>|w.iter().rev().copied().collect::<Vec<_>>();
    assert_eq!(s.bottom.weights[0],reverse_weights(&s.inner[0].weights[0]));assert_eq!(s.bottom.weights[1],s.outer[0].weights[0]);
    assert_eq!(s.top.weights[0],*s.inner.last().unwrap().weights.last().unwrap());assert_eq!(s.top.weights[1],reverse_weights(s.outer.last().unwrap().weights.last().unwrap()));
    for p in s.outer.iter().chain(&s.inner) {assert_eq!(p.knots_v,s.bottom.knots_v);assert_eq!(p.knots_v,s.top.knots_v);}
}
#[test]
fn surface_normals_follow_outer_cavity_and_end_rim_orientation() {
    let s=bellows::cubic(request(7)).unwrap();
    for (skin,sign) in [(&s.outer,1.),(&s.inner,-1.)] {for p in skin {
        let d=domain(p);let e=p.evaluate(d[0]+(d[1]-d[0])*0.371,0.37).unwrap();let n=e.unit_normal().unwrap();
        assert!(sign*(n[0]*(e.point[0]-2.)+n[1]*(e.point[1]+3.))>0.);
    }}
    assert!(s.bottom.evaluate(0.5,0.37).unwrap().unit_normal().unwrap()[2]< -0.99);
    assert!(s.top.evaluate(0.5,0.37).unwrap().unit_normal().unwrap()[2]>0.99);
}
#[test]
fn invalid_sizes_and_unrepresentable_walls_or_axial_steps_are_refused() {
    for periods in [0,257] {assert!(bellows::cubic(request(periods)).is_err());}
    let mut r=request(1);r.radial_thickness=3.5;assert!(bellows::cubic(r).is_err());
    r=request(1);r.radial_thickness=0.;assert!(bellows::cubic(r).is_err());
    r=request(1);r.height=f64::NAN;assert!(bellows::cubic(r).is_err());
    r=request(256);r.center[2]=1e8;r.height=1e-8;assert!(bellows::cubic(r).is_err());
    r=request(1);r.center[0]=1e8;r.radial_thickness=1e-10;assert!(bellows::cubic(r).is_err());
}
#[test]
fn maximum_period_budget_preserves_complete_axial_domain_and_wave_frequency() {
    let r=request(256);let s=bellows::cubic(r).unwrap();
    assert_eq!(domain(&s.outer[0])[0],0.);assert_eq!(domain(s.outer.last().unwrap())[1],1.);
    assert_eq!(s.outer.len(),s.inner.len());
    assert!(s.outer.iter().chain(&s.inner).all(|p|p.control_points.len()<=32&&p.control_points[0].len()==9));
    let u=0.1234;let patch=s.outer.iter().find(|p| {let d=domain(p);d[0]<=u&&u<=d[1]}).unwrap();
    let scaled=u*512.;let segment=scaled.floor() as usize;let t=scaled-segment as f64;
    let sign=if segment%2==0 {1.} else {-1.};let expected=4.+sign*0.6*(1.-6.*t*t+4.*t*t*t);
    let p=patch.evaluate(u,0.37).unwrap().point;
    assert!(((p[0]-2.).hypot(p[1]+3.)-expected).abs()<1e-10);
}
