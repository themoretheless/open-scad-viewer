#[path = "support/ss_root_circle.rs"]
mod circle_fixture;
use nurbs_core::ss_intersection as ss;
use std::sync::Arc;
#[allow(dead_code)]
#[path = "support/ss_original_ideal.rs"]
mod fixture;
fn curve(axis: ss::FiberAxis) -> Arc<ss::OriginalSsRootCurve> {
    let source = ss::FiberSources::new(&fixture::plane(0.), &fixture::tangent(false)).unwrap();
    let ideal = ss::qualify_original_ss_ideal(&source, [1, 1, 2, 2], Default::default()).unwrap();
    let basis = ss::qualify_ss_groebner_basis(&ideal, Default::default()).unwrap();
    let system = ss::qualify_ss_deflated_system(&basis, Default::default()).unwrap();
    let mut domain = [[0.4, 0.6], [0.1, 0.9], [0.4, 0.6], [0.1, 0.9]];
    domain[if axis == ss::FiberAxis::FirstV { 1 } else { 3 }] = [0.25, 0.75];
    let fiber = ss::qualify_deflated_ss_fiber(&system, domain, axis, Default::default())
        .unwrap()
        .into_fiber()
        .unwrap();
    Arc::new(ss::OriginalSsRootCurve::from_deflated(fiber))
}
fn port(
    point: Arc<ss::OriginalSsRootPoint>,
    target: Arc<ss::OriginalSsRootCurve>,
) -> ss::QualifiedOriginalSsPort {
    ss::bind_original_ss_port(point, target, [0; 4], Default::default()).unwrap()
}
#[test]
fn mixed_axis_join_preserves_root_valued_target_clock() {
    let a = curve(ss::FiberAxis::FirstV);
    let b = curve(ss::FiberAxis::SecondV);
    let point = Arc::new(ss::OriginalSsRootPoint::new(a, 0.375).unwrap());
    let join = port(point, b);
    assert_eq!(join.kind(), ss::OriginalSsPortKind::OriginalParameterRoot);
    let q = num_rational::BigRational::from_float(0.375).unwrap();
    assert!(join.parameter_bounds()[0] <= q && q <= join.parameter_bounds()[1]);
    join.reverify().unwrap();
}
#[test]
fn wire_requires_literal_shared_selectors_and_retains_reverse_direction() {
    let c = curve(ss::FiberAxis::SecondV);
    let a = Arc::new(ss::OriginalSsRootPoint::new(c.clone(), 0.25).unwrap());
    let b = Arc::new(ss::OriginalSsRootPoint::new(c.clone(), 0.75).unwrap());
    let forward =
        ss::OriginalSsRootEdge::new(port(a.clone(), c.clone()), port(b.clone(), c.clone()))
            .unwrap();
    let reverse =
        ss::OriginalSsRootEdge::new(port(b.clone(), c.clone()), port(a.clone(), c.clone()))
            .unwrap();
    assert!(forward.forward());
    assert!(!reverse.forward());
    // A doubled edge is closed incidence, deliberately not a Jordan/material proof.
    let wire = ss::OriginalSsRootWire::new(vec![forward.clone(), reverse]).unwrap();
    wire.reverify().unwrap();
    let outside =
        ss::qualify_original_ss_winding(&wire, 0, [0.25, 0.5], Default::default()).unwrap();
    assert_eq!(outside.winding(), 0);
    outside.reverify().unwrap();
    assert!(
        ss::qualify_original_ss_winding(
            &wire,
            0,
            [0.5, 0.5],
            ss::OriginalSsWindingLimits {
                max_segments: 2,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(ss::OriginalSsRootWire::new(vec![forward.clone()]).is_err());
    assert!(
        ss::OriginalSsRootEdge::new(port(a.clone(), c.clone()), port(a.clone(), c.clone()))
            .is_err()
    );
    let duplicate = Arc::new(ss::OriginalSsRootPoint::new(Arc::new((*c).clone()), 0.25).unwrap());
    let fake = ss::OriginalSsRootEdge::new(port(b, c.clone()), port(duplicate, c.clone())).unwrap();
    assert!(ss::OriginalSsRootWire::new(vec![forward, fake]).is_err());
    assert!(ss::bind_original_ss_port(a, c, [1, 0, 0, 0], Default::default()).is_err());
}
#[test]
fn closed_curved_mixed_axis_wire_classifies_uv_inside_and_outside() {
    let wire = circle_fixture::wire();
    let inside = ss::qualify_original_ss_winding(&wire, 0, [0.5, 0.5], Default::default()).unwrap();
    assert_eq!(inside.winding(), 1);
    let box_inside = wire
        .winding_box(0, [[0.49, 0.51]; 2], Default::default())
        .unwrap();
    assert_eq!(box_inside.winding(), 1);
    box_inside.reverify().unwrap();
    assert_eq!(
        wire.winding_box(0, [[0.85, 0.95], [0.45, 0.55]], Default::default())
            .unwrap()
            .winding(),
        0
    );
    assert!(
        wire.winding_box(
            0,
            [[0.49, 0.51], [0.80, 0.83]],
            ss::OriginalSsWindingLimits {
                max_segments: 16,
                ..Default::default()
            }
        )
        .is_err()
    );
    inside.reverify().unwrap();
    let reversed = ss::OriginalSsRootWire::new(
        wire.edges()
            .iter()
            .rev()
            .map(|e| ss::OriginalSsRootEdge::new(e.end().clone(), e.start().clone()).unwrap())
            .collect(),
    )
    .unwrap();
    assert_eq!(
        ss::qualify_original_ss_winding(&reversed, 0, [0.5, 0.5], Default::default())
            .unwrap()
            .winding(),
        -1
    );
    assert_eq!(
        ss::qualify_original_ss_winding(&wire, 1, [0.9, 0.5], Default::default())
            .unwrap()
            .winding(),
        0
    );
}

#[test]
fn original_graph_window_uses_exact_source_clocks_and_rejects_escape() {
    let c = curve(ss::FiberAxis::FirstV);
    let q = |x| num_rational::BigRational::from_float(x).unwrap();
    let original = [[0.45, 0.55], [0.3, 0.7], [0.45, 0.55], [0.3, 0.7]]
        .map(|d| d.map(q));
    let other = curve(ss::FiberAxis::SecondV);
    c.qualify_subarc_window(&other, [0.4, 0.6], Default::default()).unwrap().reverify().unwrap();
    assert!(c.qualify_subarc_window(&other, [0.1, 0.2], Default::default()).is_err());
    let proof = c.qualify_graph_window(original.clone()).unwrap();
    assert_eq!(proof.original_window(), &original);
    assert!(Arc::ptr_eq(proof.curve(), &c));
    proof.reverify().unwrap();
    let mut escaped = original.clone();
    escaped[1][0] = q(0.125);
    assert!(c.qualify_graph_window(escaped).is_err());
    let mut inverted = original;
    inverted[2] = [q(0.55), q(0.45)];
    assert!(c.qualify_graph_window(inverted).is_err());
}
