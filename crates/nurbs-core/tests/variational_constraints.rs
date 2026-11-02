use nurbs_core::{
    constrained_surface::{
        Constraint,
        variational::{self, Material, Model, Options, PointLoad},
    },
    surface::Surface,
};
fn plane(degree: usize) -> Surface {
    nurbs_core::patches::bezier(
        (0..=degree)
            .map(|i| {
                (0..=degree)
                    .map(|j| vec![i as f64 / degree as f64, j as f64 / degree as f64, 0.])
                    .collect()
            })
            .collect(),
        None,
    )
    .unwrap()
}
fn rim(s: &Surface) -> Vec<Constraint> {
    let (n, m) = (s.control_points.len(), s.control_points[0].len());
    (0..n)
        .flat_map(|i| {
            (0..m).filter_map(move |j| {
                if i == 0 || j == 0 || i == n - 1 || j == m - 1 {
                    let p = &s.control_points[i][j];
                    Some(Constraint::ControlPoint {
                        index: [i, j],
                        point: [p[0], p[1], p[2]],
                    })
                } else {
                    None
                }
            })
        })
        .collect()
}
fn assert_close(a: f64, b: f64, t: f64) {
    assert!(
        (a - b).abs() < t,
        "actual {a}, expected {b}, difference {}",
        a - b
    );
}
#[test]
fn original_rational_point_load_matches_independent_anchor_solution() {
    let mut seed = plane(2);
    seed.weights[1][1] = 1.7;
    seed.weights[2][1] = 0.6;
    let before = seed.clone();
    let mut m = Model::default();
    m.material = Material {
        membrane: 0.,
        bending: 0.,
        poisson: 0.2,
    };
    m.points = vec![PointLoad {
        uv: [0.25, 0.75],
        force: [0.2, -0.4, 1.],
    }];
    let mut o = Options::default();
    o.fitting.anchor_weight = 2.;
    let q = variational::solve(&seed, &[], &m, o).unwrap();
    let u = [0.75 * 0.75, 2. * 0.25 * 0.75, 0.25 * 0.25];
    let v = [0.25 * 0.25, 2. * 0.75 * 0.25, 0.75 * 0.75];
    let denom = (0..3)
        .flat_map(|i| (0..3).map(move |j| (i, j)))
        .map(|(i, j)| u[i] * v[j] * seed.weights[i][j])
        .sum::<f64>();
    let mut norm2 = 0.;
    for i in 0..3 {
        for j in 0..3 {
            let b = u[i] * v[j] * seed.weights[i][j] / denom;
            norm2 += b * b;
            for k in 0..3 {
                assert_close(
                    q.surface().control_points[i][j][k] - seed.control_points[i][j][k],
                    b * m.points[0].force[k] / 2.,
                    1e-12,
                );
            }
        }
    }
    let f2 = 0.2 * 0.2 + 0.4 * 0.4 + 1.;
    assert_close(
        q.diagnostics().total_potential,
        -0.5 * f2 * norm2 / 2.,
        1e-12,
    );
    assert_close(q.diagnostics().elastic_energy, 0., 1e-15);
    m.points[0].force = [99.; 3];
    seed.control_points[0][0][0] = 99.;
    assert_eq!(q.source(), &before);
    assert_eq!(q.surface().weights, before.weights);
    assert_eq!(q.model().points[0].force, [0.2, -0.4, 1.]);
    let d = q.reverify(1).unwrap();
    assert!(d.free_gradient_residual < 1e-12 && d.numerical_quadrature_only);
}
#[test]
fn clamped_biquadratic_plate_matches_integrated_bending_and_pressure() {
    let seed = plane(2);
    let c = rim(&seed);
    let mut m = Model::default();
    m.material = Material {
        membrane: 3.,
        bending: 2.,
        poisson: 0.25,
    };
    m.pressure = 2.;
    let mut o = Options::default();
    o.fitting.anchor_weight = 0.2;
    let q = variational::solve(&seed, &c, &m, o).unwrap();
    // For b(t)=2t(1-t): integral b^2=2/15, integral b=1/3,
    // integral b'^2=4/3. The full isotropic curvature numerator is 352/45.
    let stiffness = m.material.bending / (1. - 0.25 * 0.25) * 352. / 45.;
    let force = 2. / 9.;
    let z = force / (o.fitting.anchor_weight + stiffness);
    assert_close(q.surface().control_points[1][1][2], z, 1e-12);
    assert_close(
        q.diagnostics().elastic_energy,
        0.5 * stiffness * z * z,
        1e-12,
    );
    assert_close(q.diagnostics().load_work, force * z, 1e-12);
    assert_close(q.diagnostics().total_potential, -0.5 * force * z, 1e-12);
    assert!(q.diagnostics().quadrature_sites == 16);
    q.reverify(1).unwrap();
}
#[test]
fn in_plane_load_matches_symmetric_membrane_strain_including_poisson_shear() {
    let seed = plane(2);
    let c = rim(&seed);
    let mut m = Model::default();
    m.material = Material {
        membrane: 4.,
        bending: 0.,
        poisson: 0.35,
    };
    m.points = vec![PointLoad {
        uv: [0.5, 0.5],
        force: [1., 0., 0.],
    }];
    let mut o = Options::default();
    o.fitting.anchor_weight = 0.3;
    let q = variational::solve(&seed, &c, &m, o).unwrap();
    let nu = m.material.poisson;
    let stiffness = m.material.membrane / (1. - nu * nu) * 4. * (3. - nu) / 45.;
    let x = 0.25 / (o.fitting.anchor_weight + stiffness);
    assert_close(q.surface().control_points[1][1][0] - 0.5, x, 1e-12);
    assert_close(q.surface().control_points[1][1][1], 0.5, 1e-12);
    assert_close(
        q.diagnostics().elastic_energy,
        0.5 * stiffness * x * x,
        1e-12,
    );
}
#[test]
fn curved_rational_reference_annihilates_infinitesimal_rigid_modes() {
    let mut seed = plane(2);
    for i in 0..3 {
        for j in 0..3 {
            seed.control_points[i][j] = vec![
                i as f64 / 2. + if i == 2 { 0.3 } else { 0. },
                j as f64 / 2.,
                if i == 2 { 0.4 } else { 0. } + 0.2 * i as f64 * j as f64 / 4.,
            ];
            seed.weights[i][j] = 1. + 0.1 * i as f64 + 0.07 * j as f64;
        }
    }
    let mut c = Vec::new();
    let omega = [0.07, -0.04, 0.1];
    let t = [0.02, -0.01, 0.03];
    for i in 0..3 {
        for j in 0..3 {
            let p = &seed.control_points[i][j];
            let point = std::array::from_fn(|k| {
                p[k] + t[k] + omega[(k + 1) % 3] * p[(k + 2) % 3]
                    - omega[(k + 2) % 3] * p[(k + 1) % 3]
            });
            c.push(Constraint::ControlPoint {
                index: [i, j],
                point,
            });
        }
    }
    for order in [2, 4, 8] {
        let mut o = Options::default();
        o.quadrature_order = order;
        let q = variational::solve(&seed, &c, &Model::default(), o).unwrap();
        assert!(
            q.diagnostics().elastic_energy.abs() < 1e-12,
            "{}",
            q.diagnostics().elastic_energy
        );
        q.reverify(1).unwrap();
    }
}
fn rotate(p: [f64; 3]) -> [f64; 3] {
    [
        0.36 * p[0] - 0.8 * p[1] + 0.48 * p[2],
        0.48 * p[0] + 0.6 * p[1] + 0.64 * p[2],
        -0.8 * p[0] + 0.6 * p[2],
    ]
}
#[test]
fn oblique_placement_and_independent_parameter_clocks_preserve_equilibrium() {
    let seed = plane(2);
    let mut model = Model::default();
    model.pressure = 0.4;
    model.traction = [0.1, -0.2, 0.3];
    let base = variational::solve(&seed, &rim(&seed), &model, Options::default()).unwrap();
    let mut placed = seed.clone();
    for p in placed.control_points.iter_mut().flatten() {
        let q = rotate([p[0], p[1], p[2]]);
        *p = vec![q[0] + 7., q[1] - 3., q[2] + 4.];
    }
    for k in &mut placed.knots_u {
        *k = -7. + 20. * *k;
    }
    for k in &mut placed.knots_v {
        *k = 2. + 0.125 * *k;
    }
    model.traction = rotate(model.traction);
    let q = variational::solve(&placed, &rim(&placed), &model, Options::default()).unwrap();
    for i in 0..3 {
        for j in 0..3 {
            let p = &base.surface().control_points[i][j];
            let p = rotate([p[0], p[1], p[2]]);
            for k in 0..3 {
                assert_close(
                    q.surface().control_points[i][j][k],
                    p[k] + [7., -3., 4.][k],
                    1e-10,
                );
            }
        }
    }
    assert_close(
        q.diagnostics().elastic_energy,
        base.diagnostics().elastic_energy,
        1e-11,
    );
    assert_close(
        q.diagnostics().total_potential,
        base.diagnostics().total_potential,
        1e-11,
    );
}
#[test]
fn loaded_surface_preserves_complete_original_g2_boundaries() {
    use nurbs_core::surface_join::{Boundary, Decision};
    let mut seed = plane(6);
    seed.weights[3][3] = 1.2;
    let before = seed.clone();
    let mut m = Model::default();
    m.pressure = 0.5;
    let sides = [
        Boundary::UMin,
        Boundary::UMax,
        Boundary::VMin,
        Boundary::VMax,
    ];
    let q = variational::solve_preserving_g2(
        &seed,
        &sides,
        &[],
        &m,
        Options::default(),
        [1e-8, 1e-8, 1e-8],
        10000,
    )
    .unwrap();
    assert!(q.surface().control_points[3][3][2] > 0.);
    for i in 0..7 {
        for j in 0..7 {
            if i != 3 || j != 3 {
                assert_eq!(q.surface().control_points[i][j], seed.control_points[i][j]);
            }
        }
    }
    assert!(
        q.seams()
            .iter()
            .all(|(_, r)| r.decision == Decision::WithinTolerance && r.regularity_proven)
    );
    assert_eq!(seed, before);
    q.reverify(10000).unwrap();
    assert!(q.reverify(1).is_err());
}
#[test]
fn invalid_materials_budgets_singular_metrics_and_conflicting_hard_constraints_refuse() {
    let seed = plane(2);
    let before = seed.clone();
    let m = Model::default();
    for o in [
        Options {
            max_sites: 1,
            ..Options::default()
        },
        Options {
            max_assembly_terms: 1,
            ..Options::default()
        },
        Options {
            quadrature_order: 9,
            ..Options::default()
        },
    ] {
        assert!(variational::solve(&seed, &[], &m, o).is_err());
    }
    let mut bad = m.clone();
    bad.material.poisson = 0.5;
    assert!(variational::solve(&seed, &[], &bad, Options::default()).is_err());
    bad = m.clone();
    bad.points = vec![PointLoad {
        uv: [2., 0.5],
        force: [0., 0., 1.],
    }];
    assert!(variational::solve(&seed, &[], &bad, Options::default()).is_err());
    let mut o = Options::default();
    o.fitting.fairness_weight = 0.1;
    assert!(variational::solve(&seed, &[], &m, o).is_err());
    let c = [
        Constraint::Point {
            uv: [0.5, 0.5],
            point: [0.5, 0.5, 0.],
        },
        Constraint::Point {
            uv: [0.5, 0.5],
            point: [0.5, 0.5, 1.],
        },
    ];
    assert!(variational::solve(&seed, &c, &m, Options::default()).is_err());
    let mut flat = seed.clone();
    for p in flat.control_points.iter_mut().flatten() {
        p[1] = 0.;
    }
    assert!(variational::solve(&flat, &[], &m, Options::default()).is_err());
    assert_eq!(seed, before);
}

#[test]
fn every_original_knot_span_participates_and_unmodeled_crease_energy_refuses() {
    use nurbs_core::{surface::Axis, surface_edit};
    let mut seed = plane(2);
    seed.weights[1][1] = 1.3;
    seed = surface_edit::insert(&seed, Axis::U, 0.4, 1).unwrap();
    seed = surface_edit::insert(&seed, Axis::V, 0.3, 1).unwrap();
    seed = surface_edit::insert(&seed, Axis::V, 0.7, 1).unwrap();
    let q = variational::solve(&seed, &[], &Model::default(), Options::default()).unwrap();
    assert_eq!(q.diagnostics().quadrature_sites, 96);
    assert_eq!(q.surface(), &seed);
    let mut o = Options::default();
    o.max_sites = 95;
    assert!(variational::solve(&seed, &[], &Model::default(), o).is_err());
    let creased = surface_edit::insert(&seed, Axis::U, 0.4, 1).unwrap();
    assert!(variational::solve(&creased, &[], &Model::default(), Options::default()).is_err());
    let mut membrane = Model::default();
    membrane.material.bending = 0.;
    let q = variational::solve(&creased, &[], &membrane, Options::default()).unwrap();
    assert_eq!(q.surface(), &creased);
}
