use nurbs_core::thread::{self, Hand, Kind, Spec};
#[test]
fn thread_patches_follow_analytic_roll_and_multistart_lead() {
    for kind in [Kind::External, Kind::Internal] {
        for hand in [Hand::Right, Hand::Left] {
            let spec = Spec {
                starts: 2,
                turns: 2,
                kind,
                hand,
                ..Spec::default()
            };
            let a = thread::patches(spec).unwrap();
            assert_eq!(a.patches.len(), 80);
            assert_eq!(a.lead, 2.);
            assert_eq!(a.axial_extent, [0., 6.]);
            assert!(!a.rounding_certified);
            let phases = [0., 1. / 16., 3. / 8., 5. / 8., 15. / 16., 1.];
            let radii = [
                a.crest_radius,
                a.crest_radius,
                a.root_radius,
                a.root_radius,
                a.crest_radius,
                a.crest_radius,
            ];
            let sign = if hand == Hand::Right { 1. } else { -1. };
            for patch in &a.patches {
                assert!(patch.real_arithmetic_error_estimate <= spec.error_budget);
                for u in [0., 0.37, 1.] {
                    for v in [0., 0.29, 1.] {
                        let r = (1. - u) * radii[patch.profile_segment]
                            + u * radii[patch.profile_segment + 1];
                        let phase = (1. - u) * phases[patch.profile_segment]
                            + u * phases[patch.profile_segment + 1];
                        let angle = sign * std::f64::consts::FRAC_PI_2 * (patch.quarter as f64 + v);
                        let z = patch.start as f64 * spec.pitch
                            + (patch.turn as f64 + (patch.quarter as f64 + v) / 4.) * a.lead
                            + phase * spec.pitch;
                        let p = patch.surface.evaluate(u, v).unwrap().point;
                        let error = (p[0] - r * angle.cos()).hypot(p[1] - r * angle.sin());
                        assert!(error <= patch.real_arithmetic_error_estimate + 1e-11);
                        assert!((p[2] - z).abs() < 1e-11);
                    }
                }
            }
        }
    }
}
#[test]
fn internal_external_clearance_and_invalid_thread_refusals() {
    let external = thread::patches(Spec::default()).unwrap();
    let internal = thread::patches(Spec {
        kind: Kind::Internal,
        ..Spec::default()
    })
    .unwrap();
    assert!((internal.crest_radius - external.crest_radius - 0.1).abs() < 1e-12);
    assert!((internal.root_radius - external.root_radius - 0.1).abs() < 1e-12);
    for s in [
        Spec {
            starts: 0,
            ..Spec::default()
        },
        Spec {
            turns: 17,
            ..Spec::default()
        },
        Spec {
            diameter: 0.5,
            ..Spec::default()
        },
        Spec {
            pitch: 0.,
            ..Spec::default()
        },
        Spec {
            clearance: -1.,
            ..Spec::default()
        },
        Spec {
            error_budget: 1e-30,
            ..Spec::default()
        },
    ] {
        assert!(thread::patches(s).is_err());
    }
}

#[test]
fn adjacent_profile_segments_and_quarter_sweeps_match_at_boundaries() {
    let a = thread::patches(Spec::default()).unwrap();
    for quarter in 0..4 {
        for segment in 0..4 {
            let left = &a.patches[quarter * 5 + segment].surface;
            let right = &a.patches[quarter * 5 + segment + 1].surface;
            for v in [0., 0.41, 1.] {
                let p = left.evaluate(1., v).unwrap().point;
                let q = right.evaluate(0., v).unwrap().point;
                for j in 0..3 {
                    assert!((p[j] - q[j]).abs() < 1e-11);
                }
            }
        }
    }
    for quarter in 0..3 {
        for segment in 0..5 {
            let left = &a.patches[quarter * 5 + segment].surface;
            let right = &a.patches[(quarter + 1) * 5 + segment].surface;
            for u in [0., 0.41, 1.] {
                let p = left.evaluate(u, 1.).unwrap().point;
                let q = right.evaluate(u, 0.).unwrap().point;
                for j in 0..3 {
                    assert!((p[j] - q[j]).abs() < 1e-11);
                }
            }
        }
    }
    assert!(
        thread::patches(Spec {
            diameter: 1e8,
            pitch: 1e-20,
            ..Spec::default()
        })
        .is_err()
    );
}

