#[path = "../examples/support/compatible_contact_fixture.rs"]
mod fixture;
use brep_core::{face_cut, region_replacement as r};
#[test]
fn curved_nurbs_contact_body_is_g2_curvature_bounded_and_globally_embedded() {
    let (source, request, skin, contacts) = fixture::fixture();
    let before = source.clone();
    let ridge = &source.edges[request.edge].curve;
    let mid = ridge.evaluate(1.).unwrap().point;
    let chord = (ridge.control_points[0][2] + ridge.control_points.last().unwrap()[2]) * 0.5;
    assert!(
        (mid[2] - chord).abs() > 0.1,
        "Selected source edge must really be curved"
    );
    let retention = face_cut::plan_contact_retention(
        &source,
        0,
        request,
        &skin,
        [&contacts[0], &contacts[1]],
        "qualified-compatible-nurbs-contact",
        1e-8,
        Default::default(),
        |roots| Ok(fixture::paths(&source, request, &contacts, roots)),
    )
    .unwrap();
    let limits = r::EmbeddedPatchLimits {
        construction: Default::default(),
        linear_cells: 100000,
        global: brep_core::boundary_embedding::Limits {
            exact_work: 1000000,
            trim_pairs: 10000,
            trim_cells: 100000,
            trim_domain_cells: 1000000,
            spans: 10000,
            contacts: brep_core::face_contacts::Limits {
                pairs: 10000,
                cells: 1000000,
                domain_cells: 1000000,
                cells_per_pair: 1000,
                domain_cells_per_pair: 10000,
            },
        },
    };
    let body = r::sew_contact_g2_patch_set(
        retention,
        "compatible-nurbs-global-body",
        1e-8,
        limits,
        |p| Ok(fixture::patch(p, &skin)),
    )
    .unwrap();
    let saved_contacts = body.contact_edges();
    assert_ne!(saved_contacts[0], saved_contacts[1]);
    for side in 0..2 {
        assert_eq!(body.model().edges[saved_contacts[side]].curve, contacts[side].curve);
    }
    // A valid closed G2 smoothing is not automatically a prescribed R1 fillet.
    let radius = brep_core::fillet_radius::inspect_model_skins(
        body.model(), [body.patch_faces()[0]; 2], saved_contacts,
        &nurbs_core::offset::fillet_surfaces::RadiusLaw::Constant(1.),
        contacts[0].curve.domain(), [1., -1.], 1e-7, 100000, 1000000,
    ).unwrap();
    eprintln!("saved_contact_radius decision={:?} reason={} witness={:?}",
        radius.decision, radius.reason, radius.violation_witness);
    assert_eq!(radius.decision, brep_core::fillet_radius::Decision::ExceedsTolerance);
    assert!(radius.violation_witness.is_some_and(|w| w[1] > 1e-7));
    assert_eq!(source, before);
    assert_eq!(body.model().faces.len(), 8);
    assert_eq!(body.patch_faces().len(), 1);
    assert!(body.seams().complete && body.seams().within_tolerance);
    assert_eq!(body.seams().seams.len(), 6);
    assert!(body.fields().iter().all(|f| {
        f.absolute_curvature_upper
            .is_some_and(|k| k.is_finite() && k <= 10.)
    }));
    assert!(body.global().proven && body.global().intersections.absence_proven);
    assert!(body.global().agreement.all_equal && body.global().trim.all_valid);
    assert!(body.global().intersections.pairs.all_pairs_classified);
    assert_eq!(body.global().intersections.pairs.total_pairs, 28);
    let networks = body
        .global()
        .intersections
        .pairs
        .pairs
        .iter()
        .filter_map(|p| match &p.boundary {
            Some(brep_core::face_contacts::SharedBoundary::ProjectedTrimNetwork(c)) => {
                assert!(c.edges.len() >= 2);
                assert!(c.cells > 0 && c.cells <= p.source_chart_cells);
                assert!(p.source_chart_cells + p.boundary_cells + p.hull_cells <= 1000);
                Some(c)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(!networks.is_empty());
    let new = body.model().faces[body.patch_faces()[0]]
        .surface
        .evaluate(1., 0.5)
        .unwrap()
        .point;
    assert!((mid[2] - new[2] - 0.00010013580322265625).abs() < 1e-12);
    eprintln!(
        "qualified_nurbs_contact faces={} pairs={} networks={} max_curvature={:?}",
        body.model().faces.len(),
        body.global().intersections.pairs.total_pairs,
        networks.len(),
        body.fields()
            .iter()
            .filter_map(|f| f.absolute_curvature_upper)
            .reduce(f64::max)
    );
}
