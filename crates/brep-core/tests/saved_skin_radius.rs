#[path = "../examples/support/contact_graft_fixture.rs"]
mod fixture;
use brep_core::{contact_graft, fillet_radius};
#[test]
fn circular_saved_skin_keeps_radius_after_graft_and_reload() {
    let fixture = fixture::build_rolling_automatic();
    let center = [[0., 0., 0., 1.5], [0., 1., 0., 0.], [0., 0., 0., 1.5]];
    let radius = [0., 0., 0., 1.5];
    let qualified = contact_graft::attach_radius_qualified(
        &fixture.source,
        &fixture.patch,
        &fixture.mapping,
        &fixture.targets,
        center,
        radius,
        1e-8,
        1e-7,
        100000,
        1000000,
        2000000,
    )
    .unwrap();
    assert!(qualified.radius.within_tolerance);
    let graft = qualified.graft;
    let saved = graft.model.clone();
    let center = [[0., 0., 0., 1.5], [0., 1., 0., 0.], [0., 0., 0., 1.5]];
    let radius = [0., 0., 0., 1.5];
    let proof = fillet_radius::inspect_saved_skin(
        &graft.model,
        graft.patch_face,
        center,
        radius,
        1e-7,
        2000000,
    )
    .unwrap();
    println!(
        "saved skin radius: work={} error={:?}",
        proof.coefficient_work, proof.radial_error_upper_mm
    );
    assert!(proof.within_tolerance);
    let restored: brep_core::Model =
        value_codec::from_str(&value_codec::to_string(&graft.model).unwrap()).unwrap();
    assert!(
        fillet_radius::inspect_saved_skin(
            &restored,
            graft.patch_face,
            center,
            radius,
            1e-7,
            2000000
        )
        .unwrap()
        .within_tolerance
    );
    let mut changed = restored;
    changed.faces[graft.patch_face].surface.control_points[1][1][0] += 0.01;
    assert!(
        fillet_radius::inspect_saved_skin(
            &changed,
            graft.patch_face,
            center,
            radius,
            1e-7,
            2000000
        )
        .map_or(true, |report| !report.within_tolerance)
    );
    assert_eq!(graft.model, saved);
}
#[test]
fn nominal_g2_setback_is_not_mistaken_for_circular_skin_radius() {
    let f = fixture::build();
    let graft = contact_graft::attach(
        &f.source, &f.patch, &f.mapping, &f.targets, 1e-8, 100000, 1000000,
    )
    .unwrap();
    let proof = fillet_radius::inspect_saved_skin(
        &graft.model,
        graft.patch_face,
        [[0., 0., 0., 1.5], [0., 1., 0., 0.], [0., 0., 0., 1.5]],
        [0., 0., 0., 1.5],
        1e-7,
        2000000,
    )
    .unwrap();
    assert!(!proof.within_tolerance);
    assert!(
        contact_graft::attach_radius_qualified(
            &f.source,
            &f.patch,
            &f.mapping,
            &f.targets,
            [[0., 0., 0., 1.5], [0., 1., 0., 0.], [0., 0., 0., 1.5]],
            [0., 0., 0., 1.5],
            1e-8,
            1e-7,
            100000,
            1000000,
            2000000
        )
        .is_err()
    );
}

#[test]
fn variable_rolling_skin_keeps_complete_prescribed_radius_after_owned_graft() {
    let f = fixture::build_variable_rolling_automatic();
    let field = f.radius_field;
    let center = [field, [0., 1., 0., 0.], field];
    let qualified = contact_graft::attach_radius_qualified(
        &f.source, &f.patch, &f.mapping, &f.targets, center, field, 1e-8, 1e-7, 100000, 1000000,
        2000000,
    )
    .unwrap();
    assert!(qualified.radius.within_tolerance);
    let encoded = value_codec::to_string(&qualified.graft.model).unwrap();
    let restored: brep_core::Model = value_codec::from_str(&encoded).unwrap();
    let fresh = fillet_radius::inspect_saved_skin(
        &restored,
        qualified.graft.patch_face,
        center,
        field,
        1e-7,
        2000000,
    )
    .unwrap();
    assert!(fresh.within_tolerance);
    let r0 = field[1] * 2. + field[3];
    let r1 = field[1] * 8. + field[3];
    assert!((r0 - r1).abs() > 0.5);
    assert!((r0-0.7).abs()<1e-12 && (r1-1.3).abs()<1e-12);
    println!(
        "variable saved skin: endpoints={r0},{r1} error={:?} work={}",
        fresh.radial_error_upper_mm, fresh.coefficient_work
    );
    let mut wrong = field;
    wrong[3] += 0.01;
    assert!(
        !fillet_radius::inspect_saved_skin(
            &restored,
            qualified.graft.patch_face,
            center,
            wrong,
            1e-7,
            2000000
        )
        .unwrap()
        .within_tolerance
    );
}
