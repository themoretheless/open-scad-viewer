use nurbs_core::{
    curve::Curve,
    periodic_trim::{self, SurfaceCorrespondenceLimits},
    surface::{self, Surface},
};
fn tube() -> Surface {
    let profile = Curve::from_polyline(vec![
        vec![0., 0., 0.],
        vec![1., 0., 0.],
        vec![1., 1., 0.],
        vec![0., 1., 0.],
        vec![0., 0., 0.],
    ])
    .unwrap();
    surface::extrude(&profile, [0., 0., 2.]).unwrap()
}
fn limits() -> SurfaceCorrespondenceLimits {
    SurfaceCorrespondenceLimits {
        tolerance_uv: 0.001,
        tolerance_world: 0.01,
        max_rectangles: 64,
        max_deviation_cells: 30000,
    }
}
fn exact_partition() -> (Curve, periodic_trim::Report) {
    let c = Curve::from_polyline(vec![vec![3.5, 0.5], vec![4.5, 0.5]]).unwrap();
    let p = periodic_trim::split_candidates(&c, [[0., 4.], [0., 1.]], &[0.5], &[[0, 0], [1, 0]], 2)
        .unwrap();
    (c, p)
}
#[test]
fn closed_clamped_support_has_continuous_world_bound_across_seam() {
    let s = tube();
    let (c, p) = exact_partition();
    let r = periodic_trim::verify_surface_correspondence(
        &s,
        std::slice::from_ref(&c),
        std::slice::from_ref(&p),
        [[0., 4.], [0., 1.]],
        [true, false],
        limits(),
    )
    .unwrap();
    assert!(r.within_world_tolerance, "{r:?}");
    assert_eq!(
        r.seams,
        [
            Some(periodic_trim::SurfaceSeamProof::ClampedBoundaryIdentity),
            None
        ]
    );
    assert!(r.error_upper.is_finite() && r.error_upper <= limits().tolerance_world);
    // Independently authored square-cylinder map, including the seam corner.
    let map = |u: f64, v: f64| {
        let t = u.rem_euclid(4.);
        let (x, y) = if t <= 1. {
            (t, 0.)
        } else if t <= 2. {
            (1., t - 1.)
        } else if t <= 3. {
            (3. - t, 1.)
        } else {
            (0., 4. - t)
        };
        [x, y, 2. * v]
    };
    for piece in &p.pieces {
        for i in 0..=32 {
            let t = piece.source_domain[0]
                + (piece.source_domain[1] - piece.source_domain[0]) * i as f64 / 32.;
            let original = c.evaluate(t).unwrap().point;
            let local = piece.curve.evaluate(t).unwrap().point;
            let a = map(original[0], original[1]);
            let b = map(local[0], local[1]);
            let error = (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f64>().sqrt();
            assert!(error <= r.error_upper);
        }
    }
}
#[test]
fn surface_flags_do_not_replace_exact_seam_or_chart_proof() {
    let (c, p) = exact_partition();
    let mut s = tube();
    s.control_points[0][0][0] = 0.125;
    let r = periodic_trim::verify_surface_correspondence(
        &s,
        std::slice::from_ref(&c),
        std::slice::from_ref(&p),
        [[0., 4.], [0., 1.]],
        [true, false],
        limits(),
    )
    .unwrap();
    assert!(!r.within_world_tolerance && r.error_upper.is_infinite());
    assert_eq!(r.reason, Some("supporting-seam-unproven"));
    let r = periodic_trim::verify_surface_correspondence(
        &tube(),
        &[c],
        &[p],
        [[0., 1.]; 2],
        [true, false],
        limits(),
    )
    .unwrap();
    assert_eq!(r.reason, Some("supporting-chart-mismatch"));
}
#[test]
fn tolerance_qualified_uv_is_not_automatically_world_qualified() {
    let s = tube();
    let c = Curve::from_polyline(vec![vec![3.5, 0.5], vec![4.25, 0.5]]).unwrap();
    let mut p = periodic_trim::split_approximated(
        &c,
        [[0., 4.], [0., 1.]],
        1e-8,
        0.001,
        8,
        16,
        10000,
        30000,
    )
    .unwrap()
    .partition
    .unwrap();
    // Preserve a genuinely nonzero, UV-qualified saved deviation. An exact
    // unchanged partition can now satisfy this strict world bound through
    // correlated original-parameter evidence and must not be falsely refused.
    for piece in &mut p.pieces {
        for point in &mut piece.curve.control_points {
            point[1] += 1e-6;
        }
    }
    assert_eq!(
        periodic_trim::verify_partition(&c, &p, 0.001, 30000)
            .unwrap()
            .status,
        periodic_trim::AdmissionStatus::WithinTolerance
    );
    let mut strict = limits();
    strict.tolerance_world = 1e-14;
    let r = periodic_trim::verify_surface_correspondence(
        &s,
        &[c],
        &[p],
        [[0., 4.], [0., 1.]],
        [true, false],
        strict,
    )
    .unwrap();
    assert!(!r.within_world_tolerance);
    assert!(matches!(
        r.reason,
        Some("world-tolerance-unproven" | "supporting-uv-deviation-unproven")
    ));
    assert!(r.error_upper > strict.tolerance_world);
}
#[test]
fn support_rejects_forged_tiles_nonwrapped_translation_and_rectangle_budget() {
    let s = tube();
    let (c, mut p) = exact_partition();
    p.pieces[1].tile[0] = 0;
    let r = periodic_trim::verify_surface_correspondence(
        &s,
        std::slice::from_ref(&c),
        &[p],
        [[0., 4.], [0., 1.]],
        [true, false],
        limits(),
    )
    .unwrap();
    assert_eq!(r.reason, Some("supporting-translation-unproven"));
    let (_, mut p) = exact_partition();
    for piece in &mut p.pieces {
        piece.translation[1] = 0.125;
    }
    let r = periodic_trim::verify_surface_correspondence(
        &s,
        std::slice::from_ref(&c),
        &[p],
        [[0., 4.], [0., 1.]],
        [true, false],
        limits(),
    )
    .unwrap();
    assert_eq!(r.reason, Some("supporting-translation-unproven"));
    let (_, p) = exact_partition();
    let mut short = limits();
    short.max_rectangles = 1;
    assert!(
        periodic_trim::verify_surface_correspondence(
            &s,
            &[c],
            &[p],
            [[0., 4.], [0., 1.]],
            [true, false],
            short
        )
        .is_err()
    );
}

#[test]
fn periodic_storage_requires_exact_knots_beyond_validation_tolerance() {
    let profile = Curve {
        degree: 1,
        knots: vec![-1., 0., 1., 2., 3., 4., 5.],
        control_points: vec![
            vec![0., 0., 0.],
            vec![1., 0., 0.],
            vec![1., 1., 0.],
            vec![0., 1., 0.],
            vec![0., 0., 0.],
        ],
        weights: vec![1.; 5],
        periodic: true,
    };
    let mut s = surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let (c, p) = exact_partition();
    let r = periodic_trim::verify_surface_correspondence(
        &s,
        std::slice::from_ref(&c),
        std::slice::from_ref(&p),
        [[0., 4.], [0., 1.]],
        [true, false],
        limits(),
    )
    .unwrap();
    assert!(r.within_world_tolerance, "{r:?}");
    assert_eq!(
        r.seams[0],
        Some(periodic_trim::SurfaceSeamProof::PeriodicStorageIdentity)
    );
    s.knots_u[0] = f64::from_bits((-1_f64).to_bits() - 1);
    s.validate().unwrap();
    let r = periodic_trim::verify_surface_correspondence(
        &s,
        &[c],
        &[p],
        [[0., 4.], [0., 1.]],
        [true, false],
        limits(),
    )
    .unwrap();
    assert_eq!(r.reason, Some("supporting-seam-unproven"));
}

#[test]
fn rational_torus_support_verifies_both_closed_axes() {
    let s = nurbs_core::primitives::torus([0.; 3], 3., 1., 1.).unwrap();
    let chart = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    assert_eq!(chart, [[0., 1.], [0., 4.]]);
    let c = Curve::from_polyline(vec![vec![0.75, 3.75], vec![1.25, 4.25]]).unwrap();
    let p = periodic_trim::split_candidates(&c, chart, &[0.5], &[[0, 0], [1, 1]], 2).unwrap();
    let mut loose = limits();
    loose.tolerance_world = 1.;
    let r =
        periodic_trim::verify_surface_correspondence(&s, &[c], &[p], chart, [true, true], loose)
            .unwrap();
    assert!(r.within_world_tolerance, "{r:?}");
    assert!(
        r.seams
            .iter()
            .all(|p| *p == Some(periodic_trim::SurfaceSeamProof::ClampedBoundaryIdentity))
    );
}

#[test]
fn nonfinite_translation_is_an_input_error_not_an_exact_arithmetic_panic() {
    let (c, mut p) = exact_partition();
    p.pieces[0].translation[0] = f64::NAN;
    assert!(
        periodic_trim::verify_surface_correspondence(
            &tube(),
            &[c],
            &[p],
            [[0., 4.], [0., 1.]],
            [true, false],
            limits()
        )
        .is_err()
    );
}

#[test]
fn nonwrapped_chart_does_not_require_rounded_period_identity() {
    let chart = [[0.1, 1.1], [0., 1.]];
    let s = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0.1, 0.1, 1.1, 1.1],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let c = Curve::from_polyline(vec![vec![0.25, 0.25], vec![0.5, 0.5]]).unwrap();
    let p = periodic_trim::split_candidates(&c, chart, &[], &[[0, 0]], 1).unwrap();
    let r = periodic_trim::verify_surface_correspondence(
        &s,
        &[c],
        &[p],
        chart,
        [false, false],
        limits(),
    )
    .unwrap();
    assert!(r.within_world_tolerance, "{r:?}");
    assert_eq!(r.seams, [None; 2]);
}

