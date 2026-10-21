use brep_core::{material_chord, ray_parity};
const RAYS: [[f64; 3]; 2] = [[1., 0.07, 0.03], [-1., 0.09, 0.02]];
#[test]
fn complete_interior_interval_needs_fresh_boundary_and_material_evidence() {
    let model = brep_core::cuboid([0.; 3], [2.; 3]).unwrap();
    let saved = model.clone();
    let r = material_chord::inspect_finite_interval(
        &model,
        [1.; 3],
        [1., 0., 0.],
        [-0.99, 0.99],
        &RAYS,
        1e-8,
        10000,
        100000,
        1000000,
    )
    .unwrap();
    assert!(
        r.volume_preconditions == Some(true)
            && r.boundary.as_ref().unwrap().proven
            && r.interval.boundary_free
    );
    assert_eq!(r.parity.as_ref().unwrap().parity, Some(true));
    assert!(r.material_interval_qualified);
    assert_eq!(r.decision, material_chord::Decision::InsideMaterialInterval);
    assert!(r.interval.length_mm[0] <= 1.98 && r.interval.length_mm[1] >= 1.98);
    assert!(r.cells <= 10000 && r.domain_cells <= 100000);
    assert_eq!(model, saved);
    let outside = material_chord::inspect_finite_interval(
        &model,
        [3.; 3],
        [1., 0., 0.],
        [-0.99, 0.99],
        &RAYS,
        1e-8,
        10000,
        100000,
        1000000,
    )
    .unwrap();
    assert_eq!(outside.decision, material_chord::Decision::OutsideMaterial);
    assert!(!outside.material_interval_qualified);
    let limited_uv = material_chord::inspect_finite_interval(
        &model,
        [3.; 3],
        [1., 0., 0.],
        [-0.99, 0.99],
        &RAYS,
        1e-8,
        10000,
        49,
        1000000,
    )
    .unwrap();
    assert_eq!(limited_uv.decision, material_chord::Decision::Unresolved);
    assert!(limited_uv.domain_cells <= 49);
    let scaled = material_chord::inspect_finite_interval(
        &model,
        [1.; 3],
        [2., 0., 0.],
        [-0.49, 0.49],
        &RAYS,
        1e-8,
        10000,
        100000,
        1000000,
    )
    .unwrap();
    assert!(scaled.material_interval_qualified);
    assert!(scaled.interval.length_mm[0] <= 1.96 && scaled.interval.length_mm[1] >= 1.96);
    let boundary_anchor = material_chord::inspect_finite_interval(
        &model,
        [0., 1., 1.],
        [1., 0., 0.],
        [-0.99, 0.99],
        &RAYS,
        1e-8,
        10000,
        100000,
        1000000,
    )
    .unwrap();
    assert_eq!(
        boundary_anchor.decision,
        material_chord::Decision::BoundaryCrossing
    );
    assert!(!boundary_anchor.material_interval_qualified);
    // Reusing a previously valid model's topology must not admit an interior
    // fold which leaves all authored boundary curves unchanged.
    let mut folded = model.clone();
    let old = folded.faces[0].surface.clone();
    let s = &mut folded.faces[0].surface;
    s.degree_u = 4;
    s.degree_v = 4;
    s.knots_u = [vec![0.; 5], vec![1.; 5]].concat();
    s.knots_v = s.knots_u.clone();
    s.weights = vec![vec![1.; 5]; 5];
    s.control_points = (0..5)
        .map(|i| {
            (0..5)
                .map(|j| {
                    let u = i as f64 / 4.;
                    let v = j as f64 / 4.;
                    (0..3)
                        .map(|k| {
                            (1. - u) * (1. - v) * old.control_points[0][0][k]
                                + (1. - u) * v * old.control_points[0][1][k]
                                + u * (1. - v) * old.control_points[1][0][k]
                                + u * v * old.control_points[1][1][k]
                        })
                        .collect()
                })
                .collect()
        })
        .collect();
    s.control_points[2][2][0] += 100.;
    folded.validate().unwrap();
    let denied = material_chord::inspect_finite_interval(
        &folded,
        [1.; 3],
        [1., 0., 0.],
        [-0.99, 0.99],
        &RAYS,
        1e-8,
        10000,
        100000,
        1000000,
    )
    .unwrap();
    assert!(!denied.material_interval_qualified);
    let small = material_chord::inspect_finite_interval(
        &model,
        [1.; 3],
        [1., 0., 0.],
        [-0.99, 0.99],
        &RAYS,
        1e-8,
        4,
        4,
        0,
    )
    .unwrap();
    assert!(!small.material_interval_qualified);
    let ends = material_chord::inspect_interval(
        &model,
        [1.; 3],
        [1., 0., 0.],
        [-1., 1.],
        1e-8,
        1000,
        10000,
    )
    .unwrap();
    assert!(!ends.boundary_free && !ends.unresolved.is_empty());
}
#[test]
fn an_inside_anchor_does_not_certify_a_chord_crossing_a_cavity() {
    let outer = brep_core::cuboid([0.; 3], [10.; 3]).unwrap();
    let inner = brep_core::cuboid([2.; 3], [8.; 3]).unwrap();
    let model = brep_core::operations::boolean(&outer, &inner, "difference").unwrap();
    let point = [1., 4., 5.];
    assert_eq!(
        ray_parity::classify_point(&model, point, &RAYS, 1e-8, 10000, 100000)
            .unwrap()
            .parity,
        Some(true)
    );
    let across = material_chord::inspect_finite_interval(
        &model,
        point,
        [1., 0., 0.],
        [-0.99, 8.99],
        &RAYS,
        1e-8,
        10000,
        100000,
        1000000,
    )
    .unwrap();
    assert!(!across.material_interval_qualified && !across.interval.boundary_free);
    assert_eq!(across.interval.crossings.len(), 2);
    assert_eq!(across.decision, material_chord::Decision::BoundaryCrossing);
    assert!(across.boundary.is_none() && across.volume_preconditions.is_none());
    assert!(across.cells < 1000);
    let wall = material_chord::inspect_finite_interval(
        &model,
        point,
        [1., 0., 0.],
        [-0.99, 0.99],
        &RAYS,
        1e-8,
        100000,
        1000000,
        1000000,
    )
    .unwrap();
    assert!(wall.interval.boundary_free);
    assert!(wall.trim_regions.as_ref().unwrap().all_valid);
    assert!(
        wall.closure
            .as_ref()
            .is_some_and(|c| c.exact.all_equal && c.exact.all_joins_exact)
    );
    assert!(wall.boundary.as_ref().unwrap().proven);
    assert_eq!(
        wall.boundary
            .as_ref()
            .unwrap()
            .intersections
            .pairs
            .total_pairs,
        1770
    );
    assert!(
        wall.boundary
            .as_ref()
            .unwrap()
            .intersections
            .pairs
            .next_pair
            .is_none()
    );
    assert!(wall.material_interval_qualified);
    assert!(wall.cells <= 100000 && wall.domain_cells <= 1000000);
    assert!(
        wall.closure.as_ref().unwrap().exact.work + wall.boundary.as_ref().unwrap().agreement.work
            <= 1000000
    );
    assert_eq!(
        wall.decision,
        material_chord::Decision::InsideMaterialInterval
    );
    let void = material_chord::inspect_finite_interval(
        &model,
        [5., 4., 5.],
        [1., 0., 0.],
        [-0.99, 0.99],
        &RAYS,
        1e-8,
        10000,
        100000,
        1000000,
    )
    .unwrap();
    assert!(void.interval.boundary_free);
    assert_eq!(
        ray_parity::classify_point(&model, [5., 4., 5.], &RAYS, 1e-8, 10000, 100000)
            .unwrap()
            .parity,
        Some(false)
    );
    assert!(!void.material_interval_qualified);
    assert!(void.cells <= 10000 && void.domain_cells <= 100000);
    assert_eq!(void.decision, material_chord::Decision::OutsideMaterial,
        "cells={} domains={} volume={:?} trim={:?} parity={:?} boundary={:?}",void.cells,void.domain_cells,void.volume_preconditions,
        void.trim_regions.as_ref().map(|r|r.all_valid),void.parity.as_ref().map(|r|r.parity),
        void.boundary.as_ref().map(|b|(b.proven,b.agreement.all_equal,b.agreement.all_joins_exact,b.trim.all_valid,b.intersections.faces.all_faces_injective,b.intersections.pairs.all_pairs_classified,b.intersections.pairs.cells,b.intersections.pairs.next_pair)));
}
