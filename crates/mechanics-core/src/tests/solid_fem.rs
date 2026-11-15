use super::*;
use crate::solid_mesh::shapes::*;
use crate::solid_mesh::{mesh_surface, MeshOptions};
use nalgebra::Rotation3;

const E: f64 = 2000.;

fn bar_mesh(b: f64, length: f64, size: f64) -> TetMesh {
    mesh_surface(
        &extrude(&rectangle(b, b), 0, length),
        MeshOptions {
            target_size_mm: size,
            min_quality: 0.05,
        },
    )
    .unwrap()
    .mesh
}

fn run(mesh: &TetMesh, material: Material, force: [f64; 3]) -> Solution {
    solve(&Problem {
        mesh,
        material,
        fixed_groups: vec![GROUP_BOTTOM],
        loads: vec![GroupLoad {
            group: GROUP_TOP,
            force_n: force,
        }],
    })
    .unwrap()
}

fn tip_mean(mesh: &TetMesh, sol: &Solution, length: f64, axis: usize) -> f64 {
    let tip: Vec<_> = (0..mesh.nodes_mm.len())
        .filter(|&i| (mesh.nodes_mm[i][2] - length).abs() < 1e-9)
        .collect();
    tip.iter()
        .map(|&i| sol.displacements_mm[i][axis])
        .sum::<f64>()
        / tip.len() as f64
}

#[test]
fn axial_bar_matches_fl_over_ea() {
    // nu = 0 so the clamped end does not restrain lateral contraction.
    let (b, l, f) = (10., 50., 1000.);
    let mesh = bar_mesh(b, l, 2.5);
    let sol = run(
        &mesh,
        Material::Isotropic {
            young_mpa: E,
            poisson: 0.,
        },
        [0., 0., f],
    );
    let exact = f * l / (E * b * b);
    let tip = tip_mean(&mesh, &sol, l, 2);
    assert!(
        (tip - exact).abs() / exact < 1e-6,
        "tip {tip} exact {exact}"
    );
    assert!((sol.max_von_mises_mpa - f / (b * b)).abs() < 1e-6);
    assert!(sol.reaction_balance_error < 1e-8);
}

fn cantilever_exact(b: f64, l: f64, f: f64, nu: f64) -> f64 {
    let i = b.powi(4) / 12.;
    let g = E / (2. * (1. + nu));
    f * l.powi(3) / (3. * E * i) + f * l / (5. / 6. * g * b * b)
}

#[test]
fn cantilever_converges_to_timoshenko_tip_deflection() {
    let (b, l, f, nu) = (10., 60., 10., 0.3);
    let exact = cantilever_exact(b, l, f, nu);
    let material = Material::Isotropic {
        young_mpa: E,
        poisson: nu,
    };
    let mut tips = Vec::new();
    let report = convergence_study(&[2.5, 10. / 6., 1.25], 0.1, |size| {
        let mesh = bar_mesh(b, l, size);
        let sol = run(&mesh, material, [f, 0., 0.]);
        assert!(sol.reaction_balance_error < 1e-6);
        tips.push(tip_mean(&mesh, &sol, l, 0));
        Ok((mesh, sol))
    })
    .unwrap();
    eprintln!("exact {exact} tips {tips:?} report {report:?}");
    // Tet4 is stiff in bending: deflection grows monotonically toward the beam solution.
    assert!(tips[0] < tips[1] && tips[1] < tips[2] && tips[2] < exact * 1.02);
    assert!(
        (tips[2] - exact).abs() / exact < 0.1,
        "finest error too large"
    );
    assert!(report.displacement_converged && report.converged);
    assert_eq!(report.displacement_change.len(), 2);
    assert!(report.displacement_change[1] < report.displacement_change[0]);
}

fn print_material(build_dir: [f64; 3]) -> Material {
    print_material_nu(build_dir, 0.35, 0.3)
}

fn print_material_nu(build_dir: [f64; 3], nu_p: f64, nu_z: f64) -> Material {
    Material::TransverselyIsotropic {
        young_in_plane_mpa: 2400.,
        young_build_mpa: 1600.,
        poisson_in_plane: nu_p,
        poisson_build: nu_z,
        shear_build_mpa: 700.,
        build_dir,
    }
}

#[test]
fn transversely_isotropic_reduces_to_isotropic() {
    let nu = 0.3;
    let iso = Material::Isotropic {
        young_mpa: E,
        poisson: nu,
    }
    .stiffness()
    .unwrap();
    let ti = Material::TransverselyIsotropic {
        young_in_plane_mpa: E,
        young_build_mpa: E,
        poisson_in_plane: nu,
        poisson_build: nu,
        shear_build_mpa: E / (2. * (1. + nu)),
        build_dir: [0.3, -0.5, 0.8],
    }
    .stiffness()
    .unwrap();
    assert!((iso - ti).norm() < 1e-9 * iso.norm());
}

