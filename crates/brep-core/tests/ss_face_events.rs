use brep_core::{Model, ss_trimmed_face as trim};
use nurbs_core::{ss_intersection as ss,curve::Curve};
use std::sync::Arc;
#[path="../../nurbs-core/tests/support/ss_root_circle.rs"] mod circle;
fn model(hole:bool,multiple:bool)->Model {
    let polygon=|points:&[[f64;2]]| (0..points.len()).map(|i|Curve::from_polyline(vec![vec![points[i][0],points[i][1],0.25],
        vec![points[(i+1)%points.len()][0],points[(i+1)%points.len()][1],0.25]]).unwrap()).collect::<Vec<_>>();
    let mut loops=vec![polygon(&[[0.,-1.],[1.,-1.],[1.,1.],[0.,1.]])];
    if hole {loops.push(polygon(&[[0.25,-0.125],[0.25,0.125],[0.375,0.125],[0.375,-0.125]]));}
    let mut model=brep_core::planar_cap::build_sheet(&loops,0.25,1e-7,100000).unwrap();
    let rotate=|p:&mut [f64]| {let a=[p[0],p[1],p[2]];p[0]=a[2];p[1]=a[0];p[2]=a[1];};
    for v in &mut model.vertices {rotate(&mut v.point);}
    for e in &mut model.edges {for p in &mut e.curve.control_points {rotate(p);}}
    for f in &mut model.faces {for row in &mut f.surface.control_points {for p in row {rotate(p);}}}
    if multiple {
        let s=&mut model.faces[0].surface;
        let row=s.control_points[0].iter().zip(&s.control_points[1]).map(|(a,b)|a.iter().zip(b).map(|(a,b)|(a+b)/2.).collect()).collect();
        s.control_points.insert(1,row);s.weights.insert(1,vec![1.;2]);let lo=s.knots_u[1];let hi=s.knots_u[2];s.knots_u=vec![lo,lo,(lo+hi)/2.,hi,hi];
    }
    model.rebuild_topology_ids();model
}
fn census(hole:bool,multiple:bool)->Arc<trim::QualifiedSsTrimmedFaceCensus> {
    let model=Arc::new(model(hole,multiple));let wire=circle::wire();let curve=wire.edges()[2].curve().clone();let s=&model.faces[0].surface;
    let mut spans=Vec::new();
    for u in s.degree_u..s.control_points.len() {for v in s.degree_v..s.control_points[0].len() {
        if s.knots_u[u]==s.knots_u[u+1] || s.knots_v[v]==s.knots_v[v+1] {continue;}
        let source=Arc::new(curve.third_surface_candidates(s,[u,v],Default::default()).unwrap());
        spans.push(Arc::new(ss::QualifiedSsThirdSurfaceBranchCensus::qualify(source,curve.clone(),Default::default()).unwrap()));
    }}
    Arc::new(trim::QualifiedSsTrimmedFaceCensus::qualify(model,0,spans,Default::default()).unwrap())
}
#[test]
fn cox_aliases_merge_by_exact_clock_identity_and_ports_keep_the_same_selector() {
    let source=census(false,true);assert_eq!(source.records().len(),4);
    let proof=trim::QualifiedOrderedSsFaceEvents::qualify(source.clone(),Default::default()).unwrap();
    assert_eq!(proof.events().len(),2);assert!(proof.events().iter().all(|e|e.retained && e.records.len()==2));
    assert!(proof.events()[0].parameter_bounds()[1]<proof.events()[1].parameter_bounds()[0]);
    let curve=proof.point(0).unwrap().curve().clone();
    let a=proof.bind_port(0,curve.clone(),[0;4],Default::default()).unwrap();
    let b=proof.bind_port(1,curve,[0;4],Default::default()).unwrap();
    assert!(a.point().same_selector(proof.point(0).unwrap()));assert!(b.point().same_selector(proof.point(1).unwrap()));
    let edge=ss::OriginalSsRootEdge::new(a,b).unwrap();assert!(edge.forward());edge.reverify().unwrap();
    assert!(proof.point(2).is_none());proof.reverify().unwrap();
}
#[test]
fn hole_exclusions_and_every_alias_record_survive_canonical_event_ordering() {
    let source=census(true,true);let proof=trim::QualifiedOrderedSsFaceEvents::qualify(source.clone(),Default::default()).unwrap();
    assert_eq!(proof.events().iter().map(|e|e.retained).collect::<Vec<_>>(),vec![false,true]);
    let mut records=proof.events().iter().flat_map(|e|e.records.iter().copied()).collect::<Vec<_>>();records.sort();
    assert_eq!(records,(0..source.records().len()).collect::<Vec<_>>());
    for event in proof.events() {
        if event.retained {assert!(matches!(source.records()[event.canonical_record].decision,trim::Decision::Retained|trim::Decision::RetainedBoundary));}
    }
    proof.reverify().unwrap();
}
#[test]
fn exhausted_order_identity_and_work_allowances_never_emit_partial_events() {
    let source=census(false,true);
    for limits in [trim::SsFaceEventLimits {max_events:1,..Default::default()},
        trim::SsFaceEventLimits {max_comparisons:1,..Default::default()},
        trim::SsFaceEventLimits {max_refinements:0,..Default::default()},
        trim::SsFaceEventLimits {max_reserved_work:1,..Default::default()},
        trim::SsFaceEventLimits {max_clock_queries:1,..Default::default()}] {
        assert!(trim::QualifiedOrderedSsFaceEvents::qualify(source.clone(),limits).is_err());
    }
}
#[test]
fn different_elimination_polynomials_share_one_tangent_seam_event() {
    use nurbs_core::surface::Surface;
    let surface=Surface {degree_u:1,degree_v:1,knots_u:vec![0.,0.,0.5,1.,1.],knots_v:vec![0.,0.,1.,1.],
        control_points:vec![vec![vec![0.1875,0.,-1.],vec![0.1875,0.,1.]],
            vec![vec![0.1875,0.5,-1.],vec![0.1875,0.5,1.]],vec![vec![0.6875,1.,-1.],vec![0.6875,1.,1.]]],
        weights:vec![vec![1.;2];3],periodic_u:false,periodic_v:false};
    let corners=[[0.,0.],[1.,0.],[1.,1.],[0.,1.]];
    let world=[vec![0.1875,0.,-1.],vec![0.6875,1.,-1.],vec![0.6875,1.,1.],vec![0.1875,0.,1.]];
    let mut model=Model::empty(1e-7).unwrap();
    for p in &world {model.vertices.push(brep_core::Vertex {point:[p[0],p[1],p[2]]});}
    let mut coedges=Vec::new();
    for i in 0..4 {
        let curve=if i==0 || i==2 {
            let middle=if i==0 {vec![0.1875,0.5,-1.]} else {vec![0.1875,0.5,1.]};
            Curve {degree:1,knots:vec![0.,0.,0.5,1.,1.],control_points:vec![world[i].clone(),middle,world[(i+1)%4].clone()],weights:vec![1.;3],periodic:false}
        } else {Curve::from_polyline(vec![world[i].clone(),world[(i+1)%4].clone()]).unwrap()};
        model.edges.push(brep_core::Edge {vertices:[i,(i+1)%4],curve,degenerate:false});
        let uv=Curve::from_polyline(vec![corners[i].to_vec(),corners[(i+1)%4].to_vec()]).unwrap();
        coedges.push(brep_core::Coedge::new(i,false,uv));
    }
    model.loops.push(brep_core::Loop {coedges});model.faces.push(brep_core::Face {surface,outer:0,holes:vec![]});
    model.shells.push(brep_core::Shell {faces:vec![brep_core::FaceUse {face:0,reversed:false}],closed:false});model.rebuild_topology_ids();
    let model=Arc::new(model);let wire=circle::wire();let curve=wire.edges()[2].curve().clone();let mut spans=Vec::new();
    for u in [1,2] {
        let source=Arc::new(curve.third_surface_candidates(&model.faces[0].surface,[u,1],Default::default()).unwrap());
        spans.push(Arc::new(ss::QualifiedSsThirdSurfaceBranchCensus::qualify(source,curve.clone(),Default::default()).unwrap()));
    }
    assert_ne!(spans[0].source().polynomial(),spans[1].source().polynomial());
    let face=Arc::new(trim::QualifiedSsTrimmedFaceCensus::qualify(model,0,spans,Default::default()).unwrap());
    let proof=trim::QualifiedOrderedSsFaceEvents::qualify(face.clone(),Default::default()).unwrap();
    assert_eq!(proof.events().len(),1);assert_eq!(proof.events()[0].records.len(),2);assert!(proof.events()[0].retained);
    assert!(proof.work().clock_queries>0);
    for record in &proof.events()[0].records {
        assert!(proof.point(0).unwrap().same_source_point(face.point(*record).unwrap(),Default::default()).unwrap());
    }
    proof.reverify().unwrap();
}
