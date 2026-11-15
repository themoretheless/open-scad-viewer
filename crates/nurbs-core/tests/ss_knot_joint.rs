//! Cross-carrier joints on an interior original knot line.
use nurbs_core::ss_intersection as ss;
#[path = "support/ss_root_circle.rs"]
mod fixture;
use num_rational::BigRational as Q;

fn code(e: nurbs_core::Error) -> String {
    format!("{e:?}")
}

#[test]
fn agreeing_span_carriers_certify_both_knot_crossings() {
    let wire = fixture::wire_knotted();
    assert_eq!(wire.edges().len(), 10);
    // Plain single-graph joints still refuse a joint touching the knot.
    assert!(wire.qualify_joints(12, Default::default()).is_err());
    let joints = wire.qualify_knot_aware_joints(12, Default::default()).unwrap();
    let knots: Vec<_> = joints.iter().filter_map(|j| j.knot()).collect();
    assert_eq!(knots.len(), 2);
    let half = Q::new(1.into(), 2.into());
    for k in &knots {
        assert_eq!(k.coordinate(), 2);
        assert_eq!(k.knot(), &half);
        for w in k.windows() {
            w.reverify().unwrap();
        }
    }
    // The top crossing runs toward decreasing u, the bottom one increasing.
    assert!(!knots[0].increasing() && knots[1].increasing());
    for j in &joints {
        j.reverify().unwrap();
    }
}

#[test]
fn disagreeing_selectors_and_non_adjacent_carriers_refuse() {
    let wire = fixture::wire_knotted();
    let e = wire.edges();
    let n = e.len();
    let crossings: Vec<usize> = (0..n)
        .filter(|i| e[*i].qualify_knot_successor(&e[(i + 1) % n], 12, Default::default()).is_ok())
        .collect();
    assert_eq!(crossings.len(), 2);
    // Top incoming edge with the bottom outgoing edge: the spans are adjacent
    // and the free coordinate is common, but the knot-line roots differ.
    let (top, bottom) = (crossings[0], crossings[1]);
    let err = e[top]
        .qualify_knot_successor(&e[(bottom + 1) % n], 12, Default::default())
        .unwrap_err();
    assert!(code(err).contains("NURBS_SS_KNOT_JOINT_UNPROVED"));
    // A regular joint inside one span is not a knot joint.
    let plain = fixture::wire();
    let p = plain.edges();
    let err = p[0].qualify_knot_successor(&p[1], 12, Default::default()).unwrap_err();
    assert!(code(err).contains("NURBS_SS_KNOT_JOINT_UNPROVED"));
}

#[test]
fn unresolved_enclosure_at_the_knot_refuses() {
    let wire = fixture::wire_knotted();
    let e = wire.edges();
    let starved = ss::OriginalSsCurveLimits {
        max_refinements: 0,
        native_rectangles: 1,
        interval_operations: 1,
        world_rectangles: 1,
        ..Default::default()
    };
    assert!(e[0].qualify_knot_successor(&e[1], 0, starved).is_err());
    assert!(e[0].qualify_knot_successor(&e[1], 12, Default::default()).is_ok());
}
