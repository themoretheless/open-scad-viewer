use nurbs_core::{curve::Curve, periodic_trim::split_candidates};
#[test]
fn rational_crossing_retains_traversal_and_distinct_seam_representatives() {
    let c = Curve {
        degree: 1,
        knots: vec![2., 2., 8., 8.],
        control_points: vec![vec![0.5, 0.2], vec![1.5, 0.8]],
        weights: vec![1., 3.],
        periodic: false,
    };
    // Rational midpoint x=1 occurs at normalized t=1/4, not t=1/2.
    let r = split_candidates(&c, [[0., 1.]; 2], &[3.5], &[[0, 0], [1, 0]], 2).unwrap();
    assert!(r.candidate_hulls_contained);
    assert!((r.pieces[0].curve.evaluate(3.5).unwrap().point[0] - 1.).abs() < 1e-14);
    assert!(r.pieces[1].curve.evaluate(3.5).unwrap().point[0].abs() < 1e-14);
    for part in &r.pieces {
        for i in 0..=30 {
            let t = part.source_domain[0]
                + (part.source_domain[1] - part.source_domain[0]) * i as f64 / 30.;
            let a = c.evaluate(t).unwrap().point;
            let b = part.curve.evaluate(t).unwrap().point;
            for axis in 0..2 {
                assert!((a[axis] - b[axis] - part.translation[axis]).abs() < 1e-13);
            }
        }
    }
}
#[test]
fn multiple_negative_tiles_and_uncontained_candidates_are_explicit() {
    let c = Curve::from_polyline(vec![vec![-0.5, 0.5], vec![2.5, 0.5]]).unwrap();
    let r = split_candidates(
        &c,
        [[0., 1.]; 2],
        &[1. / 6., 0.5, 5. / 6.],
        &[[-1, 0], [0, 0], [1, 0], [2, 0]],
        4,
    )
    .unwrap();
    assert_eq!(r.pieces.len(), 4);
    let bad = split_candidates(&c, [[0., 1.]; 2], &[], &[[0, 0]], 1).unwrap();
    assert!(!bad.candidate_hulls_contained);
    assert!(split_candidates(&c, [[0., 1.]; 2], &[0.5, 0.5], &[[0, 0]; 3], 3).is_err());
    assert!(split_candidates(&c, [[0., 1.]; 2], &[0.5], &[[0, 0]; 2], 1).is_err());
    assert!(split_candidates(&c, [[0., 0.]; 2], &[], &[[0, 0]], 1).is_err());
    assert!(split_candidates(&c, [[0., 1.]; 2], &[f64::NAN], &[[0, 0]; 2], 2).is_err());
}

#[test]
fn automatic_seam_search_preserves_certified_rational_root_interval() {
    use nurbs_core::periodic_trim::{SearchStatus, inspect_seams};
    let c = Curve {
        degree: 1,
        knots: vec![2., 2., 8., 8.],
        control_points: vec![vec![0.5, 0.2], vec![1.5, 0.8]],
        weights: vec![1., 3.],
        periodic: false,
    };
    let r = inspect_seams(&c, [[0., 1.]; 2], 1e-8, 32, 1000).unwrap();
    assert_eq!(r.status, SearchStatus::Complete);
    assert_eq!(r.total_seams, r.seams.len());
    let events: Vec<_> = r.seams.iter().flat_map(|s| s.roots.root_events()).collect();
    assert_eq!(events.len(), 1);
    let b = events[0].parameter_bounds;
    assert!(b[0] <= 3.5 && 3.5 <= b[1] && b[1] - b[0] <= 1e-8);
    assert!(r.work <= 1000);
    assert_eq!(
        inspect_seams(&c, [[0., 1.]; 2], 1e-8, 1, 1000)
            .unwrap()
            .status,
        SearchStatus::WorkLimit
    );
    let limited = inspect_seams(&c, [[0., 1.]; 2], 1e-8, 32, 1).unwrap();
    assert_eq!(limited.status, SearchStatus::WorkLimit);
    assert!(limited.seams.len() < limited.total_seams && limited.work <= 1);
}
#[test]
fn seam_coincidence_is_not_an_isolated_cut() {
    use nurbs_core::curve_plane::Classification;
    use nurbs_core::periodic_trim::{SearchStatus, inspect_seams};
    let c = Curve::from_polyline(vec![vec![1., 0.2], vec![1., 0.8]]).unwrap();
    let r = inspect_seams(&c, [[0., 1.]; 2], 1e-8, 32, 1000).unwrap();
    assert_eq!(r.status, SearchStatus::Unresolved);
    assert!(r.seams.iter().any(|s| {
        s.roots
            .cells
            .iter()
            .any(|cell| cell.classification == Classification::Coincident)
    }));
    assert!(inspect_seams(&c, [[0., 1.]; 2], 0., 32, 1000).is_err());
}

#[test]
fn exact_tangent_seam_event_is_retained_without_guessing_a_crossing() {
    use nurbs_core::curve_plane::Classification;
    use nurbs_core::periodic_trim::{SearchStatus, inspect_seams};
    // x(t)=1+(t-1/2)^2: touches x=1 at t=1/2 and stays in tile 1.
    let c = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![1.25, 0.2], vec![0.75, 0.5], vec![1.25, 0.8]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let r = inspect_seams(&c, [[0., 1.]; 2], 1e-8, 32, 2000).unwrap();
    assert_eq!(r.status, SearchStatus::Complete);
    let seam = r
        .seams
        .iter()
        .find(|s| s.axis == 0 && s.coordinate == 1.)
        .unwrap();
    assert!(
        seam.roots
            .cells
            .iter()
            .any(|cell| cell.classification == Classification::TangencyAtMidpoint)
    );
    assert_eq!(seam.roots.root_events()[0].parameter_bounds, [0.5, 0.5]);
}

#[test]
fn original_curve_admission_includes_translation_and_rejects_damage() {
    use nurbs_core::periodic_trim::{AdmissionStatus, verify_partition};
    let c = Curve {
        degree: 1,
        knots: vec![2., 2., 8., 8.],
        control_points: vec![vec![0.5, 0.2], vec![1.5, 0.8]],
        weights: vec![1., 3.],
        periodic: false,
    };
    let mut r = split_candidates(&c, [[0., 1.]; 2], &[3.5], &[[0, 0], [1, 0]], 2).unwrap();
    let a = verify_partition(&c, &r, 0.01, 10000).unwrap();
    assert_eq!(a.status, AdmissionStatus::WithinTolerance);
    assert!(a.error_upper <= 0.01 && a.work <= 10000);
    assert_eq!(
        verify_partition(&c, &r, 1e-12, 1).unwrap().status,
        AdmissionStatus::Unresolved
    );
    r.pieces[1].translation[1] += 10.;
    assert_eq!(
        verify_partition(&c, &r, 0.01, 10000).unwrap().status,
        AdmissionStatus::Mismatch
    );
    r.pieces[1].source_domain[0] += 0.01;
    assert!(verify_partition(&c, &r, 0.01, 10000).is_err());
}