#[test]
fn finite_trim_candidates_bound_stored_z_over_complete_retained_polygons() {
    let a = thread::trim_candidates(
        Spec {
            starts: 2,
            turns: 2,
            ..Spec::default()
        },
        [0.31, 2.13],
        1e-10,
    )
    .unwrap();
    assert!(!a.topology_certified);
    assert!(!a.candidates.is_empty());
    for c in &a.candidates {
        assert!(c.within_axial_tolerance, "{c:?}");
        assert!(c.affine_z_error_upper_bound >= 0.);
        assert!(c.retained_z_bounds[0] >= 0.31 - 1e-10);
        assert!(c.retained_z_bounds[1] <= 2.13 + 1e-10);
        // Independent evaluations on vertices and inside polygon fan triangles.
        // The reported bounds arise from all control coefficients, not these samples.
        let first = c.polygon_uv[0];
        for pair in c.polygon_uv[1..].windows(2) {
            for (w0, w1, w2) in [(1., 0., 0.), (0., 1., 0.), (0., 0., 1.), (0.2, 0.3, 0.5)] {
                let u = w0 * first[0] + w1 * pair[0][0] + w2 * pair[1][0];
                let v = w0 * first[1] + w1 * pair[0][1] + w2 * pair[1][1];
                let z = c.patch.surface.evaluate(u, v).unwrap().point[2];
                assert!(z >= c.retained_z_bounds[0] && z <= c.retained_z_bounds[1]);
            }
        }
    }
    assert!(
        thread::trim_candidates(Spec::default(), [10., 11.], 1e-9)
            .unwrap()
            .candidates
            .is_empty()
    );
    for (bounds, tol) in [([1., 0.], 1e-9), ([0., f64::NAN], 1e-9), ([0., 1.], 0.)] {
        assert!(thread::trim_candidates(Spec::default(), bounds, tol).is_err());
    }
    let tight = thread::trim_candidates(Spec::default(), [0.31, 0.81], 1e-30).unwrap();
    assert!(tight.candidates.iter().any(|c| !c.within_axial_tolerance));
}

#[test]
fn thread_trim_accounts_for_every_patch_and_keeps_collapsed_candidates_unresolved() {
    use std::collections::BTreeSet;
    let a = thread::trim_candidates(
        Spec {
            starts: 2,
            turns: 2,
            ..Spec::default()
        },
        [0.31, 2.13],
        1e-10,
    )
    .unwrap();
    let mut keys = BTreeSet::new();
    let key = |p: &thread::Patch| [p.start, p.turn, p.quarter, p.profile_segment];
    for c in &a.candidates {
        assert!(keys.insert(key(&c.patch)));
    }
    for c in &a.excluded {
        assert!(keys.insert(key(&c.patch)));
        assert!(c.source_z_bounds[1] < 0.31 || c.source_z_bounds[0] > 2.13);
    }
    for c in &a.unresolved {
        assert!(keys.insert(key(&c.patch)));
    }
    assert_eq!(keys.len(), 80);
    let outside = thread::trim_candidates(Spec::default(), [10., 11.], 1e-9).unwrap();
    assert_eq!(outside.excluded.len(), 20);
    assert!(outside.unresolved.is_empty());
    // The closed slab only touches the last patch boundary at z=2. A
    // zero-area UV result cannot justify silently deleting that contact.
    let contact = thread::trim_candidates(Spec::default(), [2., 3.], 1e-9).unwrap();
    assert!(!contact.unresolved.is_empty());
    assert_eq!(
        contact.candidates.len() + contact.excluded.len() + contact.unresolved.len(),
        20
    );
    assert!(!contact.topology_certified);
}

