use brep_core::pcurve_repair::*;
use nurbs_core::{curve::Curve, curve_pullback, surface::Surface};

fn line(points: &[[f64; 2]]) -> Curve {
    Curve::from_polyline(points.iter().map(|p| p.to_vec()).collect()).unwrap()
}

fn line3(points: &[[f64; 3]]) -> Curve {
    Curve::from_polyline(points.iter().map(|p| p.to_vec()).collect()).unwrap()
}

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

fn periodic_strip() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., std::f64::consts::TAU, std::f64::consts::TAU],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![1., 0., 0.], vec![1., 0., 1.]],
            vec![vec![1., 0., 0.], vec![1., 0., 1.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}

fn uv_period() -> SurfacePeriods {
    SurfacePeriods {
        u: Some(PeriodicAxis::new(0., std::f64::consts::TAU).unwrap()),
        v: None,
    }
}

#[test]
fn seam_join_is_one_unwrapped_curve_with_integer_lift_and_c1() {
    let tau = std::f64::consts::TAU;
    let step = 0.2;
    let first = line(&[[tau - step, 0.], [tau, 0.1]]);
    let second = line(&[[0., 0.1], [step, 0.2]]);
    let joined = join_seam_polylines(&first, &second, uv_period(), 1e-12, 1e-12).unwrap();
    assert_eq!(joined.periodic_lift, [[0, 0], [1, 0]]);
    assert_eq!(joined.curve.control_points.len(), 3);
    assert!((joined.curve.control_points[2][0] - (tau + step)).abs() < 1e-12);
    assert!(joined.seam_gap <= 1e-12);
    assert!(joined.tangent_jump <= 1e-12);
    let at_seam = joined.curve.domain()[0] + 1.;
    let p = joined
        .evaluate_lifted(&periodic_strip(), at_seam, uv_period())
        .unwrap();
    let wrapped = joined
        .evaluate_lifted(&periodic_strip(), at_seam + 1e-8, uv_period())
        .unwrap();
    assert!((p[0] - wrapped[0]).abs() < 1e-8);

    let kink = line(&[[0., 0.1], [step, 0.4]]);
    let error = join_seam_polylines(&first, &kink, uv_period(), 1e-12, 1e-12).unwrap_err();
    assert_eq!(error.code, PCURVE_AMBIGUOUS);
}

#[test]
fn seam_relocation_replays_ids_and_refuses_a_cut() {
    let tau = std::f64::consts::TAU;
    let trims = vec![TrimReplay {
        persistent_id: "face/edge-17".into(),
        pcurve: line(&[[tau - 0.5, 0.2], [tau - 0.2, 0.4]]),
    }];
    let moved = relocate_seam_u(
        PeriodicAxis::new(0., tau).unwrap(),
        std::f64::consts::PI,
        42,
        &trims,
        1e-10,
    )
    .unwrap();
    assert_eq!(moved.surface_revision, 42);
    assert_eq!(moved.trims[0].persistent_id, "face/edge-17");
    assert_eq!(moved.trims[0].pcurve, trims[0].pcurve);
    let crossing = vec![TrimReplay {
        persistent_id: "cut".into(),
        pcurve: line(&[[tau - 0.5, 0.], [0.5, 1.]]),
    }];
    assert_eq!(
        relocate_seam_u(
            PeriodicAxis::new(0., tau).unwrap(),
            0.1,
            1,
            &crossing,
            1e-10
        )
        .unwrap_err()
        .code,
        PCURVE_AMBIGUOUS
    );
}

#[test]
fn pole_crossing_has_two_branches_and_only_away_tangents() {
    let surface = plane();
    let curve = line(&[[0.2, 0.2], [0.5, 0.5], [0.8, 0.2]]);
    let crossing = split_at_degenerate_pole(&curve, &surface, [0.5, 0.5], 1., 1e-12).unwrap();
    assert_eq!(crossing.incoming.curve.control_points.len(), 2);
    assert_eq!(crossing.outgoing.curve.control_points.len(), 2);
    assert_eq!(crossing.incoming.away_tangent, vec![0.3, 0.3]);
    assert!((crossing.outgoing.away_tangent[0] - 0.3).abs() < 1e-12);
    assert!((crossing.outgoing.away_tangent[1] + 0.3).abs() < 1e-12);
    assert_eq!(crossing.pole_point, vec![0.5, 0.5, 0.]);
    assert!(split_at_degenerate_pole(&curve, &surface, [0.5, 0.4], 1., 1e-12).is_err());
}

#[test]
fn same_parameter_audit_preserves_good_and_rebuilds_bad_step_trim() {
    let surface = plane();
    let edge = line3(&[[0.1, 0.2, 0.], [0.8, 0.7, 0.]]);
    let good = line(&[[0.1, 0.2], [0.8, 0.7]]);
    let bad = line(&[[0.1, 0.2], [0.5, 0.8], [0.8, 0.7]]);
    let options = AuditOptions {
        tolerance: 1e-10,
        samples: 33,
    };
    let preserved = resolve_step_trim(
        "e-good",
        &edge,
        &good,
        &surface,
        true,
        options,
        curve_pullback::Limits {
            tolerance: 1e-10,
            max_segments: 16,
            max_iterations: 16,
            agreement_cells: 4096,
        },
    )
    .unwrap();
    assert_eq!(preserved.decision, TrimDecision::Preserved);
    assert_eq!(preserved.pcurve, good);
    assert_eq!(preserved.original.samples, 33);

    let repaired = resolve_step_trim(
        "e-bad",
        &edge,
        &bad,
        &surface,
        true,
        options,
        curve_pullback::Limits {
            tolerance: 1e-10,
            max_segments: 16,
            max_iterations: 16,
            agreement_cells: 4096,
        },
    )
    .unwrap();
    assert_eq!(repaired.decision, TrimDecision::RebuiltFromEdge);
    assert_eq!(repaired.edge_id, "e-bad");
    assert!(repaired.original.max_distance > 0.1);
    assert!(repaired.repaired.max_distance <= 1e-10);
}

#[test]
fn bounded_repair_unwraps_and_snaps_but_refuses_large_domain_damage() {
    let surface = plane();
    let edge = line3(&[[0.1, 0.2, 0.], [0.8, 0.7, 0.]]);
    let damaged = line(&[[0.1 + 1e-8, 0.2], [0.8, 0.7]]);
    let repaired = repair_pcurve(
        &edge,
        &damaged,
        &surface,
        SurfacePeriods::default(),
        true,
        RepairBudget {
            tolerance: 1e-10,
            max_snap_uv: 1e-6,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(repaired.after.consistent);
    assert!(
        repaired
            .actions
            .iter()
            .any(|a| matches!(a, RepairAction::EndpointSnap { .. }))
    );

    let shortened = line(&[[0.2, 0.2714285714285714], [0.7, 0.6285714285714286]]);
    let extended = repair_pcurve(
        &edge,
        &shortened,
        &surface,
        SurfacePeriods::default(),
        true,
        RepairBudget {
            tolerance: 1e-10,
            max_snap_uv: 1e-6,
            max_extension_uv: 0.25,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        extended
            .actions
            .iter()
            .any(|a| matches!(a, RepairAction::TangentExtension { .. }))
    );
    assert!(extended.after.consistent);

    let outside = line(&[[-0.5, 0.2], [0.8, 0.7]]);
    let error = repair_pcurve(
        &edge,
        &outside,
        &surface,
        SurfacePeriods::default(),
        true,
        RepairBudget {
            tolerance: 1e-10,
            max_snap_uv: 1e-6,
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(error.code, PCURVE_AMBIGUOUS);
}

#[test]
fn orientation_and_same_sense_are_verified_not_trusted() {
    let surface = plane();
    let edge = line3(&[[0.1, 0.2, 0.], [0.8, 0.7, 0.]]);
    let backwards = line(&[[0.8, 0.7], [0.1, 0.2]]);
    let fixed = reconcile_coedge_orientation(
        &edge,
        &backwards,
        &surface,
        false,
        true,
        AuditOptions {
            tolerance: 1e-10,
            samples: 33,
        },
    )
    .unwrap();
    assert!(fixed.changed);
    assert!(!fixed.reversed);
    assert!(fixed.same_sense);
    assert_eq!(
        fixed.pcurve.control_points.first().unwrap(),
        &vec![0.1, 0.2]
    );
    assert!(
        same_sense_matches_vertices(&edge, &[0.8, 0.7, 0.], &[0.1, 0.2, 0.], false, 1e-12).unwrap()
    );
    assert!(
        !same_sense_matches_vertices(&edge, &[0.1, 0.2, 0.], &[0.8, 0.7, 0.], false, 1e-12)
            .unwrap()
    );
}

#[test]
fn loop_winding_normalization_preserves_same_sense_and_flips_coedges() {
    let make = |p: [[f64; 2]; 2]| OrientedCoedge {
        pcurve: line(&p),
        reversed: false,
        same_sense: true,
        changed: false,
    };
    let mut cw = vec![
        make([[0., 0.], [0., 1.]]),
        make([[0., 1.], [1., 1.]]),
        make([[1., 1.], [1., 0.]]),
        make([[1., 0.], [0., 0.]]),
    ];
    let area = normalize_loop_winding(&mut cw, LoopKind::Outer).unwrap();
    assert!(area < 0.);
    assert!(cw.iter().all(|c| c.reversed && c.same_sense && c.changed));
    let mut area_after = 0.;
    for c in &cw {
        let p = &c.pcurve.control_points;
        for w in p.windows(2) {
            area_after += w[0][0] * w[1][1] - w[1][0] * w[0][1];
        }
    }
    assert!(area_after > 0.);
}

#[test]
fn property_seam_wrap_roundtrip_and_plane_agreement() {
    let tau = std::f64::consts::TAU;
    let mut state = 0x9e3779b97f4a7c15_u64;
    for _ in 0..128 {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let a = ((state >> 11) as f64 / ((1_u64 << 53) as f64)) * 0.8 + 0.1;
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let b = ((state >> 11) as f64 / ((1_u64 << 53) as f64)) * 0.8 + 0.1;
        let edge = line3(&[[0.1, 0.2, 0.], [a, b, 0.]]);
        let good = line(&[[0.1, 0.2], [a, b]]);
        let audit = audit_same_parameter(
            &edge,
            &good,
            &plane(),
            SurfacePeriods::default(),
            true,
            AuditOptions {
                tolerance: 1e-10,
                samples: 33,
            },
        )
        .unwrap();
        assert!(audit.consistent, "{audit:?}");

        let start = tau - a * 0.2;
        let first = line(&[[start, 0.], [tau, 0.1]]);
        let second = line(&[[0., 0.1], [(start + 0.4).rem_euclid(tau), 0.2]]);
        let joined = join_seam_polylines(&first, &second, uv_period(), 2., 2.).unwrap();
        assert!(
            joined
                .curve
                .control_points
                .iter()
                .flatten()
                .all(|x| x.is_finite())
        );
        assert!(joined.periodic_lift[1][0].abs() <= 1);
    }
}

#[test]
fn nan_and_budget_guards_are_typed() {
    let surface = plane();
    let edge = line3(&[[0., 0., 0.], [1., 1., 0.]]);
    let pcurve = line(&[[0., 0.], [1., 1.]]);
    assert_eq!(
        audit_same_parameter(
            &edge,
            &pcurve,
            &surface,
            SurfacePeriods::default(),
            true,
            AuditOptions {
                tolerance: 1.,
                samples: 1
            }
        )
        .unwrap_err()
        .code,
        PCURVE_INVALID
    );
    assert_eq!(
        repair_pcurve(
            &edge,
            &pcurve,
            &surface,
            SurfacePeriods::default(),
            true,
            RepairBudget {
                max_projection_iterations: 0,
                ..Default::default()
            }
        )
        .unwrap_err()
        .code,
        PCURVE_BUDGET
    );
    assert!(PeriodicAxis::new(f64::NAN, 1.).is_err());
}
