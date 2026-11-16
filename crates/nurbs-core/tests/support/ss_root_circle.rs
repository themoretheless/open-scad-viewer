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
    wire_on(fixture::plane(0.))
}
pub fn wire_on(first: nurbs_core::surface::Surface) -> ss::OriginalSsRootWire {
    wire_circle_on(first,[0.5,0.5],1.)
}
pub fn wire_circle_on(first:nurbs_core::surface::Surface, center:[f64;2],scale:f64)->ss::OriginalSsRootWire {
    circle(first,center,scale,false)
}
/// Exact single knot insertion at u = 1/2 of a clamped degree-2 Bezier patch.
/// The geometry is unchanged; only the original span structure differs.
#[allow(dead_code)]
pub fn insert_half_knot_u(s: &nurbs_core::surface::Surface) -> nurbs_core::surface::Surface {
    assert_eq!((s.degree_u, s.knots_u.as_slice()), (2, &[0., 0., 0., 1., 1., 1.][..]));
    assert!(s.weights.iter().flatten().all(|w| *w == 1.));
    let p = &s.control_points;
    let mid = |a: &Vec<Vec<f64>>, b: &Vec<Vec<f64>>| -> Vec<Vec<f64>> {
        a.iter().zip(b).map(|(x, y)| x.iter().zip(y).map(|(x, y)| x * 0.5 + y * 0.5).collect()).collect()
    };
    nurbs_core::surface::Surface {
        knots_u: vec![0., 0., 0., 0.5, 1., 1., 1.],
        control_points: vec![p[0].clone(), mid(&p[0], &p[1]), mid(&p[1], &p[2]), p[2].clone()],
        weights: vec![vec![1.; p[0].len()]; 4],
        ..s.clone()
    }
}
/// The same circle on a bowl with an interior u-knot at 1/2. The two charts
/// crossing that knot are split into one graph per original span, so the
/// wire has ten edges and two cross-carrier joints on the knot line.
#[allow(dead_code)]
pub fn wire_knotted() -> ss::OriginalSsRootWire {
    circle(fixture::plane(0.), [0.5, 0.5], 1., true)
}
/// The second (bowl) support of `wire_knotted`, before knot insertion.
#[allow(dead_code)]
pub fn bowl_on(first: &nurbs_core::surface::Surface, center: [f64; 2], scale: f64) -> nurbs_core::surface::Surface {
    let mut bowl = fixture::tangent(true);
    for row in &mut bowl.control_points {
        for p in row {
            p[2] -= 0.3125_f64.powi(2);
        }
    }
    assert_eq!([first.degree_u, first.degree_v], [1, 1]);
    let origin = first.control_points[0][0].clone();
    let u: [f64; 3] = std::array::from_fn(|k| first.control_points[1][0][k] - origin[k]);
    let v: [f64; 3] = std::array::from_fn(|k| first.control_points[0][1][k] - origin[k]);
    let normal: [f64; 3] =
        std::array::from_fn(|k| u[(k + 1) % 3] * v[(k + 2) % 3] - u[(k + 2) % 3] * v[(k + 1) % 3]);
    for row in &mut bowl.control_points {
        for p in row {
            let mut old = p.clone();
            for k in 0..2 {old[k]=center[k]+scale*(old[k]-0.5);}
            for k in 0..3 {
                p[k] = origin[k] + u[k] * old[0] + v[k] * old[1] + normal[k] * old[2];
            }
        }
    }
    bowl
}
fn circle(first: nurbs_core::surface::Surface, center: [f64; 2], scale: f64, knotted: bool) -> ss::OriginalSsRootWire {
    let mut bowl = bowl_on(&first, center, scale);
    if knotted {
        assert_eq!((center, scale), ([0.5, 0.5], 1.));
        bowl = insert_half_knot_u(&bowl);
    }
    let source = ss::FiberSources::new(&first, &bowl).unwrap();
    let systems: Vec<_> = (2..if knotted { 4 } else { 3 }).map(|span| {
        let ideal = ss::qualify_original_ss_ideal(&source, [1, 1, span, 2], Default::default()).unwrap();
        let basis = ss::qualify_ss_groebner_basis(&ideal, Default::default()).unwrap();
        ss::qualify_ss_deflated_system(&basis, Default::default()).unwrap()
    }).collect();
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
    let mut charts: Vec<([f64; 2], [f64; 2], [[f64; 2]; 2])> =
        (0..8).map(|i| (vertices[i], vertices[(i + 1) % 8], boxes[i])).collect();
    if knotted {
        // Split both charts that cross the knot u = 1/2 at the exact knot.
        let mut split = Vec::new();
        for (a, b, bx) in charts {
            if (a[0] - 0.5) * (b[0] - 0.5) < 0. {
                let mid = [0.5, if a[1] > 0.5 { 0.8125 } else { 0.1875 }];
                let side = |x: f64| if x > 0.5 { [0.48, bx[0][1]] } else { [bx[0][0], 0.52] };
                let v = if a[1] > 0.5 { [bx[1][0], 0.87] } else { [0.13, bx[1][1]] };
                split.push((a, mid, [side(a[0]), v]));
                split.push((mid, b, [side(b[0]), v]));
            } else {
                split.push((a, b, bx));
            }
        }
        charts = split;
    }
    let n = charts.len();
    // Normalized clock on the second-u original span owning a chart.
    let span_of = |c: &([f64; 2], [f64; 2], [[f64; 2]; 2])| usize::from(knotted && c.0[0].max(c.1[0]) > 0.5);
    let local = |span: usize, x: f64| if knotted { 2. * x - span as f64 } else { x };
    let mut curves = Vec::new();
    for c in &charts {
        let (a, b, bx) = *c;
        let axis = if (a[0] - b[0]).abs() < (a[1] - b[1]).abs() {
            ss::FiberAxis::SecondV
        } else {
            ss::FiberAxis::SecondU
        };
        let k = if axis == ss::FiberAxis::SecondV { 1 } else { 0 };
        let span = span_of(c);
        let mut d = [bx[0].map(|x|center[0]+scale*(x-0.5)), bx[1].map(|x|center[1]+scale*(x-0.5)), bx[0], bx[1]];
        d[2 + k] = [a[k].min(b[k]) - 0.002, b[k].max(a[k]) + 0.002];
        if knotted {
            d[2] = [d[2][0].max(0.5 * span as f64), d[2][1].min(0.5 + 0.5 * span as f64)].map(|x| local(span, x));
        }
        let result = ss::qualify_deflated_ss_fiber(&systems[span], d, axis, Default::default()).unwrap();
        let report = format!("{:?}", result.report());
        let f = result
            .into_fiber()
            .unwrap_or_else(|| panic!("circle chart {c:?} not qualified: {report}"));
        curves.push(Arc::new(ss::OriginalSsRootCurve::from_deflated(f)));
    }
    let points: Vec<_> = (0..n)
        .map(|i| {
            let start = charts[i].0;
            let t = if curves[i].free_axis() == ss::FiberAxis::SecondV {
                start[1]
            } else {
                local(span_of(&charts[i]), start[0])
            };
            Arc::new(ss::OriginalSsRootPoint::new(curves[i].clone(), t).unwrap())
        })
        .collect();
    let edges = (0..n)
        .map(|i| {
            ss::OriginalSsRootEdge::new(
                port(points[i].clone(), curves[i].clone()),
                port(points[(i + 1) % n].clone(), curves[i].clone()),
            )
            .unwrap()
        })
        .collect();
    ss::OriginalSsRootWire::new(edges).unwrap()
}
