use super::*;

const E: f64 = 200_000.;
const NU: f64 = 0.3;
const A: f64 = 100.;
const I: f64 = 1e6;
const J: f64 = 2e6;
const L: f64 = 1000.;
const G: f64 = E / (2. * (1. + NU));

fn member(nodes: [usize; 2]) -> Member {
    Member {
        nodes,
        young_mpa: E,
        poisson: NU,
        area_mm2: A,
        iyy_mm4: I,
        izz_mm4: I,
        j_mm4: J,
        shear_area_y_mm2: None,
        shear_area_z_mm2: None,
        local_z_hint: None,
        release_a: [false; 3],
        release_b: [false; 3],
    }
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() <= 1e-9 * b.abs().max(1.), "{a} != {b}");
}
fn empty_loads(n: usize) -> (Vec<[f64; 3]>, Vec<[f64; 3]>) {
    (vec![[0.; 3]; n], vec![[0.; 3]; n])
}

/// Cantilever along +X, fixed at A, downward tip load at B.
fn cantilever() -> Model {
    let (mut forces, moments) = empty_loads(2);
    forces[1] = [0., 0., -1000.];
    Model {
        nodes_mm: vec![[0., 0., 0.], [L, 0., 0.]],
        members: vec![member([0, 1])],
        restrained: vec![[true; 6], [false; 6]],
        supports: vec![],
        forces_n: forces,
        moments_nmm: moments,
        loads: vec![],
    }
}

#[test]
fn cantilever_tip_load_matches_euler_bernoulli() {
    let r = solve(&cantilever()).unwrap();
    let p = 1000.;
    close(r.displacements_mm[1][2], -p * L * L * L / (3. * E * I));
    close(r.rotations_rad[1][1], p * L * L / (2. * E * I));
    close(r.reactions_n[0][2], p);
    close(r.reaction_moments_nmm[0][1], -p * L);
    let stations = &r.members[0].stations;
    close(stations[0].shear_z_n, -p);
    close(stations[0].moment_y_nmm, p * L); // hogging positive
    close(stations[DIAGRAM_STATIONS - 1].moment_y_nmm, 0.);
    close(stations[10].moment_y_nmm, p * L / 2.);
    assert!(r.max_relative_residual < 1e-12);
    assert_eq!(r.free_dofs, 6);
}

#[test]
fn cantilever_shear_area_adds_timoshenko_deflection() {
    let mut model = cantilever();
    model.members[0].shear_area_z_mm2 = Some(500.);
    let r = solve(&model).unwrap();
    let p = 1000.;
    let eb = p * L * L * L / (3. * E * I);
    let shear = p * L / (G * 500.);
    close(r.displacements_mm[1][2], -(eb + shear));
}

/// Simply supported span along +X with a downward uniform load.
fn ss_beam(w: f64) -> Model {
    let (forces, moments) = empty_loads(2);
    Model {
        nodes_mm: vec![[0., 0., 0.], [L, 0., 0.]],
        members: vec![member([0, 1])],
        // Pins: translations fixed, bending rotation ry free at both ends.
        restrained: vec![
            [true, true, true, true, false, true],
            [false, true, true, false, false, true],
        ],
        supports: vec![],
        forces_n: forces,
        moments_nmm: moments,
        loads: vec![MemberLoad::Uniform {
            member: 0,
            force_n_per_mm: [0., 0., -w],
            local_axes: false,
        }],
    }
}

#[test]
fn simply_supported_udl_matches_closed_form() {
    let w = 10.;
    let r = solve(&ss_beam(w)).unwrap();
    close(r.reactions_n[0][2], w * L / 2.);
    close(r.reactions_n[1][2], w * L / 2.);
    close(r.rotations_rad[0][1], w * L * L * L / (24. * E * I));
    close(r.rotations_rad[1][1], -w * L * L * L / (24. * E * I));
    let stations = &r.members[0].stations;
    close(stations[0].shear_z_n, -w * L / 2.);
    close(stations[10].shear_z_n, 0.);
    close(stations[10].moment_y_nmm, -w * L * L / 8.); // sagging negative
    close(stations[0].moment_y_nmm, 0.);
    close(stations[DIAGRAM_STATIONS - 1].moment_y_nmm, 0.);
    assert!(r.max_relative_residual < 1e-12);
}

#[test]
fn propped_cantilever_udl_two_modeling_routes_agree() {
    let w = 10.;
    // Route 1: fixed at A, roller at B (uz restrained, rotations free).
    let (forces, moments) = empty_loads(2);
    let roller = Model {
        nodes_mm: vec![[0., 0., 0.], [L, 0., 0.]],
        members: vec![member([0, 1])],
        restrained: vec![[true; 6], [false, true, true, false, false, true]],
        supports: vec![],
        forces_n: forces,
        moments_nmm: moments,
        loads: vec![MemberLoad::Uniform {
            member: 0,
            force_n_per_mm: [0., 0., -w],
            local_axes: true,
        }],
    };
    // Route 2: fixed-fixed with a bending hinge released at B.
    let mut fixed_fixed = roller.clone();
    fixed_fixed.restrained[1] = [true; 6];
    fixed_fixed.members[0].release_b = [false, true, false];
    for model in [&roller, &fixed_fixed] {
        let r = solve(model).unwrap();
        close(r.reactions_n[1][2], 3. * w * L / 8.);
        close(r.reactions_n[0][2], 5. * w * L / 8.);
        let stations = &r.members[0].stations;
        close(stations[0].moment_y_nmm, w * L * L / 8.);
        close(stations[DIAGRAM_STATIONS - 1].moment_y_nmm, 0.);
    }
    // The free-end rotation exists only in the roller model; with B fully
    // restrained the hinge rotation lives in the released element DOF.
    close(
        solve(&roller).unwrap().rotations_rad[1][1],
        -w * L * L * L / (48. * E * I),
    );
}

