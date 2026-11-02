use nurbs_core::{
    constrained_surface::{
        Constraint,
        network::{self, BoundaryLocks},
        rational,
        unparameterized::{self, Continuity, Guide, Method, Options},
    },
    curve::Curve,
    patches,
    surface::{Axis, Surface},
    surface_join::Boundary,
};
fn warped_plane(degree: usize) -> Surface {
    patches::bezier(
        (0..=degree)
            .map(|i| {
                (0..=degree)
                    .map(|j| {
                        let u = i as f64 / degree as f64;
                        let v = j as f64 / degree as f64;
                        let uu = if i < 2 {
                            0.
                        } else {
                            (i * (i - 1)) as f64 / (degree * (degree - 1)) as f64
                        };
                        let vv = if j < 2 {
                            0.
                        } else {
                            (j * (j - 1)) as f64 / (degree * (degree - 1)) as f64
                        };
                        vec![u + 0.2 * uu, v + 0.1 * vv, 0.]
                    })
                    .collect()
            })
            .collect(),
        None,
    )
    .unwrap()
}
fn guide(c: &Curve) -> Guide<'_> {
    Guide {
        spatial: c,
        continuity: Continuity::Position,
    }
}
fn compare(out: &unparameterized::Qualified) {
    assert!(out.work().0 <= 100000 && out.work().1 <= 1000000);
    assert!(out.reverify(100000, 1000000).unwrap().admitted);
    for g in out.network().original_guides() {
        for i in 0..=40 {
            let t = i as f64 / 40.;
            let d = g.parameters.domain();
            let uv = g
                .parameters
                .evaluate(d[0] + t * (d[1] - d[0]))
                .unwrap()
                .point;
            let p = out.surface().evaluate(uv[0], uv[1]).unwrap().point;
            let d = g.spatial.domain();
            let q = g.spatial.evaluate(d[0] + t * (d[1] - d[0])).unwrap().point;
            assert!(p.iter().zip(q).all(|(a, b)| (a - b).abs() < 1e-6));
        }
    }
}
#[test]
fn unknown_uvs_fit_a_displaced_curve_on_a_nonlinear_chart() {
    let seed = warped_plane(2);
    let mut target = seed.clone();
    for (i, row) in target.control_points.iter_mut().enumerate() {
        for p in row {
            p[2] = [0.1, 0.1, 0.4][i];
        }
    }
    let c = target.iso(Axis::V, 0.4).unwrap();
    let before = (seed.clone(), c.clone());
    let out = unparameterized::solve(&seed, &[guide(&c)], &[], None, Options::default()).unwrap();
    assert!(out.inversions() > 0);
    assert_eq!(out.original(), &seed);
    assert!(out.surface().evaluate(0.5, 0.4).unwrap().point[2] > 0.15);
    compare(&out);
    let p = out.network().original_guides()[0].parameters.clone();
    for i in 0..=10 {
        let t = i as f64 / 10.;
        let d = p.domain();
        let uv = p.evaluate(d[0] + t * (d[1] - d[0])).unwrap().point;
        assert!((uv[0] - t).abs() < 1e-7);
        assert!((uv[1] - 0.4).abs() < 1e-7);
    }
    assert_eq!((seed, c), before);
}
#[test]
fn inferred_clocks_survive_oblique_placement_reversal_and_nonunit_domains() {
    let mut seed = warped_plane(2);
    for t in &mut seed.knots_u {
        *t = -2. + 5. * *t;
    }
    for t in &mut seed.knots_v {
        *t = 5. + 4. * *t;
    }
    let mut target = seed.clone();
    for (i, row) in target.control_points.iter_mut().enumerate() {
        for p in row {
            p[2] = [0.05, 0.05, 0.2][i];
        }
    }
    // Orthogonal world placement: (x,y,z) -> (z+7, .6x-.8y-3, .8x+.6y+2).
    for s in [&mut seed, &mut target] {
        for p in s.control_points.iter_mut().flatten() {
            let q = p.clone();
            *p = vec![
                q[2] + 7.,
                0.6 * q[0] - 0.8 * q[1] - 3.,
                0.8 * q[0] + 0.6 * q[1] + 2.,
            ];
        }
    }
    let mut c = target.iso(Axis::V, 6.6).unwrap().reverse().unwrap();
    let d = c.domain();
    for t in &mut c.knots {
        *t = 7. + 4. * (*t - d[0]) / (d[1] - d[0]);
    }
    let out = unparameterized::solve(&seed, &[guide(&c)], &[], None, Options::default()).unwrap();
    compare(&out);
    let p = out.network().original_guides()[0].parameters.clone();
    assert_eq!(p.domain(), [7., 11.]);
    assert!((p.evaluate(7.).unwrap().point[0] - 3.).abs() < 1e-7);
    assert!((p.evaluate(11.).unwrap().point[0] + 2.).abs() < 1e-7);
}
#[test]
fn tangent_and_curvature_support_uvs_are_inferred_from_the_originals() {
    let seed = warped_plane(3);
    let mut support = seed.clone();
    for row in &mut support.control_points {
        for (j, p) in row.iter_mut().enumerate() {
            p[2] = 0.08 * [0.25, -1. / 12., -1. / 12., 0.25][j];
        }
    }
    let c = support.iso(Axis::V, 0.5).unwrap();
    for continuity in [
        Continuity::Tangent(&support),
        Continuity::Curvature(&support),
    ] {
        let options = Options::default();
        let out = unparameterized::solve(
            &seed,
            &[Guide {
                spatial: &c,
                continuity,
            }],
            &[],
            None,
            options,
        )
        .unwrap_or_else(|e| {
            panic!(
                "curvature {}: {e}",
                matches!(continuity, Continuity::Curvature(_))
            )
        });
        compare(&out);
        assert!(out.network().original_guides()[0].tangent.is_some());
        if matches!(continuity, Continuity::Curvature(_)) {
            assert_eq!(out.network().original_curvature_guides().len(), 1);
            let (_, _, vv) = out
                .surface()
                .evaluate(0.5, 0.5)
                .unwrap()
                .second_derivatives()
                .unwrap();
            assert!(
                vv[2] > 0.01,
                "flat source must acquire actual transverse curvature"
            );
        }
    }
}

