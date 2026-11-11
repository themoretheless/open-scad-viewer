use brep_core::fillet_contact::{self, Status};
use nurbs_core::{surface::Surface, offset::fillet_surfaces::{FilletOptions, RadiusLaw}};
fn plane(u:[f64;3],v:[f64;3])->Surface {
    let p=|a:f64,b:f64|(0..3).map(|k|10.*(u[k]*a+v[k]*b)).collect();
    Surface {degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
        control_points:vec![vec![p(0.,0.),p(0.,1.)],vec![p(1.,0.),p(1.,1.)]],
        weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false}
}
#[test]
fn bounded_search_requalifies_original_supports_and_shares_work() {
    let a=plane([1.,0.,0.],[0.,1.,0.]);
    let b=plane([0.,1.,0.],[0.,0.,1.]);
    let before=[a.clone(),b.clone()];
    let domains=[[0.,0.3,0.25,0.75],[0.25,0.75,0.,0.3]];
    let r=fillet_contact::build_g2_on_domains([&a,&b],domains,1.,&FilletOptions::default(),1e-6,100000,1000000).unwrap();
    assert_eq!(r.status,Status::VerifiedContacts);
    assert_eq!([a.clone(),b.clone()],before);
    assert!(r.cells<=100000 && r.exact_work<=1000000);
    let c=r.contacts.as_ref().unwrap();
    for (i,s) in [&a,&b].into_iter().enumerate() {
        assert!(nurbs_core::surface_g2_pcurves::verify_positions(&c[i].curve,
            [&c[i].support_pcurve,&c[i].fillet_pcurve],[s,r.fillet.surface.as_ref().unwrap()],
            [false;2],1e-6,100000,1000000).unwrap().proven);
    }
    assert_eq!(brep_core::fillet_radius::inspect(
        [&a,&b],
        r.fillet.surface.as_ref().unwrap(),
        [&c[0],&c[1]],
        brep_core::fillet_radius::RadiusCheck {
            law: &RadiusLaw::Constant(1.),
            sides: [1.,1.],
            tolerance_mm: 1e-6,
            max_cells: 100000,
            max_exact: 1000000,
        },
    ).unwrap().decision,
        brep_core::fillet_radius::Decision::WithinTolerance);
    let none=fillet_contact::build_g2_on_domains([&a,&b],domains,1.,&FilletOptions::default(),1e-6,0,0).unwrap();
    assert_eq!(none.status,Status::UnresolvedContacts);
    assert!(none.contacts.is_none());
    assert!(fillet_contact::build_g2_on_domains([&a,&b],[[0.5,0.1,0.,1.];2],1.,
        &FilletOptions::default(),1e-6,100000,1000000).is_err());
}
