use super::*;
use crate::diagnostics::{DofDiagnosis, DofIssue};

fn bar() -> Model {
    Model {
        nodes_mm: vec![[0., 0., 0.], [0., 0., 10.]],
        members: vec![Member {
            nodes: [0, 1],
            young_mpa: 2000.,
            area_mm2: 2.,
        }],
        restrained: vec![[true; 3], [true, true, false]],
        forces_n: vec![[0.; 3], [0., 0., 100.]],
    }
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() <= 1e-10 * b.abs().max(1.), "{a} != {b}");
}

#[test]
fn axial_bar_has_analytical_deflection_stress_and_reaction() {
    let response = solve(&bar()).unwrap();
    close(response.displacements_mm[1][2], 0.25);
    close(response.axial_forces_n[0], 100.);
    close(response.axial_stresses_mpa[0], 50.);
    close(response.reactions_n[0][2], -100.);
    close(response.reactions_n[1][2], 0.);
    assert!(response.max_relative_residual < 1e-12);
    assert_eq!(response.free_dofs, 1);
}

#[test]
fn refuses_the_branch_two_node_lateral_load_regression() {
    let mut model = bar();
    model.restrained[1] = [false; 3];
    model.forces_n[1] = [100., 0., 0.];
    assert_eq!(solve(&model).unwrap_err().code, "TRUSS_SINGULAR");
    model.forces_n[1] = [0.; 3];
    assert_eq!(solve(&model).unwrap_err().code, "TRUSS_SINGULAR");
}

#[test]
fn series_members_share_force_and_preserve_equilibrium() {
    let mut model = bar();
    model.nodes_mm.push([0., 0., 20.]);
    model.members.push(Member {
        nodes: [1, 2],
        young_mpa: 2000.,
        area_mm2: 2.,
    });
    model.restrained.push([true, true, false]);
    model.forces_n[1] = [0.; 3];
    model.forces_n.push([0., 0., 100.]);
    let result = solve(&model).unwrap();
    close(result.displacements_mm[1][2], 0.25);
    close(result.displacements_mm[2][2], 0.5);
    for force in result.axial_forces_n {
        close(force, 100.);
    }
    close(result.reactions_n.iter().map(|r| r[2]).sum::<f64>(), -100.);
}

#[test]
fn restraint_loads_are_reactions_not_member_force_proxies() {
    let mut model = bar();
    model.restrained[1] = [true; 3];
    let result = solve(&model).unwrap();
    assert_eq!(result.free_dofs, 0);
    assert_eq!(result.displacements_mm, vec![[0.; 3]; 2]);
    assert_eq!(result.reactions_n[1], [0., 0., -100.]);
    assert_eq!(result.axial_forces_n, [0.]);
}

#[test]
fn uniform_modulus_and_load_scaling_preserves_displacements() {
    for scale in [1e-100, 1e-20, 1., 1e20, 1e100] {
        let mut model = bar();
        model.members[0].young_mpa *= scale;
        model.forces_n[1][2] *= scale;
        let result = solve(&model).unwrap();
        close(result.displacements_mm[1][2], 0.25);
        close(result.reactions_n[0][2] / scale, -100.);
    }
}

#[test]
fn rejects_invalid_input_before_matrix_allocation() {
    let mut models = Vec::new();
    let mut m = bar();
    m.nodes_mm[1] = m.nodes_mm[0];
    models.push(m);
    let mut m = bar();
    m.nodes_mm[0][0] = f64::NAN;
    models.push(m);
    let mut m = bar();
    m.forces_n[1][2] = f64::INFINITY;
    models.push(m);
    let mut m = bar();
    m.restrained.clear();
    models.push(m);
    let mut m = bar();
    m.members[0].nodes = [0, 2];
    models.push(m);
    let mut m = bar();
    m.members.push(m.members[0].clone());
    models.push(m);
    let mut m = bar();
    m.members[0].young_mpa = 0.;
    models.push(m);
    let mut m = bar();
    m.members[0].area_mm2 = -1.;
    models.push(m);
    let mut m = bar();
    m.nodes_mm = vec![[0.; 3]; 126];
    models.push(m);
    let mut m = bar();
    m.members = vec![m.members[0].clone(); 401];
    models.push(m);
    for model in models {
        assert_eq!(solve(&model).unwrap_err().code, "TRUSS_INVALID_INPUT");
    }
}