fn combined_limits(approximate: bool) -> periodic_trim::SurfaceChartLimits {
    let mut correspondence = limits();
    correspondence.tolerance_uv = 0.01;
    correspondence.tolerance_world = 0.1;
    periodic_trim::SurfaceChartLimits {
        correspondence,
        uv: if approximate {
            periodic_trim::SurfaceUvLimits::Approximate(periodic_trim::ApproximateChartLimits {
                isotopy: periodic_trim::IsotopyLimits {
                    tolerance_uv: 0.01,
                    max_deviation_cells: 40000,
                    max_jet_cells: 256,
                    max_pairs: 20000,
                },
                max_edges: 128,
                max_images: 256,
                max_cells: 30000,
                max_checks: 10000,
                tolerance_uv: 1e-8,
            })
        } else {
            periodic_trim::SurfaceUvLimits::Exact(periodic_trim::ChartAssemblyLimits {
                max_segments: 256,
                max_links: 256,
                max_edges: 128,
                max_images: 256,
                max_cells: 30000,
                tolerance_uv: 1e-8,
            })
        },
    }
}
fn closed_trim(end: f64) -> Curve {
    Curve::from_polyline(vec![
        vec![3.5, 0.25],
        vec![end, 0.25],
        vec![end, 0.75],
        vec![3.5, 0.75],
        vec![3.5, 0.25],
    ])
    .unwrap()
}
#[test]
fn combined_surface_admission_publishes_exact_and_approximate_chart_cycles() {
    for approximate in [false, true] {
        let c = closed_trim(if approximate { 4.25 } else { 4.5 });
        let p = if approximate {
            periodic_trim::split_approximated(
                &c,
                [[0., 4.], [0., 1.]],
                1e-8,
                0.01,
                16,
                32,
                10000,
                30000,
            )
            .unwrap()
            .partition
            .unwrap()
        } else {
            periodic_trim::split_candidates(
                &c,
                [[0., 4.], [0., 1.]],
                &[0.5, 2.5],
                &[[0, 0], [1, 0], [0, 0]],
                3,
            )
            .unwrap()
        };
        let r = periodic_trim::assemble_on_surface(
            &tube(),
            &[c],
            &[p],
            [[0., 4.], [0., 1.]],
            [true, false],
            [3.75, 0.5],
            combined_limits(approximate),
        )
        .unwrap();
        assert_eq!(
            r.status,
            periodic_trim::SurfaceChartStatus::Admitted,
            "{r:?}"
        );
        assert!(r.support.within_world_tolerance);
        let graph = r.graph().unwrap();
        assert_eq!(graph.loops.len(), 2);
        let loops: Vec<_> = graph
            .loops
            .iter()
            .map(|cycle| {
                cycle
                    .iter()
                    .map(|&i| graph.edges[i].curve.clone())
                    .collect()
            })
            .collect();
        for (q, w) in [([3.75, 0.5], 1), ([0.125, 0.5], 1), ([1., 0.5], 0)] {
            let query = periodic_trim::inspect_periodic_winding(
                &loops,
                [[0., 4.], [0., 1.]],
                q,
                1e-8,
                256,
                20000,
            )
            .unwrap();
            assert_eq!(query.winding, Some(w));
        }
        match r.uv.unwrap() {
            periodic_trim::SurfaceUvAssembly::Exact(uv) => {
                assert!(!approximate && uv.source_region.periodic_winding_preserved)
            }
            periodic_trim::SurfaceUvAssembly::Approximate(uv) => {
                assert!(approximate && uv.transfer.isotopy.periodic_isotopy_preserved)
            }
        }
    }
}
#[test]
fn combined_admission_never_publishes_graph_after_surface_or_uv_failure() {
    let c = closed_trim(4.5);
    let p = periodic_trim::split_candidates(
        &c,
        [[0., 4.], [0., 1.]],
        &[0.5, 2.5],
        &[[0, 0], [1, 0], [0, 0]],
        3,
    )
    .unwrap();
    let mut s = tube();
    s.control_points[0][0][0] = 0.125;
    let r = periodic_trim::assemble_on_surface(
        &s,
        std::slice::from_ref(&c),
        std::slice::from_ref(&p),
        [[0., 4.], [0., 1.]],
        [true, false],
        [3.75, 0.5],
        combined_limits(false),
    )
    .unwrap();
    assert_eq!(r.status, periodic_trim::SurfaceChartStatus::SurfaceUnproven);
    assert!(r.graph().is_none() && r.uv.is_none());
    let mut short = combined_limits(false);
    if let periodic_trim::SurfaceUvLimits::Exact(ref mut limits) = short.uv {
        limits.max_edges = 1;
    }
    let r = periodic_trim::assemble_on_surface(
        &tube(),
        &[c],
        &[p],
        [[0., 4.], [0., 1.]],
        [true, false],
        [3.75, 0.5],
        short,
    )
    .unwrap();
    assert_eq!(r.status, periodic_trim::SurfaceChartStatus::UvUnproven);
    assert_eq!(r.reason, Some("edge-budget"));
    assert!(r.support.within_world_tolerance && r.graph().is_none());
}

