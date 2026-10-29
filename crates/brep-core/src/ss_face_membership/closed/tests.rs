use super::*;
use crate::ss_face_membership::tests::{curves, fixtures, model};
use nurbs_core::ss_intersection as ss;
fn membership<'a>(c: &'a QualifiedGraphCurves, m: &'a [Model; 2]) -> QualifiedMembership<'a> {
    qualify(c, [(&m[0], 0), (&m[1], 0)], 1e-8, Default::default()).unwrap()
}
#[test]
fn internal_original_ellipse_authors_two_real_sectors_on_both_supports() {
    let c = curves(0.0625);
    let m = fixtures(&c, None);
    let before = m.clone();
    let member = membership(&c, &m);
    let out = partition_closed_branches(&member, Default::default()).unwrap();
    assert_eq!(out.regions().len(), 2);
    for r in out.regions() {
        let model = &out.models()[r.model];
        assert_eq!(model.faces.len(), 2);
        assert_eq!(model.faces[r.faces[0]].holes.len(), 1);
        assert!(model.faces[r.faces[1]].holes.is_empty());
        assert_eq!(r.edges.len(), 4);
        assert_eq!(r.vertices.len(), 4);
        assert_eq!(model.shells[0].faces.len(), 2);
        for e in &r.edges {
            let saved = &c.edges()[e.original_edge].world;
            assert_eq!(
                model.edges[e.edge].curve,
                if e.reversed {
                    saved.reverse().unwrap()
                } else {
                    saved.clone()
                }
            );
        }
        model.validate().unwrap();
    }
    for e in &out.regions()[0].edges {
        let other = out.regions()[1]
            .edges
            .iter()
            .find(|f| f.original_edge == e.original_edge)
            .unwrap();
        let a = &out.models()[0].edges[e.edge].curve;
        let b = &out.models()[1].edges[other.edge].curve;
        assert_eq!(a, b);
    }
    out.reverify().unwrap();
    assert_eq!(m, before);
}
#[test]
fn complete_original_holes_move_to_the_correct_inner_or_outer_sector() {
    let c = curves(0.0625);
    for (hole, expected) in [
        ([[0.4375, 0.5625], [0.4375, 0.5625]], 1usize),
        ([[0.8125, 0.9375], [0.125, 0.25]], 0usize),
    ] {
        let m = fixtures(&c, Some(hole));
        let old_hole = m[0].faces[0].holes[0];
        let before = m.clone();
        let member = membership(&c, &m);
        let out = partition_closed_branches(&member, Default::default()).unwrap();
        let r = &out.regions()[0];
        let actual = &out.models()[0];
        assert!(actual.faces[r.faces[expected]].holes.contains(&old_hole));
        assert!(
            !actual.faces[r.faces[1 - expected]]
                .holes
                .contains(&old_hole)
        );
        assert_eq!(actual.loops[old_hole], m[0].loops[old_hole]);
        out.reverify().unwrap();
        assert_eq!(m, before);
    }
}
#[test]
fn excluded_closed_ss_branch_does_not_author_a_false_retained_region() {
    let c = curves(0.0625);
    let m = [
        model(c.original().source().first(), None),
        model(
            c.original().source().second(),
            Some([[0.125, 0.875], [0.125, 0.875]]),
        ),
    ];
    let member = membership(&c, &m);
    let out = partition_closed_branches(&member, Default::default()).unwrap();
    assert!(out.regions().is_empty());
    assert_eq!(out.models(), m);
    assert_eq!(c.original().kind(), QuadraticGraphSSKind::ClosedEllipse);
    out.reverify().unwrap();
}
#[test]
fn independent_native_domains_keep_the_same_world_contour_and_actual_uv() {
    let c = curves(0.0625);
    let mut second = c.original().source().second().clone();
    second.knots_u = vec![-3., -3., -3., 5., 5., 5.];
    second.knots_v = vec![7., 7., 7., 11., 11., 11.];
    let source = ss::FiberSources::new(c.original().source().first(), &second).unwrap();
    let original = ss::qualify_quadratic_graph_ss(&source, Default::default()).unwrap();
    let c = ss::construct_graph_curves(
        &original,
        ss::GraphCurveTolerances {
            world: 1e-7,
            uv: 1e-9,
        },
        Default::default(),
    )
    .unwrap();
    let m = fixtures(&c, None);
    let member = membership(&c, &m);
    let out = partition_closed_branches(&member, Default::default()).unwrap();
    let r = &out.regions()[1];
    let actual = &out.models()[r.model];
    let ring = &actual.loops[actual.faces[r.faces[1]].outer];
    assert!(
        ring.coedges
            .iter()
            .flat_map(|c| &c.pcurve.control_points)
            .all(|p| p[1] >= 7. && p[1] <= 11.)
    );
    assert_eq!(actual.faces[r.faces[1]].surface, second);
    out.reverify().unwrap();
}
#[test]
fn original_closed_membership_replay_and_shared_budgets_are_mandatory() {
    let c = curves(0.0625);
    let m = fixtures(&c, None);
    let member = membership(&c, &m);
    let mut out = partition_closed_branches(&member, Default::default()).unwrap();
    let exact = ClosedBranchLimits {
        cells: out.cells(),
        pairs: out.pairs(),
        operations: out.operations(),
    };
    assert!(partition_closed_branches(&member, exact).is_ok());
    assert!(
        partition_closed_branches(
            &member,
            ClosedBranchLimits {
                operations: exact.operations - 1,
                ..exact
            }
        )
        .is_err()
    );
    assert!(partition_closed_branches(&member, ClosedBranchLimits { cells: 1, ..exact }).is_err());
    assert!(partition_closed_branches(&member, ClosedBranchLimits { pairs: 1, ..exact }).is_err());
    let v = out.regions[0].vertices[0];
    out.models[0].vertices[v].point[2] += 0.01;
    assert!(out.reverify().is_err());
}
#[test]
fn two_support_faces_in_one_model_keep_two_sectors_each_in_original_shells() {
    let c = curves(0.0625);
    let c = c
        .original()
        .construct_graph_curves_with_dyadic_proposals(
            ss::GraphCurveTolerances {
                world: 1e-7,
                uv: 1e-9,
            },
            Default::default(),
        )
        .unwrap();
    let mut m = model(c.original().source().first(), None);
    let other = model(c.original().source().second(), None);
    let vo = m.vertices.len();
    let eo = m.edges.len();
    let lo = m.loops.len();
    m.vertices.extend(other.vertices.clone());
    m.edges.extend(other.edges.iter().cloned().map(|mut e| {
        for v in &mut e.vertices {
            *v += vo;
        }
        e
    }));
    m.loops.extend(other.loops.iter().cloned().map(|mut l| {
        for c in &mut l.coedges {
            c.edge += eo;
        }
        l
    }));
    let mut f = other.faces[0].clone();
    f.outer += lo;
    m.faces.push(f);
    m.shells.push(crate::Shell {
        faces: vec![crate::FaceUse {
            face: 1,
            reversed: false,
        }],
        closed: false,
    });
    m.rebuild_topology_ids();
    m.validate().unwrap();
    let before = m.clone();
    let member = qualify(&c, [(&m, 0), (&m, 1)], 1e-8, Default::default()).unwrap();
    let out = partition_closed_branches(&member, Default::default()).unwrap();
    assert_eq!(out.models().len(), 1);
    assert_eq!(out.side_model(), [0, 0]);
    assert_eq!(out.models()[0].faces.len(), 4);
    assert!(out.models()[0].shells.iter().all(|s| s.faces.len() == 2));
    out.reverify().unwrap();
    let sides = qualify_closed_graph_sides(&out, Default::default()).unwrap();
    assert_eq!(sides.faces().len(), 4);
    assert!(sides.faces().iter().all(|f| f.model == 0));
    assert_eq!(
        sides
            .faces()
            .iter()
            .map(|f| f.face)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        4
    );
    sides.reverify().unwrap();
    assert_eq!(m, before);
}