#[test]
fn point_moment_at_midspan_jumps_the_diagram() {
    let m = 1e6;
    let mut model = ss_beam(0.);
    model.loads = vec![MemberLoad::PointMoment {
        member: 0,
        at_mm: L / 2.,
        moment_nmm: [0., m, 0.],
        local_axes: true,
    }];
    let r = solve(&model).unwrap();
    close(r.reactions_n[0][2], -m / L);
    close(r.reactions_n[1][2], m / L);
    let stations = &r.members[0].stations;
    close(stations[0].moment_y_nmm, 0.);
    close(stations[9].moment_y_nmm, 0.45 * m); // just left of the jump
    close(stations[10].moment_y_nmm, -0.5 * m); // right-side limit
    close(stations[DIAGRAM_STATIONS - 1].moment_y_nmm, 0.);
}

#[test]
fn trapezoidal_load_carries_correct_reactions_and_moment() {
    let w = 10.;
    let mut model = ss_beam(0.);
    model.loads = vec![MemberLoad::Trapezoidal {
        member: 0,
        from_n_per_mm: [0., 0., 0.],
        to_n_per_mm: [0., 0., -w],
        local_axes: true,
    }];
    let r = solve(&model).unwrap();
    close(r.reactions_n[0][2], w * L / 6.);
    close(r.reactions_n[1][2], w * L / 3.);
    let stations = &r.members[0].stations;
    close(stations[0].shear_z_n, -w * L / 6.);
    // M(L/2) = wL²/16 sagging for a 0→w ramp.
    close(stations[10].moment_y_nmm, -w * L * L / 16.);
}

#[test]
fn point_force_on_member_matches_midspan_formulas() {
    let p = 1000.;
    let mut model = ss_beam(0.);
    model.loads = vec![MemberLoad::PointForce {
        member: 0,
        at_mm: L / 2.,
        force_n: [0., 0., -p],
        local_axes: false,
    }];
    let r = solve(&model).unwrap();
    close(r.reactions_n[0][2], p / 2.);
    close(r.reactions_n[1][2], p / 2.);
    close(r.rotations_rad[0][1], p * L * L / (16. * E * I));
    let stations = &r.members[0].stations;
    close(stations[10].moment_y_nmm, -p * L / 4.);
    close(stations[9].shear_z_n, -p / 2.);
    close(stations[10].shear_z_n, p / 2.); // right side of the point load
}

#[test]
fn axial_and_torsion_respond_independently() {
    let (mut forces, mut moments) = empty_loads(2);
    forces[1] = [100., 0., 0.];
    moments[1] = [1e6, 0., 0.];
    let model = Model {
        nodes_mm: vec![[0., 0., 0.], [L, 0., 0.]],
        members: vec![member([0, 1])],
        restrained: vec![[true; 6], [false; 6]],
        supports: vec![],
        forces_n: forces,
        moments_nmm: moments,
        loads: vec![],
    };
    let r = solve(&model).unwrap();
    close(r.displacements_mm[1][0], 100. * L / (E * A));
    close(r.rotations_rad[1][0], 1e6 * L / (G * J));
    close(r.reactions_n[0][0], -100.);
    close(r.reaction_moments_nmm[0][0], -1e6);
    let stations = &r.members[0].stations;
    close(stations[5].axial_n, 100.); // tension positive
    close(stations[5].torsion_nmm, 1e6);
    close(stations[5].moment_y_nmm, 0.);
}

#[test]
fn rotated_cantilever_preserves_response_in_global_axes() {
    let base = solve(&cantilever()).unwrap();
    // Rotate 90° about Z: X→Y, Y→−X, member along +Y, load stays −Z.
    let mut rotated = cantilever();
    rotated.nodes_mm = vec![[0., 0., 0.], [0., L, 0.]];
    // Local z hint must stay global +Z; local y = z × x = Z × Y = −X.
    let r = solve(&rotated).unwrap();
    close(r.displacements_mm[1][2], base.displacements_mm[1][2]);
    // Bending was about local y (= global Y); now about local y (= −X).
    close(r.rotations_rad[1][0], -base.rotations_rad[1][1]);
    close(r.reactions_n[0][2], base.reactions_n[0][2]);
    close(r.reaction_moments_nmm[0][0], -base.reaction_moments_nmm[0][1]);
}

#[test]
fn refuses_mechanisms_and_unstable_releases() {
    // Cantilever with a ball-joint release at the wall: free rigid rotation.
    let mut model = cantilever();
    model.members[0].release_a = [true, true, true];
    assert_eq!(solve(&model).unwrap_err().code, "FRAME_SINGULAR");
    // Torsion released at both ends: singular released block.
    let mut model = cantilever();
    model.members[0].release_a[0] = true;
    model.members[0].release_b[0] = true;
    assert_eq!(solve(&model).unwrap_err().code, "FRAME_INVALID_INPUT");
}

#[test]
fn rejects_invalid_input_before_matrix_allocation() {
    let mut models = Vec::new();
    let mut m = cantilever();
    m.members[0].poisson = 0.5;
    models.push(m);
    let mut m = cantilever();
    m.members[0].iyy_mm4 = 0.;
    models.push(m);
    let mut m = cantilever();
    m.members[0].shear_area_z_mm2 = Some(-1.);
    models.push(m);
    let mut m = cantilever();
    m.restrained = vec![[true; 6]]; // one mask short
    models.push(m);
    let mut m = cantilever();
    m.moments_nmm.clear();
    models.push(m);
    let mut m = cantilever();
    m.nodes_mm[1][1] = f64::NAN;
    models.push(m);
    let mut m = cantilever();
    m.loads = vec![MemberLoad::Uniform {
        member: 1,
        force_n_per_mm: [0., 0., -1.],
        local_axes: true,
    }];
    models.push(m);
    let mut m = cantilever();
    m.loads = vec![MemberLoad::PointForce {
        member: 0,
        at_mm: L + 1.,
        force_n: [0., 0., -1.],
        local_axes: true,
    }];
    models.push(m);
    let mut m = cantilever();
    m.members.push(m.members[0].clone()); // duplicate edge
    models.push(m);
    for model in models {
        assert_eq!(solve(&model).unwrap_err().code, "FRAME_INVALID_INPUT");
    }
}

/// Parallel hints fall back deterministically instead of failing.
fn solve_ok_hint_parallel() -> Model {
    let mut m = cantilever();
    m.members[0].local_z_hint = Some([1., 0., 0.]);
    m
}