#[test]
fn combined_world_tolerance_failure_stops_before_uv_publication() {
    let c = closed_trim(4.25);
    let mut p = periodic_trim::split_approximated(
        &c,
        [[0., 4.], [0., 1.]],
        1e-8,
        0.01,
        16,
        32,
        10000,
        30000,
    )
    .unwrap()
    .partition
    .unwrap();
    // Preserve a genuinely nonzero, UV-qualified saved deviation. An exact
    // unchanged partition can now satisfy this strict world bound through
    // correlated original-parameter evidence and must not be falsely refused.
    for piece in &mut p.pieces {
        for point in &mut piece.curve.control_points {
            point[1] += 1e-6;
        }
    }
    assert_eq!(
        periodic_trim::verify_partition(&c, &p, 0.001, 30000)
            .unwrap()
            .status,
        periodic_trim::AdmissionStatus::WithinTolerance
    );
    let mut strict = combined_limits(true);
    strict.correspondence.tolerance_world = 1e-14;
    let r = periodic_trim::assemble_on_surface(
        &tube(),
        &[c],
        &[p],
        [[0., 4.], [0., 1.]],
        [true, false],
        [3.75, 0.5],
        strict,
    )
    .unwrap();
    assert_eq!(r.status, periodic_trim::SurfaceChartStatus::SurfaceUnproven);
    assert_eq!(r.reason, Some("world-tolerance-unproven"));
    assert!(r.support.error_upper > strict.correspondence.tolerance_world);
    assert!(r.graph().is_none() && r.uv.is_none());
}

