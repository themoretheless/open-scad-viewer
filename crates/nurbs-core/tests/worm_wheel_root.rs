use nurbs_core::{
    worm,
    worm_wheel::{ChartDomain, Flank, RootPart, Spec, enclose_chart, root_chart},
};

fn source(s: Spec, part: RootPart, t: f64, b: f64) -> ([f64; 3], f64, f64, f64) {
    let w = s.worm;
    let rp = w.axial_module * w.diameter_quotient / 2.;
    let rc = rp + w.axial_module;
    let rw = w.axial_module * s.teeth as f64 / 2.;
    let p = std::f64::consts::PI * w.axial_module;
    let wc = p / 2. - w.backlash - 2. * w.axial_module * w.pressure_angle.tan();
    let hand = if w.hand == worm::Hand::Right { 1. } else { -1. };
    let h = hand * w.axial_module * w.starts as f64 / 2.;
    let eta = -hand * w.starts as f64 / s.teeth as f64;
    let (z, z0, zf) = match part {
        RootPart::CrestCylinder => (0., p - wc / 2. + wc * t, 0.),
        RootPart::CrestEdge(f) => {
            let k = if f == Flank::Descending { -1. } else { 1. } * w.pressure_angle.tan();
            let zf = (rp - rc * b.cos()) / (k * b.cos() - h * b.sin() / rc);
            (
                t * zf,
                if f == Flank::Descending {
                    wc / 2.
                } else {
                    p - wc / 2.
                },
                zf,
            )
        }
    };
    // Independently move the ORIGINAL helical edge/cylinder point first by
    // worm rotation, then into the wheel's moving frame.
    let theta = (z - z0 - s.start as f64 * p - s.axial_shift) / h;
    let phi = b - theta;
    let psi = eta * phi;
    let original = [rc * theta.cos(), rc * theta.sin(), z];
    let housing = [
        phi.cos() * original[0] - phi.sin() * original[1] - (rp + rw),
        phi.sin() * original[0] + phi.cos() * original[1],
        z,
    ];
    (
        [
            psi.cos() * housing[0] - psi.sin() * housing[2],
            housing[1],
            psi.sin() * housing[0] + psi.cos() * housing[2],
        ],
        z,
        zf,
        psi,
    )
}
fn parts() -> [RootPart; 3] {
    [
        RootPart::CrestEdge(Flank::Descending),
        RootPart::CrestEdge(Flank::Ascending),
        RootPart::CrestCylinder,
    ]
}
#[test]
fn original_root_source_boxes_and_jets_match_independent_kinematic_transforms() {
    for hand in [worm::Hand::Right, worm::Hand::Left] {
        let s = Spec {
            worm: worm::Spec {
                hand,
                starts: 2,
                ..worm::Spec::default()
            },
            start: 1,
            axial_shift: 0.3,
            ..Spec::default()
        };
        for part in parts() {
            let owner = root_chart(s, part, [-0.15, 0.15]).unwrap();
            owner.reverify().unwrap();
            let d = ChartDomain {
                u: [0.3, 0.32],
                beta: [0.02, 0.04],
            };
            let r = owner.enclose(d).unwrap();
            assert!(r.regularity_proven);
            assert!(owner.certificate().cross_norm_lower > 0.);
            for t in [0.3, 0.31, 0.32] {
                for b in [0.02, 0.03, 0.04] {
                    let p = source(s, part, t, b).0;
                    for i in 0..3 {
                        assert!(r.point[i][0] - 1e-11 <= p[i] && p[i] <= r.point[i][1] + 1e-11);
                    }
                }
            }
            let h = 1e-5;
            let a = source(s, part, 0.31 - h, 0.03).0;
            let b = source(s, part, 0.31 + h, 0.03).0;
            let c = source(s, part, 0.31, 0.03 - h).0;
            let e = source(s, part, 0.31, 0.03 + h).0;
            for i in 0..3 {
                let dt = (b[i] - a[i]) / (2. * h);
                let db = (e[i] - c[i]) / (2. * h);
                assert!(r.dt[i][0] - 1e-6 <= dt && dt <= r.dt[i][1] + 1e-6);
                assert!(r.dbeta[i][0] - 1e-6 <= db && db <= r.dbeta[i][1] + 1e-6);
            }
            let dt: [f64; 3] = std::array::from_fn(|i| (b[i] - a[i]) / (2. * h));
            let db: [f64; 3] = std::array::from_fn(|i| (e[i] - c[i]) / (2. * h));
            let cross = [
                dt[1] * db[2] - dt[2] * db[1],
                dt[2] * db[0] - dt[0] * db[2],
                dt[0] * db[1] - dt[1] * db[0],
            ];
            let norm = cross.iter().map(|x| x * x).sum::<f64>().sqrt();
            assert!(norm + 1e-6 >= owner.certificate().cross_norm_lower);
        }
    }
}

