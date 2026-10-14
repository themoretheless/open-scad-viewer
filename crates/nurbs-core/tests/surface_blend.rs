use nurbs_core::{surface::Surface,surface_blend::{self,Options,Order}};
fn support(offset:f64,degree:usize,unclamped:bool)->Surface {
    let n=if unclamped {4} else {degree+1};let nv=if unclamped {4} else {3};
    let points=(0..n).map(|i|(0..nv).map(|j|vec![i as f64/(n-1) as f64,offset+j as f64/(nv-1) as f64,0.1*i as f64*j as f64+0.07*(j*j) as f64]).collect()).collect();
    let ku=if unclamped {vec![-2.,-1.,0.,1.,2.,3.,4.]} else {std::iter::repeat_n(2.,degree+1).chain(std::iter::repeat_n(7.,degree+1)).collect()};
    let kv=if unclamped {vec![-2.,-1.,0.,1.,2.,3.,4.]} else {vec![-3.,-3.,-3.,1.,1.,1.]};
    let s=Surface {degree_u:degree,degree_v:2,knots_u:ku,knots_v:kv,control_points:points,weights:(0..n).map(|i|(0..nv).map(|j|(1.+0.2*i as f64)*(1.+0.1*j as f64)).collect()).collect(),periodic_u:false,periodic_v:false};s.validate().unwrap();s
}
fn domain(s:&Surface)->[[f64;2];2] {[[s.knots_u[s.degree_u],s.knots_u[s.control_points.len()]],[s.knots_v[s.degree_v],s.knots_v[s.control_points[0].len()]]]}
fn close(a:[f64;3],b:[f64;3],t:f64) {for k in 0..3 {assert!((a[k]-b[k]).abs()<t,"{a:?} != {b:?}");}}
#[test]
fn rational_c1_and_c2_bridges_match_complete_source_jets_with_affine_parameters() {
    let a=support(0.,2,false);let b=support(3.,3,false);let before=(a.clone(),b.clone());
    for order in [Order::C1,Order::C2] {
        let o=Options {gains:[0.8,0.6],reverse_u:[false,true],..Default::default()};
        let r=if order==Order::C1 {surface_blend::g1(&a,&b,o)} else {surface_blend::g2(&a,&b,o)}.unwrap();
        assert_eq!(r.real_arithmetic_order,order);
        for patch in &r.patches {let d=domain(patch);for t in [0.07,0.23,0.53,0.91] {let u=d[0][0]+(d[0][1]-d[0][0])*t;
            for (side,s,v,end,sign) in [(0,&a,0.,true,1.),(1,&b,1.,false,1.)] {
                let sd=domain(s);let source_u=sd[0][0]+(sd[0][1]-sd[0][0])*if o.reverse_u[side] {1.-u} else {u};
                let source_v=if end {sd[1][1]} else {sd[1][0]};
                let e=s.evaluate(source_u,source_v).unwrap();let x=patch.evaluate(u,v).unwrap();close(x.point,e.point,1e-10);
                let (xu,xv)=x.first_derivatives().unwrap();let (su,sv)=e.first_derivatives().unwrap();
                let du=(sd[0][1]-sd[0][0])*if o.reverse_u[side] {-1.} else {1.};let dv=o.gains[side]*sign;
                close(xu,su.map(|v|v*du),1e-9);close(xv,sv.map(|v|v*dv),1e-9);
                if order==Order::C2 {let (xuu,xuv,xvv)=x.second_derivatives().unwrap();let (suu,suv,svv)=e.second_derivatives().unwrap();close(xuu,suu.map(|v|v*du*du),1e-8);close(xuv,suv.map(|v|v*du*dv),1e-8);close(xvv,svv.map(|v|v*dv*dv),1e-8);}
            }
        }}
    }
    assert_eq!((a,b),before);
}
#[test]
fn unclamped_supports_and_opposite_boundary_orientations_are_admitted() {
    let a=support(0.,2,true);let b=support(3.,2,true);let o=Options {a_end:false,b_end:true,gains:[0.3,0.4],..Default::default()};
    let r=surface_blend::g2(&a,&b,o).unwrap();assert_eq!(r.patches.len(),2);
    for patch in &r.patches {let d=domain(patch);let u=(d[0][0]+d[0][1])*0.5;
        for (source,v,sv,gain) in [(&a,0.,0.,-0.3),(&b,1.,2.,-0.4)] {
            let e=source.evaluate(2.*u,sv).unwrap();let x=patch.evaluate(u,v).unwrap();close(x.point,e.point,1e-10);close(x.first_derivatives().unwrap().1,e.first_derivatives().unwrap().1.map(|v|v*gain),1e-9);
        }
    }
}
#[test]
fn resource_and_invalid_scale_failures_are_explicit() {
    let a=support(0.,2,true);let b=support(3.,2,true);
    assert!(surface_blend::g2(&a,&b,Options {max_patches:1,..Default::default()}).is_err());
    for gain in [0.,-1.,f64::NAN,f64::INFINITY] {assert!(surface_blend::g1(&a,&b,Options {gains:[gain,1.],..Default::default()}).is_err());}
    assert!(surface_blend::g2(&a,&b,Options {gains:[1e6;2],..Default::default()}).is_err());
}
