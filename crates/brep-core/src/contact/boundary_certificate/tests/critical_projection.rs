//! Original critical-quintic profile: projection rank is not surface rank.
use super::*;
fn original_body_with(rank_loss:bool)->Model {
    let curve=nurbs_core::curve::Curve {degree:5,
        knots:[vec![0.;6],vec![1.;6]].concat(),
        control_points:(0..6).map(|i| {let k=i as i64;vec![if i%2==0 {-10.}else{10.},
            if rank_loss {(2*(5-2*k)*if i%2==0 {1}else{-1}) as f64}
            else {(2*i*i) as f64-9.*i as f64+7.5}]}).collect(),
        weights:vec![1.;6],periodic:false};
    let chord=crate::line(curve.control_points[5].clone(),curve.control_points[0].clone());
    let mut model=crate::prism::extrude(&[vec![curve,chord]],0.,2.).unwrap();
    // Literal XY charts preserve exact original cap lifts (no thirds rounded
    // by rectangular-hull normalization). The physical body is unchanged.
    for face in model.faces.len()-2..model.faces.len() {
        let s=&mut model.faces[face].surface;
        let a=s.control_points[0][0].clone();let b=s.control_points[1][1].clone();
        s.knots_u=vec![a[0],a[0],b[0],b[0]];
        s.knots_v=vec![a[1],a[1],b[1],b[1]];
        let outer=model.faces[face].outer;
        for coedge in &mut model.0.loops[outer].coedges {
            let mut c=model.0.edges[coedge.edge].curve.clone();
            for p in &mut c.control_points {p.truncate(2);}
            if coedge.reversed {c=c.reverse().unwrap();}
            coedge.pcurve=c;
        }
    }
    model
}
#[test]
fn critical_quintic_original_volume_requires_full_faces_contacts_and_orientation() {
    let model=original_body_with(false);let before=model.clone();
    let full=inspect(&model,&model,1e-8,0.,limits()).unwrap();
    eprintln!("critical-quintic complete boundary: certified={} reason={:?} regularity={:?}",
        full.certified,full.reason,full.reference_regularity.as_ref().map(|r|&r.unresolved_faces));
    if let Some(v)=&full.reference_volume {
        eprintln!("volume={} embedding={} lifts={} trim={} faces={} contacts={} unresolved={:?} nesting={:?} orientations={:?}",
            v.proven,v.boundary.proven,v.boundary.agreement.all_equal,v.boundary.trim.all_valid,
            v.boundary.intersections.faces.all_faces_injective,v.boundary.intersections.pairs.all_pairs_classified,
            v.boundary.intersections.pairs.pairs.iter().filter(|p|p.reason=="pair-unresolved").map(|p|p.faces).collect::<Vec<_>>(),
            v.nesting.as_ref().map(|n|n.roles_consistent),
            v.orientations.iter().map(|o|(o.expected_outward,o.outward)).collect::<Vec<_>>());
    }
    assert!(full.reference_volume.as_ref().unwrap().boundary.intersections.faces.all_faces_injective,
        "every original face, including the critical-quintic wall, must have a whole-chart proof");
    assert!(full.certified,"the complete original volume remains unproved: {:?}",full.reason);
    assert!(full.continuous_bound && full.error_upper==Some(0.));
    assert_eq!(model,before);
}