#[test]
fn automatic_verified_split_owns_authored_seam_and_preserves_order() {
    use nurbs_core::periodic_trim::{SplitStatus, split_verified};
    let c = Curve::from_polyline(vec![vec![0.5, 0.25], vec![1., 0.5], vec![1.5, 0.75]]).unwrap();
    let r = split_verified(&c, [[0., 1.]; 2], 1e-8, 0.01, 8, 32, 2000, 10000).unwrap();
    assert_eq!(r.status, SplitStatus::VerifiedUvPartition, "{r:?}");
    let partition = r.partition.unwrap();
    assert_eq!(partition.pieces.len(), 2);
    assert_eq!(partition.pieces[0].tile, [0, 0]);
    assert_eq!(partition.pieces[1].tile, [1, 0]);
    assert_eq!(
        partition.pieces[0].source_domain[1],
        partition.pieces[1].source_domain[0]
    );
    assert!(r.admission.unwrap().error_upper <= 0.01);
}
#[test]
fn automatic_split_never_publishes_unsupported_or_unproved_partition() {
    use nurbs_core::periodic_trim::{SplitStatus, split_verified};
    let c = Curve::from_polyline(vec![vec![0.5, 0.25], vec![1., 0.5], vec![1.5, 0.75]]).unwrap();
    let low = split_verified(&c, [[0., 1.]; 2], 1e-8, 1e-12, 8, 32, 2000, 1).unwrap();
    assert_eq!(low.status, SplitStatus::DeviationUnresolved);
    assert!(low.partition.is_none());
    let small = split_verified(&c, [[0., 1.]; 2], 1e-8, 0.01, 1, 32, 2000, 10000).unwrap();
    assert_eq!(small.status, SplitStatus::PieceLimit);
    assert!(small.partition.is_none());
    let lattice = split_verified(&c, [[0., 0.3]; 2], 1e-8, 0.01, 8, 32, 2000, 10000).unwrap();
    assert_eq!(lattice.status, SplitStatus::LatticeRoundingUnresolved);
    assert!(lattice.partition.is_none());
}

#[test]
fn shifted_non_power_of_two_lattice_uses_chart_origin_for_tile_ownership() {
    use nurbs_core::periodic_trim::{SplitStatus, split_verified};
    let c = Curve::from_polyline(vec![vec![3.5, 4.25], vec![5., 4.5], vec![6.5, 4.75]]).unwrap();
    // Period U=3, nonzero origins on both axes, crossing exactly at x=5.
    let r = split_verified(&c, [[2., 5.], [4., 5.]], 1e-8, 0.01, 8, 32, 2000, 20000).unwrap();
    assert_eq!(r.status, SplitStatus::VerifiedUvPartition, "{r:?}");
    let p = r.partition.unwrap();
    assert_eq!(p.pieces.len(), 2);
    assert_eq!(p.pieces[0].tile, [0, 0]);
    assert_eq!(p.pieces[1].tile, [1, 0]);
    assert_eq!(p.pieces[1].translation, [3., 0.]);
    assert_eq!(
        p.pieces[1]
            .curve
            .evaluate(p.pieces[1].source_domain[0])
            .unwrap()
            .point[0],
        2.
    );
}

#[test]
fn rational_interval_event_is_resolved_only_by_exact_original_residual() {
    use nurbs_core::periodic_trim::{SplitStatus, split_verified};
    let c = Curve {
        degree: 1,
        knots: vec![2., 2., 8., 8.],
        control_points: vec![vec![0.5, 0.2], vec![1.5, 0.8]],
        weights: vec![1., 3.],
        periodic: false,
    };
    let r = split_verified(&c, [[0., 1.]; 2], 1e-8, 0.01, 8, 32, 2000, 10000).unwrap();
    assert_eq!(r.status, SplitStatus::VerifiedUvPartition, "{r:?}");
    let p = r.partition.unwrap();
    assert_eq!(p.pieces.len(), 2);
    assert_eq!(p.pieces[0].source_domain[1], 3.5);
    assert_eq!(r.resolved_events.len(), 1);
    assert_eq!(r.resolved_events[0].parameter, 3.5);
    assert!(r.parameter_attempts > 0 && r.parameter_attempts <= 3);
}
#[test]
fn nonrepresentable_root_is_not_snapped_into_a_verified_partition() {
    use nurbs_core::periodic_trim::{SplitStatus, split_verified};
    // The first seam x=1 is at t=1/3, not a binary64 parameter.
    let c = Curve::from_polyline(vec![vec![0., 0.25], vec![3., 0.75]]).unwrap();
    let r = split_verified(&c, [[0., 1.]; 2], 1e-8, 0.01, 8, 32, 2000, 10000).unwrap();
    assert_eq!(r.status, SplitStatus::IntervalRootNeedsConstruction);
    assert!(r.partition.is_none());
    assert!(r.parameter_attempts <= 3);
}

