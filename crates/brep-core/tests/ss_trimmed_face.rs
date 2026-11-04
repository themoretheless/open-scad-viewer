use brep_core::{Model,ss_trimmed_face as trim};
use nurbs_core::{ss_intersection as ss,curve::Curve};
use std::sync::Arc;
#[path="../../nurbs-core/tests/support/ss_root_circle.rs"] mod circle;
#[allow(dead_code)]
#[path="../../nurbs-core/tests/support/ss_original_ideal.rs"] mod fixture;
fn polygon(p:&[[f64;2]],x:f64)->Vec<Curve> {
    (0..p.len()).map(|i|Curve::from_polyline(vec![vec![p[i][0],p[i][1],x],vec![p[(i+1)%p.len()][0],p[(i+1)%p.len()][1],x]]).unwrap()).collect()
}
fn face(x:f64,lo:f64,hole:bool)->Model {
    let mut loops=vec![polygon(&[[lo,-1.],[1.,-1.],[1.,1.],[lo,1.]],x)];
    if hole {loops.push(polygon(&[[0.25,-0.125],[0.25,0.125],[0.375,0.125],[0.375,-0.125]],x));}
    let mut model=brep_core::planar_cap::build_sheet(&loops,x,1e-7,100000).unwrap();
    let rotate=|p:&mut [f64]| {let a=[p[0],p[1],p[2]];p[0]=a[2];p[1]=a[0];p[2]=a[1];};
    for v in &mut model.vertices {rotate(&mut v.point);}
    for e in &mut model.edges {for p in &mut e.curve.control_points {rotate(p);}}
    for f in &mut model.faces {for row in &mut f.surface.control_points {for p in row {rotate(p);}}}
    model.rebuild_topology_ids();model.validate().unwrap();model
}
fn spans(model:&Model,curve:&Arc<ss::OriginalSsRootCurve>)->Vec<Arc<ss::QualifiedSsThirdSurfaceBranchCensus>> {
    let s=&model.faces[0].surface;let mut out=Vec::new();
    for u in s.degree_u..s.control_points.len() {for v in s.degree_v..s.control_points[0].len() {
        if s.knots_u[u]==s.knots_u[u+1] || s.knots_v[v]==s.knots_v[v+1] {continue;}
        let source=Arc::new(curve.third_surface_candidates(s,[u,v],Default::default()).unwrap());
        out.push(Arc::new(ss::QualifiedSsThirdSurfaceBranchCensus::qualify(source,curve.clone(),Default::default()).unwrap()));
    }}out
}
fn retained(d:trim::Decision)->bool {matches!(d,trim::Decision::Retained|trim::Decision::RetainedBoundary)}
#[test]
fn authored_face_keeps_original_points_and_binds_the_immutable_snapshot() {
    let wire=circle::wire();let curve=wire.edges()[2].curve().clone();let model=Arc::new(face(0.25,0.,false));
    let native=spans(&model,&curve);let proof=trim::QualifiedSsTrimmedFaceCensus::qualify(model.clone(),0,native,Default::default()).unwrap();
    assert_eq!(proof.records().len(),2);assert!(proof.records().iter().all(|r|retained(r.decision)));
    assert!(proof.source_matches(&model));assert!(!proof.source_matches(&Arc::new((*model).clone())));
    assert_eq!(proof.identity(),model.1.faces[0]);
    for i in 0..2 {assert!(Arc::ptr_eq(proof.point(i).unwrap().curve(),&curve));}
    assert!(proof.point(2).is_none());proof.reverify().unwrap();
}
#[test]
fn real_hole_excludes_the_whole_uv_box_without_losing_other_intersections() {
    let wire=circle::wire();let curve=wire.edges()[2].curve().clone();let model=Arc::new(face(0.25,0.,true));
    let native=spans(&model,&curve);let proof=trim::QualifiedSsTrimmedFaceCensus::qualify(model.clone(),0,native.clone(),Default::default()).unwrap();
    assert_eq!(proof.records().iter().filter(|r|r.decision==trim::Decision::ExcludedTrim).count(),1);
    assert_eq!(proof.records().iter().filter(|r|retained(r.decision)).count(),1);
    assert!(proof.work().point_cells>0);proof.reverify().unwrap();
    let mut broken=(*model).clone();let hole=broken.faces[0].holes[0];broken.loops[hole].coedges[0].pcurve.control_points[0][0]+=0.125;
    assert!(trim::QualifiedSsTrimmedFaceCensus::qualify(Arc::new(broken),0,native.clone(),Default::default()).is_err());
    for limits in [trim::Limits {max_records:1,..Default::default()},trim::Limits {max_trim_cells:1,..Default::default()},
        trim::Limits {max_boundary_cells:1,..Default::default()},trim::Limits {max_point_cells:1,..Default::default()}] {
        assert!(trim::QualifiedSsTrimmedFaceCensus::qualify(model.clone(),0,native.clone(),limits).is_err());
    }
}
#[test]
fn every_nonempty_third_cox_span_is_required_and_alias_records_stay_explicit() {
    let wire=circle::wire();let curve=wire.edges()[2].curve().clone();let mut model=face(0.25,0.,false);
    let s=&mut model.faces[0].surface;assert_eq!(s.degree_u,1);assert_eq!(s.control_points.len(),2);
    let middle=s.control_points[0].iter().zip(&s.control_points[1]).map(|(a,b)|a.iter().zip(b).map(|(a,b)|(a+b)/2.).collect()).collect();
    s.control_points.insert(1,middle);s.weights.insert(1,vec![1.;s.control_points[0].len()]);
    let lo=s.knots_u[1];let hi=s.knots_u[2];s.knots_u=vec![lo,lo,(lo+hi)/2.,hi,hi];
    model.rebuild_topology_ids();let model=Arc::new(model);let native=spans(&model,&curve);assert_eq!(native.len(),2);
    let proof=trim::QualifiedSsTrimmedFaceCensus::qualify(model.clone(),0,native.clone(),Default::default()).unwrap();
    assert_eq!(proof.spans().len(),2);assert_eq!(proof.records().len(),4);
    assert!(proof.records().iter().filter(|r|retained(r.decision)).count()>=2);proof.reverify().unwrap();
    assert!(trim::QualifiedSsTrimmedFaceCensus::qualify(model.clone(),0,vec![native[0].clone()],Default::default()).is_err());
    assert!(trim::QualifiedSsTrimmedFaceCensus::qualify(model,0,vec![native[0].clone(),native[0].clone()],Default::default()).is_err());
}
#[test]
fn exact_closed_linear_boundary_is_retained_and_restricted_coedges_are_refused() {
    let mut b=fixture::plane(0.);for row in &mut b.control_points {for p in row {let v=p[1];p[1]=0.5;p[2]=v-0.5;}}
    let source=ss::FiberSources::new(&fixture::plane(0.),&b).unwrap();
    let ideal=ss::qualify_original_ss_ideal(&source,[1;4],Default::default()).unwrap();
    let basis=ss::qualify_ss_groebner_basis(&ideal,Default::default()).unwrap();let system=ss::qualify_ss_deflated_system(&basis,Default::default()).unwrap();
    let fiber=ss::qualify_deflated_ss_fiber(&system,[[0.2,0.8],[0.4,0.6],[0.25,0.75],[0.4,0.6]],ss::FiberAxis::SecondU,Default::default()).unwrap().into_fiber().unwrap();
    let curve=Arc::new(ss::OriginalSsRootCurve::from_deflated(fiber));let model=Arc::new(face(0.5,0.5,false));let native=spans(&model,&curve);
    let proof=trim::QualifiedSsTrimmedFaceCensus::qualify(model.clone(),0,native.clone(),Default::default()).unwrap();
    assert_eq!(proof.records().len(),1);assert_eq!(proof.records()[0].decision,trim::Decision::RetainedBoundary);proof.reverify().unwrap();
    let mut restricted=(*model).clone();let wire=restricted.faces[0].outer;
    restricted.loops[wire].coedges[0]=restricted.loops[wire].coedges[0].clone().with_trim(brep_topology::CoedgeTrim {
        curve_parameter:[0.,1.],pcurve_parameter:[0.,1.],periodic_lift:[[0;2];2],
    }).unwrap();
    assert!(trim::QualifiedSsTrimmedFaceCensus::qualify(Arc::new(restricted),0,native,Default::default()).is_err());
}