#[test]
fn parallel_local_z_hint_falls_back_and_solves() {
    let r = solve(&solve_ok_hint_parallel()).unwrap();
    close(r.displacements_mm[1][2], -1000. * L * L * L / (3. * E * I));
}

#[test]
fn admits_node_limit_and_member_load_budget() {
    let mut model = cantilever();
    for i in 0..MAX_MEMBER_LOADS {
        model.loads.push(MemberLoad::PointForce {
            member: 0,
            at_mm: L * (i % 10) as f64 / 10.,
            force_n: [0., 0., -1.],
            local_axes: true,
        });
    }
    let r = solve(&model).unwrap();
    assert!(r.max_relative_residual < 1e-12);
    model.loads.push(MemberLoad::Uniform {
        member: 0,
        force_n_per_mm: [0., 0., -1.],
        local_axes: true,
    });
    assert_eq!(solve(&model).unwrap_err().code, "FRAME_INVALID_INPUT");
}

/// Case G: tip −1000 N (dead); case Q: tip +600 N (wind uplift).
fn two_cases() -> (Vec<[f64; 3]>, Vec<Member>, Vec<[bool; 6]>, Vec<LoadSet>) {
    let model = cantilever();
    let (mut wind_f, wind_m) = empty_loads(2);
    wind_f[1] = [0., 0., 600.];
    let cases = vec![
        LoadSet {
            forces_n: model.forces_n.clone(),
            moments_nmm: model.moments_nmm.clone(),
            loads: vec![],
        },
        LoadSet {
            forces_n: wind_f,
            moments_nmm: wind_m,
            loads: vec![],
        },
    ];
    (model.nodes_mm, model.members, model.restrained, cases)
}

fn named_combos() -> Vec<Combination> {
    ["G", "Q", "G+1.5Q", "G-Q"]
        .into_iter()
        .zip([
            vec![1., 0.],
            vec![0., 1.],
            vec![1., 1.5],
            vec![1., -1.],
        ])
        .map(|(name, factors)| Combination {
            name: name.into(),
            factors,
        })
        .collect()
}

#[test]
fn envelopes_match_combination_arithmetic_and_governing_indices() {
    let (nodes, members, restrained, cases) = two_cases();
    let r = solve_envelopes(&nodes, &members, &restrained, &[], &cases, &named_combos()).unwrap();
    // Tip uz: G −5/3, Q +1, G+1.5Q −1/6, G−Q −8/3.
    let tip_z = &r.displacements_mm[1][2];
    close(tip_z.min, -8. / 3.);
    close(tip_z.max, 1.);
    assert_eq!(tip_z.min_combination, 3);
    assert_eq!(tip_z.max_combination, 1);
    // Fixed-end reaction moment about y: G −1e6, Q +6e5.
    let my = &r.reaction_moments_nmm[0][1];
    close(my.min, -1.6e6);
    close(my.max, 6e5);
    assert_eq!(my.min_combination, 3);
    assert_eq!(my.max_combination, 1);
    // Root station moment_y: G +1e6, Q −6e5 (hogging positive).
    let root_my = &r.members[0][0].moment_y_nmm;
    close(root_my.min, -6e5);
    close(root_my.max, 1.6e6);
    close(r.members[0][10].x_mm, L / 2.);
    // Combination max deflection: 5/3, 1, 1/6, 8/3.
    close(r.max_deflection_mm.min, 1. / 6.);
    close(r.max_deflection_mm.max, 8. / 3.);
    assert_eq!(r.max_deflection_mm.min_combination, 2);
    assert_eq!(r.max_deflection_mm.max_combination, 3);
    assert_eq!(r.combinations, vec!["G", "Q", "G+1.5Q", "G-Q"]);
    assert_eq!(r.load_cases, 2);
    assert!(r.max_relative_residual < 1e-12);
    assert_eq!(r.free_dofs, 6);
}

#[test]
fn single_case_single_combo_envelope_equals_plain_solve() {
    let (nodes, members, restrained, cases) = two_cases();
    let combos = vec![Combination {
        name: "G only".into(),
        factors: vec![1., 0.],
    }];
    let r = solve_envelopes(&nodes, &members, &restrained, &[], &cases, &combos).unwrap();
    let plain = solve(&cantilever()).unwrap();
    for node in 0..2 {
        for c in 0..3 {
            let e = r.displacements_mm[node][c];
            assert_eq!(e.min, e.max);
            close(e.min, plain.displacements_mm[node][c]);
        }
    }
    for (mi, member) in r.members.iter().enumerate() {
        for (si, s) in member.iter().enumerate() {
            let p = &plain.members[mi].stations[si];
            close(s.axial_n.min, p.axial_n);
            close(s.moment_y_nmm.max, p.moment_y_nmm);
            assert_eq!(s.shear_z_n.min_combination, 0);
        }
    }
}

#[test]
fn envelopes_reject_bad_combinations_and_bad_cases() {
    let (nodes, members, restrained, cases) = two_cases();
    // Empty cases, empty combos, factor-count mismatch.
    assert!(solve_envelopes(&nodes, &members, &restrained, &[], &[], &named_combos()).is_err());
    assert!(solve_envelopes(&nodes, &members, &restrained, &[], &cases, &[]).is_err());
    let mut bad = named_combos();
    bad[0].factors.push(1.);
    assert_eq!(
        solve_envelopes(&nodes, &members, &restrained, &[], &cases, &bad)
            .unwrap_err()
            .code,
        "COMBO_INVALID_INPUT"
    );
    // A structurally bad case aborts the whole request.
    let mut bad_cases = cases.clone();
    bad_cases[1].forces_n.pop();
    assert_eq!(
        solve_envelopes(&nodes, &members, &restrained, &[], &bad_cases, &named_combos())
            .unwrap_err()
            .code,
        "FRAME_INVALID_INPUT"
    );
}