#[test]
fn original_quadratic_basis_proves_noncentral_seam_root() {
    use nurbs_core::periodic_trim::{SplitStatus, split_verified};
    // x(q)=15/16+q², q=(t-2)/6: seam x=1 at t=3.5.
    let c = Curve {
        degree: 2,
        knots: vec![2., 2., 2., 8., 8., 8.],
        control_points: vec![vec![0.9375, 0.25], vec![0.9375, 0.5], vec![1.9375, 0.75]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let r = split_verified(&c, [[0., 1.]; 2], 1e-8, 0.01, 8, 32, 10000, 20000).unwrap();
    assert_eq!(r.status, SplitStatus::VerifiedUvPartition, "{r:?}");
    let p = r.partition.unwrap();
    assert_eq!(p.pieces.len(), 2);
    assert_eq!(p.pieces[0].source_domain[1], 3.5);
    assert_eq!(p.pieces[1].tile, [1, 0]);
}

#[test]
fn exact_lifted_connectivity_checks_periodic_winding_without_snapping() {
    use nurbs_core::periodic_trim::{LinkStatus, inspect_connectivity};
    let c = Curve::from_polyline(vec![vec![0.5, 0.25], vec![1., 0.5], vec![1.5, 0.25]]).unwrap();
    let mut p = split_candidates(&c, [[0., 1.]; 2], &[c.knots[2]], &[[0, 0], [1, 0]], 2).unwrap();
    assert!(p.candidate_hulls_contained);
    let open = inspect_connectivity(&p, [[0., 1.]; 2], None, 2).unwrap();
    assert_eq!(open.status, LinkStatus::Exact);
    assert_eq!(open.links.len(), 1);
    let closed = inspect_connectivity(&p, [[0., 1.]; 2], Some([1, 0]), 2).unwrap();
    assert_eq!(closed.status, LinkStatus::Exact);
    assert!(closed.links[1].closure);
    assert_eq!(
        inspect_connectivity(&p, [[0., 1.]; 2], Some([0, 0]), 2)
            .unwrap()
            .status,
        LinkStatus::Gap
    );
    let limited = inspect_connectivity(&p, [[0., 1.]; 2], Some([1, 0]), 1).unwrap();
    assert_eq!(limited.status, LinkStatus::Unresolved);
    assert_eq!(limited.links.len(), 1);
    p.pieces[1].translation[1] = f64::from_bits(1);
    assert_eq!(
        inspect_connectivity(&p, [[0., 1.]; 2], None, 2)
            .unwrap()
            .status,
        LinkStatus::Gap
    );
}

#[test]
fn exact_partition_identity_covers_original_curve_without_a_tolerance() {
    use nurbs_core::periodic_trim::verify_identity;
    let c = Curve::from_polyline(vec![vec![0.5, 0.25], vec![1., 0.5], vec![1.5, 0.75]]).unwrap();
    let mut p = split_candidates(&c, [[0., 1.]; 2], &[c.knots[2]], &[[0, 0], [1, 0]], 2).unwrap();
    assert!(p.candidate_hulls_contained);
    let identity = verify_identity(&c, &p, 8).unwrap();
    assert!(identity.exact, "{identity:?}");
    assert_eq!(identity.compared_segments, 2);
    assert_eq!(
        verify_identity(&c, &p, 1).unwrap().reason,
        Some("segment-budget")
    );
    p.pieces[1].curve.control_points[0][1] = 0.5f64.next_up();
    assert!(!verify_identity(&c, &p, 8).unwrap().exact);
}

#[test]
fn nonrepresentable_root_candidates_require_continuous_uv_admission() {
    use nurbs_core::periodic_trim::{LinkStatus, SplitStatus, split_approximated, verify_identity};
    let c = Curve::from_polyline(vec![vec![0., 0.25], vec![3., 0.75]]).unwrap();
    let r = split_approximated(&c, [[0., 1.]; 2], 1e-8, 0.005, 8, 32, 2000, 30000).unwrap();
    assert_eq!(r.status, SplitStatus::VerifiedUvPartition, "{r:?}");
    assert_eq!(r.approximated_events.len(), 2);
    assert_eq!(r.connectivity.unwrap().status, LinkStatus::Exact);
    assert!(r.admission.unwrap().error_upper <= 0.005);
    let p = r.partition.unwrap();
    assert_eq!(p.pieces.len(), 3);
    assert!(p.candidate_hulls_contained);
    assert!(!verify_identity(&c, &p, 16).unwrap().exact);
    for i in 0..2 {
        let left = &p.pieces[i];
        let right = &p.pieces[i + 1];
        assert_eq!(left.curve.control_points.last().unwrap()[0], 1.);
        assert_eq!(right.curve.control_points[0][0], 0.);
    }
    let low = split_approximated(&c, [[0., 1.]; 2], 1e-8, 0.005, 8, 32, 2000, 1).unwrap();
    assert_eq!(low.status, SplitStatus::DeviationUnresolved);
    assert!(low.partition.is_none());
}
#[test]
fn overlapping_interval_events_are_not_assumed_to_share_a_corner() {
    use nurbs_core::periodic_trim::{SplitStatus, split_approximated};
    let c = Curve::from_polyline(vec![vec![0., 0.], vec![3., 3.]]).unwrap();
    let r = split_approximated(&c, [[0., 1.]; 2], 1e-8, 0.005, 8, 32, 2000, 30000).unwrap();
    assert_eq!(r.status, SplitStatus::EventOrderUnresolved);
    assert!(r.partition.is_none());
}

#[test]
fn discontinuous_uv_input_is_not_treated_as_a_contour() {
    let c = Curve {
        degree: 1,
        knots: vec![0., 0., 0.5, 0.5, 1., 1.],
        control_points: vec![
            vec![0., 0.25],
            vec![0.5, 0.5],
            vec![1.5, 0.5],
            vec![2., 0.75],
        ],
        weights: vec![1.; 4],
        periodic: false,
    };
    assert!(c.validate().is_err());
    assert!(split_candidates(&c, [[0., 1.]; 2], &[0.5], &[[0, 0], [1, 0]], 2).is_err());
}

#[test]
fn exact_region_transfer_preserves_outer_loop_and_oriented_hole() {
    use nurbs_core::periodic_trim::verify_lifted_region;
    let outer = Curve::from_polyline(vec![
        vec![0.5, 0.125],
        vec![1., 0.125],
        vec![1.5, 0.125],
        vec![1.5, 0.875],
        vec![1., 0.875],
        vec![0.5, 0.875],
        vec![0.5, 0.125],
    ])
    .unwrap();
    let hole = Curve::from_polyline(vec![
        vec![0.75, 0.25],
        vec![0.75, 0.75],
        vec![1., 0.75],
        vec![1.25, 0.75],
        vec![1.25, 0.25],
        vec![1., 0.25],
        vec![0.75, 0.25],
    ])
    .unwrap();
    let outer_p = split_candidates(
        &outer,
        [[0., 1.]; 2],
        &[outer.knots[2], outer.knots[5]],
        &[[0, 0], [1, 0], [0, 0]],
        3,
    )
    .unwrap();
    let hole_p = split_candidates(
        &hole,
        [[0., 1.]; 2],
        &[hole.knots[3], hole.knots[6]],
        &[[0, 0], [1, 0], [0, 0]],
        3,
    )
    .unwrap();
    let sources = [outer, hole];
    let mut partitions = [outer_p, hole_p];
    let r = verify_lifted_region(&sources, &partitions, [[0., 1.]; 2], 32, 16).unwrap();
    assert!(
        r.lifted_winding_preserved && r.periodic_winding_preserved,
        "{r:?}"
    );
    assert_eq!(r.loops.len(), 2);
    assert_eq!(r.compared_segments, 12);
    assert_eq!(r.links, 6);
    assert!(
        !verify_lifted_region(&sources, &partitions, [[0., 1.]; 2], 32, 1)
            .unwrap()
            .lifted_winding_preserved
    );
    assert!(
        !verify_lifted_region(&sources, &partitions, [[0., 1.]; 2], 1, 16)
            .unwrap()
            .lifted_winding_preserved
    );
    partitions[1].pieces[1].tile[0] = 0;
    let wrong_tile = verify_lifted_region(&sources, &partitions, [[0., 1.]; 2], 32, 16).unwrap();
    assert!(!wrong_tile.periodic_winding_preserved);
    assert_eq!(wrong_tile.reason, Some("periodic-translation-unproven"));
    partitions[1].pieces[1].tile[0] = 1;
    partitions[1].pieces[1].curve.control_points[1][0] += 0.125;
    let damaged = verify_lifted_region(&sources, &partitions, [[0., 1.]; 2], 32, 16).unwrap();
    assert!(!damaged.lifted_winding_preserved);
    assert_eq!(damaged.reason, Some("path-identity-unproven"));
}
#[test]
fn lifted_region_refuses_open_loop_and_partial_loop_collection() {
    use nurbs_core::periodic_trim::verify_lifted_region;
    let c = Curve::from_polyline(vec![vec![0.25, 0.25], vec![0.75, 0.75]]).unwrap();
    let p = split_candidates(&c, [[0., 1.]; 2], &[], &[[0, 0]], 1).unwrap();
    let r = verify_lifted_region(&[c.clone()], &[p], [[0., 1.]; 2], 16, 8).unwrap();
    assert!(!r.lifted_winding_preserved);
    assert_eq!(r.reason, Some("source-closure-unproven"));
    assert!(verify_lifted_region(&[c], &[], [[0., 1.]; 2], 16, 8).is_err());
}

#[test]
fn rational_circle_full_knot_cuts_preserve_authored_controls_and_region() {
    use nurbs_core::periodic_trim::{
        SplitStatus, split_verified, verify_identity, verify_lifted_region,
    };
    let mut circle = nurbs_core::primitives::circle([2.5, 1., 0.], [0., 0., 1.], 1.).unwrap();
    for p in &mut circle.control_points {
        p.truncate(2);
    }
    let before = circle.clone();
    let chart = [[0., 2.5], [0., 4.]];
    let r = split_verified(&circle, chart, 1e-7, 0.02, 16, 32, 20000, 30000).unwrap();
    assert_eq!(r.status, SplitStatus::VerifiedUvPartition, "{r:?}");
    let partition = r.partition.unwrap();
    assert_eq!(partition.pieces.len(), 3);
    assert_eq!(partition.pieces[0].curve.weights, circle.weights[..3]);
    assert_eq!(partition.pieces[0].curve.control_points[1], [1., 2.]);
    let identity = verify_identity(&circle, &partition, 16).unwrap();
    assert!(identity.exact, "{identity:?}");
    assert_eq!(identity.compared_segments, 4);
    let region = verify_lifted_region(&[circle.clone()], &[partition], chart, 16, 16).unwrap();
    assert!(region.periodic_winding_preserved, "{region:?}");
    assert_eq!(circle, before);
}

#[test]
fn single_seam_chart_assembly_preserves_hole_and_closes_all_cycles() {
    use nurbs_core::periodic_trim::assemble_single_seam;
    let outer = Curve::from_polyline(vec![
        vec![0.5, 0.125],
        vec![1., 0.125],
        vec![1.5, 0.125],
        vec![1.5, 0.875],
        vec![1., 0.875],
        vec![0.5, 0.875],
        vec![0.5, 0.125],
    ])
    .unwrap();
    let hole = Curve::from_polyline(vec![
        vec![0.75, 0.25],
        vec![0.75, 0.75],
        vec![1., 0.75],
        vec![1.25, 0.75],
        vec![1.25, 0.25],
        vec![1., 0.25],
        vec![0.75, 0.25],
    ])
    .unwrap();
    let outer_p = split_candidates(
        &outer,
        [[0., 1.]; 2],
        &[outer.knots[2], outer.knots[5]],
        &[[0, 0], [1, 0], [0, 0]],
        3,
    )
    .unwrap();
    let hole_p = split_candidates(
        &hole,
        [[0., 1.]; 2],
        &[hole.knots[3], hole.knots[6]],
        &[[0, 0], [1, 0], [0, 0]],
        3,
    )
    .unwrap();
    let sources = [outer, hole];
    let partitions = [outer_p, hole_p];
    let r = assemble_single_seam(&sources, &partitions, [[0., 1.]; 2], 0, 32, 16, 32).unwrap();
    assert!(r.chart_winding_preserved, "{r:?}");
    let graph = r.graph.unwrap();
    assert_eq!(graph.loops.len(), 2);
    assert_eq!(graph.edges.len(), 10);
    assert_eq!(graph.connector_pairs.len(), 2);
    let mut used = vec![false; graph.edges.len()];
    for cycle in &graph.loops {
        for i in 0..cycle.len() {
            let index = cycle[i];
            assert!(!used[index]);
            used[index] = true;
            let c = &graph.edges[index].curve;
            let d = &graph.edges[cycle[(i + 1) % cycle.len()]].curve;
            assert_eq!(c.control_points.last().unwrap(), &d.control_points[0]);
        }
    }
    assert!(used.iter().all(|x| *x));
    // Independent polygon winding oracle; the assembly algorithm never uses it.
    let winding = |q: [f64; 2]| {
        let mut w = 0i32;
        for cycle in &graph.loops {
            for &index in cycle {
                let points = &graph.edges[index].curve.control_points;
                for e in points.windows(2) {
                    let a = &e[0];
                    let b = &e[1];
                    let cross = (b[0] - a[0]) * (q[1] - a[1]) - (b[1] - a[1]) * (q[0] - a[0]);
                    if a[1] <= q[1] && q[1] < b[1] && cross > 0. {
                        w += 1;
                    }
                    if b[1] <= q[1] && q[1] < a[1] && cross < 0. {
                        w -= 1;
                    }
                }
            }
        }
        w
    };
    for q in [[0.625, 0.5], [0.375, 0.5]] {
        assert_eq!(winding(q), 1);
    }
    for q in [[0.875, 0.5], [0.125, 0.5], [0.375, 0.9375], [0.625, 0.0625]] {
        assert_eq!(winding(q), 0);
    }

    for [a, b] in graph.connector_pairs {
        let a = &graph.edges[a].curve;
        let b = &graph.edges[b].curve;
        assert_eq!(a.control_points[0][1], b.control_points[1][1]);
        assert_eq!(a.control_points[1][1], b.control_points[0][1]);
        assert_eq!(a.control_points[0][0] - b.control_points[1][0], 1.);
    }
    let limited = assemble_single_seam(&sources, &partitions, [[0., 1.]; 2], 0, 32, 16, 1).unwrap();
    assert!(!limited.chart_winding_preserved && limited.graph.is_none());
    let no_anchor = assemble_single_seam(
        &sources,
        &partitions,
        [[0., 1.], [0.125, 0.875]],
        0,
        32,
        16,
        32,
    )
    .unwrap();
    assert!(!no_anchor.chart_winding_preserved);
    assert_eq!(no_anchor.reason, Some("empty-anchor-strip-unproven"));
    let duplicate_sources = [sources[0].clone(), sources[0].clone()];
    let duplicate_partitions = [partitions[0].clone(), partitions[0].clone()];
    let contacts = assemble_single_seam(
        &duplicate_sources,
        &duplicate_partitions,
        [[0., 1.]; 2],
        0,
        32,
        16,
        32,
    )
    .unwrap();
    assert!(contacts.source_region.periodic_winding_preserved);
    assert!(!contacts.chart_winding_preserved && contacts.graph.is_none());
    assert_eq!(contacts.reason, Some("contact-vertex-unproven"));
    let mut swapped_sources = sources.clone();
    let mut swapped_partitions = partitions.clone();
    for c in &mut swapped_sources {
        for p in &mut c.control_points {
            p.swap(0, 1);
        }
    }
    for r in &mut swapped_partitions {
        for piece in &mut r.pieces {
            piece.tile.swap(0, 1);
            piece.translation.swap(0, 1);
            for p in &mut piece.curve.control_points {
                p.swap(0, 1);
            }
        }
    }
    let v_graph = assemble_single_seam(
        &swapped_sources,
        &swapped_partitions,
        [[0., 1.]; 2],
        1,
        32,
        16,
        32,
    )
    .unwrap();
    assert!(v_graph.chart_winding_preserved, "{v_graph:?}");
    let v_graph = v_graph.graph.unwrap();
    assert_eq!(v_graph.edges.len(), 10);
    assert_eq!(v_graph.loops.len(), 2);
}

#[test]
fn single_seam_chart_assembly_retains_rational_circle_edges() {
    use nurbs_core::periodic_trim::{assemble_single_seam, split_verified};
    let mut circle = nurbs_core::primitives::circle([2.5, 1., 0.], [0., 0., 1.], 1.).unwrap();
    for p in &mut circle.control_points {
        p.truncate(2);
    }
    let chart = [[0., 2.5], [0., 4.]];
    let partition = split_verified(&circle, chart, 1e-7, 0.02, 16, 32, 20000, 30000)
        .unwrap()
        .partition
        .unwrap();
    let r = assemble_single_seam(&[circle], &[partition.clone()], chart, 0, 16, 16, 16).unwrap();
    assert!(r.chart_winding_preserved, "{r:?}");
    let graph = r.graph.unwrap();
    assert_eq!(graph.edges.len(), 5);
    assert_eq!(graph.loops.len(), 2);
    for edge in graph.edges.iter().filter(|e| e.source.is_some()) {
        let [_, piece] = edge.source.unwrap();
        assert_eq!(edge.curve, partition.pieces[piece].curve);
        assert_eq!(edge.curve.degree, 2);
    }
}

#[test]
fn two_axis_periodic_winding_accounts_for_corner_images() {
    use nurbs_core::periodic_trim::{PeriodicWindingReason, inspect_periodic_winding};
    let mut circle = nurbs_core::primitives::circle([1., 1., 0.], [0., 0., 1.], 0.25).unwrap();
    for p in &mut circle.control_points {
        p.truncate(2);
    }
    let loops = vec![vec![circle]];
    let chart = [[0., 1.]; 2];
    for point in [
        [0.125, 0.125],
        [0.875, 0.875],
        [0.125, 0.875],
        [0.875, 0.125],
        [1.125, -0.875],
    ] {
        let r = inspect_periodic_winding(&loops, chart, point, 1e-8, 64, 100000).unwrap();
        assert_eq!(r.reason, PeriodicWindingReason::Complete, "{r:?}");
        assert_eq!(r.winding, Some(1));
        assert_eq!(r.images.len(), r.required_images.unwrap());
        assert!(r.cells <= 100000);
    }
    let outside = inspect_periodic_winding(&loops, chart, [0.5, 0.5], 1e-8, 64, 100000).unwrap();
    assert_eq!(outside.winding, Some(0));
    let boundary = inspect_periodic_winding(&loops, chart, [0.25, 0.], 1e-8, 64, 100000).unwrap();
    assert!(boundary.winding.is_none());
    assert_eq!(
        boundary.reason,
        PeriodicWindingReason::ClassificationUnresolved
    );
    let images = inspect_periodic_winding(&loops, chart, [0.125, 0.125], 1e-8, 1, 100000).unwrap();
    assert!(images.winding.is_none() && images.images.is_empty());
    assert_eq!(images.reason, PeriodicWindingReason::ImageBudget);
    let work = inspect_periodic_winding(&loops, chart, [0.125, 0.125], 1e-8, 64, 1).unwrap();
    assert!(work.winding.is_none() && work.cells <= 1);
    assert!(inspect_periodic_winding(&loops, chart, [f64::NAN, 0.], 1e-8, 64, 100000).is_err());
}
#[test]
fn periodic_winding_refuses_open_contours() {
    use nurbs_core::periodic_trim::{PeriodicWindingReason, inspect_periodic_winding};
    let open = Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.]]).unwrap();
    let r = inspect_periodic_winding(&[vec![open]], [[0., 1.]; 2], [0.5, 0.5], 1e-8, 64, 100000)
        .unwrap();
    assert!(r.winding.is_none() && r.images.is_empty());
    assert_eq!(r.reason, PeriodicWindingReason::ClosureUnproven);
}

