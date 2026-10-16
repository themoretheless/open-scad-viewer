use super::*;
fn fixture(nonplanar: bool) -> (Vec<Vec<Curve>>, LoftCap, Vec<RationalPiece>) {
    let corners = [
        vec![
            [5., -1.],
            [5., -0.75],
            [5., -0.5],
            [5., 0.],
            [5., 0.5],
            [5., 0.75],
            [5., 1.],
        ],
        vec![
            [5., 1.],
            [5., 1.25],
            [5., 1.5],
            [4.75, 1.75],
            [4.5, 2.],
            [4.25, 2.],
            [4., 2.],
        ],
        vec![
            [4., 2.],
            [3.75, 2.],
            [3.5, 2.],
            [3., if nonplanar { 2.0625 } else { 2. }],
            [2.5, 2.],
            [2.25, 2.],
            [2., 2.],
        ],
        vec![
            [2., 2.],
            [1.75, 2.],
            [1.5, 2.],
            [1.25, 1.75],
            [1., 1.5],
            [1., 1.25],
            [1., 1.],
        ],
        vec![
            [1., 1.],
            [1., 0.75],
            [1., 0.5],
            [1., 0.],
            [1., -0.5],
            [1., -0.75],
            [1., -1.],
        ],
        vec![
            [1., -1.],
            [1., -1.25],
            [1., -1.5],
            [1.25, -1.75],
            [1.5, -2.],
            [1.75, -2.],
            [2., -2.],
        ],
        vec![
            [2., -2.],
            [2.25, -2.],
            [2.5, -2.],
            [3., -2.],
            [3.5, -2.],
            [3.75, -2.],
            [4., -2.],
        ],
        vec![
            [4., -2.],
            [4.25, -2.],
            [4.5, -2.],
            [4.75, -1.75],
            [5., -1.5],
            [5., -1.25],
            [5., -1.],
        ],
    ];
    let ring = corners
        .iter()
        .map(|p| Curve {
            degree: 6,
            knots: [vec![0.; 7], vec![1.; 7]].concat(),
            control_points: p.iter().map(|p| vec![p[0], p[1], 0.]).collect(),
            weights: vec![1.; 7],
            periodic: false,
        })
        .collect::<Vec<_>>();
    let trims = ring
        .iter()
        .map(|c| {
            let mut c = c.clone();
            for p in &mut c.control_points {
                p.pop();
            }
            c
        })
        .collect();
    let u = [0.75, 5.25];
    let v = [-2.5, 2.5];
    let surface = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![u[0], u[0], u[1], u[1]],
        knots_v: vec![v[0], v[0], v[1], v[1]],
        control_points: u
            .iter()
            .map(|&u| v.iter().map(|&v| vec![u, v, 0.]).collect())
            .collect(),
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let az = [
        [1., 0.],
        [1., 1.],
        [0., 1.],
        [-1., 1.],
        [-1., 0.],
        [-1., -1.],
        [0., -1.],
        [1., -1.],
        [1., 0.],
    ];
    let motion = (0..4)
        .map(|i| RationalPiece {
            weights: vec![1., 0.5, 1.],
            controls: az[2 * i..=2 * i + 2]
                .iter()
                .map(|p| [[p[0], p[1], 0.], [0., 0., 1.], [0.; 3], [0.; 3]])
                .collect(),
        })
        .collect();
    (
        vec![ring],
        LoftCap {
            surface,
            trims: vec![trims],
        },
        motion,
    )
}
fn limits() -> RationalClosedLimits {
    RationalClosedLimits {
        source_cells: 100000,
        source_exact_work: 1000000,
        construction_rows: 1000000,
        material: MaterialLimits {
            pairs: 100000,
            cells: 100000,
            domain_cells: 1000000,
        },
        max_faces: 1024,
        binding_work: 1000000,
        smoothness: crate::sweep_smoothness::complete::Limits {
            order: 2,
            exact_work: 16000000,
            agreement_work: 1000000,
            regularity_cells: 100000,
        },
        volume: IdealLimits {
            cells: 1000000,
            exact_work: 1000000,
            projection: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        },
        orientation_cells: 100000,
        cap_cells: 100000,
    }
}
fn body(a: RationalClosedAttempt) -> Box<RationalClosedBody> {
    match a {
        RationalClosedAttempt::Certified(b) => b,
        RationalClosedAttempt::Refused(r) => panic!(
            "refused {:?}, source work={} caps={} binding={:?} volume={:?} Gk={:?}",
            r.reason,
            r.source_exact_work,
            r.caps.len(),
            r.binding.as_ref().map(|p| (p.certified, p.faces, p.reason)),
            r.volume,
            r.smoothness.as_ref().map(|p| (
                &p.unresolved_edges,
                p.g2_certified,
                p.regularity.all_faces_regular
            ))
        ),
    }
}
#[test]
fn original_and_actual_rational_body_own_full_filled_planar_and_nonplanar_cap_g2() {
    for nonplanar in [false, true] {
        let (profiles, material, motion) = fixture(nonplanar);
        let b = body(
            construct_rational_closed_material(
                &profiles,
                &material,
                [0.; 3],
                &motion,
                &[[0, 2], [0, 6]],
                1e-8,
                limits(),
            )
            .unwrap(),
        );
        let r = b.report();
        assert!(r.certified && r.source_boundary_exact && r.source_g2 && r.topology_preserved);
        assert_eq!(b.model().faces.len(), 32);
        assert_eq!(r.caps.len(), 2);
        assert!(
            r.caps
                .iter()
                .all(|c| c.faces.len() == 4 && c.parameter_monotonicity.spanwise_regular)
        );
        assert_eq!(r.caps[0].source_v_bounds[0], 2.);
        assert_eq!(
            r.caps[0].source_v_bounds[1],
            if nonplanar { 2.0625 } else { 2. }
        );
        assert_eq!(b.model().validate().unwrap().boundary_edge_count, 0);
        eprintln!(
            "actual physical annular graph caps nonplanar={nonplanar}, source/actual G2, volume={:?}, exact binding={} faces={}",
            r.volume,
            r.binding.as_ref().unwrap().exact_work,
            b.model().faces.len()
        );
    }
}
#[test]
fn curvature_jump_preserves_explicit_g1_and_refuses_strict_cap_g2() {
    let (mut profiles, mut material, motion) = fixture(true);
    profiles[0][2].control_points[2][1] = 2_f64.next_up();
    material.trims[0][2].control_points[2][1] = 2_f64.next_up();
    let strict = construct_rational_closed_material(
        &profiles,
        &material,
        [0.; 3],
        &motion,
        &[[0, 2], [0, 6]],
        1e-8,
        limits(),
    )
    .unwrap();
    assert!(matches!(strict, RationalClosedAttempt::Refused(_)));
    let mut l = limits();
    l.smoothness.order = 1;
    let b = body(
        construct_rational_closed_material(
            &profiles,
            &material,
            [0.; 3],
            &motion,
            &[[0, 2], [0, 6]],
            1e-8,
            l,
        )
        .unwrap(),
    );
    assert!(b.report().source_g1 && !b.report().source_g2);
}