#[test]
fn restored_exact_controls_cannot_borrow_a_pre_restoration_jet_receipt() {
    let seed = warped_plane(2);
    let mut hard = seed
        .control_points
        .iter()
        .enumerate()
        .flat_map(|(i, row)| {
            row.iter()
                .enumerate()
                .map(move |(j, p)| Constraint::ControlPoint {
                    index: [i, j],
                    point: [p[0], p[1], p[2]],
                })
        })
        .collect::<Vec<_>>();
    hard.push(Constraint::Derivative {
        uv: [0.5, 0.5],
        axis: nurbs_core::constrained_surface::Axis::U,
        value: [1.2, 0., 0.01],
    });
    let mut options = nurbs_core::constrained_surface::Options::default();
    options.position_tolerance = 0.1;
    assert!(nurbs_core::constrained_surface::solve(&seed, &hard, options).is_err());
}

#[test]
fn support_inversion_handles_an_independent_transposed_opposite_orientation_chart() {
    let seed = warped_plane(2);
    let mut support = seed.clone();
    support.control_points = (0..3)
        .map(|i| (0..3).map(|j| seed.control_points[j][i].clone()).collect())
        .collect();
    for t in &mut support.knots_u {
        *t = 10. + 10. * *t;
    }
    for t in &mut support.knots_v {
        *t = -3. + 4. * *t;
    }
    let c = seed.iso(Axis::V, 0.4).unwrap();
    let before = (support.clone(), c.clone());
    for continuity in [
        Continuity::Tangent(&support),
        Continuity::Curvature(&support),
    ] {
        let out = unparameterized::solve(
            &seed,
            &[Guide {
                spatial: &c,
                continuity,
            }],
            &[],
            None,
            Options::default(),
        )
        .unwrap();
        compare(&out);
        let guides = out.network().original_guides();
        let sp = guides[0].tangent.unwrap().parameters;
        let d = sp.domain();
        let a = sp.evaluate(d[0]).unwrap().point;
        let b = sp.evaluate(d[1]).unwrap().point;
        assert!((a[0] - 14.).abs() < 1e-7 && (b[0] - 14.).abs() < 1e-7);
        assert!((a[1] + 3.).abs() < 1e-7 && (b[1] - 1.).abs() < 1e-7);
    }
    assert_eq!((support, c), before);
}
#[test]
fn unknown_uvs_combine_with_positive_weights_and_actual_hard_derivatives() {
    let seed = warped_plane(2);
    let mut c = seed.iso(Axis::V, 0.5).unwrap();
    for p in &mut c.control_points {
        p[2] = 0.2;
    }
    let hard = [Constraint::Derivative {
        uv: [0.5, 0.5],
        axis: nurbs_core::constrained_surface::Axis::U,
        value: [1.2, 0., 0.],
    }];
    let mut o = Options::default();
    let mut positive = rational::Options::default();
    positive.weight_bounds = [0.8, 1.2];
    o.method = Method::PositiveWeights(positive);
    let out = unparameterized::solve(&seed, &[guide(&c)], &hard, None, o).unwrap();
    assert!(out.rational().is_some());
    assert!(
        out.surface()
            .weights
            .iter()
            .flatten()
            .all(|w| *w >= 0.8 && *w <= 1.2)
    );
    let (u, _) = out
        .surface()
        .evaluate(0.5, 0.5)
        .unwrap()
        .first_derivatives()
        .unwrap();
    assert!((u[0] - 1.2).abs() < 1e-8 && u[2].abs() < 1e-8);
    compare(&out);
}
#[test]
fn inferred_interior_guide_preserves_four_original_g2_boundaries() {
    let seed = warped_plane(6);
    let mut target = seed.clone();
    target.control_points[3][3][2] = 0.3;
    let c = target.iso(Axis::V, 0.5).unwrap();
    let sides = [
        Boundary::UMin,
        Boundary::UMax,
        Boundary::VMin,
        Boundary::VMax,
    ];
    let lock = BoundaryLocks {
        reference: &seed,
        boundaries: &sides,
        tolerances: [1e-7, 1e-6, 1e-6],
    };
    let out =
        unparameterized::solve(&seed, &[guide(&c)], &[], Some(lock), Options::default()).unwrap();
    assert_eq!(
        out.reverify(100000, 1000000).unwrap().boundary_seams.len(),
        4
    );
    compare(&out);
}
#[test]
fn between_site_excursions_and_exhaustion_never_publish_sampled_inverse_authority() {
    let seed = warped_plane(2);
    let c = Curve {
        degree: 4,
        knots: vec![0., 0., 0., 0., 0., 1., 1., 1., 1., 1.],
        control_points: (0..5)
            .map(|i| vec![i as f64 / 4., 0.5, [0., 0.125, -1. / 6., 0.125, 0.][i]])
            .collect(),
        weights: vec![1.; 5],
        periodic: false,
    };
    let hard = seed
        .control_points
        .iter()
        .enumerate()
        .flat_map(|(i, row)| {
            row.iter()
                .enumerate()
                .map(move |(j, p)| Constraint::ControlPoint {
                    index: [i, j],
                    point: [p[0], p[1], p[2]],
                })
        })
        .collect::<Vec<_>>();
    let before = seed.clone();
    let mut o = Options::default();
    o.max_rounds = 2;
    assert!(unparameterized::solve(&seed, &[guide(&c)], &hard, None, o).is_err());
    o = Options::default();
    let mut scarce = network::CurvatureOptions::default();
    scarce.network.max_cells = 1;
    o.method = Method::Fixed(scarce);
    assert!(unparameterized::solve(&seed, &[guide(&c)], &[], None, o).is_err());
    assert_eq!(seed, before);
    o = Options::default();
    o.inversion_iterations = 0;
    assert!(unparameterized::solve(&seed, &[guide(&c)], &[], None, o).is_err());
}