#[test]
fn exact_uv_recipes_preserve_parallel_plane_polygon_and_thin_slabs() {
    use thread::ExactUvVertex::{Corner, EdgePlane};
    let lower = 1. / 32.;
    let upper = 1. / 8.;
    let a = thread::trim_candidates(Spec::default(), [lower, upper], 1e-10).unwrap();
    let first = a
        .candidates
        .iter()
        .find(|p| {
            p.patch.start == 0
                && p.patch.turn == 0
                && p.patch.quarter == 0
                && p.patch.profile_segment == 0
        })
        .unwrap();
    assert_eq!(first.affine_z_coefficients, [0., 1. / 16., 1. / 4.]);
    // Independently solving u/16+v/4 in [1/32,1/8] yields this pentagon.
    let expected = [
        EdgePlane {
            edge: 0,
            z_limit: lower,
        },
        Corner { corner: 1 },
        EdgePlane {
            edge: 1,
            z_limit: upper,
        },
        EdgePlane {
            edge: 3,
            z_limit: upper,
        },
        EdgePlane {
            edge: 3,
            z_limit: lower,
        },
    ];
    assert_eq!(first.exact_uv_vertices.len(), 5);
    for vertex in expected {
        assert!(first.exact_uv_vertices.contains(&vertex));
    }
    for expected in [[0.5, 0.], [1., 0.], [1., 0.25], [0., 0.5], [0., 0.125]] {
        assert!(
            first
                .polygon_uv
                .iter()
                .any(|p| (p[0] - expected[0]).abs() < 1e-14 && (p[1] - expected[1]).abs() < 1e-14)
        );
    }
    let upper = 0.125_f64.next_up();
    let thin = thread::trim_candidates(Spec::default(), [0.125, upper], 1e-10).unwrap();
    let first = thin
        .candidates
        .iter()
        .find(|p| p.patch.quarter == 0 && p.patch.profile_segment == 0)
        .unwrap();
    assert_eq!(first.exact_uv_vertices.len(), 4);
    for edge in [1, 3] {
        assert!(first.exact_uv_vertices.contains(&EdgePlane {
            edge,
            z_limit: 0.125
        }));
        assert!(first.exact_uv_vertices.contains(&EdgePlane {
            edge,
            z_limit: upper
        }));
    }
    assert!(!thin.topology_certified);
}

#[test]
fn pcurve_conversion_certifies_uv_edges_and_refuses_unattainable_tolerance() {
    let a = thread::trim_candidates(Spec::default(), [1. / 32., 1. / 8.], 1e-10).unwrap();
    let c = a
        .candidates
        .iter()
        .find(|p| p.patch.quarter == 0 && p.patch.profile_segment == 0)
        .unwrap();
    let p = c.pcurve(1e-12).unwrap();
    assert_eq!(p.curve.degree, 1);
    assert_eq!(
        p.curve.control_points.first(),
        p.curve.control_points.last()
    );
    assert!(p.max_uv_deviation_upper_bound <= 1e-12);
    let expected = [[0.5, 0.], [1., 0.], [1., 0.25], [0., 0.5], [0., 0.125]];
    // The independently derived polygon has the same cyclic vertex order.
    for (i, point) in p.curve.control_points[..5].iter().enumerate() {
        let index = expected
            .iter()
            .position(|e| (point[0] - e[0]).abs() < 1e-12 && (point[1] - e[1]).abs() < 1e-12)
            .unwrap();
        let next = expected[(index + 1) % 5];
        let v = p.curve.evaluate(i as f64 + 0.37).unwrap().point;
        let exact = [
            0.63 * expected[index][0] + 0.37 * next[0],
            0.63 * expected[index][1] + 0.37 * next[1],
        ];
        assert!((v[0] - exact[0]).hypot(v[1] - exact[1]) <= p.max_uv_deviation_upper_bound + 1e-15);
    }
    assert!(c.pcurve(1e-30).is_err());
    let mut changed = c.clone();
    changed.polygon_uv = vec![[999.; 2]; 3];
    assert_eq!(changed.pcurve(1e-12).unwrap().curve, p.curve);
    use thread::ExactUvVertex::*;
    assert!(
        thread::pcurve_from_exact_vertices(
            &[
                Corner { corner: 0 },
                Corner { corner: 9 },
                Corner { corner: 2 }
            ],
            [0., 1., 1.],
            1e-12
        )
        .is_err()
    );
}

