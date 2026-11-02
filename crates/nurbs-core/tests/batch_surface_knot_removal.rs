use nurbs_core::{surface::{Surface,Axis},foundation::approximate_edits::remove_surface_knots_report};
fn source()->Surface {
    let s=Surface {degree_u:1,degree_v:3,knots_u:vec![0.,0.,1.,1.],
        knots_v:vec![0.,0.,0.,0.,1.,1.,1.,1.],
        control_points:(0..2).map(|i|(0..4).map(|j|vec![i as f64,j as f64/3.,(j*j) as f64/16.]).collect()).collect(),
        weights:vec![vec![1.;4];2],periodic_u:false,periodic_v:false};
    s.edit_axis(Axis::V,|c| c.insert(0.25,2)?.insert(0.5,2)?.insert(0.75,2)).unwrap()
}
#[test]
fn batch_removal_is_bounded_against_original_and_rolls_back_on_tight_budget() {
    let s=source();let before=s.clone();
    let knots=vec![0.,0.,0.,0.,1.,1.,1.,1.];
    let accepted=remove_surface_knots_report(&s,Axis::V,knots.clone(),1e-8,None).unwrap();
    assert!(accepted.certificate.accepted);
    assert!(accepted.certificate.error_upper<=1e-8);
    assert_eq!(accepted.surface.knots_v,knots);
    let fresh=nurbs_core::continuity::deviation::positional_upper(&s,&accepted.surface).unwrap();
    assert_eq!(fresh,accepted.certificate.error_upper);
    let refused=remove_surface_knots_report(&s,Axis::V,knots.clone(),1e-30,None).unwrap();
    assert!(!refused.certificate.accepted);
    assert_eq!(refused.surface,s);
    assert_eq!(s,before);
    assert!(remove_surface_knots_report(&s,Axis::V,knots,0.,None).is_err());
    assert!(remove_surface_knots_report(&s,Axis::V,vec![0.,0.,0.,0.,0.3,1.,1.,1.,1.],1e-8,None).is_err());
}