#[test]
fn a_nonrational_inverse_refines_the_uv_basis_without_changing_the_locked_surface() {
    let mut seed = warped_plane(2);
    for (i, row) in seed.control_points.iter_mut().enumerate() {
        for (j, p) in row.iter_mut().enumerate() {
            // x=(.5+.5u)^2: inverse of a linear world clock contains a square root.
            p[0] = [0.25, 0.5, 1.][i];
            p[1] = j as f64 / 2.;
        }
    }
    let hard = seed
        .control_points
        .iter()
        .enumerate()
        .flat_map(|(i, row)| {
            row.iter()
                .enumerate()
                .map(move |(j, p)| Constraint::ControlPoint {
                    index: [i, j],
                    point: [p[0], p[1], p[2]],
                })
        })
        .collect::<Vec<_>>();
    let c = Curve::from_polyline(vec![vec![0.25, 0.5, 0.], vec![1., 0.5, 0.]]).unwrap();
    let mut config = network::CurvatureOptions::default();
    config.network.fitting.position_tolerance = 1e-3;
    let mut o = Options::default();
    o.method = Method::Fixed(config);
    let out = unparameterized::solve(&seed, &[guide(&c)], &hard, None, o).unwrap();
    assert!(out.rounds() > 1);
    let supported = unparameterized::solve(
        &seed,
        &[Guide {
            spatial: &c,
            continuity: Continuity::Curvature(&seed),
        }],
        &hard,
        None,
        o,
    )
    .unwrap();
    assert!(supported.rounds() > 1);
    assert!(supported.reverify(100000, 1000000).unwrap().admitted);
    let supported_guides = supported.network().original_guides();
    assert!(
        supported_guides[0]
            .tangent
            .unwrap()
            .parameters
            .control_points
            .len()
            > 2
    );
    let p = out.network().original_guides()[0].parameters.clone();
    assert!(p.control_points.len() > c.control_points.len());
    assert!(out.reverify(100000, 1000000).unwrap().admitted);
    for (p, q) in out
        .surface()
        .control_points
        .iter()
        .flatten()
        .zip(seed.control_points.iter().flatten())
    {
        assert!(p.iter().zip(q).all(|(a, b)| (a - b).abs() < 1e-8));
    }
    for i in 0..=300 {
        let t = i as f64 / 300.;
        let uv = p.evaluate(t).unwrap().point;
        let a = out.surface().evaluate(uv[0], uv[1]).unwrap().point;
        let b = c.evaluate(t).unwrap().point;
        assert!(a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-3));
    }
}
