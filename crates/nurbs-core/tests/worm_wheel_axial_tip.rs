use nurbs_core::worm_wheel::{
    AxialDomain, Flank, Spec, axial_stock_intersection, axial_tip_chart, certify_axial_tip_grid,
    fit_axial_tip, verify_packed_axial_tip,
};

fn flank(s: Spec, f: Flank, u: f64, y: f64) -> [f64; 3] {
    let w = s.worm;
    let rp = w.axial_module * w.diameter_quotient / 2.;
    let rc = rp + w.axial_module;
    let rr = rp - w.axial_module * w.dedendum_factor;
    let p = std::f64::consts::PI * w.axial_module;
    let wc = p / 2. - w.backlash - 2. * w.axial_module * w.pressure_angle.tan();
    let wr = p / 2. - w.backlash + 2. * w.axial_module * w.dedendum_factor * w.pressure_angle.tan();
    let (r, z0, k) = match f {
        Flank::Descending => (
            rc + (rr - rc) * u,
            wc / 2. + (wr - wc) * u / 2.,
            -w.pressure_angle.tan(),
        ),
        Flank::Ascending => (
            rr + (rc - rr) * u,
            p - wr / 2. + (wr - wc) * u / 2.,
            w.pressure_angle.tan(),
        ),
    };
    let b = (y / r).asin();
    let hand = if w.hand == nurbs_core::worm::Hand::Right {
        1.
    } else {
        -1.
    };
    let h = hand * w.axial_module * w.starts as f64 / 2.;
    let eta = -hand * w.starts as f64 / s.teeth as f64;
    let z = (rp - r * b.cos()) / (k * b.cos() - h * b.sin() / r);
    let theta = (z - z0 - s.start as f64 * p - s.axial_shift) / h;
    let psi = eta * (b - theta);
    let q = r * b.cos() - rp - w.axial_module * s.teeth as f64 / 2.;
    [
        psi.cos() * q - psi.sin() * z,
        y,
        psi.sin() * q + psi.cos() * z,
    ]
}
fn endpoint(s: Spec, f: Flank, y: f64) -> [f64; 3] {
    let mut d = [0., 1.];
    for _ in 0..54 {
        let u = (d[0] + d[1]) / 2.;
        let p = flank(s, f, u, y);
        if (p[0].hypot(p[2]) < 21.) == (f == Flank::Descending) {
            d[0] = u;
        } else {
            d[1] = u;
        }
    }
    flank(s, f, (d[0] + d[1]) / 2., y)
}
fn oracle(s: Spec, t: f64, y: f64) -> [f64; 3] {
    let a = endpoint(s, Flank::Descending, y);
    let b = endpoint(s, Flank::Ascending, y);
    let start = a[2].atan2(a[0]);
    let mut delta = (b[2].atan2(b[0]) - start).rem_euclid(std::f64::consts::TAU);
    if delta > std::f64::consts::PI {
        delta -= std::f64::consts::TAU;
    }
    // Independent short-arc angle formulation, rather than vector normalization.
    let theta = start + (t * delta.sin()).atan2(1. - t + t * delta.cos());
    [21. * theta.cos(), y, 21. * theta.sin()]
}

#[test]
fn original_stock_tip_has_short_arc_contact_exact_radius_and_source_jets() {
    let s = Spec::default();
    let a = axial_stock_intersection(s, Flank::Descending, [-2., 2.], 21., 128).unwrap();
    let b = axial_stock_intersection(s, Flank::Ascending, [-2., 2.], 21., 128).unwrap();
    let tip = axial_tip_chart(&a, &b, 32).unwrap();
    tip.reverify().unwrap();
    assert!(
        tip.cross_product()[1] < 0. && tip.dot_product()[0] > 0. && tip.cross_norm_lower() > 0.
    );
    let d = AxialDomain {
        t: [0.3, 0.32],
        y: [0.3, 0.32],
    };
    let r = tip.enclose(d).unwrap();
    assert!(r.regularity_proven);
    assert_eq!(r.point[1], d.y);
    assert_eq!(r.dt[1], [0., 0.]);
    assert_eq!(r.dy[1], [1., 1.]);
    for t in [0.3, 0.31, 0.32] {
        for y in [0.3, 0.31, 0.32] {
            let p = oracle(s, t, y);
            for i in 0..3 {
                assert!(r.point[i][0] - 1e-10 <= p[i] && p[i] <= r.point[i][1] + 1e-10);
            }
            assert!((p[0].hypot(p[2]) - 21.).abs() < 1e-12);
        }
    }
    let h = 1e-5;
    let p = oracle(s, 0.31 - h, 0.31);
    let q = oracle(s, 0.31 + h, 0.31);
    let x = oracle(s, 0.31, 0.31 - h);
    let z = oracle(s, 0.31, 0.31 + h);
    for i in 0..3 {
        let dt = (q[i] - p[i]) / (2. * h);
        let dy = (z[i] - x[i]) / (2. * h);
        assert!(r.dt[i][0] - 1e-6 <= dt && dt <= r.dt[i][1] + 1e-6);
        assert!(r.dy[i][0] - 1e-6 <= dy && dy <= r.dy[i][1] + 1e-6);
    }
    for y in [-2., 0., 2.] {
        let p = tip
            .position_bounds(AxialDomain {
                t: [0., 0.],
                y: [y, y],
            })
            .unwrap();
        let q = a
            .position_bounds(AxialDomain {
                t: [1., 1.],
                y: [y, y],
            })
            .unwrap();
        for i in 0..3 {
            assert!(p[i][0] <= q[i][1] && q[i][0] <= p[i][1]);
        }
        let p = tip
            .position_bounds(AxialDomain {
                t: [1., 1.],
                y: [y, y],
            })
            .unwrap();
        let q = b
            .position_bounds(AxialDomain {
                t: [1., 1.],
                y: [y, y],
            })
            .unwrap();
        for i in 0..3 {
            assert!(p[i][0] <= q[i][1] && q[i][0] <= p[i][1]);
        }
    }
}

