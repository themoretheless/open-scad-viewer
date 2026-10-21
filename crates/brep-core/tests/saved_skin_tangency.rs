#[path = "../examples/support/contact_graft_fixture.rs"]
mod fixture;
#[test]
fn actual_variable_saved_skin_has_g1_contacts_but_sharp_caps_remain_scoped() {
    let f = fixture::build_variable_rolling_automatic();
    let field = f.radius_field;
    let combined = brep_core::contact_graft::attach_radius_tangent_qualified(
        &f.source,
        &f.patch,
        &f.mapping,
        &f.targets,
        &[1, 3],
        [field, [0., 1., 0., 0.], field],
        field,
        1e-8,
        1e-7,
        1e-5,
        100000,
        1000000,
        2000000,
    )
    .unwrap();
    assert!(combined.tangency.within_tolerance);
    assert!(combined.cells <= 100000 && combined.exact_work <= 1000000);
    let q = combined.qualified;
    let model = &q.graft.model;
    let contacts = [q.graft.patch_edges[1], q.graft.patch_edges[3]];
    let report = brep_core::sweep_smoothness::tangent::inspect_edges(
        model, &contacts, 1e-5, 100000, 1000000,
    )
    .unwrap();
    println!(
        "saved variable G1: admitted={} cells={} exact={} bounds={:?}",
        report.within_tolerance,
        report.cells,
        report.exact_work,
        report
            .seams
            .iter()
            .map(|s| s.proof.normal_chord_upper)
            .collect::<Vec<_>>()
    );
    assert!(report.within_tolerance);
    assert_eq!(report.seams.len(), 2);
    assert!(report.cells <= 100000 && report.exact_work <= 1000000);
    let caps = [q.graft.patch_edges[0], q.graft.patch_edges[2]];
    assert!(
        !brep_core::sweep_smoothness::tangent::inspect_edges(model, &caps, 1e-5, 64, 1000000)
            .unwrap()
            .within_tolerance
    );
    assert!(
        brep_core::contact_graft::attach_radius_tangent_qualified(
            &f.source,
            &f.patch,
            &f.mapping,
            &f.targets,
            &[0, 2],
            [field, [0., 1., 0., 0.], field],
            field,
            1e-8,
            1e-7,
            1e-5,
            100000,
            1000000,
            2000000
        )
        .is_err()
    );
}
