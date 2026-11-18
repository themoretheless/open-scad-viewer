//! Rational clocks are chart proofs, never implicit Solid or smoothness credit.
use super::*;
use nurbs_core::curve::Curve;
fn profile(z:f64,rank_loss:bool)->Vec<Curve> {
    let c=Curve{degree:5,knots:[vec![0.;6],vec![1.;6]].concat(),control_points:(0..6).map(|i|vec![if i%2==0 {-10.}else{10.},if rank_loss {let k=i as i64;(2*(5-2*k)*if i%2==0 {1}else{-1}) as f64}else{2.*(i*i)as f64-9.*i as f64+7.5},z]).collect(),weights:(0..6).map(|i|2f64.powi(5-i)).collect(),periodic:false};
    let line=|a:Vec<f64>,b:Vec<f64>|Curve{degree:1,knots:vec![0.,0.,1.,1.],control_points:vec![a,b],weights:vec![1.;2],periodic:false};
    vec![c.clone(),line(c.control_points[5].clone(),vec![0.,10.,z]),line(vec![0.,10.,z],c.control_points[0].clone())]
}
fn rational_body_with(rank_loss:bool)->Model {
    let sections=(0..3).map(|i|vec![profile(i as f64,rank_loss)]).collect::<Vec<_>>();
    let mut model=crate::rational_section_loft(&sections).unwrap();
    for f in model.faces.len()-2..model.faces.len() {
        let s=&mut model.0.faces[f].surface;let a=s.control_points[0][0].clone();let b=s.control_points[1][1].clone();
        s.knots_u=vec![a[0],a[0],b[0],b[0]];s.knots_v=vec![a[1],a[1],b[1],b[1]];
        let wire=model.faces[f].outer;
        for c in &mut model.0.loops[wire].coedges {
            let mut p=model.0.edges[c.edge].curve.clone();for point in &mut p.control_points {point.truncate(2);}
            if c.reversed {p=p.reverse().unwrap();}c.pcurve=p;
        }
    }
    model.validate().unwrap();model
}
#[test]
fn rational_station_clock_requires_independent_boundary_volume_and_g2() {
    let model=rational_body_with(false);let before=model.clone();
    let full=inspect(&model,&model,1e-8,0.,limits()).unwrap();
    assert!(full.certified,"reason={:?} volume={:?}",full.reason,full.reference_volume.as_ref().map(|v| &v.boundary.intersections.pairs));
    assert!(full.continuous_bound && full.error_upper==Some(0.));
    let caps=[6,7];
    let g=crate::sweep_smoothness::inspect_axis(&model,&caps,crate::sweep_smoothness::Axis::Station,2_000_000).unwrap();
    assert!(g.extraction_complete && g.g1_certified && g.g2_certified);
    assert_eq!(g.edge_ids.len(),3);
    let short_g=crate::sweep_smoothness::inspect_axis(&model,&caps,crate::sweep_smoothness::Axis::Station,1).unwrap();
    assert!(!short_g.g2_certified);
    let mut short=limits();short.reference_volume.boundary.contacts.cells=1;
    let refused=inspect(&model,&model,1e-8,0.,short).unwrap();
    assert!(refused.continuous_bound && !refused.certified);
    assert_eq!(model,before);
}
#[test]
fn restricted_support_owns_exact_edges_vertices_and_one_shared_budget() {
    use crate::shared_boundary::opposite_rows::certify_restricted as certify;
    let model=rational_body_with(false);let before=model.clone();
    for pair in [[0,1],[0,4]] {
        let full=certify(&model,pair,1000);
        let proof=full.certificate.unwrap_or_else(||panic!("pair={pair:?} work={}",full.cells));
        if pair==[0,1] {assert_eq!(proof.edges.len(),1);assert!(proof.vertex.is_none());}
        else {assert!(proof.edges.is_empty());assert!(proof.vertex.is_some());}
        let n=full.cells;
        assert!(certify(&model,pair,n).certificate.is_some());
        let short=certify(&model,pair,n-1);assert!(short.certificate.is_none());assert_eq!(short.cells,n-1);
        assert!(certify(&model,pair,0).certificate.is_none());
    }
    let mut unowned=model.clone();
    // Coordinate equality alone is not vertex ownership.
    let wire=unowned.faces[4].outer;
    for c in &mut unowned.0.loops[wire].coedges {
        let mut e=unowned.0.edges[c.edge].clone();
        for v in &mut e.vertices {let p=unowned.0.vertices[*v].clone();*v=unowned.0.vertices.len();unowned.0.vertices.push(p);}
        c.edge=unowned.0.edges.len();unowned.0.edges.push(e);
    }
    assert!(certify(&unowned,[0,4],1000).certificate.is_none());
    let mut crossed=model.clone();crossed.faces[0].surface.control_points[2][1][1]+=100.;
    assert!(certify(&crossed,[0,1],1000).certificate.is_none());
    let mut invalid=model.clone();invalid.faces[0].surface.weights[2][1]=0.;
    assert!(certify(&invalid,[0,1],1000).certificate.is_none());
    assert_eq!(model,before);
}

#[test]
fn rational_clock_injectivity_never_transfers_immersion_across_a_critical_point() {
    // C(q)=10(q^5,q^4), q=(3u-2)/(2-u). Both tangent components
    // vanish exactly at u=2/3, which binary subdivision cannot hit.
    let model=rational_body_with(true);
    assert!(nurbs_core::surface_injectivity::certify_chart(&model.faces[0].surface,1000).unwrap().proven);
    let mut budget=limits();budget.regularity_cells=100;
    let report=inspect(&model,&model,1e-8,0.,budget).unwrap();
    assert!(report.continuous_bound && report.error_upper==Some(0.));
    assert!(!report.certified);
    assert_eq!(report.reason,Some("whole-boundary-regularity-unproved"));
}