#[test]
fn refuses_nearly_collinear_free_modes_and_numeric_overflow() {
    let model = Model {
        nodes_mm: vec![[0., 0., 0.], [1., 1., 0.], [1., 1. + 1e-8, 0.]],
        members: vec![
            Member {
                nodes: [0, 1],
                young_mpa: 2000.,
                area_mm2: 2.,
            },
            Member {
                nodes: [0, 2],
                young_mpa: 2000.,
                area_mm2: 2.,
            },
        ],
        restrained: vec![[false, false, true], [true; 3], [true; 3]],
        forces_n: vec![[100., 0., 0.], [0.; 3], [0.; 3]],
    };
    assert_eq!(solve(&model).unwrap_err().code, "TRUSS_SINGULAR");
    let mut model = bar();
    model.members[0].young_mpa = f64::MAX;
    assert_eq!(solve(&model).unwrap_err().code, "TRUSS_NUMERIC_RANGE");
}

#[test]
fn rotated_translated_tripod_preserves_response_and_vector_reactions() {
    let model = Model {
        nodes_mm: vec![[0.; 3], [10., 0., 0.], [0., 10., 0.], [0., 0., 10.]],
        members: (1..4)
            .map(|i| Member {
                nodes: [0, i],
                young_mpa: 2000.,
                area_mm2: 2.,
            })
            .collect(),
        restrained: vec![[false; 3], [true; 3], [true; 3], [true; 3]],
        forces_n: vec![[100., 200., 300.], [0.; 3], [0.; 3], [0.; 3]],
    };
    let response = solve(&model).unwrap();
    for (actual, expected) in response.displacements_mm[0].iter().zip([0.25, 0.5, 0.75]) {
        close(*actual, expected);
    }
    let rotate = |p: [f64; 3]| {
        let h = 0.5f64.sqrt();
        [(p[0] - p[1]) * h, (p[0] + p[1]) * h, p[2]]
    };
    let mut transformed = model.clone();
    transformed.nodes_mm = model
        .nodes_mm
        .iter()
        .map(|&p| {
            let r = rotate(p);
            std::array::from_fn(|k| r[k] + [100., -40., 17.][k])
        })
        .collect();
    transformed.forces_n = model.forces_n.iter().copied().map(rotate).collect();
    let rotated = solve(&transformed).unwrap();
    for i in 0..4 {
        for k in 0..3 {
            close(
                rotated.displacements_mm[i][k],
                rotate(response.displacements_mm[i])[k],
            );
            close(
                rotated.reactions_n[i][k],
                rotate(response.reactions_n[i])[k],
            );
        }
    }
    for k in 0..3 {
        close(
            rotated.reactions_n.iter().map(|r| r[k]).sum::<f64>(),
            -transformed.forces_n[0][k],
        );
    }
    // Reversing member endpoints must not reverse the tension convention.
    for member in &mut transformed.members {
        member.nodes.swap(0, 1);
    }
    let reversed = solve(&transformed).unwrap();
    for (a, b) in reversed.axial_forces_n.iter().zip(rotated.axial_forces_n) {
        close(*a, b);
    }
}

#[test]
fn admits_the_node_limit_with_independent_stable_tripods() {
    let mut model = Model {
        nodes_mm: vec![[10., 0., 0.], [0., 10., 0.], [0., 0., 0.]],
        members: Vec::new(),
        restrained: vec![[true; 3]; 3],
        forces_n: vec![[0.; 3]; 3],
    };
    for i in 3..MAX_NODES {
        model.nodes_mm.push([1., 2., 10. + i as f64 / 10.]);
        model.restrained.push([false; 3]);
        model.forces_n.push([1., -2., -3.]);
        for anchor in 0..3 {
            model.members.push(Member {
                nodes: [anchor, i],
                young_mpa: 2000.,
                area_mm2: 2.,
            });
        }
    }
    let result = solve(&model).unwrap();
    assert_eq!(result.free_dofs, 366);
    assert!(result.max_relative_residual < 1e-12);
    for k in 0..3 {
        close(
            result.reactions_n.iter().map(|r| r[k]).sum::<f64>(),
            -model.forces_n.iter().map(|f| f[k]).sum::<f64>(),
        );
    }
}