#[test]
fn two_axis_winding_keeps_oriented_hole_contributions() {
    use nurbs_core::periodic_trim::{PeriodicWindingReason, inspect_periodic_winding};
    let make = |radius: f64| {
        let mut c = nurbs_core::primitives::circle([1., 1., 0.], [0., 0., 1.], radius).unwrap();
        for p in &mut c.control_points {
            p.truncate(2);
        }
        c
    };
    let loops = vec![vec![make(0.375)], vec![make(0.125).reverse().unwrap()]];
    for (point, expected) in [([0.0625, 0.0625], 0), ([0.25, 0.125], 1), ([0.5, 0.5], 0)] {
        let r = inspect_periodic_winding(&loops, [[0., 1.]; 2], point, 1e-8, 64, 100000).unwrap();
        assert_eq!(r.reason, PeriodicWindingReason::Complete, "{r:?}");
        assert_eq!(r.winding, Some(expected));
    }
    let limited =
        inspect_periodic_winding(&loops, [[0., 1.]; 2], [0.25, 0.125], 1e-8, 64, 1).unwrap();
    assert!(limited.winding.is_none());
    assert_eq!(limited.reason, PeriodicWindingReason::WorkLimit);
}

#[test]
fn periodic_winding_encloses_nonrepresentable_authored_period() {
    use nurbs_core::periodic_trim::{PeriodicWindingReason, inspect_periodic_winding};
    let mut circle = nurbs_core::primitives::circle([1., 1., 0.], [0., 0., 1.], 0.25).unwrap();
    for p in &mut circle.control_points {
        p.truncate(2);
    }
    let r = inspect_periodic_winding(
        &[vec![circle]],
        [[0.1, 1.1]; 2],
        [0.125, 0.125],
        1e-8,
        64,
        100000,
    )
    .unwrap();
    assert_eq!(r.reason, PeriodicWindingReason::Complete);
    assert_eq!(r.winding, Some(1));
    let image = r.images.iter().find(|image| image.tile == [1, 1]).unwrap();
    // Exact binary64 leaf arithmetic: 1.1 - 0.1 = 1 + 3/2^55.
    // Thus the true shifted coordinate is strictly between 1.125 and next_up.
    for range in image.query {
        assert!(range[0] <= 1.125 && range[1] >= 1.125f64.next_up());
    }
    assert_eq!(image.classification.winding, Some(1));
}

