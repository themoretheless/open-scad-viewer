use brep_core::shell_thickness::contact_pair::{Band, Limits, Tolerances, thicken};
use nurbs_core::{surface::Surface, surface_edit};

fn plane() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn planes() -> [Surface; 2] {
    let a = plane();
    let mut b = plane();
    for p in b.control_points.iter_mut().flatten() {
        *p = vec![0., p[0], p[1]];
    }
    [a, b]
}
fn curved() -> [Surface; 2] {
    let mut a = plane();
    a.degree_u = 2;
    a.knots_u = vec![0., 0., 0., 1., 1., 1.];
    a.control_points = [[1., 0.], [1., 1.], [0., 1.]]
        .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 1.]])
        .to_vec();
    a.weights = [1., std::f64::consts::FRAC_1_SQRT_2, 1.]
        .map(|w| vec![w; 2])
        .to_vec();
    let mut b = plane();
    b.control_points = vec![
        vec![vec![0., 1., 0.], vec![0., 1., 1.]],
        vec![vec![0., 2., 0.], vec![0.5, 2., 1.]],
    ];
    [a, b]
}
fn curved_band() -> Band {
    Band {
        axis: 1,
        drive: [0.35, 0.55],
        first_other: [0.88, 0.96],
        second: [[0.07, 0.22], [0.3, 0.6]],
    }
}
fn check(s: [Surface; 2], reverse: [bool; 2], band: Band, normal: f64, seam: [usize; 2]) {
    let saved = s.clone();
    let l = Limits::default();
    let r = thicken(
        &s,
        reverse,
        0.1,
        band,
        Tolerances {
            uv: 1e-7,
            normal,
            sewing: 1e-7,
        },
        l,
    ).unwrap();
    assert_eq!(r.source_seam(), seam);
    assert_eq!(r.model().faces.len(), 10);
    assert_eq!(r.constructed().exposed_boundaries().len(), 6);
    assert_eq!(r.model().validate().unwrap().boundary_edge_count, 0);
    assert_eq!(r.constructed().source_patches(), &saved);
    assert!(r.world_proof().world_correspondence_proven);
    assert!(r.world_proof().error_upper_mm.unwrap() <= 1e-7);
    assert!(r.constructed().mapped_normal_offset_error_upper() <= normal);
    // Literal shared coefficient ownership, including strip clock reversal.
    for (cap, strip) in r
        .constructed()
        .cap_patches()
        .iter()
        .zip(r.uv_regions().strips())
    {
        let curve = r.contact_curve();
        for k in 0..curve.control_points.len() {
            let (u, v) = match strip.contact_boundary() {
                0 => (k, 0),
                1 => (cap.control_points.len() - 1, k),
                2 => (k, cap.control_points[0].len() - 1),
                _ => (0, k),
            };
            let j = if strip.clock_reversed() {
                curve.control_points.len() - 1 - k
            } else {
                k
            };
            assert_eq!(cap.control_points[u][v], curve.control_points[j]);
            assert_eq!(cap.weights[u][v], curve.weights[j]);
        }
    }
    assert!(r.root_queries() <= l.branch_fit.root_queries);
    assert!(r.branch_proof_cells() <= l.branch_fit.proof_cells);
    assert!(r.branch_coefficient_work() <= l.branch_fit.coefficient_work);
    assert!(r.cap_evaluations() <= l.caps.evaluations);
    assert!(r.cap_coefficient_work() <= l.caps.coefficient_work);
    assert!(r.normal_cells() <= l.caps.proof_cells);
    assert!(r.sewing_cells() <= l.sewing.proof.max_cells);
    assert!(r.exact_work() <= l.sewing.exact_work);
    assert!(r.reverify().unwrap().audit.ok);
    assert_eq!(s, saved);
}
#[test]
fn perpendicular_planes_bind_one_actual_contact_boundary() {
    check(
        planes(),
        [false; 2],
        Band {
            axis: 1,
            drive: [0.2, 0.8],
            first_other: [0.05, 0.2],
            second: [[0.1, 0.9], [0.05, 0.2]],
        },
        1e-5,
        [3, 0],
    );
}
#[test]
fn nonlinear_contact_builds_closed_caps_and_replays_original_authority() {
    check(curved(), [false; 2], curved_band(), 0.005, [1, 3]);
}
#[test]
fn transposed_source_preserves_effective_normal_and_boundary_clock() {
    let mut s = curved();
    s[0] = surface_edit::transpose(&s[0]).unwrap();
    let mut b = curved_band();
    b.axis = 0;
    check(s, [true, false], b, 0.005, [2, 3]);
}
#[test]
fn exhausted_shared_work_never_publishes_a_partial_body_or_mutates_sources() {
    let s = planes();
    let saved = s.clone();
    let b = Band {
        axis: 1,
        drive: [0.2, 0.8],
        first_other: [0.05, 0.2],
        second: [[0.1, 0.9], [0.05, 0.2]],
    };
    for field in 0..8 {
        let mut l = Limits::default();
        match field {
            0 => l.branch_fit.root_queries = 1,
            1 => l.branch_fit.coefficient_work = 1,
            2 => l.branch_fit.proof_cells = 1,
            3 => l.caps.evaluations = 1,
            4 => l.caps.coefficient_work = 1,
            5 => l.caps.proof_cells = 1,
            6 => l.sewing.exact_work = 1,
            _ => l.sewing.proof.max_cells = 1,
        }
        assert!(thicken(
            &s,
            [false; 2],
            0.1,
            b,
            Tolerances {
                uv: 1e-7,
                normal: 1e-5,
                sewing: 1e-7,
            },
            l,
        ).is_err());
    }
    assert_eq!(s, saved);
}

