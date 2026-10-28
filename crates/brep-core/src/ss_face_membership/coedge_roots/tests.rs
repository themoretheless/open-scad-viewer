use super::*;
use crate::ss_face_membership::tests::{curves, fixtures, model};
use nurbs_core::ss_intersection::GraphRootLocation;
const HOLE: [[f64; 2]; 2] = [[0.5625, 0.875], [0.4375, 0.5625]];
fn inspect<'a>(c: &'a ss::QualifiedGraphCurves, m: &'a [Model; 2]) -> QualifiedFacePairRoots<'a> {
    qualify_coedge_roots(
        c.original(),
        [(&m[0], 0), (&m[1], 0)],
        1e-8,
        Default::default(),
    )
    .unwrap()
}
fn contains(b: [f64; 2], x: f64) {
    assert!(b[0] <= x && x <= b[1], "{b:?} lacks {x}");
}
#[test]
fn all_actual_coedges_are_covered_even_with_no_boundary_events_on_a_closed_ss_loop() {
    let c = curves(0.0625);
    let m = fixtures(&c, None);
    let before = m.clone();
    let owner = inspect(&c, &m);
    assert_eq!(owner.coedges().len(), 8);
    assert!(owner.coedges().iter().all(|r| r.events().is_empty()));
    assert_eq!(owner.original().kind(), QuadraticGraphSSKind::ClosedEllipse);
    for r in owner.coedges() {
        let a = r.address();
        let model = &m[a.side];
        let actual = &model.loops[a.wire].coedges[a.coedge];
        assert_eq!(a.edge, actual.edge);
        assert_eq!(a.edge_identity, model.1.edges[actual.edge]);
        assert_eq!(a.wire_identity, model.1.loops[a.wire]);
        assert_eq!(a.face_identity, model.1.faces[a.face]);
        assert_eq!(a.shell_identity, model.1.shells[a.shell]);
        assert_eq!(r.bindings().census().boundary(), &actual.pcurve);
    }
    owner.reverify().unwrap();
    assert_eq!(m, before);
}
#[test]
fn actual_hole_crossings_keep_model_addresses_and_original_branch_bindings() {
    let c = curves(0.0625);
    let m = fixtures(&c, Some(HOLE));
    let owner = inspect(&c, &m);
    assert_eq!(owner.coedges().len(), 12);
    assert_eq!(
        owner
            .coedges()
            .iter()
            .map(|r| r.events().len())
            .sum::<usize>(),
        2
    );
    let x = 0.5 + (0.0625f64 - 0.0625 * 0.0625).sqrt();
    for r in owner.coedges().iter().filter(|r| !r.events().is_empty()) {
        let a = r.address();
        assert_eq!(a.side, 0);
        assert_eq!(a.role, CoedgeBoundaryRole::Hole(0));
        assert_eq!(a.wire, m[0].faces[0].holes[0]);
        let e = &r.events()[0];
        assert_eq!(e.other_face_location, Location::Inside);
        assert_eq!(e.other_boundary, None);
        let uv = &m[0].loops[a.wire].coedges[a.coedge].pcurve.control_points;
        let unit = (x - uv[0][0]) / (uv[1][0] - uv[0][0]);
        contains(e.pcurve_parameter, unit);
        let domain = m[0].edges[a.edge].curve.domain();
        let t = domain[0] + (domain[1] - domain[0]) * unit;
        contains(e.edge_parameter, t);
        contains(e.source_world[0], x);
        contains(e.source_world[1], uv[0][1]);
        contains(e.source_world[2], 0.);
        let p = m[0].edges[a.edge].curve.evaluate(t).unwrap().point;
        for axis in 0..3 {
            contains(e.actual_edge_world[axis], p[axis]);
        }
        assert!(matches!(
            r.bindings().bindings()[e.original_root].location,
            GraphRootLocation::Interior(_)
        ));
    }
    owner.reverify().unwrap();
}
#[test]
fn hole_exclusions_from_the_other_face_remain_in_the_complete_boundary_census() {
    let c = curves(0.0625);
    let m = [
        model(c.original().source().first(), Some(HOLE)),
        model(
            c.original().source().second(),
            Some([[0.125, 0.875], [0.125, 0.875]]),
        ),
    ];
    let owner = inspect(&c, &m);
    let events: Vec<_> = owner.coedges().iter().flat_map(|r| r.events()).collect();
    assert_eq!(owner.coedges().len(), 16);
    assert_eq!(events.len(), 2);
    assert!(
        events
            .iter()
            .all(|e| e.other_face_location == Location::Outside)
    );
    owner.reverify().unwrap();
}
#[test]
fn reversed_world_edge_and_independent_edge_clock_are_transported_once() {
    let c = curves(0.0625);
    let mut m = fixtures(&c, Some(HOLE));
    let wire = m[0].faces[0].holes[0];
    let edge = m[0].loops[wire].coedges[0].edge;
    let degree = m[0].edges[edge].curve.degree;
    m[0].edges[edge].curve.knots = vec![10.; degree + 1]
        .into_iter()
        .chain(vec![14.; degree + 1])
        .collect();
    m[0].edges[edge].curve = m[0].edges[edge].curve.reverse().unwrap();
    m[0].edges[edge].vertices.swap(0, 1);
    m[0].loops[wire].coedges[0].reversed = true;
    m[0].shells[0].faces[0].reversed = true;
    m[0].validate().unwrap();
    let owner = inspect(&c, &m);
    let record = owner
        .coedges()
        .iter()
        .find(|r| r.address().side == 0 && r.address().wire == wire && r.address().coedge == 0)
        .unwrap();
    assert!(record.address().edge_reversed);
    assert!(record.address().face_reversed);
    let x = 0.5 + (0.0625f64 - 0.0625 * 0.0625).sqrt();
    let unit = (x - HOLE[0][0]) / (HOLE[0][1] - HOLE[0][0]);
    contains(record.events()[0].pcurve_parameter, unit);
    contains(record.events()[0].edge_parameter, 10. + 4. * (1. - unit));
    owner.reverify().unwrap();
}
#[test]
fn literal_shared_trim_boundaries_have_fresh_boundary_membership_without_a_fuzzy_band() {
    let c = curves(0.0625);
    let m = [
        model(c.original().source().first(), Some(HOLE)),
        model(c.original().source().second(), Some(HOLE)),
    ];
    let owner = inspect(&c, &m);
    let events: Vec<_> = owner.coedges().iter().flat_map(|r| r.events()).collect();
    assert_eq!(events.len(), 4);
    assert!(
        events
            .iter()
            .all(|e| e.other_face_location == Location::Inside && e.other_boundary.is_some())
    );
    owner.reverify().unwrap();
    let mut shifted = HOLE;
    shifted[1][0] += 2f64.powi(-32);
    shifted[1][1] += 2f64.powi(-32);
    let m = [
        model(c.original().source().first(), Some(HOLE)),
        model(c.original().source().second(), Some(shifted)),
    ];
    assert!(
        qualify_coedge_roots(
            c.original(),
            [(&m[0], 0), (&m[1], 0)],
            1e-8,
            Default::default()
        )
        .is_err()
    );
}
#[test]
fn edited_support_and_mismatched_actual_world_edges_cannot_reuse_original_roots() {
    let c = curves(0.0625);
    let mut m = fixtures(&c, Some(HOLE));
    m[0].faces[0].surface.control_points[1][1][2] += 0.125;
    assert!(
        qualify_coedge_roots(
            c.original(),
            [(&m[0], 0), (&m[1], 0)],
            1e-8,
            Default::default()
        )
        .is_err()
    );
    let mut m = fixtures(&c, Some(HOLE));
    let wire = m[0].faces[0].holes[0];
    let edge = m[0].loops[wire].coedges[0].edge;
    m[0].edges[edge].curve.control_points[1][2] += 0.125;
    assert!(
        qualify_coedge_roots(
            c.original(),
            [(&m[0], 0), (&m[1], 0)],
            1e-8,
            Default::default()
        )
        .is_err()
    );
}
#[test]
fn full_metadata_windows_are_admitted_and_partial_windows_are_not_silently_ignored() {
    let c = curves(0.0625);
    let mut m = fixtures(&c, Some(HOLE));
    let wire = m[0].faces[0].holes[0];
    let old = m[0].loops[wire].coedges[0].clone();
    let edge = old.edge;
    let full = brep_topology::CoedgeTrim {
        curve_parameter: m[0].edges[edge].curve.domain(),
        pcurve_parameter: old.pcurve.domain(),
        periodic_lift: [[0; 2]; 2],
    };
    m[0].loops[wire].coedges[0] = old.clone().with_trim(full).unwrap();
    inspect(&c, &m).reverify().unwrap();
    let mut partial = full;
    partial.pcurve_parameter = [0.25, 0.75];
    m[0].loops[wire].coedges[0] = old.with_trim(partial).unwrap();
    assert!(
        qualify_coedge_roots(
            c.original(),
            [(&m[0], 0), (&m[1], 0)],
            1e-8,
            Default::default()
        )
        .is_err()
    );
}
#[test]
fn all_query_replays_and_event_transports_share_one_exact_operation_allowance() {
    let c = curves(0.0625);
    let m = fixtures(&c, Some(HOLE));
    let owner = inspect(&c, &m);
    let used = owner.operations();
    let faces = [(&m[0], 0), (&m[1], 0)];
    assert!(
        qualify_coedge_roots(
            c.original(),
            faces,
            1e-8,
            CoedgeRootLimits {
                operations: used,
                ..Default::default()
            }
        )
        .is_ok()
    );
    assert!(
        qualify_coedge_roots(
            c.original(),
            faces,
            1e-8,
            CoedgeRootLimits {
                operations: used - 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    for limits in [
        CoedgeRootLimits {
            face_cells: 1,
            ..Default::default()
        },
        CoedgeRootLimits {
            face_pairs: 1,
            ..Default::default()
        },
        CoedgeRootLimits {
            operations: 1,
            ..Default::default()
        },
        CoedgeRootLimits {
            coedges: 11,
            ..Default::default()
        },
        CoedgeRootLimits {
            root_depth: 65,
            ..Default::default()
        },
    ] {
        assert!(qualify_coedge_roots(c.original(), faces, 1e-8, limits).is_err());
    }
    assert!(
        qualify_coedge_roots(
            c.original(),
            faces,
            1e-8,
            CoedgeRootLimits {
                coedges: 12,
                ..Default::default()
            }
        )
        .is_ok()
    );
}
#[test]
fn source_addresses_parameters_membership_and_query_definitions_are_replayed() {
    let c = curves(0.0625);
    let m = fixtures(&c, Some(HOLE));
    let mut owner = inspect(&c, &m);
    owner.records[0].address.edge_identity = m[0].1.edges[1];
    assert!(owner.reverify().is_err());
    let mut owner = inspect(&c, &m);
    let i = owner
        .records
        .iter()
        .position(|r| !r.events.is_empty())
        .unwrap();
    owner.records[i].events[0].edge_parameter[0] += 0.01;
    assert!(owner.reverify().is_err());
    let mut owner = inspect(&c, &m);
    let i = owner
        .records
        .iter()
        .position(|r| !r.events.is_empty())
        .unwrap();
    owner.records[i].bindings = owner.records[0].bindings.clone();
    assert!(owner.reverify().is_err());
}

#[test]
fn actual_rational_pcurve_and_world_curve_clocks_are_requalified_together() {
    let c = curves(0.0625);
    let mut m = fixtures(&c, Some(HOLE));
    let wire = m[0].faces[0].holes[0];
    let edge = m[0].loops[wire].coedges[0].edge;
    m[0].loops[wire].coedges[0].pcurve.weights[1] *= 2.;
    for (i, w) in m[0].edges[edge].curve.weights.iter_mut().enumerate() {
        *w *= 2f64.powi(i as i32);
    }
    m[0].validate().unwrap();
    let owner = inspect(&c, &m);
    let r = owner
        .coedges()
        .iter()
        .find(|r| r.address().side == 0 && r.address().wire == wire && r.address().coedge == 0)
        .unwrap();
    let x = 0.5 + (0.0625f64 - 0.0625 * 0.0625).sqrt();
    let old = (x - HOLE[0][0]) / (HOLE[0][1] - HOLE[0][0]);
    let clock = old / (2. - old);
    contains(r.events()[0].pcurve_parameter, clock);
    let d = m[0].edges[edge].curve.domain();
    contains(r.events()[0].edge_parameter, d[0] + (d[1] - d[0]) * clock);
    contains(r.events()[0].source_world[0], x);
    owner.reverify().unwrap();
}
#[test]
fn actual_second_face_uses_its_independent_native_domain() {
    let c = curves(0.0625);
    let mut second = c.original().source().second().clone();
    second.knots_u = vec![-3., -3., -3., 5., 5., 5.];
    second.knots_v = vec![7., 7., 7., 11., 11., 11.];
    let sources = ss::FiberSources::new(c.original().source().first(), &second).unwrap();
    let original = ss::qualify_quadratic_graph_ss(&sources, Default::default()).unwrap();
    let m = [
        model(original.source().first(), Some(HOLE)),
        model(original.source().second(), None),
    ];
    let owner = qualify_coedge_roots(
        &original,
        [(&m[0], 0), (&m[1], 0)],
        1e-8,
        Default::default(),
    )
    .unwrap();
    let events: Vec<_> = owner.coedges().iter().flat_map(|r| r.events()).collect();
    assert_eq!(events.len(), 2);
    let x = 0.5 + (0.0625f64 - 0.0625 * 0.0625).sqrt();
    for e in events {
        contains(e.other_face_uv[0], -3. + 8. * x);
        assert!(e.other_face_uv[1][0] >= 7. && e.other_face_uv[1][1] <= 11.);
        assert_eq!(e.other_face_location, Location::Inside);
    }
    owner.reverify().unwrap();
}
#[test]
fn numerically_equal_pcurves_in_different_native_frames_never_prove_boundary_coincidence() {
    let c = curves(0.0625);
    let mut second = c.original().source().second().clone();
    let epsilon = 2f64.powi(-32);
    second.knots_v = vec![epsilon; 3]
        .into_iter()
        .chain(vec![1. + epsilon; 3])
        .collect();
    let sources = ss::FiberSources::new(c.original().source().first(), &second).unwrap();
    let original = ss::qualify_quadratic_graph_ss(&sources, Default::default()).unwrap();
    let m = [
        model(original.source().first(), Some(HOLE)),
        model(original.source().second(), Some(HOLE)),
    ];
    let faces = [(&m[0], 0), (&m[1], 0)];
    assert_eq!(
        m[0].loops[m[0].faces[0].holes[0]].coedges[0].pcurve,
        m[1].loops[m[1].faces[0].holes[0]].coedges[0].pcurve
    );
    assert!(qualify_coedge_roots(&original, faces, 1e-8, Default::default()).is_err());
    let precise = qualify_coedge_roots(&original, faces, 1e-12, Default::default()).unwrap();
    let events: Vec<_> = precise.coedges().iter().flat_map(|r| r.events()).collect();
    assert_eq!(events.len(), 4);
    assert!(events.iter().all(|e| e.other_boundary.is_none()));
    assert!(
        events
            .iter()
            .any(|e| e.other_face_location == Location::Inside)
    );
    assert!(
        events
            .iter()
            .any(|e| e.other_face_location == Location::Outside)
    );
    precise.reverify().unwrap();
}