#[test]
fn diagnose_names_unrestrained_dofs_and_enriches_the_error() {
    // The bar runs along z, so the freed node keeps axial stiffness; x/y
    // are completely unrestrained.
    let mut model = bar();
    model.restrained[1] = [false; 3];
    let error = solve(&model).unwrap_err();
    assert_eq!(error.code, "TRUSS_SINGULAR");
    assert!(error.contains("node 1 x unrestrained"));
    assert!(error.contains("node 1 y unrestrained"));
    let report = diagnose(&model).unwrap();
    assert!(!report.is_stable());
    assert_eq!(report.min_normalized_pivot, None);
    assert_eq!(
        report.issues,
        vec![
            DofDiagnosis {
                node: 1,
                dof: 0,
                dof_name: "x",
                issue: DofIssue::Unrestrained,
            },
            DofDiagnosis {
                node: 1,
                dof: 1,
                dof_name: "y",
                issue: DofIssue::Unrestrained,
            },
        ]
    );
}

#[test]
fn diagnose_names_collinear_mechanism_dofs_and_stable_margin() {
    let mechanism = Model {
        nodes_mm: vec![[0., 0., 0.], [1., 1., 0.], [1., 1. + 1e-8, 0.]],
        members: vec![
            Member {
                nodes: [0, 1],
                young_mpa: 2000.,
                area_mm2: 2.,
            },
            Member {
                nodes: [0, 2],
                young_mpa: 2000.,
                area_mm2: 2.,
            },
        ],
        restrained: vec![[false, false, true], [true; 3], [true; 3]],
        forces_n: vec![[100., 0., 0.], [0.; 3], [0.; 3]],
    };
    let error = solve(&mechanism).unwrap_err();
    assert_eq!(error.code, "TRUSS_SINGULAR");
    assert!(error.contains("node 0"));
    assert!(error.contains("in a mechanism"));
    let report = diagnose(&mechanism).unwrap();
    assert!(!report.is_stable());
    assert!(
        report
            .issues
            .iter()
            .all(|d| d.node == 0 && d.issue == DofIssue::Mechanism)
    );
    assert!(report.min_normalized_pivot.unwrap() <= 1e-12);
    let stable = diagnose(&bar()).unwrap();
    assert!(stable.is_stable());
    assert!(stable.min_normalized_pivot.unwrap() > 1e-12);
}

/// Planar two-chord lattice cantilever along +X (span 2000, chords 50
/// apart), fixed at the left pair of nodes, tip compressed in −x.
fn lattice_column(area: f64, tip_load: f64) -> Model {
    let mut nodes_mm = Vec::new();
    for i in 0..5 {
        nodes_mm.push([i as f64 * 500., 0., 0.]); // bottom chord, ids 0-4
    }
    for i in 0..5 {
        nodes_mm.push([i as f64 * 500., 50., 0.]); // top chord, ids 5-9
    }
    let bar_member = |a: usize, b: usize| Member {
        nodes: [a, b],
        young_mpa: 200_000.,
        area_mm2: area,
    };
    let mut members = Vec::new();
    for i in 0..4 {
        members.push(bar_member(i, i + 1)); // bottom chord
        members.push(bar_member(5 + i, 6 + i)); // top chord
        members.push(bar_member(i, 6 + i)); // diagonal
    }
    for i in 0..5 {
        members.push(bar_member(i, 5 + i)); // verticals
    }
    let mut restrained = vec![[false, false, true]; 10];
    restrained[0] = [true; 3];
    restrained[5] = [true; 3];
    let mut forces_n = vec![[0.; 3]; 10];
    forces_n[4] = [tip_load / 2., 0., 0.];
    forces_n[9] = [tip_load / 2., 0., 0.];
    Model {
        nodes_mm,
        members,
        restrained,
        forces_n,
    }
}

#[test]
fn buckling_lattice_column_near_euler_and_scales_with_area() {
    let reference = buckling(&lattice_column(10., -1000.), 2).unwrap();
    assert!(!reference.modes.is_empty());
    let lambda = reference.modes[0].load_factor;
    // Effective I = A·h²/2 = 12500 mm⁴ → Euler P_cr ≈ π²EI/(4L²) ≈ 1542 N;
    // lattice shear flexibility lowers the value, so admit a wide band.
    let euler = std::f64::consts::PI.powi(2) * 200_000. * 12_500. / (4. * 2000. * 2000.);
    assert!(
        (lambda - euler / 1000.).abs() < 0.35 * euler / 1000.,
        "{lambda} vs {}",
        euler / 1000.
    );
    assert!(lambda > 0.);
    assert!(reference.modes[0].relative_residual < 1e-8);
    // In-plane lateral mode: the peak component is a y displacement.
    let peak_y = reference.modes[0]
        .displacements
        .iter()
        .map(|d| d[1].abs())
        .fold(0., f64::max);
    assert_eq!(peak_y, 1.);
    // Doubling every bar area doubles the critical load exactly (both the
    // elastic and the lattice-shear stiffness scale with A).
    let doubled = buckling(&lattice_column(20., -1000.), 1).unwrap();
    close(doubled.modes[0].load_factor, 2. * lambda);
    // Tension reference: buckling appears only under the reversed load.
    let tension = buckling(&lattice_column(10., 1000.), 1).unwrap();
    assert!(tension.modes[0].load_factor < 0.);
    close(tension.modes[0].load_factor, -lambda);
}