#[test]
fn envelopes_cover_member_loads_across_cases() {
    let (nodes, members, restrained, cases) = two_cases();
    let mut uniform_case = cases[0].clone();
    uniform_case.forces_n[1] = [0., 0., 0.];
    uniform_case.loads = vec![MemberLoad::Uniform {
        member: 0,
        force_n_per_mm: [0., 0., -10.],
        local_axes: false,
    }];
    let cases = vec![uniform_case, cases[1].clone()];
    let combos = named_combos();
    let r = solve_envelopes(&nodes, &members, &restrained, &[], &cases, &combos).unwrap();
    // Root moment_y: uniform case +wL²/2 = +5e6, wind case −6e5.
    // G: 5e6, Q: −6e5, G+1.5Q: 4.1e6, G-Q: 5.6e6.
    let root = &r.members[0][0].moment_y_nmm;
    close(root.max, 5.6e6);
    assert_eq!(root.max_combination, 3);
    close(root.min, -6e5);
    assert_eq!(root.min_combination, 1);
}

/// Cantilever with a linear spring at the tip (dof z), tip load −P.
fn spring_propped(k: f64) -> Model {
    let mut m = cantilever();
    m.supports = vec![Support::Spring {
        node: 1,
        dof: 2,
        stiffness: k,
    }];
    m
}

#[test]
fn spring_support_matches_compatibility_closed_form() {
    let k = 0.6;
    let r = solve(&spring_propped(k)).unwrap();
    // Prop force from compatibility: R = P·kL³ / (3EI + kL³).
    let rb = 1000. * k * L.powi(3) / (3. * E * I + k * L.powi(3));
    close(r.reactions_n[1][2], rb);
    close(r.displacements_mm[1][2], -rb / k);
    close(r.reactions_n[0][2], 1000. - rb);
    assert!(r.max_relative_residual < 1e-12);
}

#[test]
fn unilateral_spring_engages_only_under_contact() {
    let k = 0.6;
    let mut down = spring_propped(k);
    down.supports = vec![Support::LowerSpring {
        node: 1,
        dof: 2,
        stiffness: k,
    }];
    let r = solve(&down).unwrap();
    // Downward tip load presses the spring: full spring behavior.
    let rb = 1000. * k * L.powi(3) / (3. * E * I + k * L.powi(3));
    close(r.reactions_n[1][2], rb);
    // Upward load lifts off: the spring vanishes, pure cantilever.
    let mut up = down.clone();
    up.forces_n[1] = [0., 0., 1000.];
    let r = solve(&up).unwrap();
    close(r.reactions_n[1][2], 0.);
    close(r.displacements_mm[1][2], 1000. * L.powi(3) / (3. * E * I));
}

/// Cantilever with a rigid prop below the tip, uniform load either way.
fn ground_propped(w: f64) -> Model {
    let mut m = cantilever();
    m.forces_n[1] = [0., 0., 0.];
    m.loads = vec![MemberLoad::Uniform {
        member: 0,
        force_n_per_mm: [0., 0., w],
        local_axes: false,
    }];
    m.supports = vec![Support::LowerBound { node: 1, dof: 2 }];
    m
}

#[test]
fn lower_bound_engages_and_lifts_off_by_load_direction() {
    // Downward UDL: prop engaged, propped-cantilever closed forms.
    let r = solve(&ground_propped(-10.)).unwrap();
    close(r.reactions_n[1][2], 3. * 10. * L / 8.);
    close(r.displacements_mm[1][2], 0.);
    close(r.members[0].stations[0].moment_y_nmm, 10. * L * L / 8.);
    // Upward UDL: the prop cannot pull, the contact opens; pure cantilever.
    let r = solve(&ground_propped(10.)).unwrap();
    close(r.reactions_n[1][2], 0.);
    close(r.displacements_mm[1][2], 10. * L.powi(4) / (8. * E * I));
    close(r.members[0].stations[0].moment_y_nmm, -10. * L * L / 2.);
}

#[test]
fn support_validation_rejects_bad_conditions() {
    // On a rigidly restrained DOF.
    let mut m = spring_propped(1.);
    m.supports = vec![Support::LowerBound { node: 0, dof: 2 }];
    assert_eq!(solve(&m).unwrap_err().code, "FRAME_INVALID_INPUT");
    // Duplicate DOF.
    m.supports = vec![
        Support::Spring {
            node: 1,
            dof: 2,
            stiffness: 1.,
        },
        Support::LowerBound { node: 1, dof: 2 },
    ];
    assert_eq!(solve(&m).unwrap_err().code, "FRAME_INVALID_INPUT");
    // Bad stiffness, DOF, node.
    for supports in [
        vec![Support::Spring {
            node: 1,
            dof: 2,
            stiffness: 0.,
        }],
        vec![Support::UpperSpring {
            node: 1,
            dof: 2,
            stiffness: f64::NAN,
        }],
        vec![Support::LowerBound { node: 1, dof: 6 }],
        vec![Support::LowerBound { node: 7, dof: 0 }],
    ] {
        m.supports = supports;
        assert_eq!(solve(&m).unwrap_err().code, "FRAME_INVALID_INPUT");
    }
}

#[test]
fn envelope_with_unilateral_support_solves_combinations_directly() {
    // Prop below the tip; case G presses down, case Q lifts harder.
    let down = ground_propped(-10.);
    let up = ground_propped(20.);
    let cases = vec![
        LoadSet {
            forces_n: down.forces_n.clone(),
            moments_nmm: down.moments_nmm.clone(),
            loads: down.loads.clone(),
        },
        LoadSet {
            forces_n: up.forces_n.clone(),
            moments_nmm: up.moments_nmm.clone(),
            loads: up.loads.clone(),
        },
    ];
    let combos = ["G", "Q", "G+Q"]
        .into_iter()
        .zip([vec![1., 0.], vec![0., 1.], vec![1., 1.]])
        .map(|(name, factors)| Combination {
            name: name.into(),
            factors,
        })
        .collect::<Vec<_>>();
    let r = solve_envelopes(
        &down.nodes_mm,
        &down.members,
        &down.restrained,
        &down.supports,
        &cases,
        &combos,
    )
    .unwrap();
    // G: prop engaged, R_B = 3wL/8 = 3750. Q and G+Q: net uplift, contact
    // open, R_B = 0. Superposing case responses would wrongly give 3750
    // for G+Q; the direct solve gives 0.
    let prop = &r.reactions_n[1][2];
    close(prop.max, 3750.);
    assert_eq!(prop.max_combination, 0);
    close(prop.min, 0.);
    // Tip u_z: G 0, Q 2·6.25, G+Q 6.25.
    let tip = &r.displacements_mm[1][2];
    close(tip.min, 0.);
    assert_eq!(tip.min_combination, 0);
    close(tip.max, 20. * L.powi(4) / (8. * E * I));
    assert_eq!(tip.max_combination, 1);
    // Root moment_y: G +wL²/8, Q −wL²/2, G+Q −5e6.
    let root = &r.members[0][0].moment_y_nmm;
    close(root.max, 10. * L * L / 8.);
    close(root.min, -20. * L * L / 2.);
    assert!(r.max_relative_residual < 1e-12);
}

