use nurbs_core::{
    worm,
    worm_wheel::{
        AxialDomain, Flank, RootPart, Spec, axial_root_chart, certify_axial_root_grid,
        fit_axial_root, verify_packed_axial_root,
    },
};

fn parts() -> [RootPart; 3] {
    [
        RootPart::CrestEdge(Flank::Descending),
        RootPart::CrestEdge(Flank::Ascending),
        RootPart::CrestCylinder,
    ]
}
fn oracle(s: Spec, part: RootPart, t: f64, y: f64) -> [f64; 3] {
    let w = s.worm;
    let rp = w.axial_module * w.diameter_quotient / 2.;
    let rc = rp + w.axial_module;
    let rw = w.axial_module * s.teeth as f64 / 2.;
    let p = std::f64::consts::PI * w.axial_module;
    let wc = p / 2. - w.backlash - 2. * w.axial_module * w.pressure_angle.tan();
    let beta = (y / rc).asin();
    let hand = if w.hand == worm::Hand::Right { 1. } else { -1. };
    let h = hand * w.axial_module * w.starts as f64 / 2.;
    let eta = -hand * w.starts as f64 / s.teeth as f64;
    let (z, z0) = match part {
        RootPart::CrestCylinder => (0., p - wc / 2. + wc * t),
        RootPart::CrestEdge(f) => {
            let k = if f == Flank::Descending { -1. } else { 1. } * w.pressure_angle.tan();
            (
                t * (rp - rc * beta.cos()) / (k * beta.cos() - h * beta.sin() / rc),
                if f == Flank::Descending {
                    wc / 2.
                } else {
                    p - wc / 2.
                },
            )
        }
    };
    let theta = (z - z0 - s.start as f64 * p - s.axial_shift) / h;
    let phi = beta - theta;
    let psi = eta * phi;
    let original = [rc * theta.cos(), rc * theta.sin(), z];
    let housing = [
        phi.cos() * original[0] - phi.sin() * original[1] - rp - rw,
        phi.sin() * original[0] + phi.cos() * original[1],
        z,
    ];
    [
        psi.cos() * housing[0] - psi.sin() * z,
        housing[1],
        psi.sin() * housing[0] + psi.cos() * z,
    ]
}

#[test]
fn physical_axial_sources_match_independent_positions_and_jets() {
    for hand in [worm::Hand::Right, worm::Hand::Left] {
        let s = Spec {
            worm: worm::Spec {
                hand,
                starts: 2,
                ..worm::Spec::default()
            },
            start: 1,
            ..Spec::default()
        };
        for part in parts() {
            let owner = axial_root_chart(s, part, [-2., 2.]).unwrap();
            owner.reverify().unwrap();
            assert!(owner.cross_norm_lower() > 0.);
            let d = AxialDomain {
                t: [0.3, 0.32],
                y: [0.3, 0.32],
            };
            let r = owner.enclose(d).unwrap();
            assert!(r.regularity_proven);
            assert_eq!(r.point[1], d.y);
            assert_eq!(r.dt[1], [0., 0.]);
            assert_eq!(r.dy[1], [1., 1.]);
            assert_eq!(r.sixth[0][1], [0., 0.]);
            assert_eq!(r.sixth[1][1], [0., 0.]);
            for t in [0.3, 0.31, 0.32] {
                for y in [0.3, 0.31, 0.32] {
                    let p = oracle(s, part, t, y);
                    for i in 0..3 {
                        assert!(r.point[i][0] - 1e-11 <= p[i] && p[i] <= r.point[i][1] + 1e-11);
                    }
                }
            }
            let h = 1e-5;
            let a = oracle(s, part, 0.31 - h, 0.31);
            let b = oracle(s, part, 0.31 + h, 0.31);
            let c = oracle(s, part, 0.31, 0.31 - h);
            let e = oracle(s, part, 0.31, 0.31 + h);
            for i in 0..3 {
                let dt = (b[i] - a[i]) / (2. * h);
                let dy = (e[i] - c[i]) / (2. * h);
                assert!(r.dt[i][0] - 1e-6 <= dt && dt <= r.dt[i][1] + 1e-6);
                assert!(r.dy[i][0] - 1e-6 <= dy && dy <= r.dy[i][1] + 1e-6);
            }
        }
    }
}