#[test]
fn nonplanar_original_intersection_keeps_its_actual_3d_curve() {
    let c = curves(0.0625);
    let mut first = c.original().source().first().clone();
    let mut second = c.original().source().second().clone();
    for support in [&mut first, &mut second] {
        for row in &mut support.control_points {
            for p in row {
                p[2] += 0.125 * p[0] * p[1] + 0.125 * p[0] - 0.25 * p[1] + 0.0625;
            }
        }
    }
    let source = ss::FiberSources::new(&first, &second).unwrap();
    let original = ss::qualify_quadratic_graph_ss(&source, Default::default()).unwrap();
    assert_eq!(original.kind(), QuadraticGraphSSKind::ClosedEllipse);
    let c = original
        .construct_graph_curves_with_dyadic_proposals(
            ss::GraphCurveTolerances {
                world: 1e-7,
                uv: 1e-9,
            },
            Default::default(),
        )
        .unwrap();
    let m = fixtures(&c, None);
    let member = membership(&c, &m);
    let out = partition_closed_branches(&member, Default::default()).unwrap();
    let z: Vec<_> = out.regions()[0]
        .vertices
        .iter()
        .map(|&v| out.models()[0].vertices[v].point[2])
        .collect();
    assert!(
        z.iter().copied().fold(f64::NEG_INFINITY, f64::max)
            - z.iter().copied().fold(f64::INFINITY, f64::min)
            > 0.01
    );
    // All four cardinal points lie in this plane; an interior point must
    // leave it because the common original height includes the u*v term.
    let first_edge = &out.regions()[0].edges[0];
    let point = out.models()[0].edges[first_edge.edge]
        .curve
        .evaluate(0.5)
        .unwrap()
        .point;
    assert!((point[2] - (0.1875 * point[0] - 0.1875 * point[1] + 0.03125)).abs() > 1e-4);
    for r in out.regions() {
        for e in &r.edges {
            assert!(
                c.edges()[e.original_edge].world_error_upper <= out.models()[r.model].tolerance_mm
            );
        }
    }
    let sides = qualify_closed_graph_sides(&out, Default::default()).unwrap();
    assert_eq!(sides.faces().iter().filter(|f| f.above).count(), 2);
    sides.reverify().unwrap();
    out.reverify().unwrap();
}