#[test]
fn buckling_validates_mode_count_and_propagates_singularity() {
    for bad in [0, MAX_MODES + 1] {
        assert_eq!(
            buckling(&lattice_column(10., -1000.), bad).unwrap_err().code,
            "TRUSS_INVALID_INPUT"
        );
    }
    let mut free = lattice_column(10., -1000.);
    free.restrained = vec![[false; 3]; 10];
    assert_eq!(
        buckling(&free, 1).unwrap_err().code,
        "TRUSS_SINGULAR"
    );
}

/// Fixed-free axial rod along +X from four bars (L = 1000, only x free).
fn axial_rod() -> Model {
    Model {
        nodes_mm: (0..=4).map(|i| [i as f64 * 250., 0., 0.]).collect(),
        members: (0..4)
            .map(|i| Member {
                nodes: [i, i + 1],
                young_mpa: 200_000.,
                area_mm2: 100.,
            })
            .collect(),
        restrained: [vec![[true; 3]], vec![[false, true, true]; 4]].concat(),
        forces_n: vec![[0.; 3]; 5],
    }
}

#[test]
fn modal_axial_rod_matches_rod_theory_and_scales_with_density() {
    let densities = [8e-9; 4];
    let r = modal(&axial_rod(), &densities, MassModel::Consistent, 2).unwrap();
    assert_eq!(r.modes.len(), 2);
    close(r.total_mass_t, 8e-9 * 100. * 1000.);
    // f₁ = c/(4L), c = √(E/ρ); f₃ = 3f₁ for the fixed-free rod.
    let f1 = (200_000f64 / 8e-9).sqrt() / (4. * 1000.);
    assert!(
        (r.modes[0].frequency_hz - f1).abs() < 0.02 * f1,
        "{} vs {f1}",
        r.modes[0].frequency_hz
    );
    assert!(
        (r.modes[1].frequency_hz - 3. * f1).abs() < 0.08 * 3. * f1,
        "{} vs {}",
        r.modes[1].frequency_hz,
        3. * f1
    );
    // Axial mode: the peak component is an x displacement.
    assert_eq!(
        r.modes[0]
            .displacements
            .iter()
            .map(|d| d[0].abs())
            .fold(0., f64::max),
        1.
    );
    assert!(r.modes[0].relative_residual < 1e-8);
    // Doubling density divides the frequency by √2 exactly.
    let heavy = modal(&axial_rod(), &[16e-9; 4], MassModel::Consistent, 1).unwrap();
    close(r.modes[0].frequency_hz / 2f64.sqrt(), heavy.modes[0].frequency_hz);
    // Lumped lands slightly below; zero density leaves no modes.
    let lumped = modal(&axial_rod(), &densities, MassModel::Lumped, 1).unwrap();
    let f1_lumped = lumped.modes[0].frequency_hz;
    assert!(f1_lumped > 0.9 * f1 && f1_lumped < 1.02 * f1, "{f1_lumped} vs {f1}");
    assert!(
        modal(&axial_rod(), &[0.; 4], MassModel::Lumped, 1)
            .unwrap()
            .modes
            .is_empty()
    );
}

#[test]
fn modal_validates_densities_and_mode_count() {
    for bad in [0, MAX_MODES + 1] {
        assert_eq!(
            modal(&axial_rod(), &[8e-9; 4], MassModel::Lumped, bad)
                .unwrap_err()
                .code,
            "TRUSS_INVALID_INPUT"
        );
    }
    for bad_densities in [vec![8e-9; 3], vec![8e-9, -1., 8e-9, 8e-9]] {
        assert_eq!(
            modal(&axial_rod(), &bad_densities, MassModel::Lumped, 1)
                .unwrap_err()
                .code,
            "TRUSS_INVALID_INPUT"
        );
    }
}

