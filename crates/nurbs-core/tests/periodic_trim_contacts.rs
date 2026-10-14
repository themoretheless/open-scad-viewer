use nurbs_core::{
    curve::Curve,
    periodic_trim::{self, ChartEdge},
};
fn triangle(origin: [f64; 2], a: [f64; 2], b: [f64; 2]) -> Vec<ChartEdge> {
    [[origin, a], [a, b], [b, origin]]
        .into_iter()
        .map(|p| ChartEdge {
            curve: Curve::from_polyline(p.into_iter().map(|v| v.to_vec()).collect()).unwrap(),
            source: None,
        })
        .collect()
}
fn touching_edges(scale: f64) -> Vec<ChartEdge> {
    let mut edges = triangle([0., 0.], [scale, 0.], [0., scale]);
    edges.extend(triangle([0., 0.], [-scale, 0.], [0., -scale]));
    edges
}
#[test]
fn exact_contact_order_has_bijective_successors_even_when_float_cross_underflows() {
    for scale in [1., 1e-200] {
        let edges = touching_edges(scale);
        if scale < 1. {
            assert_eq!(scale * scale, 0.);
        }
        let r = periodic_trim::inspect_contact_ownership(&edges, 12).unwrap();
        assert!(r.complete, "{r:?}");
        assert_eq!(r.contacts.len(), 1);
        let next = r.successors.unwrap();
        assert_eq!(next, vec![1, 2, 0, 4, 5, 3]);
        let mut sorted = next;
        sorted.sort();
        assert_eq!(sorted, (0..6).collect::<Vec<_>>());
        assert_eq!(r.contacts[0].pairs.len(), 2);
    }
}
#[test]
fn higher_order_rational_endpoint_ray_is_retained_without_normalizing() {
    let mut edges = touching_edges(1.);
    edges[0].curve = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0.], vec![0., 0.], vec![1., 0.]],
        weights: vec![1., 2., 3.],
        periodic: false,
    };
    let r = periodic_trim::inspect_contact_ownership(&edges, 12).unwrap();
    assert!(r.complete, "{r:?}");
    let port = r.contacts[0]
        .ports
        .iter()
        .find(|p| p.edge == 0 && !p.incoming)
        .unwrap();
    assert_eq!(port.vanishing_order, 2);
    assert_eq!(port.outward_control, [1., 0.]);
}
#[test]
fn contact_coincidence_nonalternation_stationary_spans_and_budgets_remain_unproven() {
    let first = triangle([0., 0.], [1., 0.], [0., 1.]);
    let mut duplicate = first.clone();
    duplicate.extend(first.clone());
    let r = periodic_trim::inspect_contact_ownership(&duplicate, 12).unwrap();
    assert_eq!(r.reason, Some("coincident-contact-rays"));
    assert!(r.successors.is_none());
    let mut overlap = first;
    overlap.extend(triangle([0., 0.], [1., 1.], [-1., 1.]));
    let r = periodic_trim::inspect_contact_ownership(&overlap, 12).unwrap();
    assert_eq!(r.reason, Some("nonalternating-contact-rays"));
    assert!(r.successors.is_none());
    let mut stationary = touching_edges(1.);
    stationary[0].curve.control_points[1] = vec![0., 0.];
    // Balance the stationary edge's other endpoint as well; ambiguity is local.
    stationary[1].curve.control_points[0] = vec![0., 0.];
    let r = periodic_trim::inspect_contact_ownership(&stationary, 12).unwrap();
    assert_eq!(r.reason, Some("stationary-contact-branch"));
    assert!(r.successors.is_none());
    let r = periodic_trim::inspect_contact_ownership(&touching_edges(1.), 1).unwrap();
    assert_eq!(r.reason, Some("contact-port-budget"));
    assert!(r.successors.is_none());
}
#[test]
fn touching_regions_assemble_with_explicit_contact_ownership_and_preserved_winding() {
    let make = |a: Vec<f64>, b: Vec<f64>| {
        Curve::from_polyline(vec![vec![0.5, 0.5], a, b, vec![0.5, 0.5]]).unwrap()
    };
    let sources = [
        make(vec![0.75, 0.5], vec![0.5, 0.75]),
        make(vec![0.25, 0.5], vec![0.5, 0.25]),
    ];
    let partitions: Vec<_> = sources
        .iter()
        .map(|c| periodic_trim::split_candidates(c, [[0., 1.]; 2], &[], &[[0, 0]], 1).unwrap())
        .collect();
    let r = periodic_trim::assemble_periodic_chart(
        &sources,
        &partitions,
        [[0., 1.]; 2],
        [0.625, 0.5625],
        periodic_trim::ChartAssemblyLimits {
            max_segments: 64,
            max_links: 32,
            max_edges: 32,
            max_images: 256,
            max_cells: 30000,
            tolerance_uv: 1e-8,
        },
    )
    .unwrap();
    assert!(r.chart_winding_preserved, "{r:?}");
    let graph = r.graph.unwrap();
    assert_eq!(graph.loops.len(), 2);
    let ownership = periodic_trim::inspect_contact_ownership(&graph.edges, 64).unwrap();
    assert!(ownership.complete);
    assert_eq!(ownership.contacts.len(), 1);
    let loops: Vec<Vec<Curve>> = graph
        .loops
        .iter()
        .map(|cycle| {
            cycle
                .iter()
                .map(|&i| graph.edges[i].curve.clone())
                .collect()
        })
        .collect();
    for (q, w) in [
        ([0.625, 0.5625], 1),
        ([0.375, 0.4375], 1),
        ([0.625, 0.4375], 0),
        ([0.1, 0.1], 0),
    ] {
        let r = periodic_trim::inspect_periodic_winding(&loops, [[0., 1.]; 2], q, 1e-8, 256, 20000)
            .unwrap();
        assert_eq!(r.winding, Some(w));
    }
}