fn two_seam_circle(radius: f64, reverse: bool) -> (Curve, nurbs_core::periodic_trim::Report) {
    let mut c = nurbs_core::primitives::circle([1., 1., 0.], [0., 0., 1.], radius).unwrap();
    for p in &mut c.control_points {
        p.truncate(2);
    }
    if reverse {
        c = c.reverse().unwrap();
    }
    let p = nurbs_core::periodic_trim::split_verified(
        &c,
        [[0., 1.]; 2],
        1e-7,
        0.02,
        32,
        64,
        50000,
        50000,
    )
    .unwrap()
    .partition
    .unwrap();
    (c, p)
}
fn chart_limits() -> nurbs_core::periodic_trim::ChartAssemblyLimits {
    nurbs_core::periodic_trim::ChartAssemblyLimits {
        max_segments: 64,
        max_links: 32,
        max_edges: 64,
        max_images: 64,
        max_cells: 100000,
        tolerance_uv: 1e-8,
    }
}
fn chart_winding(graph: &nurbs_core::periodic_trim::ChartBoundaryGraph, point: [f64; 2]) -> i32 {
    let loops: Vec<Vec<Curve>> = graph
        .loops
        .iter()
        .map(|cycle| {
            cycle
                .iter()
                .map(|&i| graph.edges[i].curve.clone())
                .collect()
        })
        .collect();
    nurbs_core::periodic_trim::inspect_periodic_winding(
        &loops,
        [[0., 1.]; 2],
        point,
        1e-8,
        64,
        100000,
    )
    .unwrap()
    .winding
    .unwrap()
}
#[test]
fn two_seam_chart_fixes_full_chart_constant_and_cancels_perimeter() {
    use nurbs_core::periodic_trim::assemble_periodic_chart;
    let (c, p) = two_seam_circle(0.25, false);
    let r = assemble_periodic_chart(
        &[c.clone()],
        &[p.clone()],
        [[0., 1.]; 2],
        [0.5, 0.5],
        chart_limits(),
    )
    .unwrap();
    assert!(r.chart_winding_preserved, "{r:?}");
    let calibration = r.calibration.unwrap();
    assert_eq!(calibration.source.winding, Some(0));
    assert_eq!(calibration.candidate.unwrap().winding, Some(-1));
    assert_eq!(calibration.offset, Some(1));
    let graph = r.graph.unwrap();
    assert_eq!(graph.loops.len(), 4);
    assert_eq!(graph.edges.len(), 12);
    assert_eq!(graph.edges.iter().filter(|e| e.source.is_some()).count(), 4);
    for q in [
        [0.125, 0.125],
        [0.875, 0.875],
        [0.125, 0.875],
        [0.875, 0.125],
    ] {
        assert_eq!(chart_winding(&graph, q), 1);
    }
    assert_eq!(chart_winding(&graph, [0.5, 0.5]), 0);
    let alternative = assemble_periodic_chart(
        &[c.clone()],
        &[p.clone()],
        [[0., 1.]; 2],
        [0.125, 0.125],
        chart_limits(),
    )
    .unwrap();
    assert!(alternative.chart_winding_preserved);
    assert_eq!(alternative.calibration.unwrap().offset, Some(1));
    let alternative = alternative.graph.unwrap();
    assert_eq!(alternative.loops, graph.loops);
    assert_eq!(alternative.connector_pairs, graph.connector_pairs);
    for (a, b) in alternative.edges.iter().zip(&graph.edges) {
        assert_eq!(a.curve, b.curve);
        assert_eq!(a.source, b.source);
    }
    let limited = assemble_periodic_chart(
        &[c.clone()],
        &[p.clone()],
        [[0., 1.]; 2],
        [0.5, 0.5],
        nurbs_core::periodic_trim::ChartAssemblyLimits {
            max_edges: 8,
            ..chart_limits()
        },
    )
    .unwrap();
    assert!(limited.graph.is_none() && !limited.chart_winding_preserved);
    assert_eq!(limited.reason, Some("edge-budget"));
    let no_images = assemble_periodic_chart(
        &[c],
        &[p],
        [[0., 1.]; 2],
        [0.5, 0.5],
        nurbs_core::periodic_trim::ChartAssemblyLimits {
            max_images: 1,
            ..chart_limits()
        },
    )
    .unwrap();
    assert!(no_images.graph.is_none() && !no_images.chart_winding_preserved);
}
#[test]
fn two_seam_chart_preserves_hole_and_negative_winding() {
    use nurbs_core::periodic_trim::assemble_periodic_chart;
    let (outer, op) = two_seam_circle(0.375, false);
    let (hole, hp) = two_seam_circle(0.125, true);
    let r = assemble_periodic_chart(
        &[outer, hole],
        &[op, hp],
        [[0., 1.]; 2],
        [0.5, 0.5],
        chart_limits(),
    )
    .unwrap();
    assert!(r.chart_winding_preserved, "{r:?}");
    assert_eq!(r.calibration.unwrap().offset, Some(0));
    let graph = r.graph.unwrap();
    assert_eq!(graph.loops.len(), 4);
    assert_eq!(chart_winding(&graph, [0.0625, 0.0625]), 0);
    assert_eq!(chart_winding(&graph, [0.25, 0.125]), 1);
    assert_eq!(chart_winding(&graph, [0.5, 0.5]), 0);
    let (negative, np) = two_seam_circle(0.25, true);
    let r = assemble_periodic_chart(
        &[negative],
        &[np],
        [[0., 1.]; 2],
        [0.5, 0.5],
        chart_limits(),
    )
    .unwrap();
    assert!(r.chart_winding_preserved, "{r:?}");
    assert_eq!(r.calibration.unwrap().offset, Some(-1));
    let graph = r.graph.unwrap();
    assert_eq!(chart_winding(&graph, [0.125, 0.125]), -1);
    assert_eq!(chart_winding(&graph, [0.5, 0.5]), 0);
}

