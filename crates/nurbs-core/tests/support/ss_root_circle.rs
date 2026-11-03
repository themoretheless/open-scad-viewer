use nurbs_core::ss_intersection as ss;
use std::sync::Arc;
#[allow(dead_code)]
#[path = "ss_original_ideal.rs"]
mod fixture;
fn port(
    point: Arc<ss::OriginalSsRootPoint>,
    target: Arc<ss::OriginalSsRootCurve>,
) -> ss::QualifiedOriginalSsPort {
    ss::bind_original_ss_port(point, target, [0; 4], Default::default()).unwrap()
}
pub fn wire() -> ss::OriginalSsRootWire {
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
    ss::OriginalSsRootWire::new(edges).unwrap()
}