#[test]
fn diagnose_names_released_rotations_at_a_hinge_node() {
    // Two spans along +X, fixed far ends, every rotation released where the
    // members meet: node 1 rotations have no stiffness at all.
    let mut first = member([0, 1]);
    first.release_b = [true; 3];
    let mut second = member([1, 2]);
    second.release_a = [true; 3];
    let (forces, moments) = empty_loads(3);
    let model = Model {
        nodes_mm: vec![[0., 0., 0.], [L, 0., 0.], [2. * L, 0., 0.]],
        members: vec![first, second],
        restrained: vec![[true; 6], [false; 6], [true; 6]],
        supports: vec![],
        forces_n: forces,
        moments_nmm: moments,
        loads: vec![],
    };
    let error = solve(&model).unwrap_err();
    assert_eq!(error.code, "FRAME_SINGULAR");
    assert!(error.contains("node 1 rx unrestrained"));
    assert!(error.contains("node 1 ry unrestrained"));
    assert!(error.contains("node 1 rz unrestrained"));
    let report = diagnose(&model.nodes_mm, &model.members, &model.restrained, &model.supports)
        .unwrap();
    assert!(!report.is_stable());
    assert_eq!(report.min_normalized_pivot, None);
    assert_eq!(
        report.issues,
        (3..6)
            .map(|dof| crate::diagnostics::DofDiagnosis {
                node: 1,
                dof,
                dof_name: DOF_NAMES[dof],
                issue: crate::diagnostics::DofIssue::Unrestrained,
            })
            .collect::<Vec<_>>()
    );
}

#[test]
fn diagnose_names_rigid_body_mechanism_and_stable_margin() {
    let mut free_beam = cantilever();
    free_beam.restrained = vec![[false; 6]; 2];
    let error = solve(&free_beam).unwrap_err();
    assert_eq!(error.code, "FRAME_SINGULAR");
    assert!(error.contains("in a mechanism"));
    assert!(error.contains("normalized pivot"));
    let report = diagnose(
        &free_beam.nodes_mm,
        &free_beam.members,
        &free_beam.restrained,
        &free_beam.supports,
    )
    .unwrap();
    assert!(!report.is_stable());
    assert!(
        report
            .issues
            .iter()
            .all(|d| d.issue == crate::diagnostics::DofIssue::Mechanism)
    );
    assert!(report.min_normalized_pivot.unwrap() <= 1e-12);
    // Six springs standing in for the fixed end stabilize the beam.
    let springy = diagnose(
        &free_beam.nodes_mm,
        &free_beam.members,
        &free_beam.restrained,
        &(0..6)
            .map(|dof| Support::Spring {
                node: 0,
                dof,
                stiffness: 1e9,
            })
            .collect::<Vec<_>>(),
    )
    .unwrap();
    assert!(springy.is_stable());
    let fixed = diagnose(
        &cantilever().nodes_mm,
        &cantilever().members,
        &cantilever().restrained,
        &[],
    )
    .unwrap();
    assert!(fixed.is_stable());
    assert!(fixed.min_normalized_pivot.unwrap() > 1e-12);
}

/// Four-element column along +X under a compressive (−x) tip load.
fn column(restrained: Vec<[bool; 6]>, tip_force: f64) -> (Vec<[f64; 3]>, Vec<Member>, Vec<[bool; 6]>, LoadSet) {
    let nodes_mm: Vec<[f64; 3]> = (0..=4).map(|i| [i as f64 * L / 4., 0., 0.]).collect();
    let members = (0..4).map(|i| member([i, i + 1])).collect();
    let (mut forces, moments) = empty_loads(5);
    forces[4] = [tip_force, 0., 0.];
    (
        nodes_mm,
        members,
        restrained,
        LoadSet {
            forces_n: forces,
            moments_nmm: moments,
            loads: vec![],
        },
    )
}

#[test]
fn buckling_cantilever_column_matches_euler() {
    let (nodes, members, restrained, reference) =
        column([vec![[true; 6]], vec![[false; 6]; 4]].concat(), -1000.);
    let r = buckling(&nodes, &members, &restrained, &[], &reference, 4).unwrap();
    assert_eq!(r.modes.len(), 4);
    for axial in &r.axial_forces_n {
        close(*axial, -1000.);
    }
    // P_cr = π²EI/(4L²); Iyy = Izz, so the first two modes are a
    // degenerate pair buckling in the two lateral planes.
    let p_cr = std::f64::consts::PI.powi(2) * E * I / (4. * L * L);
    let lambda = p_cr / 1000.;
    for mode in &r.modes[..2] {
        assert!(
            (mode.load_factor - lambda).abs() < 0.005 * lambda,
            "{} vs {lambda}",
            mode.load_factor
        );
        assert!(mode.relative_residual < 1e-8);
        let peak = mode
            .displacements
            .iter()
            .chain(&mode.rotations)
            .flatten()
            .map(|v| v.abs())
            .fold(0., f64::max);
        assert_eq!(peak, 1.);
    }
    // Second cantilever mode: (4.694/1.875)² ≈ 6.27 times the first.
    assert!(r.modes[2].load_factor > 5. * lambda);
}