#[test]
fn approximate_weighted_chart_retains_original_isotopy_and_candidate_winding() {
    let source = closed_trim(4.25);
    let partition = periodic_trim::split_approximated(
        &source,
        [[0., 4.], [0., 1.]],
        1e-8,
        0.01,
        16,
        32,
        10000,
        30000,
    )
    .unwrap()
    .partition
    .unwrap();
    let periodic_trim::SurfaceUvLimits::Approximate(limits) = combined_limits(true).uv else {
        panic!()
    };
    let result = periodic_trim::assemble_approximated_weighted_chart(
        &[source],
        &[partition],
        [[0., 4.], [0., 1.]],
        [3.75, 0.5],
        limits,
    )
    .unwrap();
    assert!(result.candidate_winding_preserved, "{result:?}");
    assert!(result.transfer.winding.is_some());
    let graph = result.graph.unwrap();
    assert_eq!(graph.loops.len(), 2);
    for (q, w) in [([3.75, 0.5], 1), ([0.125, 0.5], 1), ([1., 0.5], 0)] {
        assert_eq!(
            periodic_trim::inspect_weighted_winding(
                &graph,
                [[0., 4.], [0., 1.]],
                q,
                1e-8,
                256,
                30000
            )
            .unwrap()
            .winding,
            Some(w)
        );
    }
}
#[test]
fn approximate_weighted_chart_does_not_bypass_source_topology_or_budget() {
    let source = closed_trim(4.25);
    let mut partition = periodic_trim::split_approximated(
        &source,
        [[0., 4.], [0., 1.]],
        1e-8,
        0.01,
        16,
        32,
        10000,
        30000,
    )
    .unwrap()
    .partition
    .unwrap();
    let periodic_trim::SurfaceUvLimits::Approximate(limits) = combined_limits(true).uv else {
        panic!()
    };
    partition.pieces[0].translation[0] += 0.125;
    let r = periodic_trim::assemble_approximated_weighted_chart(
        &[source.clone()],
        &[partition],
        [[0., 4.], [0., 1.]],
        [3.75, 0.5],
        limits,
    );
    assert!(r.is_err() || !r.unwrap().candidate_winding_preserved);
    let p = periodic_trim::split_approximated(
        &source,
        [[0., 4.], [0., 1.]],
        1e-8,
        0.01,
        16,
        32,
        10000,
        30000,
    )
    .unwrap()
    .partition
    .unwrap();
    let mut low = limits;
    low.isotopy.max_pairs = 1;
    let r = periodic_trim::assemble_approximated_weighted_chart(
        &[source],
        &[p],
        [[0., 4.], [0., 1.]],
        [3.75, 0.5],
        low,
    )
    .unwrap();
    assert!(!r.candidate_winding_preserved && r.graph.is_none());
}

#[test]
fn closed_support_seams_allow_exact_weight_gauges_but_not_nearly_matching_ratios() {
    let (source, partition) = exact_partition();
    let mut s = tube();
    let n = s.weights.len();
    for w in &mut s.weights[n - 1] {
        *w *= 2.;
    }
    let r = periodic_trim::verify_surface_correspondence(
        &s,
        &[source.clone()],
        &[partition.clone()],
        [[0., 4.], [0., 1.]],
        [true, false],
        limits(),
    )
    .unwrap();
    assert!(r.within_world_tolerance, "{r:?}");
    assert_eq!(
        r.seams[0],
        Some(periodic_trim::SurfaceSeamProof::ClampedBoundaryIdentity)
    );
    s.weights[n - 1][1] = s.weights[n - 1][1].next_up();
    let r = periodic_trim::verify_surface_correspondence(
        &s,
        &[source],
        &[partition],
        [[0., 4.], [0., 1.]],
        [true, false],
        limits(),
    )
    .unwrap();
    assert!(!r.within_world_tolerance && r.seams[0].is_none());
}
