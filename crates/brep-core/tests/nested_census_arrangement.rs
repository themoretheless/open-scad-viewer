#[path = "support/nested_graph_body.rs"]
mod graph;
#[path = "support/closed_solid_policy.rs"]
mod policy;
use brep_core::{closed_solid as solid, mixed_root_model as mixed};
use nurbs_core::{ss_intersection as ss, trim_domain::Location};
use std::sync::Arc;
const COUNT: usize = 128;
fn family(source: &ss::FiberSources, radius: f64, mirrored: bool) -> Vec<ss::QualifiedFiber> {
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
                if mirrored {
                    domain[2] = [
                        (1. - domain[2][1]).next_down(),
                        (1. - domain[2][0]).next_up(),
                    ];
                }
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
    run(false, false);
}
#[test]
fn nested_components_assign_original_holes_to_exactly_one_piece() {
    run(true, false);
}
#[test]
fn paired_sewing_respects_a_reversed_original_uv_clock() {
    run(false, true);
}
fn operands(
    with_holes: bool,
    mirrored: bool,
) -> (
    solid::QualifiedClosedSolid,
    usize,
    solid::QualifiedClosedSolid,
) {
    let model = if with_holes {
        let square = |x: f64, y: f64, h: f64| {
            vec![
                [x - h, y - h],
                [x - h, y + h],
                [x + h, y + h],
                [x + h, y - h],
            ]
        };
        brep_core::extrude_polygon_with_holes(
            &[[0., 0.], [1., 0.], [1., 1.], [0., 1.]],
            &[
                square(0.5, 0.5, 0.04),
                square(0.5, 0.725, 0.01),
                square(0.05, 0.05, 0.02),
            ],
            -10.,
            0.,
        )
        .unwrap()
    } else {
        brep_core::cuboid([0., 0., -10.], [1., 1., 0.]).unwrap()
    };
    let aface = model
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == 0.)
        })
        .unwrap();
    let a = solid::qualify(model, policy::policy())
        .unwrap()
        .into_solid()
        .unwrap();
    assert_eq!(
        a.model().faces[aface].surface.control_points,
        vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]]
        ]
    );
    let mut p = policy::policy();
    p.geometry.boundary.contacts.cells_per_pair = 1024;
    p.geometry.boundary.contacts.domain_cells_per_pair = 10000;
    let mut model = graph::body();
    if mirrored {
        model.faces[1].surface.control_points.reverse();
        model.faces[1].surface.weights.reverse();
        let loops: Vec<_> = std::iter::once(model.faces[1].outer)
            .chain(model.faces[1].holes.iter().copied())
            .collect();
        for l in loops {
            model.loops[l].coedges.reverse();
            for c in &mut model.loops[l].coedges {
                c.reversed = !c.reversed;
                for p in &mut c.pcurve.control_points {
                    p[0] = 1. - p[0];
                }
                c.pcurve.control_points.reverse();
                c.pcurve.weights.reverse();
            }
        }
        for use_ in model.shells.iter_mut().flat_map(|s| &mut s.faces) {
            if use_.face == 1 {
                use_.reversed = !use_.reversed;
            }
        }
        model.rebuild_topology_ids();
        model.validate().unwrap();
    }
    let admitted = solid::qualify(model, p).unwrap();
    assert!(
        admitted.solid().is_some(),
        "Quartic original body must independently qualify"
    );
    let b = admitted.into_solid().unwrap();
    (a, aface, b)
}
fn run(with_holes: bool, mirrored: bool) {
    let (a, aface, b) = operands(with_holes, mirrored);
    let source =
        ss::FiberSources::new(&a.model().faces[aface].surface, &b.model().faces[1].surface)
            .unwrap();
    let mut charts = family(&source, 0.3125, mirrored);
    charts.extend(family(&source, 0.15625, mirrored));
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
                    aface,
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
    for (signature, genus) in [([true, true], 0), ([true, false], 1)] {
        let sewn =
            census.sew_paired_regions([&signature, &signature], 1e-9, [Default::default(); 2]);
        if with_holes {
            assert!(
                sewn.is_err(),
                "Original rational holes cannot be dropped during root-only sewing"
            );
        } else {
            let sewn = Arc::new(sewn.unwrap());
            assert!(sewn.boundary_embedded());
            assert!(!sewn.material_admitted());
            assert_eq!(sewn.genus(), genus);
            let t = sewn.model().topology();
            assert_eq!(t.faces.len(), 2);
            assert_eq!(t.shells[0].faces[1].reversed, !mirrored);
            assert_eq!(t.shells.len(), 1);
            assert!(t.shells[0].closed);
            assert_eq!(t.edges.len(), COUNT * (genus + 1));
            assert_eq!(t.vertices.len(), t.edges.len());
            for (edge, _) in t.edges.iter().enumerate() {
                let uses: Vec<_> = t.shells[0]
                    .faces
                    .iter()
                    .flat_map(|u| {
                        let f = &t.faces[u.face];
                        std::iter::once(f.outer)
                            .chain(f.holes.iter().copied())
                            .flat_map(|l| t.loops[l].coedges.iter())
                            .filter(move |c| c.edge == edge)
                            .map(move |c| c.reversed ^ u.reversed)
                    })
                    .collect();
                assert_eq!(uses.len(), 2);
                assert_ne!(uses[0], uses[1]);
            }
            let limits = mixed::RootBoundaryVolumeLimits {
                absolute_error: 5e-4,
                cells: 4096,
                ..Default::default()
            };
            let admission = sewn.admit_material(limits).unwrap();
            eprintln!(
                "paired volume genus={genus} mirror={mirrored}: {:?}",
                admission.report()
            );
            assert!(
                admission.report().accuracy_proven,
                "{:?}",
                admission.report()
            );
            let solid = Arc::new(
                admission
                    .into_solid()
                    .expect("Complete signed volume must admit bounded material"),
            );
            let a = 0.3125_f64.powi(2);
            let b = 0.15625_f64.powi(2);
            let expected = if genus == 1 {
                6. * std::f64::consts::PI * (a - b).powi(3)
            } else {
                6. * std::f64::consts::PI * b * b * (3. * a - b)
            };
            let volume = solid.volume();
            assert!(volume[0] > 0. && volume[0] <= expected && expected <= volume[1]);
            assert!(solid.material_admitted());
            assert_eq!(solid.genus(), genus);
            assert_eq!(solid.outward_reversed(), genus == 0);
            for (point, expected) in if genus == 0 {
                vec![
                    ([0.5, 0.5, 0.03], Some(true)),
                    ([0.5, 0.5, -0.02], Some(false)),
                    ([0.5, 0.5, 0.], None),
                ]
            } else {
                vec![
                    ([0.725, 0.5, -0.02], Some(true)),
                    (
                        [0.5 + 0.1875 * 0.9999, 0.5 + 0.25 * 0.9999, -0.00001],
                        Some(true),
                    ),
                    ([0.5, 0.5, 0.], Some(false)),
                    ([0.725, 0.5, 0.], None),
                    ([0.8125, 0.5, 0.], None),
                ]
            } {
                let query = solid
                    .classify_point(
                        point,
                        &[[0., 0., 1.], [0.137, 0.283, 1.]],
                        Default::default(),
                    )
                    .unwrap();
                assert_eq!(
                    query.parity, expected,
                    "genus={genus} mirror={mirrored} point={point:?}: {query:?}"
                );
                assert!(query.cells <= 100000 && query.domain_queries <= 8192);
                assert_eq!(
                    query.cells,
                    query.attempts.iter().map(|a| a.cells).sum::<usize>()
                );
            }
            let inside_bounds = if genus == 0 {
                [[0.49, 0.51], [0.49, 0.51], [0.02, 0.025]]
            } else {
                [[0.72, 0.73], [0.495, 0.505], [-0.025, -0.015]]
            };
            let directions = [[0., 0., 1.], [0.137, 0.283, 1.]];
            let inside_box = Arc::new(
                solid
                    .qualify_material_box(inside_bounds, &directions, Default::default())
                    .unwrap(),
            );
            eprintln!(
                "root material inside genus={genus} mirror={mirrored}: {:?}",
                inside_box.separation_work()
            );
            assert!(inside_box.inside());
            assert!(Arc::ptr_eq(inside_box.solid(), &solid));
            assert_eq!(inside_box.bounds(), inside_bounds);
            assert!(inside_box.separation_work().separation_cells <= 32768);
            assert!(inside_box.separation_work().domain_queries <= 4096);
            let mut source_patch = rational_source_patch(inside_bounds);
            let rectangle = [[2., 5.], [-3., 1.]];
            let source_material = inside_box
                .qualify_source(&source_patch, rectangle, 2)
                .unwrap();
            assert!(source_material.inside());
            assert_eq!(source_material.source_spans(), 2);
            assert_eq!(source_material.rectangle(), rectangle);
            assert_eq!(source_material.surface(), &source_patch);
            assert!(Arc::ptr_eq(source_material.material(), &inside_box));
            assert!(inside_box
                .qualify_source(&source_patch, rectangle, 1)
                .is_err());
            assert!(inside_box
                .qualify_source(&source_patch, [[1., 5.], [-3., 1.]], 2)
                .is_err());
            let mut escaped = source_patch.clone();
            escaped.control_points[0][0][2] = 10.;
            assert!(inside_box.qualify_source(&escaped, rectangle, 2).is_err());
            source_patch.weights[0][0] = 4.;
            assert_eq!(
                source_material.surface().weights[0][0],
                1.,
                "Source ownership must be immutable"
            );
            if genus == 0 && !mirrored {
                let automatic = solid
                    .qualify_surface_material_box(
                        source_material.surface(),
                        rectangle,
                        &directions,
                        mixed::RootSurfaceMaterialLimits {
                            source_spans: 2,
                            ..Default::default()
                        },
                    )
                    .unwrap();
                assert!(automatic.inside());
                assert_eq!(
                    automatic.image_enclosure(),
                    source_material.image_enclosure()
                );
            }
            if genus == 1 {
                let hole_box = solid
                    .qualify_material_box(
                        [[0.49, 0.51], [0.49, 0.51], [-0.002, 0.002]],
                        &directions,
                        Default::default(),
                    )
                    .unwrap();
                eprintln!(
                    "root material hole mirror={mirrored}: {:?}",
                    hole_box.separation_work()
                );
                assert!(
                    !hole_box.inside(),
                    "The entire box through the untrimmed plane is in the hole"
                );
            }
            assert!(solid
                .qualify_material_box(
                    inside_bounds,
                    &directions,
                    mixed::RootMaterialBoxLimits {
                        separation_cells: 1,
                        ..Default::default()
                    }
                )
                .is_err());
            let contact_bounds = if genus == 0 {
                [[0.49, 0.51], [0.49, 0.51], [-0.001, 0.001]]
            } else {
                [[0.72, 0.73], [0.495, 0.505], [-0.001, 0.001]]
            };
            assert!(
                solid
                    .qualify_material_box(
                        contact_bounds,
                        &directions,
                        mixed::RootMaterialBoxLimits {
                            separation_cells: 256,
                            domain_queries: 128,
                            ..Default::default()
                        }
                    )
                    .is_err(),
                "A box crossing actual material boundary must not qualify"
            );
            let host = Arc::new(
                solid::qualify(
                    brep_core::cuboid([-1.; 3], [2.; 3]).unwrap(),
                    policy::policy(),
                )
                .unwrap()
                .into_solid()
                .unwrap(),
            );
            let cavity = solid
                .qualify_cavity_in(host.clone(), &directions, Default::default())
                .unwrap();
            assert!(cavity.material_admitted());
            assert!(Arc::ptr_eq(cavity.outer(), &host));
            assert!(Arc::ptr_eq(cavity.inner(), &solid));
            let result = cavity.model().topology();
            assert_eq!(result.bodies.len(), 1);
            assert_eq!(result.bodies[0].inner_shells.len(), 1);
            assert_eq!(
                result.faces.len(),
                host.model().faces.len() + solid.model().topology().faces.len()
            );
            let cavity_shell = result.bodies[0].inner_shells[0];
            for (new, old) in result.shells[cavity_shell]
                .faces
                .iter()
                .zip(&solid.model().topology().shells[0].faces)
            {
                assert_eq!(new.face, old.face + host.model().faces.len());
                assert_eq!(new.reversed, !old.reversed);
            }
            let cavity_volume = cavity.volume();
            let reference = 27. - expected;
            assert!(
                cavity_volume[0] > 0.
                    && cavity_volume[0] <= reference
                    && reference <= cavity_volume[1]
            );
            eprintln!(
                "cavity genus={genus} mirror={mirrored}: volume={cavity_volume:?}, cover={:?}",
                cavity.coverage_work()
            );
            assert_eq!(cavity.coverage_work().accepted, [2, 6]);
            let void_point = if genus == 0 {
                [0.5, 0.5, 0.03]
            } else {
                [0.725, 0.5, -0.02]
            };
            assert_eq!(
                cavity
                    .classify_point(void_point, &directions, Default::default())
                    .unwrap(),
                Some(false)
            );
            assert_eq!(
                cavity
                    .classify_point([0.5, 0.5, -0.5], &directions, Default::default())
                    .unwrap(),
                Some(true)
            );
            assert_eq!(
                cavity
                    .classify_point([3.; 3], &directions, Default::default())
                    .unwrap(),
                Some(false)
            );
            assert_eq!(
                cavity
                    .classify_point([-1., 0.5, 0.5], &directions, Default::default())
                    .unwrap(),
                None
            );
            assert!(solid
                .qualify_cavity_in(
                    host.clone(),
                    &directions,
                    mixed::RootCavityLimits {
                        coverage_cells: 1,
                        ..Default::default()
                    }
                )
                .is_err());
            let inner_boundary_point = if genus == 0 {
                [0.5, 0.5, 0.]
            } else {
                [0.725, 0.5, 0.]
            };
            assert_eq!(
                cavity
                    .classify_point(inner_boundary_point, &directions, Default::default())
                    .unwrap(),
                None
            );
            if genus == 1 {
                assert_eq!(
                    cavity
                        .classify_point([0.5, 0.5, 0.], &directions, Default::default())
                        .unwrap(),
                    Some(true)
                );
            }
            for (i, old) in solid.model().topology().edges.iter().enumerate() {
                let (mixed::MixedEdge::Root(a), mixed::MixedEdge::Root(b)) = (
                    &result.edges[host.model().edges.len() + i].curve,
                    &old.curve,
                ) else {
                    panic!("Root edge was fitted or replaced")
                };
                assert!(a.curve().same_definition(b.curve()).unwrap());
                assert!(a.start().point().same_selector(b.start().point()));
            }
            if genus == 0 && !mirrored {
                assert!(solid
                    .qualify_cavity_in(
                        cavity.outer().clone(),
                        &directions,
                        mixed::RootCavityLimits {
                            absolute_error: 1e-6,
                            ..Default::default()
                        }
                    )
                    .is_err());
                let far = Arc::new(
                    solid::qualify(
                        brep_core::cuboid([2.; 3], [4.; 3]).unwrap(),
                        policy::policy(),
                    )
                    .unwrap()
                    .into_solid()
                    .unwrap(),
                );
                let denied = solid
                    .qualify_cavity_in(
                        far,
                        &directions,
                        mixed::RootCavityLimits {
                            coverage_cells: 128,
                            ..Default::default()
                        },
                    )
                    .err()
                    .expect("A disjoint host cannot contain the cavity");
                assert!(
                    denied.message.contains("outside cavity host material"),
                    "{denied:?}"
                );
            }
            let no_bands = solid
                .classify_point(
                    [0.5, 0.5, 0.03],
                    &[[0., 0., 1.]],
                    mixed::RootSolidPointLimits {
                        band_queries: 0,
                        ..Default::default()
                    },
                )
                .unwrap();
            assert!(no_bands.parity.is_none() && no_bands.attempts.is_empty());
            let limited = solid
                .classify_point(
                    [0.5, 0.5, -0.02],
                    &[[0., 0., 1.]],
                    mixed::RootSolidPointLimits {
                        ray_cells: 1,
                        domain_queries: 1,
                        ..Default::default()
                    },
                )
                .unwrap();
            assert!(limited.parity.is_none());
            assert!(limited.cells <= 1 && limited.domain_queries <= 1);
            assert!(solid
                .classify_point([0.; 3], &[], Default::default())
                .is_err());

            assert_eq!(
                solid.model().topology().shells[0].faces[0].reversed,
                genus == 0
            );
            let denied = sewn
                .admit_material(mixed::RootBoundaryVolumeLimits {
                    band_queries: 0,
                    ..limits
                })
                .unwrap();
            assert!(denied.solid().is_none());
            assert!(denied.report().signed_bounds.is_none());
            if genus == 1 {
                let low = |cells| {
                    sewn.admit_material(mixed::RootBoundaryVolumeLimits {
                        cells,
                        arc_subdivisions: 1,
                        winding_queries: 0,
                        ..limits
                    })
                    .unwrap()
                };
                let before = low(2);
                let partial = low(3);
                assert!(before.solid().is_none() && partial.solid().is_none());
                assert_eq!(
                    before.report().signed_bounds,
                    partial.report().signed_bounds,
                    "Failed sibling must preserve the complete parent sum"
                );
                let b = partial.report().signed_bounds.unwrap();
                assert!(b[0] <= expected && expected <= b[1]);
                assert_eq!(partial.report().cells, 3);
                assert!(!partial.report().accuracy_proven);
                cavity.reverify().unwrap();
                if !mirrored {
                    let default = sewn.volume_bounds(Default::default()).unwrap();
                    eprintln!("paired default volume: {default:?}");
                    assert!(
                        default.accuracy_proven,
                        "Default volume policy must qualify this case: {default:?}"
                    );
                    let b = default.signed_bounds.unwrap();
                    assert!(b[0] <= expected && expected <= b[1]);
                }
            }
        }
    }
    assert!(census
        .sew_paired_regions(
            [&[false, false], &[false, false]],
            1e-9,
            [Default::default(); 2]
        )
        .is_err());
    assert!(census
        .sew_paired_regions(
            [&[true, true], &[true, false]],
            1e-9,
            [Default::default(); 2]
        )
        .is_err());
    assert!(census
        .sew_paired_regions([&[true, true], &[true, true]], 0., [Default::default(); 2])
        .is_err());
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
        assert_eq!(
            topology.faces[arrangement.region_faces()[0]].holes.len(),
            if with_holes && support == 0 { 2 } else { 1 }
        );
        assert_eq!(
            topology.faces[arrangement.region_faces()[1]].holes.len(),
            if with_holes && support == 0 { 2 } else { 1 }
        );
        assert_eq!(
            topology.faces[arrangement.region_faces()[2]].holes.len(),
            if with_holes && support == 0 { 1 } else { 0 }
        );
        if with_holes && support == 0 {
            for &hole in &a.model().faces[aface].holes {
                let p = &a.model().loops[hole].coedges[0].pcurve.control_points[0];
                let radius2 = (p[0] - 0.5).powi(2) + (p[1] - 0.5).powi(2);
                let expected = if radius2 < 0.15625_f64.powi(2) {
                    2
                } else if radius2 < 0.3125_f64.powi(2) {
                    1
                } else {
                    0
                };
                assert!(topology.faces[arrangement.region_faces()[expected]]
                    .holes
                    .contains(&hole));
                assert_eq!(
                    arrangement
                        .region_faces()
                        .iter()
                        .filter(|&&face| topology.faces[face].holes.contains(&hole))
                        .count(),
                    1
                );
            }
            for point in [[0.5, 0.5], [0.5, 0.725], [0.05, 0.05]] {
                let rectangle = point.map(|x| [x - 0.001, x + 0.001]);
                let region = census
                    .classify_rectangle(0, rectangle, Default::default())
                    .unwrap();
                assert_eq!(region.location(), Location::Outside);
                assert!(region.signature().is_none());
                assert!(arrangement
                    .qualify_boolean_rectangle(
                        solid::BooleanMaterialOp::Union,
                        rectangle,
                        &[[1., 0.317, 0.173]],
                        Default::default()
                    )
                    .is_err());
            }
        }
        for (rectangle, signature) in [
            ([[0.49, 0.51], [0.59, 0.61]], [true, true]),
            ([[0.71, 0.73], [0.49, 0.51]], [true, false]),
            ([[0.94, 0.96], [0.04, 0.06]], [false, false]),
        ] {
            let region = census
                .classify_rectangle(support, rectangle, Default::default())
                .unwrap();
            assert_eq!(region.location(), Location::Inside);
            assert_eq!(region.signature(), Some(signature.as_slice()));
            arrangement.face_for_signature(&signature).unwrap();
        }
        if with_holes && support == 1 {
            let void = arrangement
                .qualify_boolean_rectangle(
                    solid::BooleanMaterialOp::Difference,
                    [[0.499, 0.501], [0.724, 0.726]],
                    &[[1., 0.317, 0.173], [0.239, 1., 0.419], [0.137, 0.283, 1.]],
                    Default::default(),
                )
                .unwrap();
            assert_eq!(
                void.face(),
                arrangement.face_for_signature(&[true, false]).unwrap()
            );
            assert!(!void.material().boolean_material().retain());
            assert!(!void.material().boolean_material().reverse_material_normal());
        }
        for (rectangle, signature, first_keep, second_keep) in [
            ([[0.499, 0.501], [0.599, 0.601]], [true, true], false, false),
            ([[0.724, 0.726], [0.499, 0.501]], [true, false], true, true),
            (
                [[0.949, 0.951], [0.049, 0.051]],
                [false, false],
                false,
                false,
            ),
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

#[test]
fn nested_hole_operands_prove_material_boxes_without_census_shortcuts() {
    let (a, aface, b) = operands(true, false);
    let void = solid::qualify_boolean_face_box(
        &a,
        &b,
        solid::BooleanMaterialOp::Difference,
        solid::BooleanSourceSide::Second,
        1,
        [[0.499, 0.501], [0.724, 0.726]],
        &[[1., 0.317, 0.173], [0.239, 1., 0.419], [0.137, 0.283, 1.]],
        Default::default(),
    )
    .unwrap();
    assert!(!void.retain());
    for (side, face) in [
        (solid::BooleanSourceSide::First, aface),
        (solid::BooleanSourceSide::Second, 1),
    ] {
        for rectangle in [
            [[0.499, 0.501], [0.599, 0.601]],
            [[0.724, 0.726], [0.499, 0.501]],
            [[0.949, 0.951], [0.049, 0.051]],
        ] {
            let result = solid::qualify_boolean_face_box(
                &a,
                &b,
                solid::BooleanMaterialOp::Difference,
                side,
                face,
                rectangle,
                &[[1., 0.317, 0.173], [0.239, 1., 0.419], [0.137, 0.283, 1.]],
                Default::default(),
            );
            if let Err(error) = result {
                let (source, other) = match side {
                    solid::BooleanSourceSide::First => (&a, &b),
                    solid::BooleanSourceSide::Second => (&b, &a),
                };
                let point = source.model().faces[face]
                    .surface
                    .evaluate(
                        (rectangle[0][0] + rectangle[0][1]) * 0.5,
                        (rectangle[1][0] + rectangle[1][1]) * 0.5,
                    )
                    .unwrap()
                    .point;
                let parity = other
                    .classify_point(
                        [point[0], point[1], point[2]],
                        &[[1., 0.317, 0.173], [0.239, 1., 0.419], [0.137, 0.283, 1.]],
                        100000,
                        1000000,
                    )
                    .unwrap();
                panic!("side={side:?} rectangle={rectangle:?} error={error:?} parity={parity:?}");
            }
        }
    }
}

#[test]
fn near_root_box_requires_complete_original_winding() {
    let a = brep_core::cuboid([0., 0., -10.], [1., 1., 0.]).unwrap();
    let b = graph::body();
    let aface = a
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == 0.)
        })
        .unwrap();
    let source = ss::FiberSources::new(&a.faces[aface].surface, &b.faces[1].surface).unwrap();
    let charts = family(&source, 0.3125, false);
    let wire = wire(&charts, 0.3125);
    let p: [f64; 2] = [0.5 + 0.1875 * 0.9999, 0.5 + 0.25 * 0.9999];
    let proof = wire
        .winding_box(
            0,
            p.map(|x| [x.next_down(), x.next_up()]),
            ss::OriginalSsWindingLimits {
                max_segments: 4096,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(proof.winding(), 1);
    let contact = wire
        .winding_box(0, [[0.8, 0.825], [0.48, 0.52]], Default::default())
        .unwrap_err();
    assert!(contact.message.contains("boundary selector"), "{contact:?}");
}

fn rational_source_patch(bounds: [[f64; 2]; 3]) -> nurbs_core::surface::Surface {
    let center = bounds.map(|d| d[0] * 0.5 + d[1] * 0.5);
    let widths = bounds.map(|d| (d[1] - d[0]) * 0.1);
    nurbs_core::surface::Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![2., 2., 3., 5., 5.],
        knots_v: vec![-3., -3., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..2)
                    .map(|j| {
                        vec![
                            center[0] + (i as f64 - 1.) * widths[0],
                            center[1] + (j as f64 - 0.5) * widths[1],
                            center[2] + if i == 1 { widths[2] } else { -widths[2] },
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1., 2.], vec![1.5, 1.], vec![2., 1.25]],
        periodic_u: false,
        periodic_v: false,
    }
}