#[test]
fn ownership_is_invariant_under_edge_permutation_and_signed_zero() {
    let original = touching_edges(1.);
    let order = [4, 0, 5, 2, 1, 3];
    let mut inverse = [0; 6];
    for (new, old) in order.into_iter().enumerate() {
        inverse[old] = new;
    }
    let mut edges: Vec<_> = order.into_iter().map(|i| original[i].clone()).collect();
    for (i, edge) in edges.iter_mut().enumerate() {
        for p in &mut edge.curve.control_points {
            for x in p {
                if *x == 0. && i % 2 == 0 {
                    *x = -0.;
                }
            }
        }
    }
    let r = periodic_trim::inspect_contact_ownership(&edges, 12).unwrap();
    assert!(r.complete, "{r:?}");
    let expected = [1, 2, 0, 4, 5, 3];
    let next = r.successors.unwrap();
    for i in 0..6 {
        assert_eq!(next[i], inverse[expected[order[i]]]);
    }
}

#[test]
fn balanced_multibranch_touch_on_periodic_seam_has_verified_local_ownership() {
    let make = |a: Vec<f64>, b: Vec<f64>| {
        Curve::from_polyline(vec![vec![0., 0.5], a, b, vec![0., 0.5]]).unwrap()
    };
    let sources = [
        make(vec![0.25, 0.5], vec![0.25, 0.625]),
        make(vec![0.25, 0.75], vec![0.125, 0.75]),
    ];
    let partitions: Vec<_> = sources
        .iter()
        .map(|c| periodic_trim::split_candidates(c, [[0., 1.]; 2], &[], &[[0, 0]], 1).unwrap())
        .collect();
    let r = periodic_trim::assemble_periodic_chart(
        &sources,
        &partitions,
        [[0., 1.]; 2],
        [0.125, 0.53125],
        periodic_trim::ChartAssemblyLimits {
            max_segments: 64,
            max_links: 32,
            max_edges: 32,
            max_images: 256,
            max_cells: 30000,
            tolerance_uv: 1e-8,
        },
    )
    .unwrap();
    assert!(r.chart_winding_preserved, "{r:?}");
    let graph = r.graph.unwrap();
    let ownership = periodic_trim::inspect_contact_ownership(&graph.edges, 64).unwrap();
    assert!(ownership.complete);
    assert_eq!(ownership.contacts[0].point, [0., 0.5]);
    let loops: Vec<Vec<Curve>> = graph
        .loops
        .iter()
        .map(|cycle| {
            cycle
                .iter()
                .map(|&i| graph.edges[i].curve.clone())
                .collect()
        })
        .collect();
    for (q, w) in [
        ([0.125, 0.53125], 1),
        ([0.125, 0.6875], 1),
        ([0.125, 0.59375], 0),
        ([0.875, 0.53125], 0),
    ] {
        assert_eq!(
            periodic_trim::inspect_periodic_winding(&loops, [[0., 1.]; 2], q, 1e-8, 256, 20000)
                .unwrap()
                .winding,
            Some(w)
        );
    }
}

#[test]
fn seam_flux_resolves_unbalanced_contact_of_crossing_and_touching_regions() {
    let crossing = Curve::from_polyline(vec![
        vec![0.75, 0.25],
        vec![1.25, 0.75],
        vec![0.75, 0.75],
        vec![0.75, 0.25],
    ])
    .unwrap();
    let touching = Curve::from_polyline(vec![
        vec![1., 0.5],
        vec![0.875, 0.25],
        vec![0.9375, 0.25],
        vec![1., 0.5],
    ])
    .unwrap();
    let partitions = [
        periodic_trim::split_candidates(
            &crossing,
            [[0., 1.]; 2],
            &[0.5, 1.5],
            &[[0, 0], [1, 0], [0, 0]],
            3,
        )
        .unwrap(),
        periodic_trim::split_candidates(&touching, [[0., 1.]; 2], &[], &[[0, 0]], 1).unwrap(),
    ];
    let sources = [crossing, touching];
    let limits = periodic_trim::ChartAssemblyLimits {
        max_segments: 64,
        max_links: 32,
        max_edges: 32,
        max_images: 256,
        max_cells: 30000,
        tolerance_uv: 1e-8,
    };
    let r = periodic_trim::assemble_periodic_chart(
        &sources,
        &partitions,
        [[0., 1.]; 2],
        [0.875, 0.6875],
        limits,
    )
    .unwrap();
    assert!(r.chart_winding_preserved, "{r:?}");
    let graph = r.graph.unwrap();
    assert_eq!(graph.loops.len(), 3);
    let ownership = periodic_trim::inspect_contact_ownership(&graph.edges, 64).unwrap();
    assert!(ownership.complete);
    assert!(ownership.contacts.iter().any(|c| c.point == [1., 0.5]));
    let loops: Vec<Vec<Curve>> = graph
        .loops
        .iter()
        .map(|cycle| {
            cycle
                .iter()
                .map(|&i| graph.edges[i].curve.clone())
                .collect()
        })
        .collect();
    for (q, w) in [
        ([0.875, 0.6875], 1),
        ([0.0625, 0.6875], 1),
        ([0.953125, 0.375], 1),
        ([0.9375, 0.40625], 0),
        ([0.5, 0.5], 0),
    ] {
        assert_eq!(
            periodic_trim::inspect_periodic_winding(&loops, [[0., 1.]; 2], q, 1e-8, 256, 20000)
                .unwrap()
                .winding,
            Some(w)
        );
    }
    let mut short = limits;
    short.max_edges = 4;
    let refused = periodic_trim::assemble_periodic_chart(
        &sources,
        &partitions,
        [[0., 1.]; 2],
        [0.875, 0.6875],
        short,
    )
    .unwrap();
    assert_eq!(refused.reason, Some("edge-budget"));
    assert!(refused.graph.is_none());
}