#[test]
fn stock_tip_nurbs_keeps_the_original_radius_source_and_physical_end_planes() {
    let s = Spec::default();
    let a = axial_stock_intersection(s, Flank::Descending, [-2., 2.], 21., 128).unwrap();
    let b = axial_stock_intersection(s, Flank::Ascending, [-2., 2.], 21., 128).unwrap();
    let tip = axial_tip_chart(&a, &b, 32).unwrap();
    let grid = fit_axial_tip(&tip, 1e-4, 256).unwrap();
    grid.reverify().unwrap();
    verify_packed_axial_tip(&grid, grid.packed_surfaces()).unwrap();
    assert!(grid.certificate().error_upper <= 1e-4);
    eprintln!(
        "axial tip: cells={} packed={} error_upper={}",
        grid.surfaces().len(),
        grid.packed_surfaces().len(),
        grid.certificate().error_upper
    );
    for patch in grid.surfaces() {
        for row in &patch.control_points {
            assert_eq!(row[0][1].to_bits(), patch.knots_v[5].to_bits());
            assert_eq!(row[5][1].to_bits(), patch.knots_v[6].to_bits());
        }
        let t = (patch.knots_u[5] + patch.knots_u[6]) / 2.;
        let y = (patch.knots_v[5] + patch.knots_v[6]) / 2.;
        let p = patch.evaluate(t, y).unwrap().point;
        let q = oracle(s, t, y);
        let error = (0..3).map(|i| (p[i] - q[i]).powi(2)).sum::<f64>().sqrt();
        assert!(error <= grid.certificate().error_upper + 1e-10);
    }
    let mut changed = grid.surfaces().to_vec();
    changed[0].control_points[2][2][0] += 0.01;
    assert!(
        certify_axial_tip_grid(
            &tip,
            grid.domain(),
            grid.t_breaks(),
            grid.y_breaks(),
            &changed,
            256
        )
        .unwrap()
        .control_difference_upper
            > 0.009
    );
    changed = grid.surfaces().to_vec();
    changed[0].control_points[2][0][1] += 1e-12;
    assert!(
        certify_axial_tip_grid(
            &tip,
            grid.domain(),
            grid.t_breaks(),
            grid.y_breaks(),
            &changed,
            256
        )
        .is_err()
    );
}

#[test]
fn mismatched_original_roles_radius_phase_and_domains_never_form_a_tip() {
    let s = Spec::default();
    let a = axial_stock_intersection(s, Flank::Descending, [-2., 2.], 21., 128).unwrap();
    let b = axial_stock_intersection(s, Flank::Ascending, [-2., 2.], 21., 128).unwrap();
    assert!(axial_tip_chart(&b, &a, 32).is_err());
    assert!(axial_tip_chart(&a, &a, 32).is_err());
    assert!(axial_tip_chart(&a, &b, 0).is_err());
    let other = axial_stock_intersection(
        Spec {
            axial_shift: 0.3,
            ..s
        },
        Flank::Ascending,
        [-2., 2.],
        21.,
        128,
    )
    .unwrap();
    assert!(axial_tip_chart(&a, &other, 32).is_err());
    let other = axial_stock_intersection(s, Flank::Ascending, [-2., 2.], 20.8, 128).unwrap();
    assert!(axial_tip_chart(&a, &other, 32).is_err());
    let other = b.restrict_y([-1., 1.]).unwrap();
    assert!(axial_tip_chart(&a, &other, 32).is_err());
}
