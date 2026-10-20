use nurbs_core::{
    worm,
    worm_wheel::{
        AxialDomain, Flank, Spec, axial_stock_intersection, certify_axial_stock_grid,
        fit_axial_stock, verify_packed_axial_stock,
    },
};

fn original(s: Spec, f: Flank, u: f64, y: f64) -> [f64; 3] {
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
    let beta = (y / r).asin();
    let hand = if w.hand == worm::Hand::Right { 1. } else { -1. };
    let h = hand * w.axial_module * w.starts as f64 / 2.;
    let eta = -hand * w.starts as f64 / s.teeth as f64;
    let z = (rp - r * beta.cos()) / (k * beta.cos() - h * beta.sin() / r);
    let theta = (z - z0 - s.start as f64 * p - s.axial_shift) / h;
    let phi = beta - theta;
    let psi = eta * phi;
    let old = [r * theta.cos(), r * theta.sin(), z];
    let housing = [
        phi.cos() * old[0] - phi.sin() * old[1] - rp - w.axial_module * s.teeth as f64 / 2.,
        phi.sin() * old[0] + phi.cos() * old[1],
        z,
    ];
    [
        psi.cos() * housing[0] - psi.sin() * z,
        housing[1],
        psi.sin() * housing[0] + psi.cos() * z,
    ]
}
fn root(s: Spec, f: Flank, y: f64, radius: f64) -> f64 {
    let mut d = [0., 1.];
    for _ in 0..54 {
        let u = (d[0] + d[1]) / 2.;
        let p = original(s, f, u, y);
        if (p[0].hypot(p[2]) < radius) == (f == Flank::Descending) {
            d[0] = u;
        } else {
            d[1] = u;
        }
    }
    (d[0] + d[1]) / 2.
}
fn point(s: Spec, f: Flank, t: f64, y: f64, radius: f64) -> [f64; 3] {
    let crest = if f == Flank::Descending { 0. } else { 1. };
    original(s, f, crest + (root(s, f, y, radius) - crest) * t, y)
}

#[test]
fn physical_stock_roots_cover_independent_crossings_and_preserve_original_kinematics() {
    for hand in [worm::Hand::Right, worm::Hand::Left] {
        let s = Spec {
            worm: worm::Spec {
                hand,
                ..worm::Spec::default()
            },
            ..Spec::default()
        };
        for f in [Flank::Descending, Flank::Ascending] {
            let stock = axial_stock_intersection(s, f, [-2., 2.], 21., 128).unwrap();
            stock.reverify().unwrap();
            assert!(stock.cross_norm_lower() > 0.);
            let bracket = stock.u_bracket();
            for i in 0..=100 {
                let y = -2. + 4. * i as f64 / 100.;
                let u = root(s, f, y, 21.);
                assert!(bracket[0] <= u && u <= bracket[1]);
            }
            let d = AxialDomain {
                t: [0.3, 0.32],
                y: [0.3, 0.32],
            };
            let b = stock.enclose(d).unwrap();
            assert!(b.regularity_proven);
            assert_eq!(b.point[1], d.y);
            assert_eq!(b.dt[1], [0., 0.]);
            assert_eq!(b.dy[1], [1., 1.]);
            for t in [0.3, 0.31, 0.32] {
                for y in [0.3, 0.31, 0.32] {
                    let p = point(s, f, t, y, 21.);
                    for j in 0..3 {
                        assert!(b.point[j][0] - 1e-10 <= p[j] && p[j] <= b.point[j][1] + 1e-10);
                    }
                }
            }
            let h = 1e-5;
            let a = point(s, f, 0.31 - h, 0.31, 21.);
            let c = point(s, f, 0.31 + h, 0.31, 21.);
            let e = point(s, f, 0.31, 0.31 - h, 21.);
            let g = point(s, f, 0.31, 0.31 + h, 21.);
            for j in 0..3 {
                let dt = (c[j] - a[j]) / (2. * h);
                let dy = (g[j] - e[j]) / (2. * h);
                assert!(b.dt[j][0] - 1e-6 <= dt && dt <= b.dt[j][1] + 1e-6);
                assert!(b.dy[j][0] - 1e-6 <= dy && dy <= b.dy[j][1] + 1e-6);
            }
        }
    }
}

#[test]
fn physical_stock_nurbs_replay_original_radius_phase_and_actual_end_planes() {
    let s = Spec::default();
    for f in [Flank::Descending, Flank::Ascending] {
        let stock = axial_stock_intersection(s, f, [-2., 2.], 21., 128).unwrap();
        let grid = fit_axial_stock(&stock, 1e-4, 256).unwrap();
        grid.reverify().unwrap();
        verify_packed_axial_stock(&grid, grid.packed_surfaces()).unwrap();
        assert!(grid.certificate().error_upper <= 1e-4);
        eprintln!(
            "axial stock {f:?}: cells={} packed={} error_upper={}",
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
            let q = point(s, f, t, y, 21.);
            let error = (0..3).map(|i| (p[i] - q[i]).powi(2)).sum::<f64>().sqrt();
            assert!(error <= grid.certificate().error_upper + 1e-10);
        }
        let mut changed = grid.surfaces().to_vec();
        changed[0].control_points[2][2][0] += 0.01;
        assert!(
            certify_axial_stock_grid(
                &stock,
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
            certify_axial_stock_grid(
                &stock,
                grid.domain(),
                grid.t_breaks(),
                grid.y_breaks(),
                &changed,
                256
            )
            .is_err()
        );
        let other = axial_stock_intersection(s, f, [-2., 2.], 20.8, 128).unwrap();
        assert!(
            certify_axial_stock_grid(
                &other,
                grid.domain(),
                grid.t_breaks(),
                grid.y_breaks(),
                grid.surfaces(),
                256
            )
            .unwrap()
            .error_upper
                > 0.01
        );
    }
}

#[test]
fn absent_or_singular_axial_crossings_and_outside_queries_refuse() {
    let s = Spec::default();
    for radius in [f64::NAN, 0., 18., 25.] {
        assert!(axial_stock_intersection(s, Flank::Descending, [-2., 2.], radius, 128).is_err());
    }
    assert!(axial_stock_intersection(s, Flank::Descending, [-4., 4.], 21., 128).is_err());
    assert!(axial_stock_intersection(s, Flank::Descending, [-2., 2.], 21., 0).is_err());
    let stock = axial_stock_intersection(s, Flank::Descending, [-2., 2.], 21., 128).unwrap();
    assert!(stock.restrict_y([-2.1, 0.]).is_err());
    assert!(
        stock
            .enclose(AxialDomain {
                t: [0., 1.1],
                y: [0., 0.]
            })
            .is_err()
    );
}
