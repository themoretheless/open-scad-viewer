use nurbs_core::{
    worm,
    worm_wheel::{ChartDomain, Flank, Spec, intersect_stock_cylinder},
};

// Independent original source equation, before wheel-frame rotation.
fn radial(s: Spec, f: Flank, u: f64, beta: f64) -> f64 {
    let w = s.worm;
    let rp = w.axial_module * w.diameter_quotient / 2.;
    let rc = rp + w.axial_module;
    let rr = rp - w.axial_module * w.dedendum_factor;
    let (r, k) = match f {
        Flank::Descending => (rc + (rr - rc) * u, -w.pressure_angle.tan()),
        Flank::Ascending => (rr + (rc - rr) * u, w.pressure_angle.tan()),
    };
    let h =
        if w.hand == worm::Hand::Right { 1. } else { -1. } * w.axial_module * w.starts as f64 / 2.;
    let z = (rp - r * beta.cos()) / (k * beta.cos() - h * beta.sin() / r);
    let x = r * beta.cos() - rp - w.axial_module * s.teeth as f64 / 2.;
    x.hypot(z)
}

#[test]
fn stock_crossing_contains_independent_roots_for_both_flanks_and_hands() {
    for hand in [worm::Hand::Right, worm::Hand::Left] {
        for f in [Flank::Descending, Flank::Ascending] {
            let s = Spec {
                worm: worm::Spec {
                    hand,
                    ..worm::Spec::default()
                },
                ..Spec::default()
            };
            let domain = ChartDomain {
                u: [0., 1.],
                beta: [-0.15, 0.15],
            };
            let proof = intersect_stock_cylinder(s, f, domain, 21., 128).unwrap();
            proof.reverify().unwrap();
            let b = proof.u_bracket();
            assert!(b[1] - b[0] < 0.2);
            for beta in [-0.15, 0., 0.15] {
                let lower_is_inside = radial(s, f, b[0], beta) < 21.;
                let higher_is_inside = radial(s, f, b[1], beta) < 21.;
                use nurbs_core::worm_wheel::StockInterior;
                match proof.stock_interior() {
                    StockInterior::LowerU => assert!(lower_is_inside && !higher_is_inside),
                    StockInterior::HigherU => assert!(!lower_is_inside && higher_is_inside),
                }
            }

            for i in 0..=100 {
                let beta = -0.15 + 0.30 * i as f64 / 100.;
                let mut range = [0., 1.];
                for _ in 0..52 {
                    let u = (range[0] + range[1]) / 2.;
                    if (radial(s, f, u, beta) < 21.) == proof.radius_increases_with_u() {
                        range[0] = u;
                    } else {
                        range[1] = u;
                    }
                }
                assert!(b[0] <= range[0] && range[1] <= b[1]);
            }
        }
    }
}

#[test]
fn stock_refuses_absent_crossings_bad_domains_and_insufficient_proof() {
    let s = Spec::default();
    let d = ChartDomain {
        u: [0., 1.],
        beta: [-0.04, 0.04],
    };
    for radius in [f64::NAN, 0., -1., 18., 25.] {
        assert!(intersect_stock_cylinder(s, Flank::Descending, d, radius, 128).is_err());
    }
    assert!(intersect_stock_cylinder(s, Flank::Descending, d, 21., 0).is_err());
    assert!(
        intersect_stock_cylinder(Spec { teeth: 12, ..s }, Flank::Descending, d, 7., 128).is_err()
    );
    assert!(
        intersect_stock_cylinder(
            s,
            Flank::Descending,
            ChartDomain { u: [0.5, 0.5], ..d },
            21.,
            128
        )
        .is_err()
    );
}