/// von Mises toggle: supports at (±1000, 0), apex at (0, 30) pressed down.
/// Planar model: the apex is restrained out of plane.
fn toggle(apex_load: f64) -> Model {
    Model {
        nodes_mm: vec![[-1000., 0., 0.], [1000., 0., 0.], [0., 30., 0.]],
        members: vec![
            Member {
                nodes: [0, 2],
                young_mpa: 200_000.,
                area_mm2: 100.,
            },
            Member {
                nodes: [1, 2],
                young_mpa: 200_000.,
                area_mm2: 100.,
            },
        ],
        restrained: vec![[true; 3], [true; 3], [false, false, true]],
        forces_n: vec![[0.; 3], [0.; 3], [0., -apex_load, 0.]],
    }
}

/// Exact toggle path: P(y) = 2·EA·(L₀−L)/L₀·(y/L), y = current apex height.
fn toggle_load(y: f64) -> f64 {
    let l0 = (1000f64.powi(2) + 30f64.powi(2)).sqrt();
    let l = (1000f64.powi(2) + y * y).sqrt();
    2. * 200_000. * 100. * (l0 - l) / l0 * (y / l)
}

const NR: NonlinearOptions = NonlinearOptions {
    steps: 10,
    tolerance: 1e-9,
    max_iterations: 50,
};

#[test]
fn nonlinear_toggle_tracks_the_exact_equilibrium_path() {
    // Ask for the load that belongs to apex height y = 20 (v = 10 mm down).
    let p = toggle_load(20.);
    let r = solve_nonlinear(&toggle(p), &NR).unwrap();
    assert!(r.converged);
    assert_eq!(r.load_factor, 1.);
    close(r.displacements_mm[2][1], -10.);
    assert!(r.displacements_mm[2][0].abs() < 1e-9);
    // Member force and reaction equilibrium at the exact state.
    let l0 = (1000f64.powi(2) + 900.).sqrt();
    let l = (1000f64.powi(2) + 400.).sqrt();
    let n = 200_000. * 100. * (l - l0) / l0;
    close(r.axial_forces_n[0], n);
    close(r.axial_forces_n[1], n);
    close(r.axial_stresses_mpa[0], n / 100.);
    close(r.reactions_n[0][1] + r.reactions_n[1][1], p);
    assert!(r.steps.len() <= 10 + 8);
    for step in &r.steps {
        assert!(step.relative_residual <= 1e-9);
    }
}

#[test]
fn nonlinear_small_load_matches_the_linear_solve() {
    let model = bar();
    let linear = solve(&model).unwrap();
    let r = solve_nonlinear(&model, &NR).unwrap();
    assert!(r.converged);
    close(r.displacements_mm[1][2], linear.displacements_mm[1][2]);
    close(r.axial_forces_n[0], linear.axial_forces_n[0]);
    close(r.reactions_n[0][2], linear.reactions_n[0][2]);
}

#[test]
fn nonlinear_beyond_the_limit_point_reports_snap_through() {
    // The toggle's load-controlled limit is ≈ 207.7 N; 400 N cannot be
    // reached: the solve stalls near λ ≈ 207.7/400 ≈ 0.52.
    let r = solve_nonlinear(&toggle(400.), &NR).unwrap();
    assert!(!r.converged);
    assert!(r.load_factor > 0.35 && r.load_factor < 0.65, "{}", r.load_factor);
    assert!(!r.steps.is_empty());
}

#[test]
fn nonlinear_validates_options_and_keeps_mechanism_errors() {
    let model = toggle(100.);
    for bad in [
        NonlinearOptions {
            steps: 0,
            ..NR
        },
        NonlinearOptions {
            steps: MAX_LOAD_STEPS + 1,
            ..NR
        },
        NonlinearOptions {
            tolerance: 0.,
            ..NR
        },
        NonlinearOptions {
            tolerance: 0.01,
            ..NR
        },
        NonlinearOptions {
            max_iterations: 0,
            ..NR
        },
    ] {
        assert_eq!(
            solve_nonlinear(&model, &bad).unwrap_err().code,
            "TRUSS_INVALID_INPUT"
        );
    }
    // A mechanism from the start keeps the diagnosed singularity error.
    let mut free = toggle(100.);
    free.restrained[2] = [false; 3];
    assert_eq!(
        solve_nonlinear(&free, &NR).unwrap_err().code,
        "TRUSS_SINGULAR"
    );
}
