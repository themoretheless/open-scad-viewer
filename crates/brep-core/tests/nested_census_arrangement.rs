#[path = "support/nested_graph_body.rs"]
mod graph;
#[path = "support/closed_solid_policy.rs"]
mod policy;
use brep_core::{closed_solid as solid, mixed_root_model as mixed};
use nurbs_core::{ss_intersection as ss, trim_domain::Location};
use std::sync::Arc;
const COUNT: usize = 128;
fn family(source: &ss::FiberSources, radius: f64) -> Vec<ss::QualifiedFiber> {
    (0..COUNT)
        .map(|i| {
            let angle = i as f64 * std::f64::consts::TAU / COUNT as f64;
            let half = 0.7 * std::f64::consts::TAU / COUNT as f64;
            let points = [angle - half, angle, angle + half]
                .map(|t| [0.5 + radius * t.cos(), 0.5 + radius * t.sin()]);
            let bounds: [[f64; 2]; 2] = std::array::from_fn(|k| {
                [
                    points.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min),
                    points
                        .iter()
                        .map(|p| p[k])
                        .fold(f64::NEG_INFINITY, f64::max),
                ]
            });
            let free = if angle.sin().abs() >= angle.cos().abs() {
                0
            } else {
                1
            };
            let axis = if free == 0 {
                ss::FiberAxis::FirstU
            } else {
                ss::FiberAxis::FirstV
            };
            let mut failures = Vec::new();
            for pad in [0.03, 0.06, 0.09, 0.12, 0.16] {
                let mut domain = std::array::from_fn(|k| {
                    [
                        bounds[k % 2][0] - pad * radius,
                        bounds[k % 2][1] + pad * radius,
                    ]
                });
                domain[free] = bounds[free];
                domain[2 + free] = [
                    bounds[free][0] - 0.005 * radius,
                    bounds[free][1] + 0.005 * radius,
                ];
                let r = source.certify_contracted(domain, axis, 100000).unwrap();
                failures.push((pad, r.reason(), r.root_image()));
                if let Some(f) = r.into_fiber() {
                    return f;
                }
            }
            panic!("quartic chart {radius}/{i}: {failures:?}")
        })
        .collect()
}
fn index(axis: ss::FiberAxis) -> usize {
    match axis {
        ss::FiberAxis::FirstU => 0,
        ss::FiberAxis::FirstV => 1,
        ss::FiberAxis::SecondU => 2,
        ss::FiberAxis::SecondV => 3,
    }
}
fn wire(charts: &[ss::QualifiedFiber], radius: f64) -> ss::OriginalSsRootWire {
    let curves: Vec<_> = charts
        .iter()
        .cloned()
        .map(|c| Arc::new(ss::OriginalSsRootCurve::from_native(c)))
        .collect();
    let points: Vec<_> = curves
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let angle = (i as f64 - 0.5) * std::f64::consts::TAU / COUNT as f64;
            let uv = [0.5 + radius * angle.cos(), 0.5 + radius * angle.sin()];
            Arc::new(ss::OriginalSsRootPoint::new(c.clone(), uv[index(c.free_axis()) % 2]).unwrap())
        })
        .collect();
    ss::OriginalSsRootWire::new(
        (0..COUNT)
            .map(|i| {
                let bind = |p: Arc<ss::OriginalSsRootPoint>| {
                    ss::bind_original_ss_port(p, curves[i].clone(), [0; 4], Default::default())
                        .unwrap()
                };
                ss::OriginalSsRootEdge::new(
                    bind(points[i].clone()),
                    bind(points[(i + 1) % COUNT].clone()),
                )
                .unwrap()
            })
            .collect(),
    )
    .unwrap()
}
#[test]
fn nested_original_components_build_three_distinct_retained_pieces() {
    let a = solid::qualify(
        brep_core::cuboid([0., 0., -10.], [1., 1., 0.]).unwrap(),
        policy::policy(),
    )
    .unwrap()
    .into_solid()
    .unwrap();
    let mut p = policy::policy();
    p.geometry.boundary.contacts.cells_per_pair = 1024;
    p.geometry.boundary.contacts.domain_cells_per_pair = 10000;
    let admitted = solid::qualify(graph::body(), p).unwrap();
    assert!(
        admitted.solid().is_some(),
        "Quartic original body must independently qualify"
    );
    let b = admitted.into_solid().unwrap();
    let source =
        ss::FiberSources::new(&a.model().faces[1].surface, &b.model().faces[1].surface).unwrap();
    let mut charts = family(&source, 0.3125);
    charts.extend(family(&source, 0.15625));
    let wires = [
        wire(&charts[..COUNT], 0.3125),
        wire(&charts[COUNT..], 0.15625),
    ];
    let loops: Vec<_> = wires
        .iter()
        .map(|wire| {
            Arc::new(
                mixed::qualify_paired_interior_loop(
                    &a,
                    1,
                    &b,
                    1,
                    wire,
                    [[0.5; 2]; 2],
                    Default::default(),
                )
                .unwrap(),
            )
        })
        .collect();
    let quotient: Vec<_> = charts
        .iter()
        .map(|c| {
            let r = source
                .certify_quotient(
                    c.domain(),
                    c.free_axis(),
                    ss::FiberLimits {
                        preconditioner: c.report().operator.as_ref().map(|o| o.preconditioner()),
                        ..Default::default()
                    },
                )
                .unwrap();
            let diagnostic = r.report().clone();
            r.into_fiber()
                .unwrap_or_else(|| panic!("quartic census graph: {diagnostic:?}"))
        })
        .collect();
    let r = ss::cover_quotient_fiber_surfaces(
        &quotient,
        ss::CoverageLimits {
            max_cells: 65536,
            max_rectangles: 1000000,
            max_depth: 48,
        },
    )
    .unwrap();
    let diagnostic = r.report().clone();
    let coverage = r
        .into_coverage()
        .unwrap_or_else(|| panic!("quartic census: {diagnostic:?}"));
    eprintln!(
        "nested census: charts={}, cells={}, rectangles={}",
        quotient.len(),
        coverage.report().cells,
        coverage.report().rectangles
    );
    let anchors: Vec<_> = quotient
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let d = c.domain()[index(c.free_axis())];
            mixed::InteriorCensusAnchor {
                component: i / COUNT,
                edge: i % COUNT,
                parameter: (d[0] + d[1]) * 0.5,
                phase: [0; 4],
            }
        })
        .collect();
    let census = Arc::new(
        mixed::qualify_interior_ss_census(loops, coverage, &anchors, Default::default()).unwrap(),
    );
    assert_eq!(census.components().len(), 2);
    assert!(census.separation_queries() > 0);
    census.reverify().unwrap();
    for support in 0..2 {
        let arrangement = Arc::new(
            census
                .build_face_arrangement(support, Default::default())
                .unwrap(),
        );
        assert_eq!(arrangement.parents(), &[None, Some(0)]);
        assert_eq!(arrangement.region_faces().len(), 3);
        assert!(arrangement.face_for_signature(&[false, true]).is_err());
        let topology = arrangement.model().topology();
        assert_eq!(topology.faces[1].holes.len(), 1);
        assert_eq!(topology.faces[arrangement.region_faces()[1]].holes.len(), 1);
        assert!(
            topology.faces[arrangement.region_faces()[2]]
                .holes
                .is_empty()
        );
        for (rectangle, signature) in [
            ([[0.48, 0.52]; 2], [true, true]),
            ([[0.48, 0.52], [0.71, 0.73]], [true, false]),
            ([[0.03, 0.05]; 2], [false, false]),
        ] {
            let region = census
                .classify_rectangle(support, rectangle, Default::default())
                .unwrap();
            assert_eq!(region.location(), Location::Inside);
            assert_eq!(region.signature(), Some(signature.as_slice()));
            arrangement.face_for_signature(&signature).unwrap();
        }
        for (rectangle, signature, first_keep, second_keep) in [
            ([[0.499, 0.501]; 2], [true, true], false, false),
            ([[0.499, 0.501], [0.724, 0.726]], [true, false], true, true),
            ([[0.049, 0.051]; 2], [false, false], false, false),
        ] {
            let proof = arrangement
                .qualify_boolean_rectangle(
                    solid::BooleanMaterialOp::Difference,
                    rectangle,
                    &[[1., 0.317, 0.173], [0.239, 1., 0.419], [0.137, 0.283, 1.]],
                    Default::default(),
                )
                .unwrap();
            assert_eq!(
                proof.face(),
                arrangement.face_for_signature(&signature).unwrap()
            );
            assert_eq!(
                proof.material().boolean_material().retain(),
                if support == 0 {
                    first_keep
                } else {
                    second_keep
                }
            );
            assert_eq!(
                proof
                    .material()
                    .boolean_material()
                    .reverse_material_normal(),
                support == 1 && second_keep
            );
        }
    }
}