#[test]
fn mapped_pcurve_certifies_world_deviation_and_refuses_budget_failures() {
    let a = thread::trim_candidates(Spec::default(), [1. / 32., 1. / 8.], 1e-10).unwrap();
    let c = a
        .candidates
        .iter()
        .find(|p| p.patch.quarter == 0 && p.patch.profile_segment == 0)
        .unwrap();
    let m = c.pcurve_with_world_bound(1e-12, 1e-10, 10).unwrap();
    assert!(m.max_3d_deviation_upper_bound <= 1e-10);
    assert!(
        m.derivative_norm_upper_bounds
            .iter()
            .all(|x| x.is_finite() && *x > 0.)
    );
    assert!(c.pcurve_with_world_bound(1e-12, 1e-30, 10).is_err());
    assert!(c.pcurve_with_world_bound(1e-12, 1e-10, 0).is_err());
    let exact = [[0.5, 0.], [1., 0.], [1., 0.25], [0., 0.5], [0., 0.125]];
    for (i, p) in m.pcurve.curve.control_points[..5].iter().enumerate() {
        let index = exact
            .iter()
            .position(|e| (p[0] - e[0]).abs() < 1e-12 && (p[1] - e[1]).abs() < 1e-12)
            .unwrap();
        let next = exact[(index + 1) % 5];
        for t in [0., 0.37, 1.] {
            let rounded = m.pcurve.curve.evaluate(i as f64 + t).unwrap().point;
            let ideal = [
                (1. - t) * exact[index][0] + t * next[0],
                (1. - t) * exact[index][1] + t * next[1],
            ];
            let p = c
                .patch
                .surface
                .evaluate(rounded[0], rounded[1])
                .unwrap()
                .point;
            let q = c.patch.surface.evaluate(ideal[0], ideal[1]).unwrap().point;
            let distance = (0..3).map(|j| (p[j] - q[j]).powi(2)).sum::<f64>().sqrt();
            assert!(distance <= m.max_3d_deviation_upper_bound + 1e-14);
        }
    }
}

#[test]
fn mapped_edges_require_continuous_agreement_and_keep_budget_exhaustion() {
    use nurbs_core::curve_surface_agreement::Status;
    let a = thread::trim_candidates(Spec::default(), [1. / 32., 1. / 8.], 1e-10).unwrap();
    let c = a
        .candidates
        .iter()
        .find(|p| p.patch.quarter == 0 && p.patch.profile_segment == 0)
        .unwrap();
    let edges = c.mapped_edges(1e-12, 1e-3, 128, 100000).unwrap();
    assert_eq!(edges.edges.len(), 5);
    assert!(edges.all_edges_within_tolerance, "{edges:?}");
    assert!(edges.verification_cells <= 100000);
    for edge in &edges.edges {
        assert_eq!(edge.agreement.status, Status::WithinTolerance);
        for i in 0..=20 {
            let t = i as f64 / 20.;
            let uv = edge.pcurve.evaluate(t).unwrap().point;
            let expected = c.patch.surface.evaluate(uv[0], uv[1]).unwrap().point;
            let d = edge.curve.domain();
            let actual = edge.curve.evaluate(d[0] + (d[1] - d[0]) * t).unwrap().point;
            let error = (0..3)
                .map(|j| (actual[j] - expected[j]).powi(2))
                .sum::<f64>()
                .sqrt();
            assert!(error <= 1e-3 + 1e-12);
        }
    }
    let none = c.mapped_edges(1e-12, 1e-3, 128, 0).unwrap();
    assert!(!none.all_edges_within_tolerance);
    assert_eq!(none.verification_cells, 0);
    assert!(
        none.edges
            .iter()
            .all(|e| e.agreement.status == Status::Unresolved)
    );
    let limited = c.mapped_edges(1e-12, 1e-12, 1, 10).unwrap();
    assert!(!limited.all_edges_within_tolerance);
    assert!(limited.verification_cells <= 10);
}

