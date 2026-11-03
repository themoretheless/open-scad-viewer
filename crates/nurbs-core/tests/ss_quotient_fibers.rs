use nurbs_core::{ss_intersection as ss, surface::Surface};
#[path = "common/periodic_extrusion.rs"]
mod fixture;
#[allow(dead_code)]
#[path = "support/ss_original_ideal.rs"]
mod geometry;
use geometry::curved_periodic_cutter as curved;
fn transformed(mut s: Surface) -> Surface {
    for (row, weights) in s.control_points.iter_mut().zip(&mut s.weights) {
        for (p, w) in row.iter_mut().zip(weights) {
            let d = 1. + 0.125 * p[0] + 0.0625 * p[1] + 0.125 * p[2];
            *w *= d;
            for x in p {
                *x /= d;
            }
        }
    }
    s
}
fn chart() -> [[f64; 2]; 4] {
    [[-0.03125, 0.03125], [0.4, 0.6], [0.4, 0.6], [0.35, 0.65]]
}
#[test]
fn original_curved_curved_fiber_crosses_periodic_shore_without_unflagging() {
    for rational in [false, true] {
        let (a, b) = if rational {
            (transformed(fixture::source(1)), transformed(curved()))
        } else {
            (fixture::source(1), curved())
        };
        let source = ss::FiberSources::new(&a, &b).unwrap();
        let r = source
            .certify_quotient(chart(), ss::FiberAxis::FirstU, Default::default())
            .unwrap();
        let report = r.report().clone();
        let f = r.into_fiber().unwrap_or_else(|| panic!("{report:?}"));
        f.reverify().unwrap();
        assert!(f.source().first().periodic_u);
        assert_eq!(f.source().first(), &a);
        assert_eq!(f.source().second(), &b);
        f.at(0., Default::default())
            .unwrap()
            .into_fiber()
            .unwrap()
            .reverify()
            .unwrap();
        for u in [[0., 0.015625], [3.984375, 4.]] {
            let mut d = chart();
            d[0] = u;
            assert!(f.contains_original_box(d).unwrap());
        }
        let mut d = chart();
        d[0] = [1., 2.];
        assert!(!f.contains_original_box(d).unwrap());
    }
}
#[test]
fn operand_reversal_and_transposed_source_preserve_quotient_root() {
    let a = fixture::source(1);
    let transposed = nurbs_core::surface_edit::transpose(&a).unwrap();
    let original = chart();
    ss::FiberSources::new(&transposed, &curved())
        .unwrap()
        .certify_quotient(
            [original[1], original[0], original[2], original[3]],
            ss::FiberAxis::FirstV,
            Default::default(),
        )
        .unwrap()
        .into_fiber()
        .unwrap()
        .reverify()
        .unwrap();
    let b = curved();
    let d = chart();
    let source = ss::FiberSources::new(&b, &a).unwrap();
    source
        .certify_quotient(
            [d[2], d[3], d[0], d[1]],
            ss::FiberAxis::SecondU,
            Default::default(),
        )
        .unwrap()
        .into_fiber()
        .unwrap()
        .reverify()
        .unwrap();
}
#[test]
fn nonperiodic_extrapolation_full_period_and_work_exhaustion_do_not_publish() {
    let source = ss::FiberSources::new(&fixture::source(1), &curved()).unwrap();
    let mut d = chart();
    d[2] = [-0.1, 0.1];
    assert!(
        source
            .certify_quotient(d, ss::FiberAxis::FirstU, Default::default())
            .is_err()
    );
    let mut d = chart();
    d[0] = [0., 4.];
    assert!(
        source
            .certify_quotient(d, ss::FiberAxis::FirstU, Default::default())
            .is_err()
    );
    let r = source
        .certify_quotient(
            chart(),
            ss::FiberAxis::FirstU,
            ss::FiberLimits {
                max_rectangles: 1,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(r.fiber().is_none());
    assert!(r.report().rectangles <= 1);
}
fn atlas(source: &ss::FiberSources) -> Vec<ss::QualifiedQuotientFiber> {
    let original = fixture::source(1);
    let mut charts = Vec::new();
    for i in 0..8 {
        let lo = (i as f64 / 8. - 0.0625).max(0.);
        let hi = ((i + 1) as f64 / 8. + 0.0625).min(1.);
        let solve = |v: f64| {
            let mut low = -0.5;
            let mut high = 0.5;
            for _ in 0..60 {
                let mid = (low + high) * 0.5;
                let phase = if mid < 0. { mid + 4. } else { mid };
                if original.evaluate(phase, 0.5).unwrap().point[1] < v - 0.5 {
                    low = mid
                } else {
                    high = mid
                }
            }
            (low + high) * 0.5
        };
        let u = [solve(lo) - 0.08, solve(hi) + 0.08];
        let d = [u, [0.3, 0.7], [0.2, 0.8], [lo, hi]];
        let r = source
            .certify_quotient(d, ss::FiberAxis::SecondV, Default::default())
            .unwrap();
        let diagnostic = r.report().clone();
        charts.push(
            r.into_fiber()
                .unwrap_or_else(|| panic!("chart {i}: {diagnostic:?}")),
        );
    }
    charts
}
#[test]
fn complete_original_product_census_covers_the_shore_crossing_arc() {
    let source = ss::FiberSources::new(&fixture::source(1), &curved()).unwrap();
    let charts = atlas(&source);
    let r = ss::cover_quotient_fiber_surfaces(&charts, Default::default()).unwrap();
    let report = r.report().clone();
    let cover = r.into_coverage().unwrap_or_else(|| panic!("{report:?}"));
    assert!(cover.covers_entire_surfaces());
    cover.reverify().unwrap();
    assert!(
        cover
            .nodes()
            .iter()
            .any(|n| matches!(n.kind(), Some(ss::QuotientCoverageKind::RootImage { .. })))
    );
    for limits in [
        ss::CoverageLimits {
            max_cells: 1,
            ..Default::default()
        },
        ss::CoverageLimits {
            max_rectangles: 1,
            ..Default::default()
        },
    ] {
        assert!(
            ss::cover_quotient_fiber_surfaces(&charts, limits)
                .unwrap()
                .coverage()
                .is_none()
        );
    }
    let r = ss::cover_quotient_fiber_surfaces(&charts[1..7], Default::default()).unwrap();
    assert!(
        r.coverage().is_none(),
        "missing original branches gained coverage"
    );
}
#[test]
fn automatic_discovery_requires_complete_original_quotient_census() {
    let source = ss::FiberSources::new(&fixture::source(1), &curved()).unwrap();
    let r = ss::discover_quotient_fiber_surfaces(&source, Default::default()).unwrap();
    let report = r.report().clone();
    let owner = r.into_coverage().unwrap_or_else(|| panic!("{report:?}"));
    assert!(owner.covers_entire_surfaces());
    owner.reverify().unwrap();
}

#[test]
fn native_chart_closure_uses_the_proved_periodic_upper_alias() {
    let source = ss::FiberSources::new(&fixture::source(1), &curved()).unwrap();
    let mut domain = chart();
    domain[0] = [3.96875, 4.];
    let r = source
        .certify(domain, ss::FiberAxis::FirstU, Default::default())
        .unwrap();
    let report = r.report().clone();
    let f = r.into_fiber().unwrap_or_else(|| panic!("{report:?}"));
    f.at(4., Default::default())
        .unwrap()
        .into_fiber()
        .unwrap()
        .reverify()
        .unwrap();
}