#[test]
fn seam_flux_uses_opposite_perimeter_orientation_on_v_axis() {
    let make = |p: Vec<Vec<f64>>| {
        Curve::from_polyline(p.into_iter().map(|v| vec![v[1], v[0]]).collect()).unwrap()
    };
    let crossing = make(vec![
        vec![0.75, 0.25],
        vec![1.25, 0.75],
        vec![0.75, 0.75],
        vec![0.75, 0.25],
    ]);
    let touching = make(vec![
        vec![1., 0.5],
        vec![0.875, 0.25],
        vec![0.9375, 0.25],
        vec![1., 0.5],
    ]);
    // Coordinate exchange reverses orientation; preserve its negative winding.
    let partitions = [
        periodic_trim::split_candidates(
            &crossing,
            [[0., 1.]; 2],
            &[0.5, 1.5],
            &[[0, 0], [0, 1], [0, 0]],
            3,
        )
        .unwrap(),
        periodic_trim::split_candidates(&touching, [[0., 1.]; 2], &[], &[[0, 0]], 1).unwrap(),
    ];
    let r = periodic_trim::assemble_periodic_chart(
        &[crossing, touching],
        &partitions,
        [[0., 1.]; 2],
        [0.6875, 0.875],
        periodic_trim::ChartAssemblyLimits {
            max_segments: 64,
            max_links: 32,
            max_edges: 32,
            max_images: 256,
            max_cells: 30000,
            tolerance_uv: 1e-8,
        },
    )
    .unwrap();
    assert!(r.chart_winding_preserved, "{r:?}");
    let graph = r.graph.unwrap();
    let loops: Vec<Vec<Curve>> = graph
        .loops
        .iter()
        .map(|cycle| {
            cycle
                .iter()
                .map(|&i| graph.edges[i].curve.clone())
                .collect()
        })
        .collect();
    for (q, w) in [
        ([0.6875, 0.875], -1),
        ([0.6875, 0.0625], -1),
        ([0.375, 0.953125], -1),
        ([0.40625, 0.9375], 0),
    ] {
        assert_eq!(
            periodic_trim::inspect_periodic_winding(&loops, [[0., 1.]; 2], q, 1e-8, 256, 20000)
                .unwrap()
                .winding,
            Some(w)
        );
    }
}

#[test]
fn perimeter_flux_resolves_multibranch_corner_contact_with_winding_correction() {
    let crossing = Curve::from_polyline(vec![
        vec![0.75, 0.75],
        vec![1., 1.],
        vec![1.25, 1.25],
        vec![1., 1.375],
        vec![0.75, 1.25],
        vec![0.625, 1.],
        vec![0.75, 0.75],
    ])
    .unwrap();
    let touching = Curve::from_polyline(vec![
        vec![1., 1.],
        vec![0.875, 0.75],
        vec![0.9375, 0.75],
        vec![1., 1.],
    ])
    .unwrap();
    for reversed in [false, true] {
        let sources = if reversed {
            [crossing.reverse().unwrap(), touching.reverse().unwrap()]
        } else {
            [crossing.clone(), touching.clone()]
        };
        let tiles = if reversed {
            [[0, 0], [0, 1], [1, 1], [0, 0]]
        } else {
            [[0, 0], [1, 1], [0, 1], [0, 0]]
        };
        let partitions = [
            periodic_trim::split_candidates(&sources[0], [[0., 1.]; 2], &[1., 3., 5.], &tiles, 4)
                .unwrap(),
            periodic_trim::split_candidates(&sources[1], [[0., 1.]; 2], &[], &[[0, 0]], 1).unwrap(),
        ];
        let r = periodic_trim::assemble_periodic_chart(
            &sources,
            &partitions,
            [[0., 1.]; 2],
            [0.5, 0.5],
            periodic_trim::ChartAssemblyLimits {
                max_segments: 128,
                max_links: 64,
                max_edges: 128,
                max_images: 256,
                max_cells: 30000,
                tolerance_uv: 1e-8,
            },
        )
        .unwrap();
        assert!(r.chart_winding_preserved, "{r:?}");
        let sign = if reversed { -1 } else { 1 };
        assert_eq!(r.calibration.as_ref().unwrap().offset, Some(sign));
        let graph = r.graph.unwrap();
        if !reversed {
            assert_eq!(graph.loops.len(), 4);
        }
        let ownership = periodic_trim::inspect_contact_ownership(&graph.edges, 256).unwrap();
        assert!(ownership.complete);
        assert!(ownership.contacts.iter().any(|c| c.point == [1., 1.]));
        let loops: Vec<Vec<Curve>> = graph
            .loops
            .iter()
            .map(|cycle| {
                cycle
                    .iter()
                    .map(|&i| graph.edges[i].curve.clone())
                    .collect()
            })
            .collect();
        for (q, w) in [
            ([0.875, 0.9375], sign),
            ([0.953125, 0.875], sign),
            ([0.0625, 0.125], sign),
            ([0.9375, 0.90625], 0),
            ([0.5, 0.5], 0),
        ] {
            assert_eq!(
                periodic_trim::inspect_periodic_winding(&loops, [[0., 1.]; 2], q, 1e-8, 256, 20000)
                    .unwrap()
                    .winding,
                Some(w)
            );
        }
    }
}