#[test]
fn missing_original_seam_and_wrong_contact_band_are_refused() {
    let s = planes();
    let good = Band {
        axis: 1,
        drive: [0.2, 0.8],
        first_other: [0.05, 0.2],
        second: [[0.1, 0.9], [0.05, 0.2]],
    };
    let mut detached = s.clone();
    for p in detached[1].control_points.iter_mut().flatten() {
        p[0] += 2.;
    }
    assert!(
        thicken(
            &detached,
            [false; 2],
            0.1,
            good,
            Tolerances {
                uv: 1e-7,
                normal: 1e-5,
                sewing: 1e-7,
            },
            Limits::default(),
        )
        .is_err()
    );
    let wrong = Band {
        first_other: [0.3, 0.4],
        ..good
    };
    assert!(
        thicken(
            &s,
            [false; 2],
            0.1,
            wrong,
            Tolerances {
                uv: 1e-7,
                normal: 1e-5,
                sewing: 1e-7,
            },
            Limits::default(),
        )
        .is_err()
    );
    assert_eq!(s, planes());
}

#[test]
fn unseeded_discovery_constructs_local_skin_and_retains_boundary_pending() {
    use brep_core::shell_thickness::contact_pair::{DiscoveryLimits, discover};
    let s = planes();
    let saved = s.clone();
    let mut limits = DiscoveryLimits::default();
    limits.max_constructions = 1;
    let r = discover(
        &s,
        [false; 2],
        0.1,
        Tolerances {
            uv: 1e-7,
            normal: 1e-5,
            sewing: 1e-7,
        },
        limits,
    ).unwrap();
    assert!(!r.attempts().is_empty(), "no qualified discovered path");
    let a = &r.attempts()[0];
    assert!(a.local_skin().is_some(), "{}", a.reason());
    assert_eq!(
        a.local_skin()
            .unwrap()
            .model()
            .validate()
            .unwrap()
            .boundary_edge_count,
        0
    );
    assert!(!r.complete_contact_coverage());
    assert!(!r.discovery().coverage.pending.is_empty());
    assert!(!a.covers_all_original_roots());
    assert!(r.attempts().len() <= limits.max_constructions);
    r.reverify().unwrap();
    assert_eq!(s, saved);
}

#[test]
fn automatic_absence_and_exhaustion_never_become_a_complete_skin() {
    use brep_core::shell_thickness::contact_pair::{DiscoveryLimits, discover};
    let mut s = planes();
    for p in s[1].control_points.iter_mut().flatten() {
        p[0] += 3.;
    }
    let r = discover(
        &s,
        [false; 2],
        0.1,
        Tolerances {
            uv: 1e-7,
            normal: 1e-5,
            sewing: 1e-7,
        },
        DiscoveryLimits::default(),
    )
    .unwrap();
    assert!(r.discovery().coverage.absence_proven);
    assert!(r.attempts().is_empty() && !r.complete_contact_coverage());
    let mut l = DiscoveryLimits::default();
    l.search.predictor_queries = 1;
    l.search.qualification.boxes = 1;
    l.search.qualification.path_cells = 16;
    l.search.path.cells = 1;
    let r = discover(
        &planes(),
        [false; 2],
        0.1,
        Tolerances {
            uv: 1e-7,
            normal: 1e-5,
            sewing: 1e-7,
        },
        l,
    ).unwrap();
    assert!(!r.complete_contact_coverage());
    assert!(!r.discovery().coverage.pending.is_empty());
    l.max_constructions = 0;
    assert!(discover(
        &s,
        [false; 2],
        0.1,
        Tolerances {
            uv: 1e-7,
            normal: 1e-5,
            sewing: 1e-7,
        },
        l,
    ).is_err());
}