#[test]
fn finite_axial_nurbs_have_actual_exact_end_planes_and_fresh_source_error() {
    let s = Spec::default();
    for part in parts() {
        let root = axial_root_chart(s, part, [-2., 2.]).unwrap();
        let grid = fit_axial_root(&root, 1e-4, 256).unwrap();
        grid.reverify().unwrap();
        verify_packed_axial_root(&grid, grid.packed_surfaces()).unwrap();
        assert_eq!(grid.domain(), root.domain());
        assert!(grid.certificate().error_upper <= 1e-4);
        eprintln!(
            "axial {part:?}: cells={} packed={} error_upper={}",
            grid.surfaces().len(),
            grid.packed_surfaces().len(),
            grid.certificate().error_upper
        );
        for patch in grid.surfaces() {
            for row in &patch.control_points {
                assert_eq!(row[0][1].to_bits(), patch.knots_v[5].to_bits());
                assert_eq!(row[5][1].to_bits(), patch.knots_v[6].to_bits());
            }
            for i in 1..6 {
                for j in 0..6 {
                    assert_eq!(
                        patch.control_points[i][j][1].to_bits(),
                        patch.control_points[0][j][1].to_bits()
                    );
                }
            }
            for t in [
                patch.knots_u[5],
                (patch.knots_u[5] + patch.knots_u[6]) / 2.,
                patch.knots_u[6],
            ] {
                for y in [
                    patch.knots_v[5],
                    (patch.knots_v[5] + patch.knots_v[6]) / 2.,
                    patch.knots_v[6],
                ] {
                    let p = patch.evaluate(t, y).unwrap().point;
                    let q = oracle(s, part, t, y);
                    let error = (0..3).map(|i| (p[i] - q[i]).powi(2)).sum::<f64>().sqrt();
                    assert!(error <= grid.certificate().error_upper + 1e-10);
                }
            }
        }
        let mut changed = grid.surfaces().to_vec();
        changed[0].control_points[2][2][0] += 0.01;
        assert!(
            certify_axial_root_grid(
                &root,
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
            certify_axial_root_grid(
                &root,
                grid.domain(),
                grid.t_breaks(),
                grid.y_breaks(),
                &changed,
                256
            )
            .is_err()
        );
        let shifted = axial_root_chart(
            Spec {
                axial_shift: 0.3,
                ..s
            },
            part,
            [-2., 2.],
        )
        .unwrap();
        assert!(
            certify_axial_root_grid(
                &shifted,
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
        assert!(fit_axial_root(&root, 1e-15, 1).is_err());
    }
}

#[test]
fn unsupported_axial_domains_never_become_snapped_geometry() {
    let s = Spec::default();
    for y in [[-6., 6.], [2., -2.], [0., f64::NAN], [-5., 5.]] {
        assert!(axial_root_chart(s, RootPart::CrestEdge(Flank::Descending), y).is_err());
    }
    let root = axial_root_chart(s, RootPart::CrestCylinder, [-2., 2.]).unwrap();
    for d in [
        AxialDomain {
            t: [0., 1.],
            y: [-2.1, 0.],
        },
        AxialDomain {
            t: [0., 1.1],
            y: [0., 0.],
        },
        AxialDomain {
            t: [0., 1.],
            y: [f64::NAN, 0.],
        },
    ] {
        assert!(root.position_bounds(d).is_err());
        assert!(root.enclose(d).is_err());
    }
    let point = axial_root_chart(s, RootPart::CrestCylinder, [0., 0.]).unwrap();
    assert!(fit_axial_root(&point, 1e-4, 256).is_err());
}
