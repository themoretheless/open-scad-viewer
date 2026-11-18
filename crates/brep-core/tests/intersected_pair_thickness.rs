use brep_core::shell_thickness::intersected_pair::{Limits, OffsetSettings, Search, thicken};
use nurbs_core::{surface::Surface, surface_contact::Verdict};
fn cylinder() -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![1., 0., 0.], vec![1., 0., 1.]],
            vec![vec![1., 1., 0.], vec![1., 1., 1.]],
            vec![vec![0., 1., 0.], vec![0., 1., 1.]],
        ],
        weights: vec![
            vec![1.; 2],
            vec![std::f64::consts::FRAC_1_SQRT_2; 2],
            vec![1.; 2],
        ],
        periodic_u: false,
        periodic_v: false,
    }
}
fn plane() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 1., 0.], vec![0., 1., 1.]],
            vec![vec![0., 2., 0.], vec![0., 2., 1.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn cylinder_plane_contact_is_found_without_authored_caps_roots_or_windows() {
    let sources = [cylinder(), plane()];
    let saved = sources.clone();
    let r = thicken(
        &sources,
        [false; 2],
        OffsetSettings {
            signed_thickness: 0.1,
            normal_tolerance: 0.01,
            normal_cells: 100000,
            sewing_tolerance: 1e-7,
        },
        Limits::default(),
        Search::default(),
    )
    .unwrap();
    assert_eq!(r.source_seam(), [1, 3]);
    assert_eq!(r.model().faces.len(), 10);
    assert!(matches!(r.section().certificate, Some(Verdict::Witness(_))));
    assert!(r.section().iterations > 0);
    assert!(r.section().residual_mm <= 1e-11);
    assert!(r.mapped().source_windows()[0][0][1] < 1.);
    assert!(r.mapped().source_windows()[1][0][0] > 0.);
    assert!(r.mapped().mapped_normal_offset_error_upper() <= 0.01);
    assert_eq!(r.model().validate().unwrap().boundary_edge_count, 0);
    assert!(r.reverify().unwrap().audit.ok);
    assert_eq!(sources, saved);
}
#[test]
fn nonunit_source_clocks_and_reversed_chart_orientation_are_retained() {
    let mut a = cylinder();
    a.knots_u = vec![2., 2., 2., 7., 7., 7.];
    a.knots_v = vec![-3., -3., 1., 1.];
    let mut b = plane();
    b.knots_u = vec![10., 10., 20., 20.];
    b.knots_v = vec![4., 4., 8., 8.];
    for row in &mut b.control_points {
        row.reverse();
    }
    for row in &mut b.weights {
        row.reverse();
    }
    let sources = [a, b];
    let r = thicken(
        &sources,
        [false, true],
        OffsetSettings {
            signed_thickness: 0.1,
            normal_tolerance: 0.01,
            normal_cells: 100000,
            sewing_tolerance: 1e-7,
        },
        Limits::default(),
        Search::default(),
    )
    .unwrap();
    assert_eq!(r.model().faces.len(), 10);
    assert!(r.reverify().unwrap().boundary_complete);
    for i in 0..2 {
        assert_eq!(r.model().faces[i].surface, sources[i]);
    }
}
#[test]
fn missing_or_ambiguous_original_seams_and_incomplete_search_refuse() {
    let sources = [cylinder(), plane()];
    let mut detached = sources.clone();
    for p in detached[1].control_points.iter_mut().flatten() {
        p[0] += 0.2;
    }
    assert!(
        thicken(
            &detached,
            [false; 2],
            OffsetSettings {
                signed_thickness: 0.1,
                normal_tolerance: 0.01,
                normal_cells: 100000,
                sewing_tolerance: 1e-7,
            },
            Limits::default(),
            Search::default(),
        )
        .is_err()
    );
    let duplicates = [plane(), plane()];
    assert!(
        thicken(
            &duplicates,
            [false, true],
            OffsetSettings {
                signed_thickness: 0.1,
                normal_tolerance: 0.01,
                normal_cells: 100000,
                sewing_tolerance: 1e-7,
            },
            Limits::default(),
            Search::default(),
        )
        .is_err()
    );
    let search = Search {
        iterations: 1,
        ..Search::default()
    };
    assert!(
        thicken(
            &sources,
            [false; 2],
            OffsetSettings {
                signed_thickness: 0.1,
                normal_tolerance: 0.01,
                normal_cells: 100000,
                sewing_tolerance: 1e-7,
            },
            Limits::default(),
            search,
        )
        .is_err()
    );
    assert!(
        thicken(
            &sources,
            [false; 2],
            OffsetSettings {
                signed_thickness: 0.1,
                normal_tolerance: 0.01,
                normal_cells: 1,
                sewing_tolerance: 1e-7,
            },
            Limits::default(),
            Search::default(),
        )
        .is_err()
    );
}
#[test]
fn extension_outside_original_supports_is_not_silently_extrapolated() {
    let sources = [cylinder(), plane()];
    assert!(
        thicken(
            &sources,
            [false; 2],
            OffsetSettings {
                signed_thickness: -0.1,
                normal_tolerance: 0.01,
                normal_cells: 100000,
                sewing_tolerance: 1e-7,
            },
            Limits::default(),
            Search::default(),
        )
        .is_err()
    );
    let search = Search {
        iterations: 0,
        ..Search::default()
    };
    assert!(
        thicken(
            &sources,
            [false; 2],
            OffsetSettings {
                signed_thickness: 0.1,
                normal_tolerance: 0.01,
                normal_cells: 100000,
                sewing_tolerance: 1e-7,
            },
            Limits::default(),
            search,
        )
        .is_err()
    );
}

#[test]
fn gauges_and_projective_source_clocks_have_complete_seam_proofs() {
    for clock in [0., 2., 3.] {
        let mut a = cylinder();
        let mut b = plane();
        if clock > 0. {
            for w in &mut a.weights {
                w[1] *= clock;
            }
        } else {
            for w in b.weights.iter_mut().flatten() {
                *w *= 2.;
            }
        }
        let sources = [a, b];
        let limits = Limits::default();
        let r = thicken(
            &sources,
            [false; 2],
            OffsetSettings {
                signed_thickness: 0.1,
                normal_tolerance: 0.01,
                normal_cells: 100000,
                sewing_tolerance: 1e-7,
            },
            limits,
            Search::default(),
        )
        .unwrap();
        assert_eq!(r.source_seam(), [1, 3]);
        assert_eq!(r.model().validate().unwrap().boundary_edge_count, 0);
        assert!(r.exact_work() <= limits.exact_work);
        assert!(r.sewing_proof_cells() <= limits.proof.max_cells);
        if clock == 3. {
            assert!(
                r.sewing_proof_cells() > r.mapped().evidence().cells,
                "Rounded projective UV clock needs the complete source lift proof"
            );
        }
        assert!(r.reverify().unwrap().audit.ok);
        for i in 0..2 {
            assert_eq!(r.model().faces[i].surface, sources[i]);
        }
    }
}

#[test]
fn transposed_first_surface_drives_the_other_parameter_axis() {
    let s = cylinder();
    let a = Surface {
        degree_u: s.degree_v,
        degree_v: s.degree_u,
        knots_u: s.knots_v.clone(),
        knots_v: s.knots_u.clone(),
        control_points: (0..2)
            .map(|j| (0..3).map(|i| s.control_points[i][j].clone()).collect())
            .collect(),
        weights: (0..2)
            .map(|j| (0..3).map(|i| s.weights[i][j]).collect())
            .collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let sources = [a, plane()];
    let r = thicken(
        &sources,
        [true, false],
        OffsetSettings {
            signed_thickness: 0.1,
            normal_tolerance: 0.01,
            normal_cells: 100000,
            sewing_tolerance: 1e-7,
        },
        Limits::default(),
        Search::default(),
    )
    .unwrap();
    assert_eq!(r.section().fixed_axis, 0);
    assert_eq!(r.source_seam(), [2, 3]);
    assert!(r.reverify().unwrap().audit.ok);
}

#[test]
fn a_section_root_cannot_admit_a_varying_nonrectangular_intersection() {
    let mut b = plane();
    b.control_points[1][1][0] = 0.5;
    let sources = [cylinder(), b];
    // Establish that the section itself succeeds; refusal must come from
    // constructing/proving the full rectangular-cap proposal, not root search.
    let section = nurbs_core::offset_contact_predictor::propose(
        nurbs_core::offset_contact_pcurve::OffsetFaces {
            surfaces: sources.each_ref(),
            distances: [0.1; 2],
        },
        [[1., 0.5], [0., 0.5]],
        1,
        1e-11,
        32,
        1e-3,
        128,
    )
    .unwrap();
    assert!(matches!(section.certificate, Some(Verdict::Witness(_))));
    let error = match thicken(
        &sources,
        [false; 2],
        OffsetSettings {
            signed_thickness: 0.1,
            normal_tolerance: 0.01,
            normal_cells: 100000,
            sewing_tolerance: 1e-7,
        },
        Limits::default(),
        Search::default(),
    ) {
        Ok(_) => panic!("A midpoint root must not qualify the varying full seam"),
        Err(e) => e,
    };
    assert!(!error.message.contains("section remains unproven"));
}

#[test]
fn coincident_endpoints_do_not_establish_a_shared_source_curve() {
    let mut b =
        nurbs_core::surface_edit::elevate(&plane(), nurbs_core::surface::Axis::V, 2).unwrap();
    b.control_points[0][1][0] = 0.05;
    let sources = [cylinder(), b];
    let err = match thicken(
        &sources,
        [false; 2],
        OffsetSettings {
            signed_thickness: 0.1,
            normal_tolerance: 0.01,
            normal_cells: 100000,
            sewing_tolerance: 1e-7,
        },
        Limits::default(),
        Search::default(),
    ) {
        Ok(_) => panic!("Different complete curves were mistaken for a shared seam"),
        Err(e) => e,
    };
    assert!(err.message.contains("shared boundary"), "{}", err.message);
}