#[test]
fn planar_cap_edges_reverify_modified_controls_and_share_budget() {
    use nurbs_core::curve_surface_agreement::Status;
    let a = thread::trim_candidates(Spec::default(), [1. / 32., 1. / 8.], 1e-10).unwrap();
    let c = a
        .candidates
        .iter()
        .find(|p| p.patch.quarter == 0 && p.patch.profile_segment == 0)
        .unwrap();
    for plane in [1. / 32., 1. / 8.] {
        let e = c.planar_edges(plane, 1e-12, 1e-3, 128, 100000).unwrap();
        assert_eq!(e.edges.len(), 1);
        assert!(e.all_edges_within_tolerance);
        assert!(e.verification_cells <= 100000);
        assert!(!e.topology_certified);
        for edge in &e.edges {
            assert_eq!(edge.mapped.agreement.status, Status::WithinTolerance);
            assert!(
                edge.mapped
                    .curve
                    .control_points
                    .iter()
                    .all(|p| p[2] == plane)
            );
            for t in [0., 0.37, 1.] {
                let domain = edge.mapped.curve.domain();
                let p = edge
                    .mapped
                    .curve
                    .evaluate(domain[0] + t * (domain[1] - domain[0]))
                    .unwrap()
                    .point;
                assert!((p[2] - plane).abs() < 1e-15);
            }
        }
    }
    let e = c.planar_edges(1. / 8., 1e-12, 1e-3, 128, 0).unwrap();
    assert!(!e.all_edges_within_tolerance);
    assert_eq!(e.verification_cells, 0);
    assert_eq!(e.edges[0].mapped.agreement.status, Status::Unresolved);
    let e = c.planar_edges(0.2, 1e-12, 1e-3, 128, 100000).unwrap();
    assert!(e.edges.is_empty());
    assert!(!e.all_edges_within_tolerance);
}

#[test]
fn complete_interior_thread_cap_fragments_form_one_unique_endpoint_cycle() {
    let a = thread::trim_candidates(
        Spec {
            turns: 2,
            ..Spec::default()
        },
        [1.13, 1.87],
        1e-10,
    )
    .unwrap();
    let mut curves = Vec::new();
    let mut contexts = Vec::new();
    for c in &a.candidates {
        let count = c
            .exact_uv_vertices
            .iter()
            .filter(|v| {
                matches!(v,
            thread::ExactUvVertex::EdgePlane {z_limit,..} if *z_limit==1.13)
            })
            .count();
        if count < 2 {
            continue;
        }
        let edges = c.planar_edges(1.13, 1e-12, 1e-3, 128, 100000).unwrap();
        assert!(edges.all_edges_within_tolerance);
        for edge in edges.edges {
            curves.push(edge.mapped.curve);
            contexts.push((edge.mapped.pcurve, c.patch.surface.clone()));
        }
    }
    assert!(curves.len() > 3);
    let r = nurbs_core::curve_chain::assemble(&curves, 1e-8, 65536).unwrap();
    assert!(r.all_chains_closed_within_tolerance, "{r:?}");
    assert_eq!(r.cyclic_chains.len(), 1);
    assert_eq!(r.cyclic_chains[0].len(), curves.len());
    assert!(r.pairs.iter().all(|p| p.gap_bounds[1] <= 1e-8));
    let closed = nurbs_core::curve_chain::close(&curves, 1e-8, 1e-8, 65536).unwrap();
    assert!(closed.within_tolerance);
    let shared = closed.curves.unwrap();
    assert!(
        nurbs_core::curve_chain::assemble(&shared, 1e-12, 65536)
            .unwrap()
            .all_endpoints_equal
    );
    for (edge, (pcurve, surface)) in shared.iter().zip(&contexts) {
        let agreement =
            nurbs_core::curve_surface_agreement::verify(edge, pcurve, surface, false, 1e-3, 100000)
                .unwrap();
        assert_eq!(
            agreement.status,
            nurbs_core::curve_surface_agreement::Status::WithinTolerance
        );
        assert!(edge.control_points.iter().all(|p| p[2] == 1.13));
    }
}
