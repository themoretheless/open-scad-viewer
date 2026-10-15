use nurbs_core::{surface::Surface,surface_blend::{self,Options,Order,Boundary}};
fn support(offset:f64,degree:usize,unclamped:bool)->Surface {
    let n=if unclamped {4} else {degree+1};let nv=if unclamped {4} else {3};
    let points=(0..n).map(|i|(0..nv).map(|j|vec![i as f64/(n-1) as f64,offset+j as f64/(nv-1) as f64,0.1*i as f64*j as f64+0.07*(j*j) as f64]).collect()).collect();
    let ku=if unclamped {vec![-2.,-1.,0.,1.,2.,3.,4.]} else {std::iter::repeat_n(2.,degree+1).chain(std::iter::repeat_n(7.,degree+1)).collect()};
    let kv=if unclamped {vec![-2.,-1.,0.,1.,2.,3.,4.]} else {vec![-3.,-3.,-3.,1.,1.,1.]};
    let s=Surface {degree_u:degree,degree_v:2,knots_u:ku,knots_v:kv,control_points:points,weights:(0..n).map(|i|(0..nv).map(|j|1.+0.2*i as f64+0.1*j as f64+0.03*(j*j) as f64+0.04*(i*j) as f64).collect()).collect(),periodic_u:false,periodic_v:false};s.validate().unwrap();s
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
    let a=support(0.,2,true);let b=support(3.,2,true);let o=Options {boundaries:[Boundary::VStart,Boundary::VEnd],gains:[0.3,0.4],..Default::default()};
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

#[test]
fn u_boundaries_match_transposed_source_jets() {
    let a=support(0.,2,false);let b=support(3.,3,false);
    let o=Options {boundaries:[Boundary::UEnd,Boundary::UStart],gains:[0.5;2],..Default::default()};
    let r=surface_blend::g2(&a,&b,o).unwrap();
    for patch in &r.patches {for u in [0.13,0.37,0.83] {for (source,su,v) in [(&a,7.,0.),(&b,2.,1.)] {
        let e=source.evaluate(su,-3.+4.*u).unwrap();let x=patch.evaluate(u,v).unwrap();close(x.point,e.point,1e-10);
        let (eu,ev)=e.first_derivatives().unwrap();let (xu,xv)=x.first_derivatives().unwrap();close(xu,ev.map(|v|v*4.),1e-9);close(xv,eu.map(|v|v*0.5),1e-9);
        let (euu,euv,evv)=e.second_derivatives().unwrap();let (xuu,xuv,xvv)=x.second_derivatives().unwrap();close(xuu,evv.map(|v|v*16.),1e-8);close(xuv,euv.map(|v|v*2.),1e-8);close(xvv,euu.map(|v|v*0.25),1e-8);
    }}}
}
#[test]
fn rational_linear_support_has_nonzero_cartesian_second_jet() {
    let support=|offset:f64|Surface {degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],control_points:vec![vec![vec![0.,offset,0.],vec![0.,offset+1.,0.]],vec![vec![1.,offset,0.],vec![1.,offset+1.,0.]]],weights:vec![vec![1.,2.];2],periodic_u:false,periodic_v:false};
    let r=surface_blend::g2(&support(0.),&support(3.),Options {gains:[0.1;2],..Default::default()}).unwrap();
    let s=&r.patches[0];
    // Source y=offset+2v/(1+v): y'=2/(1+v)^2, y''=-4/(1+v)^3.
    for (v,y,dy,ddy) in [(0.,1.,0.05,-0.005),(1.,3.,0.2,-0.04)] {
        let e=s.evaluate(0.37,v).unwrap();assert!((e.point[1]-y).abs()<1e-12);
        assert!((e.first_derivatives().unwrap().1[1]-dy).abs()<1e-12);
        assert!((e.second_derivatives().unwrap().2[1]-ddy).abs()<1e-11);
    }
}

#[test]
fn saved_rational_curved_edge_blend_is_qualified_over_complete_supports() {
    let support=|offset:f64|Surface{degree_u:2,degree_v:1,knots_u:vec![0.,0.,0.,1.,1.,1.],knots_v:vec![0.,0.,1.,1.],
        control_points:[[0.,0.],[1.,0.25],[2.,0.]].iter().map(|p|vec![vec![p[0],offset+p[1],0.],vec![p[0],offset+p[1]+1.,0.]]).collect(),
        weights:vec![vec![1.;2],vec![0.5;2],vec![1.;2]],periodic_u:false,periodic_v:false};
    let a=support(0.);let b=support(3.);let before=(a.clone(),b.clone());
    let options=Options{gains:[0.5;2],..Default::default()};
    let report=surface_blend::g2_qualified(&a,&b,options,[1e-7,1e-7,1e-5],256).unwrap();
    assert!(report.complete&&report.within_tolerance,"complete={} within={} cells={}",report.complete,report.within_tolerance,report.cells);
    assert_eq!(report.support_seams.len(),2);assert!(report.internal_seams.is_empty());assert!(report.cells<=256);
    let refused=surface_blend::g2_qualified(&a,&b,options,[1e-7,1e-7,1e-5],0).unwrap();
    assert!(!refused.complete&&!refused.within_tolerance);assert!(refused.support_seams.is_empty());
    assert_eq!((a,b),before);
}