#[test]
fn corner_flux_uses_authored_chart_origin_and_keeps_budget_refusal_explicit() {
    let shift = |p: Vec<Vec<f64>>| {
        Curve::from_polyline(p.into_iter().map(|v| vec![v[0] + 2., v[1] - 3.]).collect()).unwrap()
    };
    let crossing = shift(vec![
        vec![0.75, 0.75],
        vec![1., 1.],
        vec![1.25, 1.25],
        vec![1., 1.375],
        vec![0.75, 1.25],
        vec![0.625, 1.],
        vec![0.75, 0.75],
    ]);
    let touching = shift(vec![
        vec![1., 1.],
        vec![0.875, 0.75],
        vec![0.9375, 0.75],
        vec![1., 1.],
    ]);
    let sources = [crossing, touching];
    let chart = [[2., 3.], [-3., -2.]];
    let partitions = [
        periodic_trim::split_candidates(
            &sources[0],
            chart,
            &[1., 3., 5.],
            &[[0, 0], [1, 1], [0, 1], [0, 0]],
            4,
        )
        .unwrap(),
        periodic_trim::split_candidates(&sources[1], chart, &[], &[[0, 0]], 1).unwrap(),
    ];
    let mut limits = periodic_trim::ChartAssemblyLimits {
        max_segments: 128,
        max_links: 64,
        max_edges: 128,
        max_images: 256,
        max_cells: 30000,
        tolerance_uv: 1e-8,
    };
    let r =
        periodic_trim::assemble_periodic_chart(&sources, &partitions, chart, [2.5, -2.5], limits)
            .unwrap();
    assert!(r.chart_winding_preserved, "{r:?}");
    let graph = r.graph.unwrap();
    let loops: Vec<Vec<Curve>> = graph
        .loops
        .iter()
        .map(|cycle| {
            cycle
                .iter()
                .map(|&i| graph.edges[i].curve.clone())
                .collect()
        })
        .collect();
    for (q, w) in [
        ([2.875, -2.0625], 1),
        ([2.953125, -2.125], 1),
        ([2.9375, -2.09375], 0),
    ] {
        assert_eq!(
            periodic_trim::inspect_periodic_winding(&loops, chart, q, 1e-8, 256, 20000)
                .unwrap()
                .winding,
            Some(w)
        );
    }
    limits.max_edges = 5;
    let r =
        periodic_trim::assemble_periodic_chart(&sources, &partitions, chart, [2.5, -2.5], limits)
            .unwrap();
    assert_eq!(r.reason, Some("edge-budget"));
    assert!(r.graph.is_none());
}

fn tangent_circles() -> [Curve; 2] {
    let r = 0.125;
    let w = std::f64::consts::FRAC_1_SQRT_2;
    let left = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.],
        control_points: vec![
            vec![0., 0.],
            vec![0., r],
            vec![-r, r],
            vec![-2. * r, r],
            vec![-2. * r, 0.],
            vec![-2. * r, -r],
            vec![-r, -r],
            vec![0., -r],
            vec![0., 0.],
        ]
        .into_iter()
        .map(|p| vec![p[0] + 0.5, p[1] + 0.5])
        .collect(),
        weights: vec![1., w, 1., w, 1., w, 1., w, 1.],
        periodic: false,
    };
    let mut right = left.clone();
    for p in &mut right.control_points {
        p[0] = 1. - p[0];
    }
    [left, right.reverse().unwrap()]
}
#[test]
fn rational_external_tangency_has_distinct_bend_sides_and_exact_ownership() {
    let circles = tangent_circles();
    let edges: Vec<_> = circles
        .into_iter()
        .enumerate()
        .map(|(i, c)| ChartEdge {
            curve: c,
            source: Some([i, 0]),
        })
        .collect();
    let r = periodic_trim::inspect_contact_ownership(&edges, 4).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.successors, Some(vec![0, 1]));
    assert_eq!(r.contacts.len(), 1);
    let ports = &r.contacts[0].ports;
    assert_eq!(ports.iter().filter(|p| p.bend_side == -1).count(), 2);
    assert_eq!(ports.iter().filter(|p| p.bend_side == 1).count(), 2);
    assert!(
        ports
            .iter()
            .all(|p| p.vanishing_order == 1 && p.normal_vanishing_order == Some(2))
    );
    // Different internally tangent curves with the same bend side are not
    // ordered by control magnitude or by the source edge index.
    let mut nested = edges[0].clone();
    for p in &mut nested.curve.control_points {
        for x in p {
            *x = 0.5 + 2. * (*x - 0.5);
        }
    }
    let nested_report =
        periodic_trim::inspect_contact_ownership(&[edges[0].clone(), nested], 4).unwrap();
    assert!(!nested_report.complete && nested_report.successors.is_none());
    let duplicate = vec![edges[0].clone(), edges[0].clone()];
    let r = periodic_trim::inspect_contact_ownership(&duplicate, 4).unwrap();
    assert_eq!(r.reason, Some("coincident-contact-rays"));
    assert!(r.successors.is_none());
}
#[test]
fn tangent_rational_regions_assemble_without_welding_or_losing_winding() {
    let sources = tangent_circles();
    let partitions: Vec<_> = sources
        .iter()
        .map(|c| periodic_trim::split_candidates(c, [[0., 1.]; 2], &[], &[[0, 0]], 1).unwrap())
        .collect();
    let r = periodic_trim::assemble_periodic_chart(
        &sources,
        &partitions,
        [[0., 1.]; 2],
        [0.375, 0.5],
        periodic_trim::ChartAssemblyLimits {
            max_segments: 64,
            max_links: 32,
            max_edges: 32,
            max_images: 256,
            max_cells: 30000,
            tolerance_uv: 1e-8,
        },
    )
    .unwrap();
    assert!(r.chart_winding_preserved, "{r:?}");
    let graph = r.graph.unwrap();
    assert_eq!(graph.loops.len(), 2);
    // Authored rational edges are retained byte-for-byte, including weights.
    for edge in &graph.edges {
        let [source, _] = edge.source.unwrap();
        assert_eq!(edge.curve, sources[source]);
    }
    let loops: Vec<Vec<Curve>> = graph
        .loops
        .iter()
        .map(|cycle| {
            cycle
                .iter()
                .map(|&i| graph.edges[i].curve.clone())
                .collect()
        })
        .collect();
    for (q, w) in [
        ([0.375, 0.5], 1),
        ([0.625, 0.5], 1),
        ([0.5, 0.625], 0),
        ([0.125, 0.5], 0),
    ] {
        assert_eq!(
            periodic_trim::inspect_periodic_winding(&loops, [[0., 1.]; 2], q, 1e-8, 256, 20000)
                .unwrap()
                .winding,
            Some(w)
        );
    }
}