fn independent_edge(s: Spec, f: Flank, beta: f64) -> [f64; 3] {
    independent_chart(s, f, 1., beta)
}
fn independent_chart(s: Spec, f: Flank, t: f64, beta: f64) -> [f64; 3] {
    let mut range = [0., 1.];
    let increasing = f == Flank::Descending;
    for _ in 0..52 {
        let u = (range[0] + range[1]) / 2.;
        if (radial(s, f, u, beta) < 21.) == increasing {
            range[0] = u;
        } else {
            range[1] = u;
        }
    }
    let root = (range[0] + range[1]) / 2.;
    let crest = if f == Flank::Descending { 0. } else { 1. };
    let u = crest + (root - crest) * t;
    let w = s.worm;
    let rp = w.axial_module * w.diameter_quotient / 2.;
    let rc = rp + w.axial_module;
    let rr = rp - w.axial_module * w.dedendum_factor;
    let p = std::f64::consts::PI * w.axial_module;
    let wc = p / 2. - w.backlash - 2. * w.axial_module * w.pressure_angle.tan();
    let wr = p / 2. - w.backlash + 2. * w.axial_module * w.dedendum_factor * w.pressure_angle.tan();
    let (r, k, z0) = match f {
        Flank::Descending => (
            rc + (rr - rc) * u,
            -w.pressure_angle.tan(),
            wc / 2. + (wr - wc) * u / 2.,
        ),
        Flank::Ascending => (
            rr + (rc - rr) * u,
            w.pressure_angle.tan(),
            p - wr / 2. + (wr - wc) * u / 2.,
        ),
    };
    let sign = if w.hand == worm::Hand::Right { 1. } else { -1. };
    let h = sign * w.axial_module * w.starts as f64 / 2.;
    let z = (rp - r * beta.cos()) / (k * beta.cos() - h * beta.sin() / r);
    let theta = (z - z0 - s.start as f64 * p - s.axial_shift) / h;
    let psi = -sign * w.starts as f64 / s.teeth as f64 * (beta - theta);
    let x = r * beta.cos() - rp - w.axial_module * s.teeth as f64 / 2.;
    [
        psi.cos() * x - psi.sin() * z,
        r * beta.sin(),
        psi.sin() * x + psi.cos() * z,
    ]
}

#[test]
fn restricted_stock_edge_positions_and_implicit_tangents_match_independent_source() {
    for f in [Flank::Descending, Flank::Ascending] {
        let s = Spec::default();
        let proof = intersect_stock_cylinder(
            s,
            f,
            ChartDomain {
                u: [0., 1.],
                beta: [-0.04, 0.04],
            },
            21.,
            64,
        )
        .unwrap();
        let tangents = proof.tangent_bounds().unwrap();
        for beta in [-0.03, 0., 0.03] {
            let section = proof.section(beta).unwrap();
            assert!(section.u_bracket()[1] - section.u_bracket()[0] < 1e-10);
            let bounds = section.position_bounds().unwrap();
            let expected = independent_edge(s, f, beta);
            let h = 1e-5;
            let plus = independent_edge(s, f, beta + h);
            let minus = independent_edge(s, f, beta - h);
            for i in 0..3 {
                assert!(bounds[i][0] - 1e-12 <= expected[i] && expected[i] <= bounds[i][1] + 1e-12);
                let derivative = (plus[i] - minus[i]) / (2. * h);
                assert!(tangents[i][0] - 1e-6 <= derivative && derivative <= tangents[i][1] + 1e-6);
            }
        }
        assert!(proof.section(0.05).is_err());
        assert!(proof.section(f64::NAN).is_err());
    }
}

#[test]
fn true_stock_trimmed_chart_bounds_include_independent_positions_and_jets() {
    for f in [Flank::Descending, Flank::Ascending] {
        let s = Spec::default();
        let proof = intersect_stock_cylinder(
            s,
            f,
            ChartDomain {
                u: [0., 1.],
                beta: [-0.01, 0.01],
            },
            21.,
            64,
        )
        .unwrap();
        let bounds = proof.enclose_trimmed_chart([0.25, 0.26]).unwrap();
        assert!(bounds.regularity_proven);
        assert!(
            bounds
                .sixth
                .iter()
                .flatten()
                .flatten()
                .all(|x| x.is_finite())
        );
        for t in [0.25, 0.255, 0.26] {
            for beta in [-0.01, 0., 0.01] {
                let p = independent_chart(s, f, t, beta);
                for i in 0..3 {
                    assert!(
                        bounds.point[i][0] - 1e-12 <= p[i] && p[i] <= bounds.point[i][1] + 1e-12
                    );
                }
            }
        }
        let h = 1e-5;
        let a = independent_chart(s, f, 0.255 - h, 0.);
        let b = independent_chart(s, f, 0.255 + h, 0.);
        let c = independent_chart(s, f, 0.255, -h);
        let d = independent_chart(s, f, 0.255, h);
        for i in 0..3 {
            let dt = (b[i] - a[i]) / (2. * h);
            let db = (d[i] - c[i]) / (2. * h);
            assert!(bounds.dt[i][0] - 1e-6 <= dt && dt <= bounds.dt[i][1] + 1e-6);
            assert!(bounds.dbeta[i][0] - 1e-6 <= db && db <= bounds.dbeta[i][1] + 1e-6);
        }
        for bad in [[-0.1, 0.1], [0.9, 0.8], [0., f64::NAN]] {
            assert!(proof.enclose_trimmed_chart(bad).is_err());
        }
    }
}