#[test]
fn unseeded_nonlinear_pair_builds_from_discovered_original_paths() {
    use brep_core::shell_thickness::contact_pair::{DiscoveryLimits, discover};
    let s = curved();
    let saved = s.clone();
    let mut l = DiscoveryLimits::default();
    l.max_constructions = 4;
    let r = discover(
        &s,
        [false; 2],
        0.1,
        Tolerances {
            uv: 1e-7,
            normal: 0.005,
            sewing: 1e-7,
        },
        l,
    ).unwrap();
    assert!(
        r.attempts().iter().any(|a| a.local_skin().is_some()),
        "paths={} reasons={:?} predictors={} path_attempts={} proposal_cells={} numerical_errors={:?}",
        r.discovery().requests.len(),
        r.attempts().iter().map(|a| a.reason()).collect::<Vec<_>>(),
        r.discovery().predictor_queries,
        r.discovery().path_attempts,
        r.discovery().proposal_path_cells,
        r.discovery().numerical_failures
    );
    for a in r.attempts() {
        if let Some(skin) = a.local_skin() {
            assert!(skin.world_proof().error_upper_mm.unwrap() <= 1e-7);
            assert!(skin.constructed().mapped_normal_offset_error_upper() <= 0.005);
            assert_eq!(skin.model().validate().unwrap().boundary_edge_count, 0);
        }
    }
    assert!(!r.complete_contact_coverage() && !r.discovery().coverage.pending.is_empty());
    assert_eq!(r.boundary_contacts().faces.len(), 8);
    assert!(r.boundary_contacts().endpoints.iter().any(|p| p.boundary()
        == nurbs_core::surface_offset::boundary_contact::Boundary {
            source: 1,
            axis: 1,
            high: true
        }
        && p.regular_inward_branch_proven()));
    assert!(r.boundary_contacts().boundary_coverage_proven());
    let index = r
        .boundary_contacts()
        .endpoints
        .iter()
        .position(|p| {
            p.boundary()
                == nurbs_core::surface_offset::boundary_contact::Boundary {
                    source: 1,
                    axis: 1,
                    high: true,
                }
                && p.regular_inward_branch_proven()
        })
        .unwrap();
    let links = r
        .connect_endpoint(
            index,
            nurbs_core::surface_offset::boundary_contact::attachment::bridge::Limits::default(),
        )
        .unwrap();
    assert!(
        links.attempts.iter().any(|a| a.connected()),
        "{:?}",
        links
            .attempts
            .iter()
            .map(|a| a.reason())
            .collect::<Vec<_>>()
    );
    links.reverify().unwrap();
    let region = r
        .close_path(
            0,
            [0, 3],
            1e-7,
            1e-7,
            nurbs_core::surface_offset::contact_curve::chain::Limits::default(),
            nurbs_core::surface_offset::contact_curve::chain::region::Limits::default(),
        )
        .unwrap();
    let chain = region.chain();
    assert!(!chain.pieces().is_empty());
    assert!(
        chain
            .pieces()
            .iter()
            .all(|p| p.world_proof().world_correspondence_proven)
    );
    assert!(region.simplicity().proven_simple);
    assert!(
        region
            .lift_proofs()
            .iter()
            .all(|p| p.lift_correspondence_proven)
    );
    let range = region
        .uv_loop()
        .iter()
        .flat_map(|c| &c.control_points)
        .fold([f64::INFINITY, f64::NEG_INFINITY], |r, p| {
            [r[0].min(p[1]), r[1].max(p[1])]
        });
    let v = [
        range[0] + (range[1] - range[0]) * 0.4,
        range[0] + (range[1] - range[0]) * 0.6,
    ];
    let points = [[0.2, v[0]], [0.3, v[0]], [0.3, v[1]], [0.2, v[1]]];
    let hole: Vec<_> = (0..4)
        .map(|i| {
            nurbs_core::curve::Curve::from_polyline(vec![
                points[i].to_vec(),
                points[(i + 1) % 4].to_vec(),
            ])
            .unwrap()
        })
        .collect();
    let compound = region
        .with_nested(
            &[hole],
            nurbs_core::surface_offset::contact_curve::chain::region::compound::Limits::default(),
        )
        .unwrap();
    assert_eq!(compound.uv_domain().nodes()[1].parent, Some(0));
    assert_eq!(compound.world_loops().len(), 2);
    assert_eq!(
        compound
            .classify([[0.25; 2], [(v[0] + v[1]) * 0.5; 2]], 10000)
            .unwrap()
            .location,
        nurbs_core::trim_domain::Location::Outside
    );
    assert_eq!(
        compound
            .classify([[0.4; 2], [(v[0] + v[1]) * 0.5; 2]], 10000)
            .unwrap()
            .location,
        nurbs_core::trim_domain::Location::Inside
    );
    let cap = compound.cap(0.005, 1e-6, Default::default()).unwrap();
    assert!(cap.chart_proof().certified);
    let sheet =
        brep_core::shell_thickness::contact_pair::cap::build(cap, Default::default()).unwrap();
    assert_eq!(sheet.model().faces.len(), 1);
    assert_eq!(sheet.model().faces[0].holes.len(), 1);
    assert!(sheet.model().bodies.is_empty());
    assert!(!sheet.model().shells[0].closed);
    assert!(sheet.boundary_proof().complete);
    sheet.reverify().unwrap();
    r.reverify().unwrap();
    assert_eq!(s, saved);
}
