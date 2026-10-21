#[path = "../examples/support/corner_close_fixture.rs"]
mod fixture;
use brep_core::{
    corner_close,
    wall_clearance::{self, Decision},
};
fn setup() -> (brep_core::Model, usize, Vec<usize>) {
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
    (out.model, out.face, floor)
}
#[test]
fn whole_selected_opposite_set_has_a_bounded_global_minimum_and_witness() {
    let (model, face, floor) = setup();
    assert_eq!(floor.len(), 9);
    let before = model.clone();
    let r =
        wall_clearance::inspect(&model, face, &floor, 2.9, 1e-7, 1e-8, 100000, 1000000).unwrap();
    assert!(r.coverage_complete && r.pending_faces.is_empty());
    assert!(r.converged);
    assert_eq!(r.pairs.len(), 9);
    assert_eq!(r.decision, Decision::MeetsRequiredClearance);
    assert!(r.lower_bound_mm <= 3. && r.upper_bound_mm.unwrap() >= 3.);
    assert!(r.upper_bound_mm.unwrap() - r.lower_bound_mm <= 1e-7);
    let witness = &r.pairs[r.witness_pair.unwrap()];
    assert!(witness.distance.points.is_some());
    assert!(witness.distance.parameters.is_some() && witness.distance.point_enclosures.is_some());
    assert!(r.cells <= 100000 && r.domain_cells <= 1000000);
    assert_eq!(model, before);
    let thin = wall_clearance::inspect(&model, face, &[8], 3.1, 1e-7, 1e-8, 10000, 100000).unwrap();
    assert_eq!(thin.decision, Decision::BelowRequiredClearance);
}
#[test]
fn thin_closed_control_is_found_with_an_original_face_witness() {
    let (mut model, face, floor) = setup();
    for v in &mut model.vertices {
        if v.point[2] == -3. {
            v.point[2] = -0.05;
        }
    }
    for e in &mut model.edges {
        for p in &mut e.curve.control_points {
            if p[2] == -3. {
                p[2] = -0.05;
            }
        }
    }
    for f in &mut model.faces {
        for p in f.surface.control_points.iter_mut().flatten() {
            if p[2] == -3. {
                p[2] = -0.05;
            }
        }
    }
    model.rebuild_topology_ids();
    model.validate().unwrap();
    let before = model.clone();
    let r =
        wall_clearance::inspect(&model, face, &floor, 0.1, 1e-7, 1e-8, 100000, 1000000).unwrap();
    assert_eq!(r.decision, Decision::BelowRequiredClearance);
    assert!(r.lower_bound_mm <= 0.05 && r.upper_bound_mm.unwrap() >= 0.05);
    assert!(r.converged && r.coverage_complete);
    assert!(r.pairs[r.witness_pair.unwrap()].distance.points.is_some());
    assert_eq!(model, before);
}
#[test]
fn missing_coverage_and_exact_threshold_never_fabricate_a_pass() {
    let (model, face, floor) = setup();
    let zero = wall_clearance::inspect(&model, face, &floor, 2.9, 1e-7, 1e-8, 0, 0).unwrap();
    assert_eq!(zero.decision, Decision::Unresolved);
    assert_eq!(zero.lower_bound_mm, 0.);
    assert_eq!(zero.pending_faces, floor);
    assert!(zero.upper_bound_mm.is_none());
    let limited = wall_clearance::inspect(&model, face, &floor, 2.9, 1e-7, 1e-8, 1, 10).unwrap();
    assert!(!limited.coverage_complete);
    assert_eq!(limited.lower_bound_mm, 0.);
    assert_eq!(limited.decision, Decision::Unresolved);
    let at_limit =
        wall_clearance::inspect(&model, face, &[8], 3., 1e-7, 1e-8, 10000, 100000).unwrap();
    assert_eq!(at_limit.decision, Decision::Unresolved);
    assert!(
        wall_clearance::inspect(&model, face, &[8, 8], 2.9, 1e-7, 1e-8, 10000, 100000).is_err()
    );
}
