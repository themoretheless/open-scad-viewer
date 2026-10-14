use brep_core::{rational_section_loft,sweep_retained_caps,sweep_cap_contacts::Budgets};
use nurbs_core::{curve::Curve,primitives::line};
fn rings(z:f64)->Vec<Vec<Curve>>{
    let p=[[0.,0.,z],[2.,0.,z],[2.,2.,z],[0.,2.,z]];
    vec![(0..4).map(|i|line(p[i],p[(i+1)%4]).unwrap()).collect()]
}
fn budgets()->Budgets {Budgets {max_walls:1024,max_exact_work:1000000,max_chart_cells:1000,max_trim_pairs:100000,max_trim_cells:100000,max_trim_domain_cells:1000000}}
#[test]
fn actual_caps_require_both_endpoint_contours_and_shared_exact_work(){
    let endpoints=[rings(0.),rings(10.)];
    let model=rational_section_loft(&endpoints).unwrap();
    let proof=sweep_retained_caps::inspect(&model,&endpoints,budgets(),1024).unwrap();
    assert!(proof.exact,"{proof:?}");assert_eq!(proof.inspected_edges,8);
    let short=sweep_retained_caps::inspect(&model,&endpoints,Budgets {max_exact_work:1,..budgets()},1024).unwrap();
    assert!(!short.exact);assert!(short.exact_work<=1);
    let preparation_only=sweep_retained_caps::inspect(&model,&endpoints,Budgets {max_exact_work:2,..budgets()},1024).unwrap();
    assert!(!preparation_only.exact);
    assert_eq!(preparation_only.exact_work,2);
    assert_eq!(preparation_only.reason,"work-limit");
    let mut wrong=endpoints.clone();wrong[1][0][0].control_points[0][0]+=0.125;
    assert!(!sweep_retained_caps::inspect(&model,&wrong,budgets(),1024).unwrap().exact);
    assert!(!sweep_retained_caps::inspect(&model,&endpoints,budgets(),7).unwrap().exact);
}

#[test]
fn actual_hollow_caps_keep_each_hole_owned_by_its_endpoint(){
    use nurbs_core::primitives::circle;
    let hollow=|z|vec![vec![circle([0.,0.,z],[0.,0.,1.],2.).unwrap()],
        vec![circle([0.,0.,z],[0.,0.,1.],0.5).unwrap().reverse().unwrap()]];
    let endpoints=[hollow(0.),hollow(10.)];
    let model=rational_section_loft(&endpoints).unwrap();
    let proof=sweep_retained_caps::inspect(&model,&endpoints,budgets(),1024).unwrap();
    assert!(proof.exact,"{proof:?}");assert_eq!(proof.inspected_edges,16);
    let mut wrong=endpoints.clone();wrong[1].pop();
    assert!(!sweep_retained_caps::inspect(&model,&wrong,budgets(),1024).unwrap().exact);
    let mut swapped=endpoints.clone();swapped[1].swap(0,1);
    assert!(!sweep_retained_caps::inspect(&model,&swapped,budgets(),1024).unwrap().exact);
}
