use nurbs_core::{
    worm,
    worm_wheel::{ChartDomain, Flank, Spec, Status, contacts, cover_chart, enclose_chart},
};
fn oracle(s: Spec, f: Flank, u: f64, b: f64) -> [f64; 3] {
    let w = s.worm;
    let m = w.axial_module;
    let p = std::f64::consts::PI * m;
    let rp = m * w.diameter_quotient / 2.;
    let rc = rp + m;
    let rr = rp - m * w.dedendum_factor;
    let wc = p / 2. - w.backlash - 2. * m * w.pressure_angle.tan();
    let wr = p / 2. - w.backlash + 2. * m * w.dedendum_factor * w.pressure_angle.tan();
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
    let sign = if w.hand == worm::Hand::Right { 1. } else { -1. };
    let h = sign * m * w.starts as f64 / 2.;
    let z = (rp - r * b.cos()) / (k * b.cos() - h * b.sin() / r);
    let t = (z - z0 - s.start as f64 * p - s.axial_shift) / h;
    let phi = b - t;
    let psi = -sign * w.starts as f64 * phi / s.teeth as f64;
    let x = r * b.cos() - (rp + m * s.teeth as f64 / 2.);
    [
        psi.cos() * x - psi.sin() * z,
        r * b.sin(),
        psi.sin() * x + psi.cos() * z,
    ]
}
#[test]
fn entire_boxes_cover_independent_positions_and_first_derivatives() {
    for hand in [worm::Hand::Right, worm::Hand::Left] {
        for f in [Flank::Descending, Flank::Ascending] {
            let s = Spec {
                worm: worm::Spec {
                    hand,
                    ..worm::Spec::default()
                },
                ..Spec::default()
            };
            let cell = enclose_chart(
                s,
                f,
                ChartDomain {
                    u: [0.2, 0.22],
                    beta: [-0.02, 0.02],
                },
            )
            .unwrap();
            for u in [0.2, 0.21, 0.22] {
                for b in [-0.02, 0., 0.02] {
                    let q = oracle(s, f, u, b);
                    for d in 0..3 {
                        assert!(
                            cell.point()[d][0] - 1e-10 <= q[d]
                                && q[d] <= cell.point()[d][1] + 1e-10
                        );
                    }
                    let point = enclose_chart(
                        s,
                        f,
                        ChartDomain {
                            u: [u, u],
                            beta: [b, b],
                        },
                    )
                    .unwrap();
                    assert!(point.regularity_proven());
                    let h = 1e-5;
                    let up = oracle(s, f, u + h, b);
                    let um = oracle(s, f, u - h, b);
                    let bp = oracle(s, f, u, b + h);
                    let bm = oracle(s, f, u, b - h);
                    for d in 0..3 {
                        let du = (up[d] - um[d]) / (2. * h);
                        let db = (bp[d] - bm[d]) / (2. * h);
                        assert!(point.du()[d][0] - 5e-7 <= du && du <= point.du()[d][1] + 5e-7);
                        assert!(
                            point.dbeta()[d][0] - 5e-7 <= db && db <= point.dbeta()[d][1] + 5e-7
                        );
                    }
                }
            }
        }
    }
}
#[test]
fn explicit_chart_crosses_the_unresolved_angular_root_fold() {
    let spec = Spec {
        worm: worm::Spec {
            dedendum_factor: 1.,
            ..worm::Spec::default()
        },
        axial_shift: -std::f64::consts::PI / 4.,
        ..Spec::default()
    };
    assert_eq!(
        contacts(spec, Flank::Descending, 0.5, 0., 1e-9)
            .unwrap()
            .status(),
        Status::Unresolved
    );
    let cell = enclose_chart(
        spec,
        Flank::Descending,
        ChartDomain {
            u: [0.5, 0.5],
            beta: [0., 0.],
        },
    )
    .unwrap();
    assert!(cell.regularity_proven());
    for (b, q) in cell.point().iter().zip([-20., 0., 0.]) {
        assert!(b[0] - 1e-10 <= q && q <= b[1] + 1e-10);
    }
}
#[test]
fn full_requested_chart_rectangles_have_source_complete_regular_covers() {
    for f in [Flank::Descending, Flank::Ascending] {
        let cover = cover_chart(
            Spec::default(),
            f,
            ChartDomain {
                u: [0., 1.],
                beta: [-0.05, 0.05],
            },
            1023,
            14,
        )
        .unwrap();
        assert!(cover.complete(), "{} leaves", cover.cells());
        cover.reverify().unwrap();
        assert!(
            cover
                .leaves()
                .all(|(_, b)| b.is_some_and(|b| b.regularity_proven()))
        );
        let area = cover
            .leaves()
            .map(|(d, _)| (d.u[1] - d.u[0]) * (d.beta[1] - d.beta[0]))
            .sum::<f64>();
        assert!((area - 0.1).abs() < 1e-14);
    }
}
#[test]
fn unproved_denominators_and_missing_work_never_claim_regular_coverage() {
    let domain = ChartDomain {
        u: [0., 1.],
        beta: [-1.4, 1.4],
    };
    assert!(enclose_chart(Spec::default(), Flank::Descending, domain).is_err());
    let cover = cover_chart(Spec::default(), Flank::Descending, domain, 1, 0).unwrap();
    assert!(!cover.complete());
    cover.reverify().unwrap();
    assert!(cover_chart(Spec::default(), Flank::Descending, domain, 0, 0).is_err());
    assert!(
        cover_chart(
            Spec::default(),
            Flank::Descending,
            ChartDomain {
                u: [1., 0.],
                ..domain
            },
            10,
            4
        )
        .is_err()
    );
}

#[test]
fn useful_wide_source_charts_cover_both_hands_and_original_multi_start_phase() {
    for (hand, flank, start, starts) in [
        (worm::Hand::Right, Flank::Descending, 0, 1),
        (worm::Hand::Left, Flank::Ascending, 1, 2),
    ] {
        let spec = Spec {
            worm: worm::Spec {
                hand,
                starts,
                ..worm::Spec::default()
            },
            start,
            ..Spec::default()
        };
        let domain = ChartDomain {
            u: [0., 1.],
            beta: [-0.3, 0.3],
        };
        let cover = cover_chart(spec, flank, domain, 4095, 18).unwrap();
        assert!(cover.complete(), "{} leaves", cover.cells());
        cover.reverify().unwrap();
        assert_eq!(cover.domain(), domain);
        assert_eq!(cover.flank(), flank);
        assert_eq!(cover.specification().start, start);
        let area = cover
            .leaves()
            .map(|(d, _)| (d.u[1] - d.u[0]) * (d.beta[1] - d.beta[0]))
            .sum::<f64>();
        assert!((area - 0.6).abs() < 1e-13);
    }
}
