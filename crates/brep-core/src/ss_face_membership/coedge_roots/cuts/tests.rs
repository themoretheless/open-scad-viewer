use super::*;
use crate::ss_face_membership::tests::{curves, fixtures, model};
const HOLE: [[f64; 2]; 2] = [[0.5625, 0.875], [0.4375, 0.5625]];
fn roots<'a>(c: &'a ss::QualifiedGraphCurves, m: &'a [Model; 2]) -> QualifiedFacePairRoots<'a> {
    qualify_coedge_roots(
        c.original(),
        [(&m[0], 0), (&m[1], 0)],
        1e-8,
        Default::default(),
    )
    .unwrap()
}
#[test]
fn cuts_are_bounded_against_original_ss_and_actual_edge_and_query() {
    let c = curves(0.0625);
    let m = fixtures(&c, Some(HOLE));
    let before = m.clone();
    let r = roots(&c, &m);
    let out = qualify_cut_representatives(&r, Default::default()).unwrap();
    assert_eq!(out.nodes().len(), 2);
    assert_eq!(out.cuts().len(), 2);
    for cut in out.cuts() {
        let e = &r.records[cut.record].events[cut.event];
        let p = out.nodes()[cut.node].position;
        assert!(cut.source_error_upper <= cut.world_tolerance);
        assert!(cut.actual_edge_error_upper <= cut.world_tolerance);
        assert!(cut.uv_error_upper <= out.limits.uv_tolerance);
        assert!(
            e.edge_parameter[0] <= cut.edge_parameter && cut.edge_parameter <= e.edge_parameter[1]
        );
        assert!(
            e.pcurve_parameter[0] <= cut.pcurve_parameter
                && cut.pcurve_parameter <= e.pcurve_parameter[1]
        );
        let radius = (p[0] - 0.5).powi(2) + (p[1] - 0.5).powi(2);
        assert!((radius - 0.0625).abs() < 1e-8);
        assert!(p[2].abs() < 1e-8);
    }
    out.reverify().unwrap();
    assert_eq!(m, before);
}
#[test]
fn literally_common_boundaries_use_two_nodes_for_four_actual_cuts() {
    let c = curves(0.0625);
    let m = [
        model(c.original().source().first(), Some(HOLE)),
        model(c.original().source().second(), Some(HOLE)),
    ];
    let r = roots(&c, &m);
    let out = qualify_cut_representatives(&r, Default::default()).unwrap();
    assert_eq!(out.cuts().len(), 4);
    assert_eq!(out.nodes().len(), 2);
    for ni in 0..2 {
        let uses: Vec<_> = out.cuts().iter().filter(|v| v.node == ni).collect();
        assert_eq!(uses.len(), 2);
        assert_ne!(
            r.records[uses[0].record].address.side,
            r.records[uses[1].record].address.side
        );
    }
    out.reverify().unwrap();
}
#[test]
fn outside_events_and_internal_closed_loop_do_not_invent_edge_cuts() {
    let c = curves(0.0625);
    for m in [
        fixtures(&c, None),
        [
            model(c.original().source().first(), Some(HOLE)),
            model(
                c.original().source().second(),
                Some([[0.125, 0.875], [0.125, 0.875]]),
            ),
        ],
    ] {
        let r = roots(&c, &m);
        let out = qualify_cut_representatives(&r, Default::default()).unwrap();
        assert_eq!(r.original.kind(), QuadraticGraphSSKind::ClosedEllipse);
        assert!(out.cuts().is_empty());
        assert!(out.nodes().is_empty());
        out.reverify().unwrap();
    }
}
#[test]
fn rational_actual_clocks_are_selected_independently() {
    let c = curves(0.0625);
    let mut m = fixtures(&c, Some(HOLE));
    let wire = m[0].faces[0].holes[0];
    let edge = m[0].loops[wire].coedges[0].edge;
    m[0].loops[wire].coedges[0].pcurve.weights[1] *= 2.;
    for (i, w) in m[0].edges[edge].curve.weights.iter_mut().enumerate() {
        *w *= 2f64.powi(i as i32);
    }
    for knot in &mut m[0].edges[edge].curve.knots {
        *knot = 10. + (*knot - HOLE[0][0]) / (HOLE[0][1] - HOLE[0][0]) * 4.;
    }
    let r = roots(&c, &m);
    let out = qualify_cut_representatives(&r, Default::default()).unwrap();
    let cut = out
        .cuts()
        .iter()
        .find(|cut| r.records[cut.record].address.edge == edge)
        .unwrap();
    assert!(cut.edge_parameter > 10.);
    assert!(cut.pcurve_parameter < 1.);
    out.reverify().unwrap();
}
#[test]
fn budgets_and_mutated_public_snapshots_cannot_grant_cut_authority() {
    let c = curves(0.0625);
    let m = fixtures(&c, Some(HOLE));
    let r = roots(&c, &m);
    let mut out = qualify_cut_representatives(&r, Default::default()).unwrap();
    let exact = CutRepresentativeLimits {
        operations: out.operations(),
        curve_spans: out.curve_spans(),
        cuts: out.cuts().len(),
        ..Default::default()
    };
    assert!(qualify_cut_representatives(&r, exact).is_ok());
    assert!(
        qualify_cut_representatives(
            &r,
            CutRepresentativeLimits {
                operations: exact.operations - 1,
                ..exact
            }
        )
        .is_err()
    );
    assert!(
        qualify_cut_representatives(
            &r,
            CutRepresentativeLimits {
                curve_spans: exact.curve_spans - 1,
                ..exact
            }
        )
        .is_err()
    );
    assert!(qualify_cut_representatives(&r, CutRepresentativeLimits { cuts: 1, ..exact }).is_err());
    let saved = out.cuts.clone();
    out.cuts[0].edge_parameter += 0.01;
    assert!(out.reverify().is_err());
    out.cuts = saved;
    out.nodes[0].position[2] += 0.01;
    assert!(out.reverify().is_err());
}
#[test]
fn representative_refuses_an_endpoint_only_binary64_band() {
    let mut used = 0;
    assert!(representative([1., 1.], [0., 1.], &mut used, 2100).is_err());
    assert!(representative([0.75, 0.25], [0., 1.], &mut 0, 2100).is_err());
    assert!(distance(&[0., 0., 0.], &[[0., 0.], [0., 0.], [1e-8, 1e-8]]).unwrap() >= 1e-8);
}
