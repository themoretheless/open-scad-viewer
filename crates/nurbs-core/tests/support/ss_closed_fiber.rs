#![allow(dead_code)] // shared by several test targets; each one uses a subset
use nurbs_core::{
    ss_intersection::{self as ss, FiberAxis, FiberSources, QualifiedFiber},
    surface::Surface,
};
use std::sync::Arc;
pub fn plane() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
pub fn circle() -> FiberSources {
    let a = plane();
    let mut b = a.clone();
    b.degree_u = 2;
    b.degree_v = 2;
    b.knots_u = vec![0., 0., 0., 1., 1., 1.];
    b.knots_v = b.knots_u.clone();
    let q = [0.25, -0.25, 0.25];
    b.control_points = (0..3)
        .map(|i| {
            (0..3)
                .map(|j| vec![i as f64 / 2., j as f64 / 2., q[i] + q[j] - 0.0625])
                .collect()
        })
        .collect();
    b.weights = vec![vec![1.; 3]; 3];
    FiberSources::new(&a, &b).unwrap()
}
pub fn family(source: &FiberSources, ellipses: &[([f64; 2], [f64; 2])]) -> Vec<QualifiedFiber> {
    let mut charts = vec![];
    for &(center, radius) in ellipses {
        for i in 0..32 {
            let angle = i as f64 * std::f64::consts::TAU / 32.;
            let half = std::f64::consts::PI / 24.;
            // Choose in the ellipse's normalized parameter metric. The native
            // operator still independently qualifies every proposed chart.
            let free = if angle.sin().abs() >= angle.cos().abs() {
                0
            } else {
                1
            };
            let points = [angle - half, angle, angle + half].map(|t| {
                [
                    center[0] + radius[0] * t.cos(),
                    center[1] + radius[1] * t.sin(),
                ]
            });
            let bounds: [[f64; 2]; 2] = std::array::from_fn(|k| {
                [
                    points.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min),
                    points
                        .iter()
                        .map(|p| p[k])
                        .fold(f64::NEG_INFINITY, f64::max),
                ]
            });
            let mut domain: [[f64; 2]; 4] = std::array::from_fn(|k| {
                [
                    bounds[k % 2][0] - 0.2 * radius[k % 2],
                    bounds[k % 2][1] + 0.2 * radius[k % 2],
                ]
            });
            domain[free] = bounds[free];
            domain[2 + free] = [
                bounds[free][0] - 0.04 * radius[free],
                bounds[free][1] + 0.04 * radius[free],
            ];
            let axis = if free == 0 {
                FiberAxis::FirstU
            } else {
                FiberAxis::FirstV
            };
            let r = source.certify_contracted(domain, axis, 100000).unwrap();
            let reason = r.reason();
            charts.push(
                r.into_fiber()
                    .unwrap_or_else(|| panic!("chart {} / {i}: {reason}", charts.len() / 32)),
            );
        }
    }
    charts
}
pub fn circle_family(source: &FiberSources) -> Vec<QualifiedFiber> {
    family(source, &[([0.5, 0.5], [0.25, 0.25])])
}
pub fn owner(
    charts: &[QualifiedFiber],
    domain: Option<[[f64; 2]; 4]>,
) -> Arc<ss::DiscoveredMixedFiberComponents> {
    let r = if let Some(d) = domain {
        ss::cover_mixed_fiber_box(charts, d, Default::default())
    } else {
        ss::cover_mixed_fiber_surfaces(charts, Default::default())
    }
    .unwrap();
    let report = r.report().clone();
    let cover = r
        .into_coverage()
        .unwrap_or_else(|| panic!("coverage: {report:?}"));
    let r = ss::connect_mixed_fiber_cover_automatic(&cover, Default::default()).unwrap();
    let reason = r.reason();
    if r.components().is_none() {
        let failed = r
            .searches()
            .iter()
            .filter(|s| !s.reason.starts_with("qualified"))
            .map(|s| {
                (
                    s.charts,
                    &s.reason,
                    s.rectangles,
                    s.attempts.len(),
                    s.attempts.last().map(|a| &a.reason),
                )
            })
            .collect::<Vec<_>>();
        eprintln!("component work={} failed={failed:?}", r.rectangles());
    }
    Arc::new(
        r.into_components()
            .unwrap_or_else(|| panic!("components: {reason}")),
    )
}
pub fn two_ovals() -> FiberSources {
    let a = plane();
    let mut b = a.clone();
    b.degree_u = 2;
    b.degree_v = 2;
    b.knots_u = vec![0., 0., 0., 0.25, 0.5, 0.75, 1., 1., 1.];
    b.knots_v = vec![0., 0., 0., 1., 1., 1.];
    let x = [0., 0.125, 0.375, 0.625, 0.875, 1.];
    let z = [0.0625, -0.0625, 0.0625, 0.0625, -0.0625, 0.0625];
    let q = [0.25, -0.25, 0.25];
    b.control_points = (0..6)
        .map(|i| {
            (0..3)
                .map(|j| vec![x[i], j as f64 / 2., z[i] + q[j]])
                .collect()
        })
        .collect();
    b.weights = vec![vec![1.; 3]; 6];
    FiberSources::new(&a, &b).unwrap()
}
