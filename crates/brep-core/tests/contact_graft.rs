#[path = "../examples/support/contact_graft_fixture.rs"]
mod fixture;
#[path = "../examples/support/curved_contact_body_fixture.rs"]
mod curved_fixture;
use brep_core::contact_graft;
#[test]
fn variable_curved_body_keeps_fits_but_refuses_unproved_original_cap_membership() {
    let f=curved_fixture::build().unwrap();let before=(f.source.clone(),f.patch.clone());
    let fitted=contact_graft::fit_boundaries_with_charts(&f.source,&f.patch,&f.mapping,&f.keeps,&f.source_pcurves,1e-8,100000,1000000).unwrap();
    assert_eq!(fitted.adjustments.iter().map(|a|a.edge).collect::<Vec<_>>(),vec![0,2]);
    assert!(fitted.adjustments.iter().all(|a|a.displacement_upper_mm<=f.source.tolerance_mm/8.&&a.source_face<f.source.faces.len()));
    assert_eq!(fitted.model.vertices,f.patch.vertices);
    for i in [1,3] {assert_eq!(fitted.model.edges[i].curve,f.patch.edges[i].curve);}
    for e in &fitted.model.edges {
        assert_eq!(e.curve.control_points[0],fitted.model.vertices[e.vertices[0]].point);
        assert_eq!(*e.curve.control_points.last().unwrap(),fitted.model.vertices[e.vertices[1]].point);
    }
    let targets=contact_graft::plan_targets(&f.source,&fitted.model,&f.mapping,&f.keeps).unwrap();
    // This binding/end fixture is not a solid-graft authorization. Its
    // original cap crossing has an independent exact conic regression in
    // cad-predicates::tiny_finite_cap_crossing_is_not_admitted_and_bounded_inward_proposal_is_separated.
    let error=contact_graft::attach_with_charts(&f.source,&fitted.model,&f.mapping,&targets,&fitted.source_pcurves,
        1e-8,100000-fitted.cells,1000000-fitted.exact_work).err().expect("Incomplete original cap membership must refuse");
    assert_eq!(error.code,"BREP_CONTACT_GRAFT_UNPROVEN");
    assert!(error.message.contains("membership")||error.message.contains("census"));
    assert_eq!((f.source,f.patch),before);
}
#[test]
fn variable_radius_contacts_are_root_bound_capped_and_grafted_into_a_closed_body() {
    let f = fixture::build_variable_automatic();
    let before = f.source.clone();
    let out = contact_graft::attach_planned(
        &f.source,
        &f.patch,
        &f.mapping,
        &f.targets.map(|t| t.keep_vertex),
        1e-8,
        100000,
        1000000,
    )
    .unwrap();
    assert!(out.trims.all_valid);
    assert_eq!(
        out.closure.closure.status,
        brep_core::watertight::Closure::WithinTolerance
    );
    assert_eq!(out.model.bodies.len(), 1);
    assert!(out.model.shells.iter().all(|s| s.closed));
    let points = f
        .mapping
        .map(|v| out.model.vertices[out.source_vertices[v].unwrap()].point);
    assert!((points[0][0] - points[1][2]).abs() < 1e-6);
    assert!((points[3][0] - points[2][2]).abs() < 1e-6);
    assert!(points[3][0] - points[0][0] > 0.2);
    assert!(points[0][0] > 0.5 && points[3][0] < 1.5);
    let g2 = brep_core::sweep_smoothness::geometric::inspect_edges(
        &out.model,
        &[out.patch_edges[1], out.patch_edges[3]],
        brep_core::sweep_smoothness::geometric::Tolerances {
            position: f.source.tolerance_mm,
            normal: 1e-5,
            curvature: 1e-4,
        },
        100000 - out.cells,
    )
    .unwrap();
    assert!(g2.complete && g2.within_tolerance);
    assert!(out.cells + g2.cells <= 100000);
    let radius = brep_core::fillet_radius::inspect_model(
        &out.model,
        out.patch_face,
        [out.patch_edges[3], out.patch_edges[1]],
        &nurbs_core::offset::fillet_surfaces::RadiusLaw::Linear {
            start: 0.5,
            end: 1.5,
        },
        f.law_domain,
        [-1., -1.],
        1e-6,
        100000 - out.cells - g2.cells,
        1000000 - out.exact_work - g2.exact_work,
    )
    .unwrap();
    assert_eq!(
        radius.decision,
        brep_core::fillet_radius::Decision::WithinTolerance,
        "reason={} bounds={:?} cells={} positions={} curves={:?}",
        radius.reason,
        radius.maximum_center_gap_mm,
        radius.cells,
        radius.positions.len(),
        [3, 1].map(|i| {
            let c = &out.model.edges[out.patch_edges[i]].curve;
            (c.degree, c.knots.clone(), c.weights.clone())
        })
    );
    assert!(out.cells + g2.cells + radius.cells <= 100000);
    assert!(out.exact_work + g2.exact_work + radius.exact_work <= 1000000);
    assert!(radius.maximum_center_gap_mm.unwrap()[1] <= 1e-6);
    assert!(radius.parameter_drift_upper_mm.unwrap() > 0.);
    assert!(radius.parameter_drift_upper_mm.unwrap() < 1e-10);
    let renormalized = brep_core::fillet_radius::inspect_model(
        &out.model,
        out.patch_face,
        [out.patch_edges[3], out.patch_edges[1]],
        &nurbs_core::offset::fillet_surfaces::RadiusLaw::Linear {
            start: 0.5,
            end: 1.5,
        },
        out.model.edges[out.patch_edges[3]].curve.domain(),
        [-1., -1.],
        1e-6,
        100000,
        1000000,
    )
    .unwrap();
    assert_eq!(
        renormalized.decision,
        brep_core::fillet_radius::Decision::ExceedsTolerance
    );
    let saved = value_codec::to_string(&out.model).unwrap();
    let restored: brep_core::Model = value_codec::from_str(&saved).unwrap();
    restored.validate().unwrap();
    assert_eq!(restored, out.model);
    let rechecked = brep_core::fillet_radius::inspect_model(
        &restored,
        out.patch_face,
        [out.patch_edges[3], out.patch_edges[1]],
        &nurbs_core::offset::fillet_surfaces::RadiusLaw::Linear {
            start: 0.5,
            end: 1.5,
        },
        f.law_domain,
        [-1., -1.],
        1e-6,
        100000,
        1000000,
    )
    .unwrap();
    assert_eq!(
        rechecked.decision,
        brep_core::fillet_radius::Decision::WithinTolerance
    );
    assert_eq!(f.source, before);
}
#[test]
fn automatic_source_roots_and_target_faces_form_a_closed_transition() {
    let f = fixture::build_automatic();
    let out = contact_graft::attach_planned(
        &f.source,
        &f.patch,
        &f.mapping,
        &f.targets.map(|t| t.keep_vertex),
        1e-8,
        100000,
        1000000,
    )
    .unwrap();
    assert!(out.trims.all_valid);
    assert_eq!(
        out.closure.closure.status,
        brep_core::watertight::Closure::WithinTolerance
    );
    assert_eq!(out.model.bodies.len(), 1);
    assert!(out.model.shells.iter().all(|s| s.closed));
    let g2 = brep_core::sweep_smoothness::geometric::inspect_edges(
        &out.model,
        &[out.patch_edges[1], out.patch_edges[3]],
        brep_core::sweep_smoothness::geometric::Tolerances {
            position: f.source.tolerance_mm,
            normal: 1e-5,
            curvature: 1e-4,
        },
        100000 - out.cells,
    )
    .unwrap();
    assert!(g2.complete && g2.within_tolerance);
}
#[test]
fn bound_transition_replaces_corner_sectors_with_one_closed_owned_boundary() {
    let fixture = fixture::build();
    let source = &fixture.source;
    let patch = &fixture.patch;
    let mapping = fixture.mapping;
    let targets = contact_graft::plan_targets(
        source,
        patch,
        &mapping,
        &fixture.targets.map(|t| t.keep_vertex),
    )
    .unwrap();
    assert_eq!(
        targets.iter().map(|t| t.face).collect::<Vec<_>>(),
        fixture.targets.map(|t| t.face)
    );
    let before = source.clone();
    let out = contact_graft::attach_planned(
        source,
        patch,
        &mapping,
        &fixture.targets.map(|t| t.keep_vertex),
        1e-8,
        100000,
        1000000,
    )
    .unwrap();
    assert_eq!(out.model.vertices.len(), 10);
    assert_eq!(out.model.edges.len(), 15);
    assert_eq!(out.model.faces.len(), 7);
    assert!(out.model.shells.iter().all(|s| s.closed));
    assert_eq!(out.model.bodies.len(), 1);
    assert_eq!(
        out.closure.closure.status,
        brep_core::watertight::Closure::WithinTolerance
    );
    assert!(out.trims.all_valid && out.model.persistent_naming_complete());
    let g2 = brep_core::sweep_smoothness::geometric::inspect_edges(
        &out.model,
        &[out.patch_edges[1], out.patch_edges[3]],
        brep_core::sweep_smoothness::geometric::Tolerances {
            position: source.tolerance_mm,
            normal: 1e-5,
            curvature: 1e-4,
        },
        100000 - out.cells,
    )
    .unwrap();
    assert!(g2.complete && g2.within_tolerance);
    assert!(g2.cells + out.cells <= 100000);

    assert!(out.cells <= 100000 && out.exact_work <= 1000000);
    for (i, face) in source.faces.iter().enumerate() {
        assert_eq!(out.model.faces[i].surface, face.surface);
    }
    for &edge in &out.patch_edges {
        assert_eq!(
            out.model
                .loops
                .iter()
                .flat_map(|l| &l.coedges)
                .filter(|c| c.edge == edge)
                .count(),
            2
        );
    }
    for &v in &mapping {
        assert!(out.source_vertices[v].is_some());
    }
    for i in 0..source.faces.len() {
        if !out.changed_faces.contains(&i) {
            assert_eq!(out.model.1.faces[i], source.1.faces[i]);
        }
    }
    for (i, &edge) in out.patch_edges.iter().enumerate() {
        assert_eq!(out.model.1.edges[edge], patch.1.edges[i]);
    }
    assert_eq!(source, &before);
    let mut wrong = targets.clone();
    wrong[0].keep_vertex = mapping[0];
    assert!(contact_graft::attach(source, patch, &mapping, &wrong, 1e-8, 100000, 1000000).is_err());
    assert!(contact_graft::attach(source, patch, &mapping, &targets, 1e-8, 1, 1000000).is_err());
    assert_eq!(source, &before);
}
#[test]
fn automatic_target_planning_rejects_invalid_maps_and_retention_without_mutation() {
    let f = fixture::build();
    let before = f.source.clone();
    let keep = f.targets.map(|t| t.keep_vertex);
    assert!(contact_graft::plan_targets(&f.source, &f.patch, &[], &keep).is_err());
    let mut duplicate = f.mapping;
    duplicate[1] = duplicate[0];
    assert!(contact_graft::plan_targets(&f.source, &f.patch, &duplicate, &keep).is_err());
    let mut bad_keep = keep;
    bad_keep[0] = f.mapping[f.patch.edges[0].vertices[0]];
    assert!(contact_graft::plan_targets(&f.source, &f.patch, &f.mapping, &bad_keep).is_err());
    assert_eq!(f.source, before);
}