#[test]
fn stock_nurbs_fit_keeps_original_implicit_source_and_saved_control_receipts() {
    use nurbs_core::worm_wheel::{certify_stock_grid, fit_stock_chart, verify_packed_grid};
    for f in [Flank::Descending, Flank::Ascending] {
        let s = Spec::default();
        let domain = ChartDomain {
            u: [0., 1.],
            beta: [-0.01, 0.01],
        };
        let stock = intersect_stock_cylinder(s, f, domain, 21., 64).unwrap();
        let grid = fit_stock_chart(&stock, 1e-3, 128).unwrap();
        eprintln!(
            "stock fit {f:?}: cells={} packed={} error_upper={}",
            grid.surfaces().len(),
            grid.packed_surfaces().len(),
            grid.certificate().error_upper
        );
        assert!(grid.stock_source().is_some());
        assert!(grid.certificate().error_upper <= 1e-3);
        assert!(grid.certificate().source_regular && grid.certificate().saved_regular);
        grid.reverify().unwrap();
        verify_packed_grid(&grid, grid.packed_surfaces()).unwrap();
        for patch in grid.surfaces() {
            let u = (
                patch.knots_u[patch.degree_u],
                patch.knots_u[patch.control_points.len()],
            );
            let b = (
                patch.knots_v[patch.degree_v],
                patch.knots_v[patch.control_points[0].len()],
            );
            for t in [u.0, (u.0 + u.1) / 2., u.1] {
                for beta in [b.0, (b.0 + b.1) / 2., b.1] {
                    let p = patch.evaluate(t, beta).unwrap().point;
                    let q = independent_chart(s, f, t, beta);
                    let distance = (0..3).map(|i| (p[i] - q[i]).powi(2)).sum::<f64>().sqrt();
                    assert!(distance <= grid.certificate().error_upper + 1e-10);
                }
            }
        }
        let mut changed = grid.surfaces().to_vec();
        changed[0].control_points[2][2][0] += 0.01;
        let replay = certify_stock_grid(
            &stock,
            grid.domain(),
            grid.u_breaks(),
            grid.beta_breaks(),
            &changed,
            128,
        )
        .unwrap();
        assert!(replay.control_difference_upper > 0.009);
        let other = intersect_stock_cylinder(s, f, domain, 20.8, 64).unwrap();
        assert!(
            certify_stock_grid(
                &other,
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
        assert!(fit_stock_chart(&stock, 1e-15, 1).is_err());
    }
}

#[test]
fn wide_left_multistart_stock_fit_preserves_the_original_contact_phase() {
    use nurbs_core::worm_wheel::fit_stock_chart;
    let s = Spec {
        worm: worm::Spec {
            hand: worm::Hand::Left,
            starts: 2,
            ..worm::Spec::default()
        },
        start: 1,
        ..Spec::default()
    };
    let stock = intersect_stock_cylinder(
        s,
        Flank::Ascending,
        ChartDomain {
            u: [0., 1.],
            beta: [-0.15, 0.15],
        },
        21.,
        128,
    )
    .unwrap();
    let grid = fit_stock_chart(&stock, 1e-4, 128).unwrap();
    grid.reverify().unwrap();
    eprintln!(
        "wide stock: cells={} packed={} error_upper={}",
        grid.surfaces().len(),
        grid.packed_surfaces().len(),
        grid.certificate().error_upper
    );
    for patch in grid.surfaces() {
        let t = (patch.knots_u[5] + patch.knots_u[6]) / 2.;
        let beta = (patch.knots_v[5] + patch.knots_v[6]) / 2.;
        let p = patch.evaluate(t, beta).unwrap().point;
        let q = independent_chart(s, Flank::Ascending, t, beta);
        let error = (0..3).map(|i| (p[i] - q[i]).powi(2)).sum::<f64>().sqrt();
        assert!(error <= grid.certificate().error_upper + 1e-10);
    }
    assert!(grid.certificate().error_upper <= 1e-4);
    assert!(stock.restrict_beta([-0.2, 0.]).is_err());
}