#[test]
fn filled_cap_charts_survive_moving_affine_axes_translation_and_weight_gauges() {
    let (profiles, material, mut motion) = fixture(true);
    let basis = [[1., 0.125, 0.25], [0., 1., 0.125], [0.125, 0., 1.]];
    for piece in &mut motion {
        for p in &mut piece.controls {
            p[1] = [p[0][0] / 8., p[0][1] / 8., 1.];
            for col in 0..3 {
                let q = p[col];
                p[col] = std::array::from_fn(|k| {
                    basis[0][k] * q[0] + basis[1][k] * q[1] + basis[2][k] * q[2]
                });
            }
            p[3] = [1., 2., 3.];
        }
    }
    for w in &mut motion[1].weights {
        *w *= 2.;
    }
    let b = body(
        construct_rational_closed_material(
            &profiles,
            &material,
            [0.; 3],
            &motion,
            &[[0, 2], [0, 6]],
            1e-8,
            limits(),
        )
        .unwrap(),
    );
    assert!(b.report().source_g2 && b.report().topology_preserved);
    assert_eq!(b.report().boundary_error_upper, Some(0.));
    assert_eq!(b.report().caps.len(), 2);
}

#[test]
fn sharp_cap_join_is_explicit_c0_and_cannot_be_hidden_by_an_empty_cap_selection() {
    let (mut profiles, mut material, motion) = fixture(true);
    profiles[0][2].control_points[1][1] = 2_f64.next_up();
    material.trims[0][2].control_points[1][1] = 2_f64.next_up();
    for caps in [vec![], vec![[0, 2], [0, 6]]] {
        let mut l = limits();
        l.smoothness.order = 1;
        assert!(matches!(
            construct_rational_closed_material(
                &profiles, &material, [0.; 3], &motion, &caps, 1e-8, l
            )
            .unwrap(),
            RationalClosedAttempt::Refused(_)
        ));
    }
    let mut l = limits();
    l.smoothness.order = 0;
    let b = body(
        construct_rational_closed_material(
            &profiles,
            &material,
            [0.; 3],
            &motion,
            &[[0, 2], [0, 6]],
            1e-8,
            l,
        )
        .unwrap(),
    );
    assert!(b.report().certified && !b.report().source_g1 && !b.report().source_g2);
    assert_eq!(b.report().seam_continuity, "C0");
    assert!(b.report().c0_regularity.as_ref().unwrap().all_faces_regular);
    assert!(b.report().c0_agreement.as_ref().unwrap().all_equal);
}