#[test]
fn convex_edge_normal_velocity_products_have_independent_whole_domain_factors() {
    for hand in [worm::Hand::Right, worm::Hand::Left] {
        for starts in [1, 2] {
            let s = Spec {
                worm: worm::Spec {
                    hand,
                    starts,
                    ..worm::Spec::default()
                },
                ..Spec::default()
            };
            let w = s.worm;
            let rc = w.axial_module * (w.diameter_quotient / 2. + 1.);
            let center = w.axial_module * (w.diameter_quotient + s.teeth as f64) / 2.;
            let sign = if hand == worm::Hand::Right { 1. } else { -1. };
            let h = sign * w.axial_module * starts as f64 / 2.;
            let eta = -sign * starts as f64 / s.teeth as f64;
            for f in [Flank::Descending, Flank::Ascending] {
                let part = RootPart::CrestEdge(f);
                let owner = root_chart(s, part, [-0.15, 0.15]).unwrap();
                let factor = owner.certificate().normal_cone_product_factor.unwrap();
                assert!(factor[1] < 0.);
                let outward = if f == Flank::Descending { 1. } else { -1. };
                let k = -outward * w.pressure_angle.tan();
                for t in [0.25, 0.5, 0.75] {
                    for b in [-0.15, 0., 0.15] {
                        let z = source(s, part, t, b).1;
                        let v = [
                            -rc * b.sin() - eta * z,
                            rc * b.cos(),
                            eta * (rc * b.cos() - center),
                        ];
                        let nf = [
                            h * b.sin() - k * rc * b.cos(),
                            -h * b.cos() - k * rc * b.sin(),
                            rc,
                        ];
                        let cylinder = b.cos() * v[0] + b.sin() * v[1];
                        let flank = outward * (0..3).map(|i| nf[i] * v[i]).sum::<f64>();
                        let actual = cylinder * flank / (t * (1. - t));
                        assert!(
                            actual < 0.
                                && factor[0] - 1e-10 <= actual
                                && actual <= factor[1] + 1e-10
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn root_boundaries_are_original_flank_crest_and_the_next_period_gap() {
    let s = Spec::default();
    let cylinder = root_chart(s, RootPart::CrestCylinder, [-0.15, 0.15]).unwrap();
    for beta in [-0.15, 0., 0.15] {
        for f in [Flank::Descending, Flank::Ascending] {
            let edge = root_chart(s, RootPart::CrestEdge(f), [-0.15, 0.15]).unwrap();
            let a = edge
                .position_bounds(ChartDomain {
                    u: [1., 1.],
                    beta: [beta, beta],
                })
                .unwrap();
            let crest = if f == Flank::Descending { 0. } else { 1. };
            let b = enclose_chart(
                s,
                f,
                ChartDomain {
                    u: [crest, crest],
                    beta: [beta, beta],
                },
            )
            .unwrap()
            .point();
            for i in 0..3 {
                assert!(a[i][0] <= b[i][1] && b[i][0] <= a[i][1]);
            }
        }
        let a = cylinder
            .position_bounds(ChartDomain {
                u: [0., 0.],
                beta: [beta, beta],
            })
            .unwrap();
        let edge = root_chart(s, RootPart::CrestEdge(Flank::Ascending), [-0.15, 0.15]).unwrap();
        let b = edge
            .position_bounds(ChartDomain {
                u: [0., 0.],
                beta: [beta, beta],
            })
            .unwrap();
        for i in 0..3 {
            assert!(a[i][0] <= b[i][1] && b[i][0] <= a[i][1]);
        }
        let p = source(s, RootPart::CrestEdge(Flank::Descending), 0., beta).0;
        let angle = -std::f64::consts::TAU / s.teeth as f64;
        let expected = [
            angle.cos() * p[0] - angle.sin() * p[2],
            p[1],
            angle.sin() * p[0] + angle.cos() * p[2],
        ];
        let actual = source(s, RootPart::CrestCylinder, 1., beta).0;
        for i in 0..3 {
            assert!((actual[i] - expected[i]).abs() < 1e-11);
        }
    }
}

#[test]
fn original_sixth_coefficients_match_independent_affine_trigonometric_series() {
    let s = Spec::default();
    let b = 0.03;
    let t = 0.31;
    let w = s.worm;
    let rp = w.axial_module * w.diameter_quotient / 2.;
    let rc = rp + w.axial_module;
    let rw = w.axial_module * s.teeth as f64 / 2.;
    let wc = std::f64::consts::PI * w.axial_module / 2.
        - w.backlash
        - 2. * w.axial_module * w.pressure_angle.tan();
    for part in parts() {
        let owner = root_chart(s, part, [-0.15, 0.15]).unwrap();
        let r = owner
            .enclose(ChartDomain {
                u: [t, t],
                beta: [b, b],
            })
            .unwrap();
        let (p, _, zf, psi) = source(s, part, t, b);
        let omega = match part {
            RootPart::CrestCylinder => -wc / rw,
            _ => zf / rw,
        };
        let mut expected = [
            -omega.powi(6) * p[0] / 720.,
            0.,
            -omega.powi(6) * p[2] / 720.,
        ];
        if matches!(part, RootPart::CrestEdge(_)) {
            expected[0] -= 6. * omega.powi(5) * zf * psi.cos() / 720.;
            expected[2] -= 6. * omega.powi(5) * zf * psi.sin() / 720.;
        }
        for i in 0..3 {
            assert!(
                r.sixth[0][i][0] - 1e-18 <= expected[i] && expected[i] <= r.sixth[0][i][1] + 1e-18
            );
        }
        let beta_y = -rc * b.sin() / 720.;
        assert!(r.sixth[1][1][0] - 1e-14 <= beta_y && beta_y <= r.sixth[1][1][1] + 1e-14);
    }
}

#[test]
fn unsupported_cones_invalid_specs_and_outside_queries_are_explicit_refusals() {
    let s = Spec::default();
    assert!(root_chart(s, RootPart::CrestEdge(Flank::Descending), [-0.9, 0.9]).is_err());
    assert!(root_chart(s, RootPart::CrestCylinder, [-0.9, 0.9]).is_ok());
    for beta in [[0.2, 0.1], [0., f64::NAN], [-1.5, 0.]] {
        assert!(root_chart(s, RootPart::CrestCylinder, beta).is_err());
    }
    assert!(
        root_chart(
            Spec { teeth: 11, ..s },
            RootPart::CrestCylinder,
            [-0.1, 0.1]
        )
        .is_err()
    );
    let owner = root_chart(s, RootPart::CrestCylinder, [-0.1, 0.1]).unwrap();
    for domain in [
        ChartDomain {
            u: [0.2, 0.1],
            beta: [0., 0.],
        },
        ChartDomain {
            u: [0., 1.],
            beta: [0., 0.2],
        },
        ChartDomain {
            u: [0., f64::NAN],
            beta: [0., 0.],
        },
    ] {
        assert!(owner.position_bounds(domain).is_err());
        assert!(owner.enclose(domain).is_err());
    }
}

#[test]
fn root_nurbs_fits_replay_the_original_source_role_and_actual_control_definitions() {
    use nurbs_core::worm_wheel::{certify_root_grid, fit_root_chart, verify_packed_root_grid};
    for hand in [worm::Hand::Right, worm::Hand::Left] {
        let s = Spec {
            worm: worm::Spec {
                hand,
                ..worm::Spec::default()
            },
            ..Spec::default()
        };
        for part in parts() {
            let root = root_chart(s, part, [-0.15, 0.15]).unwrap();
            let grid = fit_root_chart(&root, 1e-4, 128).unwrap();
            assert_eq!(grid.source().part(), part);
            assert!(grid.certificate().error_upper <= 1e-4);
            assert!(grid.certificate().source_regular && grid.certificate().saved_regular);
            grid.reverify().unwrap();
            verify_packed_root_grid(&grid, grid.packed_surfaces()).unwrap();
            let mut changed_packing = grid.packed_surfaces().to_vec();
            assert_eq!(changed_packing[0].knots_u[0].to_bits(), 0f64.to_bits());
            changed_packing[0].knots_u[0] = -0.;
            assert!(verify_packed_root_grid(&grid, &changed_packing).is_err());

            eprintln!(
                "root {hand:?}/{part:?}: cells={} packed={} error_upper={}",
                grid.surfaces().len(),
                grid.packed_surfaces().len(),
                grid.certificate().error_upper
            );
            for patch in grid.surfaces() {
                for t in [
                    patch.knots_u[5],
                    (patch.knots_u[5] + patch.knots_u[6]) / 2.,
                    patch.knots_u[6],
                ] {
                    for b in [
                        patch.knots_v[5],
                        (patch.knots_v[5] + patch.knots_v[6]) / 2.,
                        patch.knots_v[6],
                    ] {
                        let p = patch.evaluate(t, b).unwrap().point;
                        let q = source(s, part, t, b).0;
                        let error = (0..3).map(|i| (p[i] - q[i]).powi(2)).sum::<f64>().sqrt();
                        assert!(error <= grid.certificate().error_upper + 1e-10);
                    }
                }
            }
            let mut changed = grid.surfaces().to_vec();
            changed[0].control_points[2][2][0] += 0.01;
            assert!(
                certify_root_grid(
                    &root,
                    grid.domain(),
                    grid.u_breaks(),
                    grid.beta_breaks(),
                    &changed,
                    128
                )
                .unwrap()
                .control_difference_upper
                    > 0.009
            );
            changed = grid.surfaces().to_vec();
            changed[0].weights[0][0] = 2.;
            assert!(
                certify_root_grid(
                    &root,
                    grid.domain(),
                    grid.u_breaks(),
                    grid.beta_breaks(),
                    &changed,
                    128
                )
                .is_err()
            );
            let shifted = root_chart(
                Spec {
                    axial_shift: 0.3,
                    ..s
                },
                part,
                [-0.15, 0.15],
            )
            .unwrap();
            assert!(
                certify_root_grid(
                    &shifted,
                    grid.domain(),
                    grid.u_breaks(),
                    grid.beta_breaks(),
                    grid.surfaces(),
                    128
                )
                .unwrap()
                .error_upper
                    > 0.01
            );
            assert!(fit_root_chart(&root, 1e-15, 1).is_err());
        }
    }
}

#[test]
fn original_embedding_has_independent_axial_radius_and_short_arc_invariants() {
    use nurbs_core::worm_wheel::RootEmbedding;
    let s = Spec::default();
    for part in parts() {
        let root = root_chart(s, part, [-0.15, 0.15]).unwrap();
        for b in [-0.15, 0., 0.15] {
            match &root.certificate().source_embedding {
                RootEmbedding::AxialAndStrictRadius => {
                    assert!(matches!(part, RootPart::CrestEdge(_)));
                    let mut previous = None;
                    for i in 0..=20 {
                        let p = source(s, part, i as f64 / 20., b).0;
                        let radius = p[0].hypot(p[2]);
                        if let Some(r) = previous {
                            assert!(radius > r);
                        }
                        previous = Some(radius);
                    }
                }
                RootEmbedding::AxialAndShortArc { angular_span } => {
                    assert_eq!(part, RootPart::CrestCylinder);
                    let angle = (source(s, part, 1., b).3 - source(s, part, 0., b).3).abs();
                    assert!(angular_span[0] - 1e-14 <= angle && angle <= angular_span[1] + 1e-14);
                    assert!(angular_span[0] > 0. && angular_span[1] < std::f64::consts::TAU);
                }
            }
        }
        let mut previous = None;
        for i in 0..=20 {
            let beta = -0.15 + 0.3 * i as f64 / 20.;
            let y = source(s, part, 0.4, beta).0[1];
            if let Some(p) = previous {
                assert!(y > p);
            }
            previous = Some(y);
        }
    }
}