#[test]
fn buckling_pinned_pinned_column_matches_euler_and_tension_flips_sign() {
    let restrained = vec![
        // Bending rotations free, torsion fixed: a torsion chain free at
        // both ends would be a rigid spin mode, not a pinned-pinned strut.
        [true, true, true, true, false, false],
        [false; 6],
        [false; 6],
        [false; 6],
        // Roller: axial DOF free so the tip load compresses the column.
        [false, true, true, false, false, false],
    ];
    let (nodes, members, restrained, reference) = column(restrained, -1000.);
    let r = buckling(&nodes, &members, &restrained, &[], &reference, 2).unwrap();
    let p_cr = std::f64::consts::PI.powi(2) * E * I / (L * L);
    let lambda = p_cr / 1000.;
    assert!((r.modes[0].load_factor - lambda).abs() < 0.005 * lambda);
    // The same column under tension buckles only under the reversed load.
    let (nodes, members, restrained, tension) = column(restrained.clone(), 1000.);
    let r = buckling(&nodes, &members, &restrained, &[], &tension, 2).unwrap();
    for axial in &r.axial_forces_n {
        close(*axial, 1000.);
    }
    for mode in &r.modes {
        assert!(mode.load_factor < 0.);
    }
    assert!((r.modes[0].load_factor + lambda).abs() < 0.005 * lambda);
}

#[test]
fn buckling_rejects_unilateral_supports_bad_mode_counts_and_singular() {
    let (nodes, members, restrained, reference) =
        column([vec![[true; 6]], vec![[false; 6]; 4]].concat(), -1000.);
    for bad_modes in [0, MAX_MODES + 1] {
        assert_eq!(
            buckling(&nodes, &members, &restrained, &[], &reference, bad_modes)
                .unwrap_err()
                .code,
            "FRAME_INVALID_INPUT"
        );
    }
    assert_eq!(
        buckling(
            &nodes,
            &members,
            &restrained,
            &[Support::LowerBound { node: 4, dof: 1 }],
            &reference,
            1,
        )
        .unwrap_err()
        .code,
        "FRAME_INVALID_INPUT"
    );
    let free: Vec<[bool; 6]> = vec![[false; 6]; 5];
    assert_eq!(
        buckling(&nodes, &members, &free, &[], &reference, 1)
            .unwrap_err()
            .code,
        "FRAME_SINGULAR"
    );
}

#[test]
fn modal_cantilever_matches_beam_theory_and_scales_with_density() {
    let (nodes, members, restrained, _) =
        column([vec![[true; 6]], vec![[false; 6]; 4]].concat(), 0.);
    let densities = [8e-9; 4];
    let r = modal(&nodes, &members, &restrained, &[], &densities, MassModel::Consistent, 5)
        .unwrap();
    assert_eq!(r.modes.len(), 5);
    close(r.total_mass_t, 8e-9 * A * L);
    // Spectrum of a cantilever with Iyy = Izz: a degenerate bending pair at
    // f1 = β₁²/(2π)·√(EI/(ρA))/L² (β₁ = 1.8751), then torsion
    // f_t = √((GJ)/(ρIp))/(4L), axial f_a = √(E/ρ)/(4L), then the second
    // bending pair at β₂ = 4.6941.
    let omega1 = 1.8751f64.powi(2) * (E * I / (8e-9 * A)).sqrt() / (L * L);
    let f1 = omega1 / (2. * std::f64::consts::PI);
    // f_t = c_t/(4L) and f_a = c_a/(4L) with wave speeds √(GJ/(ρIp)) and
    // √(E/ρ) (fixed-free rod: ω₁ = πc/(2L)).
    let f_torsion = (G * J / (8e-9 * 2. * I)).sqrt() / (4. * L);
    let f_axial = (E / 8e-9f64).sqrt() / (4. * L);
    let f2 = (4.6941f64 / 1.8751).powi(2) * f1;
    for (mode, (expected, tol)) in r
        .modes
        .iter()
        .zip([(f1, 0.005), (f1, 0.005), (f_torsion, 0.02), (f_axial, 0.02), (f2, 0.01)])
    {
        assert!(
            (mode.frequency_hz - expected).abs() < tol * expected,
            "{} vs {expected}",
            mode.frequency_hz
        );
        assert!(mode.relative_residual < 1e-8);
    }
    close(r.modes[0].omega_rad_s, 2. * std::f64::consts::PI * r.modes[0].frequency_hz);
    // Mode characters: bending is lateral, torsion is rx, axial is x.
    let lateral_peak = |m: &FrameModalMode| {
        m.displacements
            .iter()
            .map(|d| d[1].abs().max(d[2].abs()))
            .fold(0., f64::max)
    };
    assert_eq!(lateral_peak(&r.modes[0]), 1.);
    assert_eq!(lateral_peak(&r.modes[1]), 1.);
    assert_eq!(
        r.modes[2]
            .rotations
            .iter()
            .map(|r| r[0].abs())
            .fold(0., f64::max),
        1.
    );
    assert_eq!(
        r.modes[3]
            .displacements
            .iter()
            .map(|d| d[0].abs())
            .fold(0., f64::max),
        1.
    );
    // Doubling every density divides all frequencies by √2 exactly.
    let heavy = modal(
        &nodes,
        &members,
        &restrained,
        &[],
        &[16e-9; 4],
        MassModel::Consistent,
        4,
    )
    .unwrap();
    for (a, b) in r.modes.iter().zip(&heavy.modes) {
        close(a.frequency_hz / 2f64.sqrt(), b.frequency_hz);
    }
    // Lumped mass lands in the same neighborhood, converging from below.
    let lumped = modal(&nodes, &members, &restrained, &[], &densities, MassModel::Lumped, 1)
        .unwrap();
    let f1_lumped = lumped.modes[0].frequency_hz;
    assert!(f1_lumped > 0.85 * f1 && f1_lumped < 1.05 * f1, "{f1_lumped} vs {f1}");
    // Massless members leave no finite-frequency modes.
    let massless = modal(&nodes, &members, &restrained, &[], &[0.; 4], MassModel::Lumped, 2)
        .unwrap();
    assert!(massless.modes.is_empty());
    assert_eq!(massless.total_mass_t, 0.);
}