#[test]
fn two_seam_chart_refuses_unsupported_perimeter_multiplicity() {
    use nurbs_core::periodic_trim::assemble_periodic_chart;
    let c = Curve::from_polyline(vec![
        vec![0.25, 0.25],
        vec![1., 1.],
        vec![1.5, 0.25],
        vec![1.5, 1.],
        vec![1.5, 1.5],
        vec![1., 1.5],
        vec![0.25, 1.5],
        vec![0.25, 1.],
        vec![0.25, 0.25],
    ])
    .unwrap();
    let cuts = [c.knots[2], c.knots[4], c.knots[6], c.knots[8]];
    let p = split_candidates(
        &c,
        [[0., 1.]; 2],
        &cuts,
        &[[0, 0], [1, 0], [1, 1], [0, 1], [0, 0]],
        5,
    )
    .unwrap();
    let r =
        assemble_periodic_chart(&[c], &[p], [[0., 1.]; 2], [0.625, 0.375], chart_limits()).unwrap();
    assert!(r.source_region.periodic_winding_preserved);
    assert!(!r.chart_winding_preserved && r.graph.is_none());
    assert_eq!(r.reason, Some("perimeter-multiplicity-unproven"));
}

#[test]
fn two_seam_chart_resolves_unique_simultaneous_corner_transition() {
    use nurbs_core::periodic_trim::{assemble_periodic_chart, split_verified};
    let c = Curve::from_polyline(vec![
        vec![0.75, 0.75],
        vec![1., 1.],
        vec![1.25, 1.25],
        vec![1., 1.375],
        vec![0.75, 1.25],
        vec![0.625, 1.],
        vec![0.75, 0.75],
    ])
    .unwrap();
    let p = split_candidates(
        &c,
        [[0., 1.]; 2],
        &[c.knots[2], c.knots[4], c.knots[6]],
        &[[0, 0], [1, 1], [0, 1], [0, 0]],
        4,
    )
    .unwrap();
    let r = assemble_periodic_chart(
        &[c.clone()],
        &[p.clone()],
        [[0., 1.]; 2],
        [0.5, 0.5],
        chart_limits(),
    )
    .unwrap();
    assert!(r.chart_winding_preserved, "{r:?}");
    let graph = r.graph.unwrap();
    assert_eq!(graph.loops.len(), 3);
    for q in [[0.0625, 0.125], [0.875, 0.125], [0.875, 0.9375]] {
        assert_eq!(chart_winding(&graph, q), 1);
    }
    for q in [[0.125, 0.0625], [0.125, 0.9375], [0.5, 0.5]] {
        assert_eq!(chart_winding(&graph, q), 0);
    }
    let signed = assemble_periodic_chart(
        &[c.clone()],
        &[p],
        [[-0., 1.]; 2],
        [0.5, 0.5],
        chart_limits(),
    )
    .unwrap();
    assert!(signed.chart_winding_preserved);
    assert_eq!(signed.graph.unwrap().loops, graph.loops);
    let reversed = c.reverse().unwrap();
    let rp = split_verified(&reversed, [[0., 1.]; 2], 1e-7, 0.02, 32, 64, 50000, 50000)
        .unwrap()
        .partition
        .unwrap();
    let r = assemble_periodic_chart(
        &[reversed],
        &[rp],
        [[0., 1.]; 2],
        [0.5, 0.5],
        chart_limits(),
    )
    .unwrap();
    assert!(r.chart_winding_preserved, "{r:?}");
    assert_eq!(chart_winding(&r.graph.unwrap(), [0.0625, 0.125]), -1);
}

#[test]
fn two_seam_chart_resolves_crossing_with_corner_touch() {
    use nurbs_core::periodic_trim::assemble_periodic_chart;
    let c = Curve::from_polyline(vec![
        vec![0.75, 0.75],
        vec![1., 1.],
        vec![1.25, 0.75],
        vec![1.25, 1.],
        vec![1.25, 1.25],
        vec![1., 1.25],
        vec![0.75, 1.25],
        vec![0.75, 1.],
        vec![0.75, 0.75],
    ])
    .unwrap();
    let p = split_candidates(
        &c,
        [[0., 1.]; 2],
        &[c.knots[2], c.knots[4], c.knots[6], c.knots[8]],
        &[[0, 0], [1, 0], [1, 1], [0, 1], [0, 0]],
        5,
    )
    .unwrap();
    let r = assemble_periodic_chart(&[c], &[p], [[0., 1.]; 2], [0.5, 0.5], chart_limits()).unwrap();
    assert!(r.chart_winding_preserved, "{r:?}");
    let graph = r.graph.unwrap();
    assert_eq!(graph.loops.len(), 4);
    for q in [
        [0.875, 0.9375],
        [0.125, 0.9375],
        [0.125, 0.125],
        [0.875, 0.125],
    ] {
        assert_eq!(chart_winding(&graph, q), 1);
    }
    for q in [[0.9375, 0.875], [0.0625, 0.8125], [0.5, 0.5]] {
        assert_eq!(chart_winding(&graph, q), 0);
    }
}

