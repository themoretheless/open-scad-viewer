use super::*;
fn plane(sign:f64)->Surface {
    Surface{degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
        control_points:vec![vec![vec![0.,0.,0.],vec![0.,sign,0.]],vec![vec![sign,0.,0.],vec![sign,sign,0.]]],
        weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false}
}
#[test]
fn shared_corner_workspace_preserves_direct_decisions_and_zero_budget_refusal() {
    let mut a=plane(-1.);let mut b=plane(1.);
    a.control_points[1][1][2]=0.01;b.control_points[1][1][2]=0.01;
    let rects=[[[0.,0.1];2];2];
    let mut cache=SharedCornerWorkspace::new([&a,&b],1e-4).unwrap();
    let direct=inspect_shared_corner_chords([&a,&b],rects,[1.,1.,0.],1e-4,64).unwrap();
    assert!(direct.0);assert_eq!(cache.inspect(rects,[1.,1.,0.],64).unwrap(),direct);
    let warmed=cache.inspect(rects,[1.,1.,0.],64).unwrap();
    assert!(warmed.0);assert_eq!(warmed.1,0);assert!(cache.cache_hits()>=3);
    assert_eq!(cache.inspect(rects,[1.,1.,0.],0).unwrap(),(false,0));
    for direction in [[0.;3],[0.,0.,1.],[-1.,-1.,0.]] {
        let expected=inspect_shared_corner_chords([&a,&b],rects,direction,1e-4,64).unwrap();
        let actual=cache.inspect(rects,direction,64).unwrap();
        assert_eq!(actual.0,expected.0);assert!(actual.1<=expected.1);
    }
    let mut invalid=rects;invalid[0][0]=[-0.1,0.1];
    assert!(cache.inspect(invalid,[1.,1.,0.],0).is_err());
    assert!(cache.inspect(rects,[f64::NAN,0.,0.],64).is_err());
    b.control_points[0][0][2]=1e-15;
    let mut other=SharedCornerWorkspace::new([&a,&b],1e-4).unwrap();
    assert_eq!(other.inspect(rects,[1.,1.,0.],64).unwrap(),(false,0));
}
#[test]
fn owns_original_paths_and_accounts_normal_work() {
    let a=plane(-1.);let b=plane(1.);let rects=[[[0.,0.1];2];2];
    let (cert,used)=qualify_corner_chords([&a,&b],[[false;2];2],rects,[1.,1.,0.],1e-6,3).unwrap();
    let cert=cert.unwrap();assert_eq!(used,3);assert!(std::ptr::eq(cert.sources()[0],&a));
    assert_eq!(inspect_shared_corner_chords([&a,&b],rects,[1.,1.,0.],1e-6,3).unwrap(),(true,3));
    assert!(!inspect_shared_corner_chords([&a,&b],rects,[1.,1.,0.],1e-6,2).unwrap().0);
    assert!(std::ptr::eq(cert.sources()[1],&b));assert_eq!(cert.rectangles(),rects);
    assert!(qualify_corner_chords([&a,&b],[[false;2];2],rects,[1.,1.,0.],1e-6,2).unwrap().0.is_none());
    assert!(qualify_corner_chords([&a,&a],[[false;2];2],rects,[1.,1.,0.],1e-6,4).unwrap().0.is_none());
}
#[test]
fn exact_anchor_uses_authored_clamped_corner_and_charges_work() {
    let s=plane(-1.);let uv=[0.,0.];let world=[0.,0.,0.];
    let (proof,work)=qualify_surface_anchor(&s,uv,world,1).unwrap();
    assert!(proof.is_some());assert_eq!(work,1);
    assert!(qualify_surface_anchor(&s,uv,world,0).unwrap().0.is_none());
    assert!(qualify_surface_anchor(&s,uv,[0.,0.,1e-15],1).unwrap().0.is_none());
}
#[test]
fn rejects_approximate_common_corner_and_invalid_projection() {
    let a=plane(-1.);let mut b=plane(1.);b.control_points[0][0][2]=1e-15;
    assert_eq!(inspect_shared_corner_chords([&a,&b],[[[0.,0.1];2];2],[1.,1.,0.],1e-6,4).unwrap(),(false,0));
    assert!(qualify_corner_chords([&a,&b],[[false;2];2],[[[0.,0.1];2];2],[1.,1.,0.],1e-6,4).is_err());
    assert!(qualify_corner_chords([&a,&a],[[false;2];2],[[[0.,0.1];2];2],[f64::NAN,0.,0.],1e-6,4).is_err());
}
#[test]
fn shared_corner_covers_curved_charts_and_refuses_zero_projection() {
    let mut a=plane(-1.);let mut b=plane(1.);
    a.control_points[1][1][2]=0.01;b.control_points[1][1][2]=0.01;
    let rects=[[[0.,0.1];2];2];
    // Independent charts S=(sign*u,sign*v,0.01*u*v) share only
    // their authored corner. Neither chart is planar.
    assert!(inspect_shared_corner_chords([&a,&b],rects,[1.,1.,0.],1e-4,3).unwrap().0);
    assert!(!inspect_shared_corner_chords([&a,&b],rects,[0.;3],1e-4,4).unwrap().0);
    assert_eq!(inspect_shared_corner_chords([&a,&b],rects,[1.,1.,0.],1e-4,0).unwrap(),(false,0));
    let mut invalid=rects;invalid[0][0]=[-0.1,0.1];
    assert!(inspect_shared_corner_chords([&a,&b],invalid,[1.,1.,0.],1e-4,0).is_err());
    invalid[0][0]=[0.,0.];
    assert!(inspect_shared_corner_chords([&a,&b],invalid,[1.,1.,0.],1e-4,0).is_err());
}
#[test]
fn covers_reflected_affine_parameter_domains_without_rebinding_sources() {
    let mut a=plane(-1.);let b=plane(1.);
    a.control_points.reverse();a.weights.reverse();
    for row in &mut a.control_points {row.reverse();}
    for row in &mut a.weights {row.reverse();}
    a.knots_u=vec![2.,2.,5.,5.];a.knots_v=vec![-4.,-4.,2.,2.];
    let rects=[[[4.9,5.],[1.9,2.]],[[0.,0.1];2]];
    let (cert,used)=qualify_corner_chords([&a,&b],[[true;2],[false;2]],rects,[1.,1.,0.],1e-6,3).unwrap();
    assert!(cert.is_some());assert_eq!(used,3);
    assert!(qualify_corner_chords([&b,&a],[[false;2],[true;2]],[rects[1],rects[0]],[-1.,-1.,0.],1e-6,3).unwrap().0.is_some());
    assert_eq!(inspect_shared_corner_chords([&a,&b],rects,[1.,1.,0.],1e-6,3).unwrap(),(true,3));
    assert_eq!(inspect_shared_corner_chords([&b,&a],[rects[1],rects[0]],[-1.,-1.,0.],1e-6,3).unwrap(),(true,3));
}
#[test]
fn exact_anchor_on_large_identity_plane() {
    let s=Surface{degree_u:1,degree_v:1,knots_u:vec![-20.,-20.,20.,20.],knots_v:vec![-20.,-20.,20.,20.],
        control_points:vec![vec![vec![-20.,-20.,6.],vec![-20.,20.,6.]],vec![vec![20.,-20.,6.],vec![20.,20.,6.]]],weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false};
    let (proof,work)=qualify_surface_anchor(&s,[20.,0.],[20.,0.,6.],10000000).unwrap();
    assert!(proof.is_some(),"work {work}");
}
#[test]
fn exact_interior_anchor_refuses_approximation_and_insufficient_work() {
    let s=plane(1.);
    let (cert,work)=qualify_surface_anchor(&s,[0.25,0.5],[0.25,0.5,0.],100000).unwrap();
    let cert=cert.unwrap();assert!(std::ptr::eq(cert.surface(),&s));
    assert_eq!(cert.uv(),[0.25,0.5]);assert_eq!(cert.world(),[0.25,0.5,0.]);
    assert!(work>0);
    assert!(qualify_surface_anchor(&s,[0.25,0.5],[0.25,0.5,1e-15],100000).unwrap().0.is_none());
    assert!(qualify_surface_anchor(&s,[0.25,0.5],[0.25,0.5,0.],0).unwrap().0.is_none());
    assert!(qualify_surface_anchor(&s,[1.25,0.5],[1.25,0.5,0.],100000).unwrap().0.is_none());
}
#[test]
fn paired_interior_anchors_bind_paths_and_refuse_crossing_rectangles() {
    let a=plane(1.);let b=plane(1.);
    let first=qualify_surface_anchor(&a,[0.5,0.5],[0.5,0.5,0.],100000).unwrap().0.unwrap();
    let second=qualify_surface_anchor(&b,[0.5,0.5],[0.5,0.5,0.],100000).unwrap().0.unwrap();
    let rects=[[[0.4,0.5];2],[[0.5,0.6];2]];
    let (cert,used)=qualify_anchored_chords([&first,&second],rects,[1.,1.,0.],1e-6,3).unwrap();
    assert!(cert.is_some());assert_eq!(used,3);
    assert!(qualify_anchored_chords([&first,&second],[[[0.4,0.6];2];2],[1.,1.,0.],1e-6,3).unwrap().0.is_none());
    let different=qualify_surface_anchor(&b,[0.25,0.5],[0.25,0.5,0.],100000).unwrap().0.unwrap();
    assert!(qualify_anchored_chords([&first,&different],rects,[1.,1.,0.],1e-6,3).is_err());
}
#[test]
fn polar_anchor_paths_charge_restrictions_and_reject_foreign_factors() {
    let s=Surface{degree_u:2,degree_v:1,knots_u:vec![0.,0.,0.,1.,1.,1.],knots_v:vec![0.,0.,1.,1.],
        control_points:(0..3).map(|u|(0..2).map(|v|vec![u as f64/2.,if u==2 {v as f64}else{0.},0.]).collect()).collect(),
        weights:vec![vec![1.;2];3],periodic_u:false,periodic_v:false};
    let plane=plane(-1.);
    let first=qualify_surface_anchor(&s,[0.,0.],[0.,0.,0.],100000).unwrap().0.unwrap();
    let second=qualify_surface_anchor(&plane,[0.,0.],[0.,0.,0.],100000).unwrap().0.unwrap();
    let d=crate::rational_pole_differential::prepare(&s,false,10000000).unwrap().certificate.unwrap();
    let n=crate::factored_surface_normals::prepare(&s,false,10000000).unwrap().certificate.unwrap();
    let rects=[[[0.,0.001],[0.,0.0001]],[[0.,0.001];2]];
    let (cert,used)=qualify_polar_anchored_chords([&first,&second],rects,[Some(&d),None],[Some(&n),None],[-1.,-1.,0.],1e-6,8).unwrap();
    assert!(cert.is_some());assert_eq!(used,8);
    assert!(qualify_polar_anchored_chords([&first,&second],rects,[Some(&d),None],[Some(&n),None],[0.;3],1e-6,8).unwrap().0.is_none());
    let (adaptive,adaptive_spans)=qualify_polar_anchored_chords_with_projections([&first,&second],rects,[Some(&d),None],[Some(&n),None],&[[0.;3]],1e-6,8).unwrap();
    assert!(adaptive.is_some());assert_eq!(adaptive_spans,8);
    assert!(qualify_polar_anchored_chords([&first,&second],rects,[Some(&d),None],[Some(&n),None],[-1.,-1.,0.],1e-6,7).unwrap().0.is_none());
    let twin=s.clone();let foreign=crate::rational_pole_differential::prepare(&twin,false,10000000).unwrap().certificate.unwrap();
    assert!(qualify_polar_anchored_chords([&first,&second],rects,[Some(&foreign),None],[Some(&n),None],[-1.,-1.,0.],1e-6,8).is_err());
}