#[test]
fn all_original_caps_volume_construction_and_boundary_owner_budgets_refuse_partial_solids() {
    let (profiles, material, motion) = fixture(false);
    let full = body(
        construct_rational_closed_material(
            &profiles,
            &material,
            [0.; 3],
            &motion,
            &[[0, 2], [0, 6]],
            1e-8,
            limits(),
        )
        .unwrap(),
    );
    let r = full.report();
    let mut cases = Vec::new();
    for budget in [0, r.source_cells - 1] {
        cases.push(RationalClosedLimits {
            source_cells: budget,
            ..limits()
        });
    }
    for budget in [0, r.source_exact_work - 1] {
        cases.push(RationalClosedLimits {
            source_exact_work: budget,
            ..limits()
        });
    }
    for budget in [0, r.construction_rows - 1] {
        cases.push(RationalClosedLimits {
            construction_rows: budget,
            ..limits()
        });
    }
    for budget in [0, r.binding.as_ref().unwrap().exact_work - 1] {
        cases.push(RationalClosedLimits {
            binding_work: budget,
            ..limits()
        });
    }
    for budget in [0, r.cap_cells - 1] {
        cases.push(RationalClosedLimits {
            cap_cells: budget,
            ..limits()
        });
    }
    for budget in [0, r.orientation_cells - 1] {
        cases.push(RationalClosedLimits {
            orientation_cells: budget,
            ..limits()
        });
    }
    for budget in [0, r.volume.as_ref().unwrap().cells - 1] {
        let mut l = limits();
        l.volume.cells = budget;
        cases.push(l);
    }
    for budget in [0, r.volume.as_ref().unwrap().exact_work - 1] {
        let mut l = limits();
        l.volume.exact_work = budget;
        cases.push(l);
    }
    for budget in [0, r.smoothness.as_ref().unwrap().exact_work - 1] {
        let mut l = limits();
        l.smoothness.exact_work = budget;
        cases.push(l);
    }
    for budget in [
        0,
        r.smoothness
            .as_ref()
            .unwrap()
            .edge_correspondence
            .as_ref()
            .unwrap()
            .work
            - 1,
    ] {
        let mut l = limits();
        l.smoothness.agreement_work = budget;
        cases.push(l);
    }
    for budget in [0, r.smoothness.as_ref().unwrap().regularity.cells - 1] {
        let mut l = limits();
        l.smoothness.regularity_cells = budget;
        cases.push(l);
    }
    let mut l = limits();
    l.material.cells = 0;
    cases.push(l);
    let mut l = limits();
    l.material.pairs = 0;
    cases.push(l);
    let mut l = limits();
    l.material.domain_cells = 0;
    cases.push(l);
    for (i, l) in cases.into_iter().enumerate() {
        let RationalClosedAttempt::Refused(r) = construct_rational_closed_material(
            &profiles,
            &material,
            [0.; 3],
            &motion,
            &[[0, 2], [0, 6]],
            1e-8,
            l,
        )
        .unwrap() else {
            panic!("short stage {i} returned Solid");
        };
        assert!(!r.certified && !r.topology_preserved && !r.source_g2);
        assert!(
            r.source_cells <= l.source_cells
                && r.source_exact_work <= l.source_exact_work
                && r.construction_rows <= l.construction_rows
                && r.cap_cells <= l.cap_cells
                && r.orientation_cells <= l.orientation_cells
        );
    }
    let mut malformed = motion;
    malformed[3].weights[0] = 0.;
    let mut l = limits();
    l.cap_cells = 0;
    assert!(
        construct_rational_closed_material(
            &profiles,
            &material,
            [0.; 3],
            &malformed,
            &[[0, 2], [0, 6]],
            1e-8,
            l
        )
        .is_err()
    );
}
