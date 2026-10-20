use brep_core::profile_certificate as p;
use nurbs_core::curve::Curve;
use std::sync::Arc;
fn own(sources: &[Curve]) -> p::OwnedOriginalCarriers {
    p::own_original_carriers(Arc::new(
        p::assemble_original_contact_atlas(sources, 1e-12, p::ContactAtlasLimits::default())
            .unwrap(),
    ))
    .unwrap()
}
fn line(a: [f64; 2], b: [f64; 2]) -> Curve {
    Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap()
}
#[test]
fn endpoint_contact_is_a_single_cut_and_no_contact_keeps_whole_source() {
    let owner = own(&[
        line([0., 0.], [1., 1.]),
        line([1., 1.], [2., 0.]),
        line([3., 0.], [4., 1.]),
    ]);
    owner.reverify_source().unwrap();
    assert_eq!(owner.carriers().len(), 3);
    let c = owner.carriers();
    assert!(matches!(
        c[0].cuts[1],
        p::OriginalCarrierCut::Contact { .. }
    ));
    assert!(matches!(
        c[1].cuts[0],
        p::OriginalCarrierCut::Contact { .. }
    ));
    assert!(
        c[2].cuts
            .iter()
            .all(|x| matches!(x, p::OriginalCarrierCut::DomainEndpoint(_)))
    );
}
#[test]
fn irrational_cuts_retain_original_curve_and_complete_coverage() {
    let parabola = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        weights: vec![1.; 3],
        control_points: vec![vec![-3., 0.], vec![0., 6.], vec![3., 0.]],
        periodic: false,
    };
    let sources = vec![parabola, line([-4., 1.5], [4., 1.5])];
    let owner = own(&sources);
    owner.reverify_source().unwrap();
    assert_eq!(owner.atlas().sources(), sources);
    for source in 0..2 {
        let carriers = owner
            .carriers()
            .iter()
            .filter(|c| c.source == source)
            .collect::<Vec<_>>();
        assert_eq!(carriers.len(), 3);
        for pair in carriers.windows(2) {
            assert_eq!(pair[0].cuts[1], pair[1].cuts[0]);
        }
        assert!(matches!(
            carriers[0].cuts[0],
            p::OriginalCarrierCut::DomainEndpoint(_)
        ));
        assert!(matches!(
            carriers[2].cuts[1],
            p::OriginalCarrierCut::DomainEndpoint(_)
        ));
    }
}
#[test]
fn self_contact_keeps_distinct_traversals_at_one_physical_point() {
    let c=Curve{degree:3,knots:vec![0.,0.,0.,0.,1.,1.,1.,1.],weights:vec![1.;4],control_points:vec![vec![3.,-1.5],vec![-1.,3.5],vec![-1.,-3.5],vec![3.,1.5]],periodic:false};
    let owner=own(&[c]);owner.reverify_source().unwrap();
    assert_eq!(owner.carriers().len(),3);
    let first=&owner.carriers()[0].cuts[1];
    let second=&owner.carriers()[1].cuts[1];
    match (first,second) {
        (p::OriginalCarrierCut::Contact{parameter_group:a,point_group:x},p::OriginalCarrierCut::Contact{parameter_group:b,point_group:y})=>{assert_ne!(a,b);assert_eq!(x,y);}
        _=>panic!("missing original self-contact cuts"),
    }
}