#[test]
fn straight_tangent_branch_orders_before_counterclockwise_bending_branch() {
    let mut edges = triangle([0., 0.], [0., -1.], [1., 0.]);
    edges.push(ChartEdge {
        curve: Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![0.5, 0.], vec![1., 1.]],
            weights: vec![1., 2., 1.],
            periodic: false,
        },
        source: None,
    });
    edges.push(ChartEdge {
        curve: Curve::from_polyline(vec![vec![1., 1.], vec![0., 0.]]).unwrap(),
        source: None,
    });
    let r = periodic_trim::inspect_contact_ownership(&edges, 10).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.successors, Some(vec![1, 2, 0, 4, 3]));
    let contact = &r.contacts[0];
    let straight = contact
        .ports
        .iter()
        .find(|p| p.edge == 2 && p.incoming)
        .unwrap();
    assert_eq!(straight.bend_side, 0);
    assert_eq!(straight.normal_vanishing_order, None);
    let curved = contact
        .ports
        .iter()
        .find(|p| p.edge == 3 && !p.incoming)
        .unwrap();
    assert_eq!(curved.bend_side, 1);
    assert_eq!(curved.normal_vanishing_order, Some(2));
}

fn exponent_lenses() -> [Curve; 2] {
    let bezier = |xs: Vec<f64>, ys: Vec<f64>| Curve {
        degree: 4,
        knots: vec![0., 0., 0., 0., 0., 1., 1., 1., 1., 1.],
        control_points: xs.into_iter().zip(ys).map(|(x, y)| vec![x, y]).collect(),
        weights: vec![1.; 5],
        periodic: false,
    };
    let lower_a = bezier(vec![0., 1.5, 3., 4.5, 6.], vec![0., 0., 0., 0., 0.25]);
    let upper_a = bezier(vec![0., 1.5, 3., 4.5, 6.], vec![0., 0., 0., 0.125, 0.5]);
    let lower_b = bezier(vec![0., 1.5, 3., 4.5, 6.], vec![0., 0., 0.125, 0.375, 0.75]);
    let upper_b = bezier(vec![0., 0., 1., 3., 6.], vec![0., 0., 0., 1., 4.]);
    let combine = |lower: Curve, upper: Curve| {
        let a = lower.control_points.last().unwrap();
        let b = upper.control_points.last().unwrap();
        let connector: Vec<Vec<f64>> = (0..=4)
            .map(|i| vec![a[0], a[1] + (b[1] - a[1]) * i as f64 / 4.])
            .collect();
        let reverse = upper.reverse().unwrap();
        let mut cp = lower.control_points;
        cp.extend(connector.into_iter().skip(1));
        cp.extend(reverse.control_points.into_iter().skip(1));
        Curve {
            degree: 4,
            knots: vec![
                0., 0., 0., 0., 0., 1., 1., 1., 1., 2., 2., 2., 2., 3., 3., 3., 3., 3.,
            ],
            weights: vec![1.; cp.len()],
            control_points: cp,
            periodic: false,
        }
    };
    [combine(lower_a, upper_a), combine(lower_b, upper_b)]
}
#[test]
fn equal_side_tangent_order_uses_radial_exponents_including_cusps() {
    for reflect in [false, true] {
        let mut curves = exponent_lenses();
        if reflect {
            for c in &mut curves {
                for p in &mut c.control_points {
                    p[1] = -p[1];
                }
            }
        }
        let edges: Vec<_> = curves
            .into_iter()
            .map(|c| ChartEdge {
                curve: c,
                source: None,
            })
            .collect();
        let r = periodic_trim::inspect_contact_ownership(&edges, 4).unwrap();
        assert!(r.complete, "{r:?}");
        let ports = &r.contacts[0].ports;
        let order: Vec<_> = ports.iter().map(|p| (p.edge, p.incoming)).collect();
        assert_eq!(
            order,
            if reflect {
                vec![(1, true), (1, false), (0, true), (0, false)]
            } else {
                vec![(0, false), (0, true), (1, false), (1, true)]
            }
        );
        let cusp = ports.iter().find(|p| p.edge == 1 && p.incoming).unwrap();
        assert_eq!(cusp.vanishing_order, 2);
        assert_eq!(cusp.normal_vanishing_order, Some(3));
        assert_eq!(cusp.radial_bend_exponent(), Some([1, 2]));
        let regular = ports.iter().find(|p| p.edge == 1 && !p.incoming).unwrap();
        assert_eq!(regular.radial_bend_exponent(), Some([1, 1]));
        assert_eq!(
            ports.iter().map(|p| p.bend_side).collect::<Vec<_>>(),
            vec![if reflect { -1 } else { 1 }; 4]
        );
    }
}
#[test]
fn radial_order_is_independent_of_knots_and_positive_weights() {
    let curves = exponent_lenses();
    let edges: Vec<_> = curves
        .iter()
        .cloned()
        .map(|c| ChartEdge {
            curve: c,
            source: None,
        })
        .collect();
    let original = periodic_trim::inspect_contact_ownership(&edges, 4).unwrap();
    let mut scaled = edges;
    for (i, edge) in scaled.iter_mut().enumerate() {
        for knot in &mut edge.curve.knots {
            *knot = if i == 0 {
                2. + 8. * *knot
            } else {
                -3. + 0.125 * *knot
            };
        }
    }
    for edge in &mut scaled {
        for (i, w) in edge.curve.weights.iter_mut().enumerate() {
            *w = 2_f64.powi((i % 5) as i32);
        }
    }
    let r = periodic_trim::inspect_contact_ownership(&scaled, 4).unwrap();
    assert!(r.complete);
    assert_eq!(r.successors, original.successors);
    assert_eq!(
        r.contacts[0]
            .ports
            .iter()
            .map(|p| p.radial_bend_exponent())
            .collect::<Vec<_>>(),
        original.contacts[0]
            .ports
            .iter()
            .map(|p| p.radial_bend_exponent())
            .collect::<Vec<_>>()
    );
}
#[test]
fn exponent_ordered_lenses_preserve_analytically_known_fill_and_gap() {
    // At x=6*s, region A lies between y=s^4/4 and y=s^3/2.
    // Region B lies between y=3*s^2/4 and y=4*s^(3/2).
    let sources = exponent_lenses();
    let chart = [[-1., 7.], [-1., 5.]];
    let partitions: Vec<_> = sources
        .iter()
        .map(|c| periodic_trim::split_candidates(c, chart, &[], &[[0, 0]], 1).unwrap())
        .collect();
    let r = periodic_trim::assemble_periodic_chart(
        &sources,
        &partitions,
        chart,
        [3., 0.03125],
        periodic_trim::ChartAssemblyLimits {
            max_segments: 64,
            max_links: 32,
            max_edges: 32,
            max_images: 256,
            max_cells: 30000,
            tolerance_uv: 1e-8,
        },
    )
    .unwrap();
    assert!(r.chart_winding_preserved, "{r:?}");
    let graph = r.graph.unwrap();
    assert_eq!(graph.loops.len(), 2);
    let loops: Vec<Vec<Curve>> = graph
        .loops
        .iter()
        .map(|cycle| {
            cycle
                .iter()
                .map(|&i| graph.edges[i].curve.clone())
                .collect()
        })
        .collect();
    for (q, w) in [
        ([3., 0.03125], 1),
        ([3., 0.5], 1),
        ([3., 0.125], 0),
        ([3., 2.], 0),
    ] {
        assert_eq!(
            periodic_trim::inspect_periodic_winding(&loops, chart, q, 1e-8, 256, 20000)
                .unwrap()
                .winding,
            Some(w)
        );
    }
}

