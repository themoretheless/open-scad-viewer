#[path = "../examples/support/corner_close_fixture.rs"]
mod fixture;
use brep_core::{
    corner_close,
    wall_opposition::{self, Decision},
};
#[test]
fn shell_orientation_and_trimmed_normals_identify_the_complete_floor_set() {
    let (source, uses, options) = fixture::fixture_extended();
    let out = corner_close::close(
        &source,
        0,
        uses,
        options,
        1e-8,
        [1e-7, 1e-5, 1e-4],
        100000,
        1000000,
    )
    .unwrap();
    let floor = out
        .model
        .faces
        .iter()
        .enumerate()
        .filter(|(_, f)| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == -3.)
        })
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    let before = out.model.clone();
    let r =
        wall_opposition::inspect(&out.model, 0, out.face, -0.5, 1e-8, 100000 - out.cells).unwrap();
    assert!(r.selection_complete);
    assert_eq!(r.opposing_faces, floor);
    assert_eq!(r.candidate_faces, floor);
    assert_eq!(r.excluded_faces.len(), 20);
    assert_eq!(r.faces.len(), 29);
    assert!(r.cells + out.cells <= 100000);
    for f in &r.faces {
        let b = f.dot_bounds.unwrap();
        match f.decision {
            Decision::Opposing => assert!(b[1] <= -0.5),
            Decision::NonOpposing => assert!(b[0] > -0.5),
            Decision::Unresolved => panic!("unexpected unknown"),
        }
    }
    assert_eq!(out.model, before);
    let (a, b) = (0.37_f64, 0.61_f64);
    let (ca, sa) = (a.cos(), a.sin());
    let (cb, sb) = (b.cos(), b.sin());
    let rotated = brep_core::transform::affine(
        &out.model,
        [
            [ca * cb, -sa, ca * sb, 3.],
            [sa * cb, ca, sa * sb, -2.],
            [-sb, 0., cb, 5.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let rotation = wall_opposition::inspect(&rotated, 0, out.face, -0.5, 1e-8, 100000).unwrap();
    assert!(rotation.selection_complete);
    assert_eq!(rotation.candidate_faces, floor);
    let automatic = brep_core::wall_clearance::inspect_shell(
        &out.model, 0, out.face, -0.5, 2.9, 1e-7, 1e-8, 100000, 1000000,
    )
    .unwrap();
    assert_eq!(automatic.selection.candidate_faces, floor);
    assert!(automatic.selection.selection_complete);
    let clearance = automatic.candidate_clearance.unwrap();
    assert_eq!(
        clearance.decision,
        brep_core::wall_clearance::Decision::MeetsRequiredClearance
    );
    assert!(clearance.lower_bound_mm <= 3. && clearance.upper_bound_mm.unwrap() >= 3.);
    assert!(automatic.cells <= 100000 && automatic.domain_cells <= 1000000);
}
#[test]
fn unresolved_normals_stay_candidates_and_never_disappear_with_zero_work() {
    let m = brep_core::cuboid([0.; 3], [2., 3., 4.]).unwrap();
    let r = wall_opposition::inspect(&m, 0, 0, -0.5, 1e-8, 0).unwrap();
    assert!(!r.selection_complete && r.opposing_faces.is_empty() && r.excluded_faces.is_empty());
    assert_eq!(r.candidate_faces.len(), 5);
    assert_eq!(r.cells, 0);
    let automatic =
        brep_core::wall_clearance::inspect_shell(&m, 0, 0, -0.5, 1., 1e-7, 1e-8, 0, 0).unwrap();
    assert!(!automatic.selection.selection_complete && automatic.candidate_clearance.is_none());
    assert!(
        r.faces
            .iter()
            .all(|f| f.decision == Decision::Unresolved && f.dot_bounds.is_none())
    );
    assert!(wall_opposition::inspect(&m, 0, 0, f64::NAN, 1e-8, 100).is_err());
}

#[path = "../examples/support/contact_graft_fixture.rs"]
mod rolling_fixture;
#[test]
fn multispan_skin_normals_exclude_joined_faces_and_caps_without_claiming_wall_pairing() {
    let f = rolling_fixture::build_variable_rolling_automatic();
    let field = f.radius_field;
    let q = brep_core::contact_graft::attach_radius_tangent_qualified(
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
    let model = &q.qualified.graft.model;
    let source = q.qualified.graft.patch_face;
    let expected = model
        .faces
        .iter()
        .enumerate()
        .filter(|(_, f)| {
            [0, 2].iter().any(|&k| {
                f.surface
                    .control_points
                    .iter()
                    .flatten()
                    .all(|p| p[k] == 10.)
            })
        })
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    let selection = wall_opposition::inspect(model, 0, source, -0.5, 1e-8, 25000).unwrap();
    assert_eq!(selection.candidate_faces, expected);
    assert_eq!(selection.excluded_faces.len(), 4);
    assert!(!selection.selection_complete);
    let clearance = brep_core::wall_clearance::inspect(
        model,
        source,
        &selection.candidate_faces,
        1.,
        1e-7,
        1e-8,
        2000,
        20000,
    )
    .unwrap();
    assert!(clearance.coverage_complete);
    assert_eq!(
        clearance.decision,
        brep_core::wall_clearance::Decision::MeetsRequiredClearance
    );
    assert!(clearance.lower_bound_mm > 8. && clearance.upper_bound_mm.is_some());
    assert!(
        q.cells + selection.cells + clearance.cells <= 100000 && clearance.domain_cells <= 20000
    );
}
