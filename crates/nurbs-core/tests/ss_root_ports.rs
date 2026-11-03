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
    let mut bowl = fixture::tangent(true);
    for row in &mut bowl.control_points {
        for p in row {
            p[2] -= 0.3125_f64.powi(2);
        }
    }
    let source = ss::FiberSources::new(&fixture::plane(0.), &bowl).unwrap();
    let ideal = ss::qualify_original_ss_ideal(&source, [1, 1, 2, 2], Default::default()).unwrap();
    let basis = ss::qualify_ss_groebner_basis(&ideal, Default::default()).unwrap();
    let system = ss::qualify_ss_deflated_system(&basis, Default::default()).unwrap();
    let vertices: [[f64; 2]; 8] = [
        [0.6875, 0.75],
        [0.3125, 0.75],
        [0.25, 0.6875],
        [0.25, 0.3125],
        [0.3125, 0.25],
        [0.6875, 0.25],
        [0.75, 0.3125],
        [0.75, 0.6875],
    ];
    let boxes = [
        [[0.30, 0.70], [0.73, 0.84]],
        [[0.235, 0.33], [0.67, 0.77]],
        [[0.16, 0.27], [0.30, 0.70]],
        [[0.235, 0.33], [0.23, 0.33]],
        [[0.30, 0.70], [0.16, 0.27]],
        [[0.67, 0.77], [0.23, 0.33]],
        [[0.73, 0.84], [0.30, 0.70]],
        [[0.67, 0.77], [0.67, 0.77]],
    ];
    let mut curves = Vec::new();
    for i in 0..8 {
        let axis = if i == 2 || i == 6 {
            ss::FiberAxis::SecondV
        } else {
            ss::FiberAxis::SecondU
        };
        let k = if axis == ss::FiberAxis::SecondV { 1 } else { 0 };
        let mut d = [boxes[i][0], boxes[i][1], boxes[i][0], boxes[i][1]];
        let a = vertices[i][k];
        let b = vertices[(i + 1) % 8][k];
        d[2 + k] = [a.min(b) - 0.002, b.max(a) + 0.002];
        let result = ss::qualify_deflated_ss_fiber(&system, d, axis, Default::default()).unwrap();
        let report = format!("{:?}", result.report());
        let f = result
            .into_fiber()
            .unwrap_or_else(|| panic!("circle chart {i} not qualified: {report}"));
        curves.push(Arc::new(ss::OriginalSsRootCurve::from_deflated(f)));
    }
    let points: Vec<_> = (0..8)
        .map(|i| {
            let k = if curves[i].free_axis() == ss::FiberAxis::SecondV {
                1
            } else {
                0
            };
            Arc::new(ss::OriginalSsRootPoint::new(curves[i].clone(), vertices[i][k]).unwrap())
        })
        .collect();
    let edges = (0..8)
        .map(|i| {
            ss::OriginalSsRootEdge::new(
                port(points[i].clone(), curves[i].clone()),
                port(points[(i + 1) % 8].clone(), curves[i].clone()),
            )
            .unwrap()
        })
        .collect();
    let wire = ss::OriginalSsRootWire::new(edges).unwrap();
    let inside = ss::qualify_original_ss_winding(&wire, 0, [0.5, 0.5], Default::default()).unwrap();
    assert_eq!(inside.winding(), 1);
    inside.reverify().unwrap();
    let reversed=ss::OriginalSsRootWire::new(wire.edges().iter().rev().map(|e|
        ss::OriginalSsRootEdge::new(e.end().clone(),e.start().clone()).unwrap()).collect()).unwrap();
    assert_eq!(ss::qualify_original_ss_winding(&reversed,0,[0.5,0.5],Default::default()).unwrap().winding(),-1);
    assert_eq!(
        ss::qualify_original_ss_winding(&wire, 1, [0.9, 0.5], Default::default())
            .unwrap()
            .winding(),
        0
    );
}