#[test]
fn modal_validates_input_and_refuses_unilateral_supports() {
    let (nodes, members, restrained, _) =
        column([vec![[true; 6]], vec![[false; 6]; 4]].concat(), 0.);
    let densities = [8e-9; 4];
    for bad in [0, MAX_MODES + 1] {
        assert_eq!(
            modal(&nodes, &members, &restrained, &[], &densities, MassModel::Lumped, bad)
                .unwrap_err()
                .code,
            "FRAME_INVALID_INPUT"
        );
    }
    for bad_densities in [
        vec![8e-9; 3],
        vec![8e-9, -1., 8e-9, 8e-9],
        vec![8e-9, f64::NAN, 8e-9, 8e-9],
    ] {
        assert_eq!(
            modal(&nodes, &members, &restrained, &[], &bad_densities, MassModel::Lumped, 1)
                .unwrap_err()
                .code,
            "FRAME_INVALID_INPUT"
        );
    }
    assert_eq!(
        modal(
            &nodes,
            &members,
            &restrained,
            &[Support::UpperBound { node: 4, dof: 2 }],
            &densities,
            MassModel::Lumped,
            1,
        )
        .unwrap_err()
        .code,
        "FRAME_INVALID_INPUT"
    );
}

/// Two-element beam of total length L with a unit downward force at the
/// mid node; `fixed_b` selects a fully fixed end B vs a vertical prop.
/// Out-of-plane and torsion DOFs are restrained at the free nodes, as is
/// standard when a planar model is run through a 3D solver — a full
/// bending hinge must not release the out-of-plane stability.
fn propped_beam(fixed_b: bool) -> (Vec<[f64; 3]>, Vec<Member>, Vec<[bool; 6]>, LoadSet) {
    let nodes = vec![[0., 0., 0.], [L / 2., 0., 0.], [L, 0., 0.]];
    let members = vec![member([0, 1]), member([1, 2])];
    let restrained = vec![
        [true; 6],
        [false, false, true, true, true, false],
        if fixed_b {
            [true; 6]
        } else {
            [false, true, true, true, true, false]
        },
    ];
    let (mut forces, moments) = empty_loads(3);
    forces[1] = [0., -1., 0.];
    (
        nodes,
        members,
        restrained,
        LoadSet {
            forces_n: forces,
            moments_nmm: moments,
            loads: vec![],
        },
    )
}

#[test]
fn collapse_cantilever_matches_exact_plastic_load() {
    let model = cantilever();
    let mp = 2.5e6;
    let r = collapse(
        &model.nodes_mm,
        &model.members,
        &model.restrained,
        &[],
        &LoadSet {
            forces_n: model.forces_n.clone(),
            moments_nmm: model.moments_nmm.clone(),
            loads: vec![],
        },
        &[Some(mp)],
        16,
    )
    .unwrap();
    // One base hinge at λ = Mp/(P·L); a hinged cantilever is a mechanism.
    assert_eq!(r.status, CollapseStatus::Mechanism);
    assert_eq!(r.hinges.len(), 1);
    assert_eq!(r.hinges[0].member, 0);
    assert!(r.hinges[0].at_node_a);
    let expected = mp / (1000. * L);
    close(r.hinges[0].load_factor, expected);
    close(r.collapse_load_factor.unwrap(), expected);
}

#[test]
fn collapse_propped_cantilever_matches_classical_solution() {
    let (nodes, members, restrained, loads) = propped_beam(false);
    let mp = 1e6;
    let r = collapse(
        &nodes,
        &members,
        &restrained,
        &[],
        &loads,
        &[Some(mp), Some(mp)],
        16,
    )
    .unwrap();
    // M_A = 3PL/16 yields first at λ₁ = 16Mp/(3L); then both mid-span ends
    // reach Mp together and the beam is a mechanism at λ = 6Mp/L.
    assert_eq!(r.status, CollapseStatus::Mechanism);
    assert_eq!(r.hinges.len(), 3);
    assert_eq!(r.hinges[0].member, 0);
    assert!(r.hinges[0].at_node_a);
    close(r.hinges[0].load_factor, 16. * mp / (3. * L));
    let mut mid: Vec<(usize, bool)> = r.hinges[1..]
        .iter()
        .map(|h| (h.member, h.at_node_a))
        .collect();
    mid.sort();
    assert_eq!(mid, vec![(0, false), (1, true)]);
    for h in &r.hinges[1..] {
        close(h.load_factor, 6. * mp / L);
    }
    close(r.collapse_load_factor.unwrap(), 6. * mp / L);
}

#[test]
fn collapse_fixed_fixed_beam_yields_all_ends_together() {
    let (nodes, members, restrained, loads) = propped_beam(true);
    let mp = 1e6;
    let r = collapse(
        &nodes,
        &members,
        &restrained,
        &[],
        &loads,
        &[Some(mp), Some(mp)],
        16,
    )
    .unwrap();
    // |M| = PL/8 at all four element ends: all yield at λ = 8Mp/L at once.
    assert_eq!(r.status, CollapseStatus::Mechanism);
    assert_eq!(r.hinges.len(), 4);
    for h in &r.hinges {
        close(h.load_factor, 8. * mp / L);
    }
    close(r.collapse_load_factor.unwrap(), 8. * mp / L);
}

#[test]
fn collapse_reports_elastic_unlimited_and_hinge_limit() {
    let (nodes, members, restrained, loads) = propped_beam(true);
    let mp = 1e6;
    // Only member 0 can yield: both its ends hinge at λ = 8Mp/L, then the
    // elastic member 1 carries the mid load as a cantilever from B.
    let r = collapse(
        &nodes,
        &members,
        &restrained,
        &[],
        &loads,
        &[Some(mp), None],
        16,
    )
    .unwrap();
    assert_eq!(r.status, CollapseStatus::ElasticUnlimited);
    assert_eq!(r.collapse_load_factor, None);
    assert_eq!(r.hinges.len(), 2);
    for h in &r.hinges {
        assert_eq!(h.member, 0);
        close(h.load_factor, 8. * mp / L);
    }
    // A budget of one hinge stops the propped cantilever after the first.
    let (nodes, members, restrained, loads) = propped_beam(false);
    let r = collapse(
        &nodes,
        &members,
        &restrained,
        &[],
        &loads,
        &[Some(mp), Some(mp)],
        1,
    )
    .unwrap();
    assert_eq!(r.status, CollapseStatus::HingeLimit);
    assert_eq!(r.collapse_load_factor, None);
    assert_eq!(r.hinges.len(), 1);
    close(r.hinges[0].load_factor, 16. * mp / (3. * L));
}