#[test]
fn print_orientation_changes_axial_stiffness() {
    let (b, l, f) = (10., 50., 1000.);
    let mesh = bar_mesh(b, l, 2.5);
    // Upright print: layers stacked along the bar (load across layers).
    // Zero Poisson ratios keep the clamped end from restraining lateral contraction.
    let upright = run(&mesh, print_material_nu([0., 0., 1.], 0., 0.), [0., 0., f]);
    // Lying print: layers stacked across the bar (load along layers).
    let lying = run(&mesh, print_material_nu([1., 0., 0.], 0., 0.), [0., 0., f]);
    let (u, y) = (
        tip_mean(&mesh, &upright, l, 2),
        tip_mean(&mesh, &lying, l, 2),
    );
    // Uniaxial stress: tip = F L / (E_dir A).
    assert!(
        (u - f * l / (1600. * b * b)).abs() / u < 1e-6,
        "upright {u}"
    );
    assert!((y - f * l / (2400. * b * b)).abs() / y < 1e-6, "lying {y}");
    assert!((u / y - 1.5).abs() < 1e-6);
}

#[test]
fn print_orientation_changes_bending_but_rotation_of_whole_problem_does_not() {
    let l = [
        [0., 0.],
        [20., 0.],
        [20., 5.],
        [5., 5.],
        [5., 20.],
        [0., 20.],
    ];
    let report = mesh_surface(
        &extrude(&l, 3, 6.),
        MeshOptions {
            target_size_mm: 1.5,
            min_quality: 0.05,
        },
    )
    .unwrap();
    let mesh = report.mesh;
    // Clamp the end of one arm (edge 0 side, y = 0 face) and pull the other arm's end.
    let fixed = GROUP_SIDE; // y = 0 face of the horizontal arm
    let loaded = GROUP_SIDE + 4; // y = 20 face of the vertical arm (edge 4: (5,20)->(0,20))
    let solve_with = |mesh: &TetMesh, build: [f64; 3], force: [f64; 3]| {
        solve(&Problem {
            mesh,
            material: print_material(build),
            fixed_groups: vec![fixed],
            loads: vec![GroupLoad {
                group: loaded,
                force_n: force,
            }],
        })
        .unwrap()
    };
    let flat = solve_with(&mesh, [0., 0., 1.], [10., 0., 0.]);
    let on_edge = solve_with(&mesh, [1., 0., 0.], [10., 0., 0.]);
    assert!(flat.reaction_balance_error < 1e-6 && on_edge.reaction_balance_error < 1e-6);
    let ratio = on_edge.max_displacement_mm / flat.max_displacement_mm;
    eprintln!(
        "flat {} on_edge {} ratio {ratio}",
        flat.max_displacement_mm, on_edge.max_displacement_mm
    );
    // Only the horizontal arm's bending stress runs across layers when printed on edge,
    // so the softening lies strictly between 1 and E_in_plane / E_build = 1.5.
    assert!(
        ratio > 1.03 && ratio < 1.5,
        "orientation must matter: ratio {ratio}"
    );
    assert!((on_edge.max_von_mises_mpa - flat.max_von_mises_mpa).abs() > 1e-6);

    // Rotate geometry, load and build direction together: physical answer is unchanged.
    let rot = Rotation3::from_euler_angles(0.4, -0.7, 1.1);
    let rotated = TetMesh {
        nodes_mm: mesh
            .nodes_mm
            .iter()
            .map(|p| (rot * Vector3::from(*p)).into())
            .collect(),
        tets: mesh.tets.clone(),
        boundary: mesh.boundary.clone(),
    };
    let r = |v: [f64; 3]| -> [f64; 3] { (rot * Vector3::from(v)).into() };
    let turned = solve_with(&rotated, r([1., 0., 0.]), r([10., 0., 0.]));
    let rel = (turned.max_displacement_mm - on_edge.max_displacement_mm).abs()
        / on_edge.max_displacement_mm;
    assert!(rel < 1e-6, "rotation invariance {rel}");
    let rel =
        (turned.max_von_mises_mpa - on_edge.max_von_mises_mpa).abs() / on_edge.max_von_mises_mpa;
    assert!(rel < 1e-6, "rotation invariance stress {rel}");
}

#[test]
fn rejects_missing_supports_and_bad_material() {
    let mesh = bar_mesh(2., 4., 1.);
    let mut p = Problem {
        mesh: &mesh,
        material: Material::Isotropic {
            young_mpa: E,
            poisson: 0.3,
        },
        fixed_groups: vec![99],
        loads: vec![],
    };
    assert!(solve(&p).is_err());
    p.fixed_groups = vec![GROUP_BOTTOM];
    p.loads = vec![GroupLoad {
        group: 77,
        force_n: [1., 0., 0.],
    }];
    assert!(solve(&p).is_err());
    p.loads.clear();
    p.material = Material::Isotropic {
        young_mpa: E,
        poisson: 0.5,
    };
    assert!(solve(&p).is_err());
    assert!(convergence_study(&[1., 2.], 0.1, |_| unreachable!()).is_err());
}
