use brep_core::{rational_section_loft,sweep_retained_walls};
use nurbs_core::{curve::Curve,primitives::line};
fn ring(z:f64)->Vec<Vec<Curve>>{
    let p=[[0.,0.,z],[2.,0.,z],[2.,2.,z],[0.,2.,z]];
    vec![(0..4).map(|i|line(p[i],p[(i+1)%4]).unwrap()).collect()]
}
#[test]
fn retained_wall_union_requires_full_uv_and_body_ownership(){
    let sections=[ring(0.),ring(10.)];
    let model=rational_section_loft(&sections).unwrap();
    let proof=sweep_retained_walls::inspect(&model,&sections,false,1024,1000000).unwrap();
    assert!(proof.certified,"{proof:?}");assert_eq!(proof.inspected_faces,4);
    let short=sweep_retained_walls::inspect(&model,&sections,false,1024,0).unwrap();
    assert!(!short.certified);assert_eq!(short.exact_work,0);
    let mut trimmed=model.clone();
    let wire=trimmed.faces[0].outer;
    for usage in &mut trimmed.loops[wire].coedges {
        for point in &mut usage.pcurve.control_points {point[0]*=0.5;}
    }
    assert!(!sweep_retained_walls::inspect(&trimmed,&sections,false,1024,1000000).is_ok_and(|r|r.certified));
    let mut orphan=model.clone();orphan.shells[0].faces.pop();
    assert!(!sweep_retained_walls::inspect(&orphan,&sections,false,1024,1000000).is_ok_and(|r|r.certified));
    let mut changed=sections.clone();changed[1][0][0].control_points[0][0]+=0.125;
    assert!(!sweep_retained_walls::inspect(&model,&changed,false,1024,1000000).unwrap().certified);
}