#[test]
fn collapse_validates_input() {
    let model = cantilever();
    let loads = LoadSet {
        forces_n: model.forces_n.clone(),
        moments_nmm: model.moments_nmm.clone(),
        loads: vec![],
    };
    let good = &[Some(1e6)];
    for bad in [
        &[][..],
        &[None],
        &[Some(0.)],
        &[Some(-1.)],
        &[Some(f64::NAN)],
        &[Some(1e6), None],
    ] {
        assert_eq!(
            collapse(
                &model.nodes_mm,
                &model.members,
                &model.restrained,
                &[],
                &loads,
                bad,
                16,
            )
            .unwrap_err()
            .code,
            "FRAME_INVALID_INPUT"
        );
    }
    for bad_budget in [0, MAX_PLASTIC_HINGES + 1] {
        assert_eq!(
            collapse(
                &model.nodes_mm,
                &model.members,
                &model.restrained,
                &[],
                &loads,
                good,
                bad_budget,
            )
            .unwrap_err()
            .code,
            "FRAME_INVALID_INPUT"
        );
    }
    assert_eq!(
        collapse(
            &model.nodes_mm,
            &model.members,
            &model.restrained,
            &[Support::LowerBound { node: 1, dof: 2 }],
            &loads,
            good,
            16,
        )
        .unwrap_err()
        .code,
        "FRAME_INVALID_INPUT"
    );
    // Axial-only load: no bending at yieldable sections.
    let mut axial = loads.clone();
    axial.forces_n[1] = [1000., 0., 0.];
    assert_eq!(
        collapse(
            &model.nodes_mm,
            &model.members,
            &model.restrained,
            &[],
            &axial,
            good,
            16,
        )
        .unwrap_err()
        .code,
        "FRAME_INVALID_INPUT"
    );
    // A structure that is a mechanism before any yielding keeps its error.
    assert!(
        collapse(
            &model.nodes_mm,
            &model.members,
            &vec![[false; 6]; 2],
            &[],
            &loads,
            good,
            16,
        )
        .is_err()
    );
}

/// Simply supported beam of length L, two elements, planar restraints.
fn simply_supported() -> (Vec<[f64; 3]>, Vec<Member>, Vec<[bool; 6]>) {
    let nodes = vec![[0., 0., 0.], [L / 2., 0., 0.], [L, 0., 0.]];
    let members = vec![member([0, 1]), member([1, 2])];
    let restrained = vec![
        [true, true, true, true, true, false],
        [false, false, true, true, true, false],
        [false, true, true, true, true, false],
    ];
    (nodes, members, restrained)
}

/// Load positions sweeping the beam at quarter points (both elements).
fn sweep_positions() -> Vec<(usize, f64)> {
    vec![
        (0, 0.),
        (0, L / 4.),
        (0, L / 2.),
        (1, 0.),
        (1, L / 4.),
        (1, L / 2.),
    ]
}

#[test]
fn influence_reaction_and_moment_match_classical_lines() {
    let (nodes, members, restrained) = simply_supported();
    let positions = sweep_positions();
    let down = [0., -1., 0.];
    // Support reaction at A: the line is 1 − x/L.
    let r = influence(
        &nodes,
        &members,
        &restrained,
        &[],
        &positions,
        down,
        &InfluenceTarget::Reaction { node: 0, dof: 1 },
    )
    .unwrap();
    for (v, x) in r.values.iter().zip([0., 250., 500., 500., 750., 1000.]) {
        close(*v, 1. - x / L);
    }
    // Bending moment at mid-span: the triangle peaking at a·b/L = 250.
    let r = influence(
        &nodes,
        &members,
        &restrained,
        &[],
        &positions,
        down,
        &InfluenceTarget::MemberResultant {
            member: 0,
            at_mm: L / 2.,
            resultant: Resultant::MomentZ,
        },
    )
    .unwrap();
    for (v, expected) in r.values.iter().zip([0., 125., 250., 250., 125., 0.]) {
        close(v.abs(), expected);
    }
}

#[test]
fn influence_displacement_matches_reciprocity() {
    let (nodes, members, restrained) = simply_supported();
    let positions = sweep_positions();
    let down = [0., -1., 0.];
    // Mid-span deflection for a load at x equals, by Betti–Maxwell, the
    // deflection at x for a unit load at mid-span: v = −x(3L²−4x²)/(48EI).
    let r = influence(
        &nodes,
        &members,
        &restrained,
        &[],
        &positions,
        down,
        &InfluenceTarget::Displacement { node: 1, dof: 1 },
    )
    .unwrap();
    let exact = |x: f64| {
        let x = x.min(L - x);
        -x * (3. * L * L - 4. * x * x) / (48. * E * I)
    };
    for (v, x) in r.values.iter().zip([0., 250., 500., 500., 750., 1000.]) {
        close(*v, exact(x));
    }
}

#[test]
fn influence_validates_input() {
    let (nodes, members, restrained) = simply_supported();
    let positions = sweep_positions();
    let down = [0., -1., 0.];
    let reaction = InfluenceTarget::Reaction { node: 0, dof: 1 };
    assert!(
        influence(&nodes, &members, &restrained, &[], &[], down, &reaction).is_err()
    );
    assert!(
        influence(
            &nodes,
            &members,
            &restrained,
            &[],
            &[(0, L * 2.)],
            down,
            &reaction,
        )
        .is_err()
    );
    assert!(
        influence(&nodes, &members, &restrained, &[], &[(7, 0.)], down, &reaction)
            .is_err()
    );
    assert!(
        influence(
            &nodes,
            &members,
            &restrained,
            &[],
            &positions,
            [0., 0., 0.],
            &reaction,
        )
        .is_err()
    );
    // Reaction target on a free DOF.
    assert!(
        influence(
            &nodes,
            &members,
            &restrained,
            &[],
            &positions,
            down,
            &InfluenceTarget::Reaction { node: 1, dof: 1 },
        )
        .is_err()
    );
    // Unilateral support refused.
    assert!(
        influence(
            &nodes,
            &members,
            &restrained,
            &[Support::LowerBound { node: 1, dof: 1 }],
            &positions,
            down,
            &reaction,
        )
        .is_err()
    );
}