fn isotopy_limits() -> nurbs_core::periodic_trim::IsotopyLimits {
    nurbs_core::periodic_trim::IsotopyLimits {
        tolerance_uv: 0.01,
        max_deviation_cells: 40000,
        max_jet_cells: 256,
        max_pairs: 20000,
    }
}
fn approximated_loop(c: &Curve) -> nurbs_core::periodic_trim::Report {
    nurbs_core::periodic_trim::split_approximated(
        c,
        [[0., 1.]; 2],
        1e-8,
        0.01,
        32,
        64,
        10000,
        30000,
    )
    .unwrap()
    .partition
    .unwrap()
}
#[test]
fn approximated_seam_partition_has_certified_whole_family_isotopy() {
    use nurbs_core::periodic_trim::{AdmissionStatus, verify_identity, verify_isotopy};
    let c = Curve::from_polyline(vec![
        vec![0.5, 0.25],
        vec![1.25, 0.25],
        vec![1.25, 0.75],
        vec![0.5, 0.75],
        vec![0.5, 0.25],
    ])
    .unwrap();
    let p = approximated_loop(&c);
    assert!(!verify_identity(&c, &p, 32).unwrap().exact);
    let r = verify_isotopy(&[c], &[p], [[0., 1.]; 2], isotopy_limits()).unwrap();
    assert!(
        r.lifted_isotopy_preserved && r.periodic_isotopy_preserved,
        "{r:?}"
    );
    assert_eq!(
        r.loops[0].admission.as_ref().unwrap().status,
        AdmissionStatus::WithinTolerance
    );
    assert!(r.jet_cells <= 256 && r.pair_checks <= r.pair_requests && r.pair_requests <= 20000);
    assert_eq!(r.cells.len(), r.leaf_cells);
    assert!(r.cells.iter().all(|c| c.direction_mask != 0));
    assert!(r.periodic_clearance.unwrap().iter().all(|x| *x > 0.));
}
#[test]
fn isotopy_checks_hole_collisions_during_transition_not_only_endpoints() {
    use nurbs_core::periodic_trim::{AdmissionStatus, IsotopyLimits, verify_isotopy};
    let outer = Curve::from_polyline(vec![
        vec![0.5, 0.125],
        vec![1.375, 0.125],
        vec![1.375, 0.875],
        vec![0.5, 0.875],
        vec![0.5, 0.125],
    ])
    .unwrap();
    let hole = Curve::from_polyline(vec![
        vec![0.75, 0.25],
        vec![0.75, 0.75],
        vec![1.125, 0.75],
        vec![1.125, 0.25],
        vec![0.75, 0.25],
    ])
    .unwrap();
    let mut partitions = [approximated_loop(&outer), approximated_loop(&hole)];
    let sources = [outer, hole];
    let r = verify_isotopy(&sources, &partitions, [[0., 1.]; 2], isotopy_limits()).unwrap();
    assert!(
        r.lifted_isotopy_preserved && r.periodic_isotopy_preserved,
        "{r:?}"
    );
    for piece in &mut partitions[1].pieces {
        piece.translation[0] += 1.;
    }
    let unsafe_move = verify_isotopy(
        &sources,
        &partitions,
        [[0., 1.]; 2],
        IsotopyLimits {
            tolerance_uv: 2.,
            max_jet_cells: 64,
            max_pairs: 2000,
            ..isotopy_limits()
        },
    )
    .unwrap();
    assert!(!unsafe_move.lifted_isotopy_preserved && !unsafe_move.periodic_isotopy_preserved);
    assert!(
        unsafe_move
            .loops
            .iter()
            .all(|p| p.admission.as_ref().unwrap().status == AdmissionStatus::WithinTolerance)
    );
    assert!(
        unsafe_move.jet_cells <= 64
            && unsafe_move.pair_requests <= 2000
            && unsafe_move.cells.is_empty()
    );
}
#[test]
fn isotopy_refuses_a_collapsing_linear_family_of_two_simple_loops() {
    use nurbs_core::periodic_trim::{AdmissionStatus, IsotopyLimits, verify_isotopy};
    let c = Curve::from_polyline(vec![
        vec![0.25, 0.25],
        vec![0.75, 0.25],
        vec![0.75, 0.75],
        vec![0.25, 0.75],
        vec![0.25, 0.25],
    ])
    .unwrap();
    let mut p = split_candidates(&c, [[0., 1.]; 2], &[], &[[0, 0]], 1).unwrap();
    for point in &mut p.pieces[0].curve.control_points {
        point[0] = 1. - point[0];
        point[1] = 1. - point[1];
    }
    let r = verify_isotopy(
        &[c],
        &[p],
        [[0., 1.]; 2],
        IsotopyLimits {
            tolerance_uv: 2.,
            max_jet_cells: 32,
            ..isotopy_limits()
        },
    )
    .unwrap();
    assert!(!r.lifted_isotopy_preserved && r.cells.is_empty());
    assert_eq!(
        r.loops[0].admission.as_ref().unwrap().status,
        AdmissionStatus::WithinTolerance
    );
    assert!(r.jet_cells <= 32);
}

#[test]
fn isotopy_budget_exhaustion_never_publishes_partial_certificate() {
    use nurbs_core::periodic_trim::verify_isotopy;
    let c = Curve::from_polyline(vec![
        vec![0.25, 0.25],
        vec![0.75, 0.25],
        vec![0.75, 0.75],
        vec![0.25, 0.75],
        vec![0.25, 0.25],
    ])
    .unwrap();
    let p = split_candidates(&c, [[0., 1.]; 2], &[], &[[0, 0]], 1).unwrap();
    for (jets, pairs, reason) in [(1, 20000, "jet-budget"), (256, 1, "pair-budget")] {
        let mut limits = isotopy_limits();
        limits.max_jet_cells = jets;
        limits.max_pairs = pairs;
        let r = verify_isotopy(
            std::slice::from_ref(&c),
            std::slice::from_ref(&p),
            [[0., 1.]; 2],
            limits,
        )
        .unwrap();
        assert_eq!(r.reason, Some(reason));
        assert!(!r.lifted_isotopy_preserved && !r.periodic_isotopy_preserved);
        assert!(r.cells.is_empty() && r.periodic_clearance.is_none());
        assert!(r.jet_cells <= jets && r.pair_requests <= pairs);
    }
}

#[test]
fn lifted_isotopy_does_not_certify_colliding_periodic_images() {
    use nurbs_core::periodic_trim::verify_isotopy;
    let square = |x: f64| {
        Curve::from_polyline(vec![
            vec![x, 0.25],
            vec![x + 0.25, 0.25],
            vec![x + 0.25, 0.5],
            vec![x, 0.5],
            vec![x, 0.25],
        ])
        .unwrap()
    };
    let sources = [square(0.125), square(1.125)];
    let partitions: Vec<_> = sources
        .iter()
        .map(|c| split_candidates(c, [[0., 2.]; 2], &[], &[[0, 0]], 1).unwrap())
        .collect();
    let r = verify_isotopy(&sources, &partitions, [[0., 1.]; 2], isotopy_limits()).unwrap();
    assert!(r.lifted_isotopy_preserved, "{r:?}");
    assert!(!r.periodic_isotopy_preserved);
    assert_eq!(r.periodic_reason, Some("periodic-image-clearance-unproven"));
    assert!(r.periodic_clearance.is_none());
}