fn coefficient_lenses(ys: [f64; 4]) -> [Curve; 2] {
    let make = |lo: f64, hi: f64| Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 2., 2., 3., 3., 3.],
        control_points: vec![
            vec![0., 0.],
            vec![1., 0.],
            vec![2., lo],
            vec![2., (lo + hi) / 2.],
            vec![2., hi],
            vec![1., 0.],
            vec![0., 0.],
        ],
        weights: vec![1.; 7],
        periodic: false,
    };
    [make(ys[0], ys[1]), make(ys[2], ys[3])]
}
#[test]
fn equal_exponent_contact_uses_disjoint_signed_coefficients_including_cyclic_wrap() {
    for reflect in [false, true] {
        let mut curves = coefficient_lenses([1., 2., 3., 4.]);
        if reflect {
            for c in &mut curves {
                for p in &mut c.control_points {
                    p[1] = -p[1];
                }
            }
        }
        let edges: Vec<_> = curves
            .into_iter()
            .map(|c| ChartEdge {
                curve: c,
                source: None,
            })
            .collect();
        let r = periodic_trim::inspect_contact_ownership(&edges, 4).unwrap();
        assert!(r.complete, "{r:?}");
        let ports = &r.contacts[0].ports;
        assert_eq!(
            ports
                .iter()
                .map(|p| (p.edge, p.incoming))
                .collect::<Vec<_>>(),
            if reflect {
                vec![(1, true), (1, false), (0, true), (0, false)]
            } else {
                vec![(0, false), (0, true), (1, false), (1, true)]
            }
        );
        let expected = if reflect {
            [-1., -0.75, -0.5, -0.25]
        } else {
            [0.25, 0.5, 0.75, 1.]
        };
        for (port, value) in ports.iter().zip(expected) {
            let [lo, hi] = port.regular_bend_coefficient.unwrap();
            assert!(
                lo <= value && value <= hi && hi - lo < 1e-10,
                "{port:?} vs {value}"
            );
        }
    }
    let a = 1.;
    let b = f64::from_bits(1_f64.to_bits() + 1);
    let c = f64::from_bits(1_f64.to_bits() + 2);
    let d = f64::from_bits(1_f64.to_bits() + 3);
    let edges: Vec<_> = coefficient_lenses([a, b, c, d])
        .into_iter()
        .map(|c| ChartEdge {
            curve: c,
            source: None,
        })
        .collect();
    let unresolved = periodic_trim::inspect_contact_ownership(&edges, 4).unwrap();
    assert!(!unresolved.complete && unresolved.successors.is_none());
    assert_eq!(unresolved.reason, Some("coincident-contact-rays"));
}
#[test]
fn regular_coefficient_accounts_for_nonuniform_knots_weights_and_endpoint_side() {
    for rational in [false, true] {
        let curve = Curve {
            degree: 2,
            knots: vec![2., 2., 2., 3., 7., 7., 7.],
            control_points: vec![vec![0., 0.], vec![1., 0.], vec![2., 1.], vec![3., 2.]],
            weights: if rational {
                vec![2., 3., 5., 7.]
            } else {
                vec![1.; 4]
            },
            periodic: false,
        };
        let mut edges = vec![
            ChartEdge {
                curve,
                source: None,
            },
            ChartEdge {
                curve: Curve::from_polyline(vec![vec![3., 2.], vec![0., 0.]]).unwrap(),
                source: None,
            },
        ];
        edges.extend(triangle([0., 0.], [0., -1.], [1., 0.]));
        for incoming in [false, true] {
            let mut query = edges.clone();
            if incoming {
                for edge in &mut query {
                    edge.curve = edge.curve.reverse().unwrap();
                }
            }
            let r = periodic_trim::inspect_contact_ownership(&query, 10).unwrap();
            assert!(r.complete, "{r:?}");
            let port = r.contacts[0]
                .ports
                .iter()
                .find(|p| p.edge == 0 && p.incoming == incoming)
                .unwrap();
            let [lo, hi] = port.regular_bend_coefficient.unwrap();
            // C'(a)=(2,0), normal C''(a)=2/(1*5) for unit weights;
            // rational weights multiply the geometric coefficient by 10/9.
            let expected = if rational { 1. / 18. } else { 1. / 20. };
            assert!(
                lo <= expected && expected <= hi && hi - lo < 1e-10,
                "{port:?} vs {expected}"
            );
        }
    }
}
#[test]
fn internally_tangent_oriented_hole_assembles_and_preserves_material() {
    let [inner, _] = tangent_circles();
    let mut outer = inner.clone();
    for p in &mut outer.control_points {
        for x in p {
            *x = 0.5 + 2. * (*x - 0.5);
        }
    }
    let sources = [outer, inner.reverse().unwrap()];
    let chart = [[-0.5, 1.5]; 2];
    let partitions: Vec<_> = sources
        .iter()
        .map(|c| periodic_trim::split_candidates(c, chart, &[], &[[0, 0]], 1).unwrap())
        .collect();
    let r = periodic_trim::assemble_periodic_chart(
        &sources,
        &partitions,
        chart,
        [0.125, 0.5],
        periodic_trim::ChartAssemblyLimits {
            max_segments: 64,
            max_links: 32,
            max_edges: 32,
            max_images: 256,
            max_cells: 30000,
            tolerance_uv: 1e-8,
        },
    )
    .unwrap();
    assert!(r.chart_winding_preserved, "{r:?}");
    let graph = r.graph.unwrap();
    assert_eq!(graph.loops.len(), 1);
    let ownership = periodic_trim::inspect_contact_ownership(&graph.edges, 64).unwrap();
    assert!(ownership.complete);
    assert_eq!(ownership.successors, Some(vec![1, 0]));
    let loops: Vec<Vec<Curve>> = graph
        .loops
        .iter()
        .map(|cycle| {
            cycle
                .iter()
                .map(|&i| graph.edges[i].curve.clone())
                .collect()
        })
        .collect();
    for (q, w) in [
        ([0.125, 0.5], 1),
        ([0.25, 0.6875], 1),
        ([0.375, 0.5], 0),
        ([0.625, 0.5], 0),
    ] {
        assert_eq!(
            periodic_trim::inspect_periodic_winding(&loops, chart, q, 1e-8, 256, 20000)
                .unwrap()
                .winding,
            Some(w)
        );
    }
    let surface = nurbs_core::surface::Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-0.5, -0.5, 1.5, 1.5],
        knots_v: vec![-0.5, -0.5, 1.5, 1.5],
        control_points: vec![
            vec![vec![-0.5, -0.5, 0.], vec![-0.5, 1.5, 0.]],
            vec![vec![1.5, -0.5, 0.], vec![1.5, 1.5, 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let admitted = periodic_trim::assemble_on_surface(
        &surface,
        &sources,
        &partitions,
        chart,
        [false, false],
        [0.125, 0.5],
        periodic_trim::SurfaceChartLimits {
            correspondence: periodic_trim::SurfaceCorrespondenceLimits {
                tolerance_uv: 0.01,
                tolerance_world: 0.1,
                max_rectangles: 1,
                max_deviation_cells: 40000,
            },
            uv: periodic_trim::SurfaceUvLimits::Exact(periodic_trim::ChartAssemblyLimits {
                max_segments: 64,
                max_links: 32,
                max_edges: 32,
                max_images: 256,
                max_cells: 30000,
                tolerance_uv: 1e-8,
            }),
        },
    )
    .unwrap();
    assert_eq!(
        admitted.status,
        periodic_trim::SurfaceChartStatus::Admitted,
        "{admitted:?}"
    );
    assert_eq!(admitted.graph().unwrap().loops.len(), 1);
}

fn powered_cusp_lenses() -> [Curve; 2] {
    let make = |lo: f64, hi: f64| {
        let lower: Vec<Vec<f64>> = vec![0., 0., 1., 3., 6., 10.]
            .into_iter()
            .enumerate()
            .map(|(i, x)| vec![x, if i == 5 { lo } else { 0. }])
            .collect();
        let upper: Vec<Vec<f64>> = vec![0., 0., 1., 3., 6., 10.]
            .into_iter()
            .enumerate()
            .map(|(i, x)| vec![x, if i == 5 { hi } else { 0. }])
            .collect();
        let mut cp = lower;
        cp.extend((1..=5).map(|i| vec![10., lo + (hi - lo) * i as f64 / 5.]));
        cp.extend(upper.into_iter().rev().skip(1));
        Curve {
            degree: 5,
            knots: vec![
                0., 0., 0., 0., 0., 0., 1., 1., 1., 1., 1., 2., 2., 2., 2., 2., 3., 3., 3., 3., 3.,
                3.,
            ],
            weights: vec![1.; cp.len()],
            control_points: cp,
            periodic: false,
        }
    };
    [make(1., 2.), make(3., 4.)]
}
#[test]
fn equal_higher_order_cusps_use_powered_coefficients_without_fractional_roots() {
    for reflect in [false, true] {
        let mut curves = powered_cusp_lenses();
        if reflect {
            for c in &mut curves {
                for p in &mut c.control_points {
                    p[1] = -p[1];
                }
            }
        }
        let edges: Vec<_> = curves
            .into_iter()
            .map(|c| ChartEdge {
                curve: c,
                source: None,
            })
            .collect();
        let r = periodic_trim::inspect_contact_ownership(&edges, 4).unwrap();
        assert!(r.complete, "{r:?}");
        let ports = &r.contacts[0].ports;
        assert_eq!(
            ports
                .iter()
                .map(|p| (p.edge, p.incoming))
                .collect::<Vec<_>>(),
            if reflect {
                vec![(1, true), (1, false), (0, true), (0, false)]
            } else {
                vec![(0, false), (0, true), (1, false), (1, true)]
            }
        );
        let amplitudes = if reflect {
            [4., 3., 2., 1.]
        } else {
            [1., 2., 3., 4.]
        };
        for (port, c) in ports.iter().zip(amplitudes) {
            assert_eq!(port.radial_bend_exponent(), Some([3, 2]));
            assert!(port.regular_bend_coefficient.is_none());
            let powered = port.radial_coefficient_power.unwrap();
            assert_eq!(powered.power, 2);
            // x=10*t², y=c*t⁵ => |angular coefficient|²=c²/10⁵.
            let expected = c * c / 100000.;
            let [lo, hi] = powered.magnitude;
            assert!(
                lo <= expected && expected <= hi && hi - lo < 1e-10,
                "{port:?} vs {expected}"
            );
        }
    }
}
#[test]
fn higher_coefficient_uses_all_endpoint_knot_gaps_and_rational_weights() {
    for rational in [false, true] {
        let curve = Curve {
            degree: 3,
            knots: vec![2., 2., 2., 2., 3., 5., 8., 8., 8., 8.],
            control_points: vec![
                vec![0., 0.],
                vec![1., 0.],
                vec![2., 0.],
                vec![3., 1.],
                vec![4., 2.],
                vec![5., 3.],
            ],
            weights: if rational {
                vec![2., 3., 4., 5., 6., 7.]
            } else {
                vec![1.; 6]
            },
            periodic: false,
        };
        let mut edges = vec![
            ChartEdge {
                curve,
                source: None,
            },
            ChartEdge {
                curve: Curve::from_polyline(vec![vec![5., 3.], vec![0., 0.]]).unwrap(),
                source: None,
            },
        ];
        edges.extend(triangle([0., 0.], [0., -1.], [1., 0.]));
        for incoming in [false, true] {
            let mut query = edges.clone();
            if incoming {
                for edge in &mut query {
                    edge.curve = edge.curve.reverse().unwrap();
                }
            }
            let r = periodic_trim::inspect_contact_ownership(&query, 10).unwrap();
            assert!(r.complete, "{r:?}");
            let port = r.contacts[0]
                .ports
                .iter()
                .find(|p| p.edge == 0 && p.incoming == incoming)
                .unwrap();
            assert_eq!(port.radial_bend_exponent(), Some([2, 1]));
            let coefficient = port.radial_coefficient_power.unwrap();
            assert_eq!(coefficient.power, 1);
            // h=(1,3,6), unit beta1=3 and beta3=1/18.
            // Rational leading coefficient multiplies 1/486 by 20/27.
            let expected = if rational { 10. / 6561. } else { 1. / 486. };
            let [lo, hi] = coefficient.magnitude;
            assert!(
                lo <= expected && expected <= hi && hi - lo < 1e-10,
                "{port:?} vs {expected}"
            );
        }
    }
}
#[test]
fn higher_order_cusp_regions_keep_analytic_material_and_gap_after_assembly() {
    let sources = powered_cusp_lenses();
    let chart = [[-1., 11.], [-1., 5.]];
    let partitions: Vec<_> = sources
        .iter()
        .map(|c| periodic_trim::split_candidates(c, chart, &[], &[[0, 0]], 1).unwrap())
        .collect();
    let r = periodic_trim::assemble_periodic_chart(
        &sources,
        &partitions,
        chart,
        [2.5, 0.046875],
        periodic_trim::ChartAssemblyLimits {
            max_segments: 64,
            max_links: 32,
            max_edges: 32,
            max_images: 256,
            max_cells: 30000,
            tolerance_uv: 1e-8,
        },
    )
    .unwrap();
    assert!(r.chart_winding_preserved, "{r:?}");
    let graph = r.graph.unwrap();
    assert_eq!(graph.loops.len(), 2);
    let loops: Vec<Vec<Curve>> = graph
        .loops
        .iter()
        .map(|cycle| {
            cycle
                .iter()
                .map(|&i| graph.edges[i].curve.clone())
                .collect()
        })
        .collect();
    // At x=2.5, t=1/2, so t^5=1/32 for every cusp branch.
    for (q, w) in [
        ([2.5, 0.046875], 1),
        ([2.5, 0.109375], 1),
        ([2.5, 0.078125], 0),
        ([2.5, 0.15625], 0),
    ] {
        assert_eq!(
            periodic_trim::inspect_periodic_winding(&loops, chart, q, 1e-8, 256, 20000)
                .unwrap()
                .winding,
            Some(w)
        );
    }
}
