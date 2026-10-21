use nurbs_core::{
    curve::Curve,
    periodic_trim::{self, ChartAssemblyLimits, WeightedChartGraph},
};
fn limits() -> ChartAssemblyLimits {
    ChartAssemblyLimits {
        max_segments: 128,
        max_links: 64,
        max_edges: 128,
        max_images: 256,
        max_cells: 30000,
        tolerance_uv: 1e-8,
    }
}
fn circle() -> Curve {
    let r = 0.125;
    let w = std::f64::consts::FRAC_1_SQRT_2;
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.],
        control_points: vec![
            vec![0.5, 0.5],
            vec![0.5, 0.5 + r],
            vec![0.5 - r, 0.5 + r],
            vec![0.5 - 2. * r, 0.5 + r],
            vec![0.5 - 2. * r, 0.5],
            vec![0.5 - 2. * r, 0.5 - r],
            vec![0.5 - r, 0.5 - r],
            vec![0.5, 0.5 - r],
            vec![0.5, 0.5],
        ],
        weights: vec![1., w, 1., w, 1., w, 1., w, 1.],
        periodic: false,
    }
}
fn assemble(sources: &[Curve]) -> periodic_trim::WeightedChartAssembly {
    let p: Vec<_> = sources
        .iter()
        .map(|c| periodic_trim::split_candidates(c, [[0., 1.]; 2], &[], &[[0, 0]], 1).unwrap())
        .collect();
    periodic_trim::assemble_weighted_periodic_chart(
        sources,
        &p,
        [[0., 1.]; 2],
        [0.375, 0.5],
        limits(),
    )
    .unwrap()
}
fn winding(graph: &WeightedChartGraph, q: [f64; 2]) -> i32 {
    periodic_trim::inspect_weighted_winding(graph, [[0., 1.]; 2], q, 1e-8, 256, 30000)
        .unwrap()
        .winding
        .unwrap()
}
fn coverage(graph: &WeightedChartGraph) {
    let mut ids: Vec<_> = graph
        .edges
        .iter()
        .flat_map(|e| e.contributions.iter().map(|p| p.expanded_edge))
        .chain(
            graph
                .cancellations
                .iter()
                .flatten()
                .map(|p| p.expanded_edge),
        )
        .collect();
    ids.sort();
    assert_eq!(ids, (0..graph.original_edges).collect::<Vec<_>>());
    for edge in &graph.edges {
        assert_eq!(
            edge.contributions
                .iter()
                .map(|p| i32::from(p.orientation))
                .sum::<i32>(),
            edge.coefficient as i32
        );
    }
    for group in &graph.cancellations {
        assert_eq!(
            group.iter().map(|p| i32::from(p.orientation)).sum::<i32>(),
            0
        );
    }
}
#[test]
fn duplicate_rational_boundaries_are_compact_and_keep_integer_winding() {
    let c = circle();
    let r = assemble(&[c.clone(), c.clone(), c.clone()]);
    assert!(r.winding_preserved, "{r:?}");
    let g = r.graph.unwrap();
    coverage(&g);
    assert_eq!(g.edges.len(), 1);
    assert_eq!(g.edges[0].coefficient, 3);
    assert_eq!(g.edges[0].curve, c);
    assert_eq!(g.original_edges, 3);
    assert_eq!(g.expanded_edges, 3);
    assert_eq!(winding(&g, [0.375, 0.5]), 3);
    assert_eq!(winding(&g, [0.75, 0.5]), 0);
}
#[test]
fn exact_opposite_boundaries_cancel_to_an_empty_region() {
    let c = circle();
    let r = assemble(&[c.clone(), c.reverse().unwrap()]);
    assert!(r.winding_preserved, "{r:?}");
    let g = r.graph.unwrap();
    coverage(&g);
    assert!(g.edges.is_empty() && g.loops.is_empty());
    assert_eq!(g.expanded_edges, 0);
    assert_eq!(g.cancellations.len(), 1);
    assert_eq!(g.cancellations[0].len(), 2);
    for q in [[0.375, 0.5], [0.5, 0.5], [0.75, 0.5]] {
        assert_eq!(winding(&g, q), 0);
    }
}
#[test]
fn negative_net_multiplicity_uses_an_actual_original_reverse_encoding() {
    let c = circle();
    let reverse = c.reverse().unwrap();
    let r = assemble(&[c, reverse.clone(), reverse.clone(), reverse.clone()]);
    assert!(r.winding_preserved, "{r:?}");
    let g = r.graph.unwrap();
    coverage(&g);
    assert_eq!(g.edges.len(), 1);
    assert_eq!(g.edges[0].coefficient, 2);
    assert_eq!(g.edges[0].curve, reverse);
    assert_eq!(g.expanded_edges, 2);
    assert_eq!(winding(&g, [0.375, 0.5]), -2);
}
#[test]
fn general_perimeter_multiplicity_preserves_periodized_polygon_winding() {
    let c = Curve::from_polyline(vec![
        vec![0.25, 0.25],
        vec![1., 1.],
        vec![1.5, 0.25],
        vec![1.5, 1.],
        vec![1.5, 1.5],
        vec![1., 1.5],
        vec![0.25, 1.5],
        vec![0.25, 1.],
        vec![0.25, 0.25],
    ])
    .unwrap();
    let p = periodic_trim::split_candidates(
        &c,
        [[0., 1.]; 2],
        &[1., 3., 5., 7.],
        &[[0, 0], [1, 0], [1, 1], [0, 1], [0, 0]],
        5,
    )
    .unwrap();
    let ordinary = periodic_trim::assemble_periodic_chart(
        std::slice::from_ref(&c),
        std::slice::from_ref(&p),
        [[0., 1.]; 2],
        [0.625, 0.375],
        limits(),
    )
    .unwrap();
    assert_eq!(ordinary.reason, Some("perimeter-multiplicity-unproven"));
    let r = periodic_trim::assemble_weighted_periodic_chart(
        std::slice::from_ref(&c),
        &[p],
        [[0., 1.]; 2],
        [0.625, 0.375],
        limits(),
    )
    .unwrap();
    assert!(r.winding_preserved, "{r:?}");
    let g = r.graph.unwrap();
    coverage(&g);
    assert!(g.edges.iter().any(|e| e.coefficient > 1));
    // Independent polygon crossing oracle with a complete finite image range.
    let oracle = |q: [f64; 2]| {
        let mut result = 0;
        for u in -3..=3 {
            for v in -3..=3 {
                let q = [q[0] + f64::from(u), q[1] + f64::from(v)];
                for e in c.control_points.windows(2) {
                    let a = &e[0];
                    let b = &e[1];
                    let cross = (b[0] - a[0]) * (q[1] - a[1]) - (b[1] - a[1]) * (q[0] - a[0]);
                    if a[1] <= q[1] && q[1] < b[1] && cross > 0. {
                        result += 1;
                    }
                    if b[1] <= q[1] && q[1] < a[1] && cross < 0. {
                        result -= 1;
                    }
                }
            }
        }
        result
    };
    assert_eq!(oracle([0.375, 0.8125]), 2);
    for q in [
        [0.625, 0.375],
        [0.875, 0.8125],
        [0.125, 0.1875],
        [0.375, 0.8125],
        [0.875, 0.375],
    ] {
        assert_eq!(winding(&g, q), oracle(q));
    }
}
#[test]
fn weighted_queries_reject_inconsistent_use_counts_and_partial_budget_outputs() {
    let c = circle();
    let sources = [c.clone(), c.clone(), c];
    let mut graph = assemble(&sources).graph.unwrap();
    graph.edges[0].coefficient = 2;
    assert!(
        periodic_trim::inspect_weighted_winding(
            &graph,
            [[0., 1.]; 2],
            [0.375, 0.5],
            1e-8,
            256,
            30000
        )
        .is_err()
    );
    graph.edges[0].coefficient = 3;
    graph.loops[0][0] = 999;
    assert!(
        periodic_trim::inspect_weighted_winding(
            &graph,
            [[0., 1.]; 2],
            [0.375, 0.5],
            1e-8,
            256,
            30000
        )
        .is_err()
    );
    let p: Vec<_> = sources
        .iter()
        .map(|c| periodic_trim::split_candidates(c, [[0., 1.]; 2], &[], &[[0, 0]], 1).unwrap())
        .collect();
    let mut short = limits();
    short.max_edges = 1;
    let r = periodic_trim::assemble_weighted_periodic_chart(
        &sources,
        &p,
        [[0., 1.]; 2],
        [0.375, 0.5],
        short,
    )
    .unwrap();
    assert_eq!(r.reason, Some("edge-budget"));
    assert!(!r.winding_preserved && r.graph.is_none());
}
#[test]
fn opposite_cancellation_does_not_round_the_parameter_knot_reflection() {
    assert_ne!((0.1_f64 + 1.1) - 1.1, 0.1);
    let c = Curve {
        degree: 3,
        knots: vec![0.1, 0.1, 0.1, 0.1, 1.1, 1.1, 1.1, 1.1],
        control_points: vec![vec![0., 0.], vec![0.75, 0.], vec![0.75, 0.75], vec![0., 0.]],
        weights: vec![1.; 4],
        periodic: false,
    };
    let mut reverse = c.clone();
    reverse.control_points.reverse();
    reverse.weights.reverse();
    let sources = [c, reverse];
    let chart = [[-1., 1.]; 2];
    let p: Vec<_> = sources
        .iter()
        .map(|c| periodic_trim::split_candidates(c, chart, &[], &[[0, 0]], 1).unwrap())
        .collect();
    let r = periodic_trim::assemble_weighted_periodic_chart(
        &sources,
        &p,
        chart,
        [-0.5, -0.5],
        limits(),
    )
    .unwrap();
    assert!(r.winding_preserved, "{r:?}");
    let g = r.graph.unwrap();
    coverage(&g);
    assert!(g.edges.is_empty());
}

#[test]
fn projective_weight_aliases_and_reverse_cancellation_are_order_independent() {
    let a = circle();
    let mut b = a.clone();
    for w in &mut b.weights {
        *w *= 2.;
    }
    let reverse = b.reverse().unwrap();
    for sources in [
        vec![a.clone(), b.clone(), reverse.clone()],
        vec![b.clone(), reverse.clone(), a.clone()],
        vec![reverse.clone(), a.clone(), b.clone()],
        vec![a.clone(), reverse, b],
    ] {
        let result = assemble(&sources);
        assert!(result.winding_preserved, "{result:?}");
        let graph = result.graph.unwrap();
        coverage(&graph);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].coefficient, 1);
        assert_eq!(winding(&graph, [0.375, 0.5]), 1);
    }
}