#[test]
fn supported_opposite_rows_own_topology_side_signs_and_the_shared_budget() {
    use crate::shared_boundary::opposite_rows::certify;
    let model=original_body_with(false);let before=model.clone();
    let full=certify(&model,[0,1],1000);
    assert_eq!(full.certificate.as_ref().unwrap().edges.len(),2);
    let measured=full.cells;
    assert!(certify(&model,[0,1],measured).certificate.is_some());
    let short=certify(&model,[0,1],measured-1);
    assert!(short.certificate.is_none());assert_eq!(short.cells,measured-1);
    let zero=certify(&model,[0,1],0);assert!(zero.certificate.is_none());assert_eq!(zero.cells,0);
    // Keep the same curve and vertices, but remove one shared EDGE identity.
    let mut unowned=model.clone();let wire=unowned.faces[0].outer;
    let edge=full.certificate.unwrap().edges[0];
    let index=unowned.loops[wire].coedges.iter().position(|c|c.edge==edge).unwrap();
    let copy=unowned.edges[edge].clone();let next=unowned.edges.len();
    unowned.edges.push(copy);unowned.loops[wire].coedges[index].edge=next;
    assert!(certify(&unowned,[0,1],1000).certificate.is_none());
    let mut crossed=model.clone();crossed.faces[0].surface.control_points[2][1][1]+=100.;
    assert!(certify(&crossed,[0,1],1000).certificate.is_none());
    let mut coincident=model.clone();
    for p in coincident.faces[0].surface.control_points.iter_mut().flatten() {p[1]=10.+p[0]/4.;}
    assert!(certify(&coincident,[0,1],1000).certificate.is_none());
    let mut invalid=model.clone();invalid.faces[0].surface.weights[2][1]=f64::NAN;
    assert!(certify(&invalid,[0,1],1000).certificate.is_none());
    assert!(inspect(&invalid,&invalid,1e-8,0.,limits()).is_err());
    assert_eq!(model,before);
    eprintln!("two supported original boundary rows: {measured} shared geometry cells");
}
#[test]
fn injective_critical_chart_does_not_supply_the_missing_rank_certificate() {
    // C(q)=10(q^5,q^4) remains injective, but BOTH profile derivatives vanish
    // at q=0. A topological chart proof must never silently establish rank.
    let model=original_body_with(true);
    let wall=&model.faces[0].surface;
    let (du,_)=wall.evaluate(0.5,0.5).unwrap().first_derivatives().unwrap();
    assert!(du.iter().all(|&x|x==0.),"the authored source has genuine rank loss");
    assert!(nurbs_core::surface_injectivity::certify_chart(wall,1000).unwrap().proven);
    assert!(!nurbs_core::surface_injectivity::certify_contraction(wall,1000).unwrap().proven);
    let full=inspect(&model,&model,1e-8,0.,limits()).unwrap();
    assert!(full.continuous_bound && full.error_upper==Some(0.));
    assert!(!full.certified);
    assert_eq!(full.reason,Some("whole-boundary-regularity-unproved"));
    assert!(full.reference_regularity.as_ref().unwrap().unresolved_faces.contains(&0));
}

#[test]
fn affine_mixed_critical_body_owns_joint_boundary_volume_and_refusal_budgets() {
    let original=original_body_with(false);
    let matrix=[[1.,1.,1.,8.],[1.,2.,1.,-4.],[1.,1.,2.,2.],[0.,0.,0.,1.]];
    let model=crate::transform::affine(&original,matrix).unwrap();let before=model.clone();
    let full=inspect(&model,&model,1e-8,0.,limits()).unwrap();
    assert!(full.certified,"reason={:?} volume={:?}",full.reason,
        full.reference_volume.as_ref().map(|v|(v.boundary.intersections.faces.clone(),v.boundary.intersections.pairs.clone())));
    assert!(full.continuous_bound && full.error_upper==Some(0.));
    for v in [full.reference_volume.as_ref().unwrap(),full.retained_volume.as_ref().unwrap()] {
        assert!(v.proven && v.boundary.intersections.faces.all_faces_injective && v.boundary.intersections.pairs.all_pairs_classified);
        assert_eq!(v.nesting.as_ref().unwrap().roles_consistent,Some(true));
        assert!(v.orientations.iter().all(|s|s.outward==Some(s.expected_outward)));
    }
    let mut short=limits();short.linear_cells_per_boundary=1;
    short.reference_volume.boundary.spans=1;short.retained_volume.boundary.spans=1;
    let refused=inspect(&model,&model,1e-8,0.,short).unwrap();
    assert!(refused.continuous_bound && !refused.certified);
    let singular=crate::transform::affine(&original_body_with(true),matrix).unwrap();
    let refused=inspect(&singular,&singular,1e-8,0.,limits()).unwrap();
    assert!(refused.continuous_bound && !refused.certified);
    assert_eq!(refused.reason,Some("whole-boundary-regularity-unproved"));
    assert_eq!(model,before);
}
