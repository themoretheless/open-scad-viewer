#[path = "../examples/support/face_window_fixture.rs"]
mod fixture;
use brep_core::{
    face_window,
    sweep_smoothness::geometric::{self, Tolerances},
};
use nurbs_core::surface_join::Decision;
fn tolerances() -> Tolerances {
    Tolerances {
        position: 1e-7,
        normal: 1e-7,
        curvature: 1e-5,
    }
}
#[test]
fn whole_trimmed_window_seams_qualify_g2_on_original_source_pcurves() {
    let (source, face, patch, boundary) = fixture::build().unwrap();
    let built = face_window::replace(&source, face, &patch, &boundary, 1e-8, 1000000).unwrap();
    let before = built.model.clone();
    let r = geometric::inspect_edges(&built.model, &built.edges, tolerances(), 100000).unwrap();
    assert!(
        r.complete && r.within_tolerance,
        "cells={} seams={:?}",
        r.cells,
        r.seams.iter().map(|s| &s.inspection).collect::<Vec<_>>()
    );
    assert_eq!(r.seams.len(), 4);
    assert!(r.cells <= 100000 && r.exact_work <= 1000000);
    assert!(
        r.seams
            .iter()
            .all(|s| s.positions.as_ref().is_some_and(|p| p.proven))
    );
    assert_eq!(built.model, before);
}
#[test]
fn g1_and_g2_failures_remain_distinct_despite_exact_shared_positions() {
    let (source, face, patch, boundary) = fixture::build().unwrap();
    for row in [1, 2] {
        let mut broken = patch.clone();
        broken.control_points[3][row][2] -= 0.1;
        let built = face_window::replace(&source, face, &broken, &boundary, 1e-8, 1000000).unwrap();
        let r = geometric::inspect_edges(&built.model, &[built.edges[0]], tolerances(), 100000)
            .unwrap();
        let seam = r.seams[0].inspection.as_ref().unwrap();
        assert!(r.complete && !r.within_tolerance, "{seam:?}");
        assert_eq!(seam.decision, Decision::CorrespondenceExceedsTolerance);
        assert!(r.seams[0].positions.as_ref().unwrap().proven);
        if row == 2 {
            assert!(
                seam.branches[0].normal_upper.unwrap() <= tolerances().normal,
                "{seam:?}"
            );
            assert!(seam.branches[0].tensor_lower > tolerances().curvature);
        } else {
            assert!(seam.branches[0].normal_lower > tolerances().normal);
        }
    }
}
#[test]
fn missing_shape_budget_or_broken_owned_curve_never_admits_g2() {
    let (source, face, patch, boundary) = fixture::build().unwrap();
    let mut built = face_window::replace(&source, face, &patch, &boundary, 1e-8, 1000000).unwrap();
    let r = geometric::inspect_edges(&built.model, &built.edges, tolerances(), 0).unwrap();
    assert!(!r.complete && !r.within_tolerance);
    built.model.edges[built.edges[0]].curve.control_points[3][2] += 0.01;
    let r =
        geometric::inspect_edges(&built.model, &[built.edges[0]], tolerances(), 100000).unwrap();
    assert!(!r.within_tolerance);
    assert!(!r.seams[0].positions.as_ref().unwrap().proven);
}

#[test]
fn declared_degenerate_edges_do_not_receive_a_regular_seam_certificate() {
    let (source, face, patch, boundary) = fixture::build().unwrap();
    let mut built = face_window::replace(&source, face, &patch, &boundary, 1e-8, 1000000).unwrap();
    let edge = built.edges[0];
    built.model.edges[edge].degenerate = true;
    let before = built.model.clone();
    let error = geometric::inspect_edges(&built.model, &[edge], tolerances(), 100000).err().unwrap();
    assert_eq!(error.code, "BREP_INVALID_TOPOLOGY");
    assert_eq!(built.model, before);
}