fn anchor_limits() -> nurbs_core::periodic_trim::AnchorLimits {
    nurbs_core::periodic_trim::AnchorLimits {
        isotopy: isotopy_limits(),
        tolerance_uv: 1e-8,
        max_images: 256,
        max_cells: 20000,
        max_checks: 10000,
    }
}
#[test]
fn approximate_anchor_transfer_preserves_interior_exterior_and_shifted_winding() {
    use nurbs_core::periodic_trim::verify_anchor_winding;
    let c = Curve::from_polyline(vec![
        vec![0.5, 0.25],
        vec![1.25, 0.25],
        vec![1.25, 0.75],
        vec![0.5, 0.75],
        vec![0.5, 0.25],
    ])
    .unwrap();
    let p = approximated_loop(&c);
    for (anchor, expected) in [
        ([0.875, 0.5], 1),
        ([0.125, 0.5], 1),
        ([0.375, 0.5], 0),
        ([1.125, 0.5], 1),
    ] {
        let r = verify_anchor_winding(
            std::slice::from_ref(&c),
            std::slice::from_ref(&p),
            [[0., 1.]; 2],
            anchor,
            anchor_limits(),
        )
        .unwrap();
        assert_eq!(r.winding, Some(expected), "{r:?}");
        assert_eq!(r.reason, None);
    }
    let mut limits = anchor_limits();
    limits.max_checks = 1;
    let r = verify_anchor_winding(&[c], &[p], [[0., 1.]; 2], [0.875, 0.5], limits).unwrap();
    assert_eq!(r.reason, Some("anchor-check-budget"));
    assert!(r.winding.is_none() && r.source.is_none());
}
#[test]
fn anchor_transfer_refuses_point_swept_by_moving_boundary() {
    use nurbs_core::periodic_trim::verify_anchor_winding;
    let c = Curve::from_polyline(vec![
        vec![0.25, 0.25],
        vec![0.75, 0.25],
        vec![0.75, 0.75],
        vec![0.25, 0.75],
        vec![0.25, 0.25],
    ])
    .unwrap();
    let mut p = split_candidates(&c, [[0., 1.]; 2], &[], &[[0, 0]], 1).unwrap();
    p.pieces[0].translation[0] = 0.125;
    let mut limits = anchor_limits();
    limits.isotopy.tolerance_uv = 0.25;
    let r = verify_anchor_winding(&[c], &[p], [[0., 1.]; 2], [0.8125, 0.5], limits).unwrap();
    assert!(r.isotopy.periodic_isotopy_preserved);
    assert_eq!(r.reason, Some("anchor-homotopy-clearance-unproven"));
    assert!(r.winding.is_none() && r.source.is_none());
}

fn approximate_chart_limits() -> nurbs_core::periodic_trim::ApproximateChartLimits {
    nurbs_core::periodic_trim::ApproximateChartLimits {
        isotopy: isotopy_limits(),
        max_edges: 128,
        max_images: 256,
        max_cells: 30000,
        max_checks: 10000,
        tolerance_uv: 1e-8,
    }
}
#[test]
fn approximated_chart_assembles_nonrepresentable_seam_root_and_hole() {
    use nurbs_core::periodic_trim::{assemble_approximated_chart, verify_identity};
    let outer = Curve::from_polyline(vec![
        vec![0.5, 0.125],
        vec![1.375, 0.125],
        vec![1.375, 0.875],
        vec![0.5, 0.875],
        vec![0.5, 0.125],
    ])
    .unwrap();
    let hole = Curve::from_polyline(vec![
        vec![0.75, 0.25],
        vec![0.75, 0.75],
        vec![1.125, 0.75],
        vec![1.125, 0.25],
        vec![0.75, 0.25],
    ])
    .unwrap();
    let partitions = [approximated_loop(&outer), approximated_loop(&hole)];
    assert!(!verify_identity(&outer, &partitions[0], 128).unwrap().exact);
    let sources = [outer, hole];
    for anchor in [[0.625, 0.5], [0.875, 0.5], [0.4375, 0.5]] {
        let r = assemble_approximated_chart(
            &sources,
            &partitions,
            [[0., 1.]; 2],
            anchor,
            approximate_chart_limits(),
        )
        .unwrap();
        assert!(r.candidate_winding_preserved, "{r:?}");
        assert!(r.transfer.isotopy.periodic_isotopy_preserved);
        let graph = r.graph.unwrap();
        for q in [[0.625, 0.5], [0.25, 0.5], [0.875, 0.1875]] {
            assert_eq!(chart_winding(&graph, q), 1);
        }
        for q in [[0.875, 0.5], [0.0625, 0.5], [0.4375, 0.5]] {
            assert_eq!(chart_winding(&graph, q), 0);
        }
    }
}
#[test]
fn approximate_chart_refuses_forged_tiles_and_partial_edge_budget() {
    use nurbs_core::periodic_trim::assemble_approximated_chart;
    let c = Curve::from_polyline(vec![
        vec![0.5, 0.25],
        vec![1.25, 0.25],
        vec![1.25, 0.75],
        vec![0.5, 0.75],
        vec![0.5, 0.25],
    ])
    .unwrap();
    let mut p = approximated_loop(&c);
    let mut limits = approximate_chart_limits();
    limits.max_edges = 1;
    let r = assemble_approximated_chart(
        std::slice::from_ref(&c),
        std::slice::from_ref(&p),
        [[0., 1.]; 2],
        [0.875, 0.5],
        limits,
    )
    .unwrap();
    assert_eq!(r.reason, Some("edge-budget"));
    assert!(r.graph.is_none() && !r.candidate_winding_preserved);
    p.pieces[0].tile[0] += 1;
    let r = assemble_approximated_chart(
        &[c],
        &[p],
        [[0., 1.]; 2],
        [0.875, 0.5],
        approximate_chart_limits(),
    )
    .unwrap();
    assert_eq!(r.reason, Some("periodic-translation-unproven"));
    assert!(r.graph.is_none() && !r.candidate_winding_preserved);
}

#[test]
fn approximate_chart_rechecks_containment_despite_claimed_report_flag() {
    use nurbs_core::periodic_trim::assemble_approximated_chart;
    let c = Curve::from_polyline(vec![
        vec![0.25, 0.25],
        vec![0.75, 0.25],
        vec![0.75, 0.75],
        vec![0.25, 0.75],
        vec![0.25, 0.25],
    ])
    .unwrap();
    let mut p = split_candidates(&c, [[0., 1.]; 2], &[], &[[0, 0]], 1).unwrap();
    for cp in &mut p.pieces[0].curve.control_points {
        cp[0] += 1.;
    }
    p.pieces[0].translation[0] = -1.;
    p.pieces[0].tile[0] = -1;
    assert!(p.candidate_hulls_contained);
    let r = assemble_approximated_chart(
        &[c],
        &[p],
        [[0., 1.]; 2],
        [0.5, 0.5],
        approximate_chart_limits(),
    )
    .unwrap();
    assert_eq!(r.reason, Some("candidate-chart-unproven"));
    assert!(r.transfer.isotopy.periodic_isotopy_preserved);
    assert!(r.graph.is_none() && !r.candidate_winding_preserved);
}

#[test]
fn seam_search_retains_arbitrary_rational_tangency_without_snapping_its_root() {
    use nurbs_core::periodic_trim::{SearchStatus, SplitStatus, inspect_seams, split_verified};
    let c = Curve {
        degree: 2,
        knots: vec![2., 2., 2., 7., 7., 7.],
        control_points: vec![vec![1., 0.], vec![-1., 0.5], vec![1., 1.]],
        weights: vec![1., 2., 4.],
        periodic: false,
    };
    let search = inspect_seams(&c, [[0., 2.], [-1., 2.]], 1e-7, 32, 10000).unwrap();
    assert_eq!(search.status, SearchStatus::Complete, "{search:?}");
    let seam = search
        .seams
        .iter()
        .find(|s| s.axis == 0 && s.coordinate == 0.)
        .unwrap();
    assert!(
        seam.roots.cells.iter().any(|cell| cell.classification
            == nurbs_core::curve_plane::Classification::TangencyInInterval)
    );
    let events = seam.roots.root_events();
    let event = &events[0];
    assert!(event.parameter_bounds[0] <= 11. / 3. && 11. / 3. <= event.parameter_bounds[1]);
    let split =
        split_verified(&c, [[0., 2.], [-1., 2.]], 1e-7, 0.01, 16, 32, 10000, 10000).unwrap();
    assert_eq!(split.status, SplitStatus::IntervalRootNeedsConstruction);
    assert!(split.partition.is_none());
}
