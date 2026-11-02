use brep_core::minkowski3d::curved::{self, NurbsGraphLimits};
#[path = "support/independent_graph_solids.rs"]
mod fixture;
#[test]
fn independent_curved_carriers_have_a_complete_trimmed_two_cycle_graph_across_original_seams() {
    let (a, b) = fixture::solids();
    let before = (a.clone(), b.clone());
    let graph =
        curved::certify_nurbs_intersection_graph(&a, &b, NurbsGraphLimits::default()).unwrap();
    let c = graph.certificate();
    println!(
        "pairs={} vertices={} arcs={} cycles={} work={:?}",
        c.pairs.len(),
        c.vertices.len(),
        c.arcs.len(),
        c.cycles.len(),
        c.work
    );
    assert_eq!(c.pairs.len(), 36);
    assert_eq!(c.vertices.len(), 8);
    assert_eq!(c.arcs.len(), 8);
    assert_eq!(c.cycles.len(), 2);
    assert!(c.cycles.iter().all(|x| x.arcs.len() == 4));
    assert!(
        c.pairs
            .iter()
            .flat_map(|p| &p.pieces)
            .any(|p| p.inside == [false, true])
    );
    assert!(
        c.pairs
            .iter()
            .filter(|p| p.separating_axis.is_none())
            .count()
            >= 5
    );
    let analytic = [0.5 - 0.125_f64.sqrt(), 0.5 + 0.125_f64.sqrt()];
    let mut irrational_vertices = 0;
    for (i, key) in c.vertices.iter().enumerate() {
        let enclosure = graph.vertex_enclosure(i).unwrap();
        if key.operand == 0
            && a.edges[key.edge]
                .curve
                .control_points
                .iter()
                .all(|p| p[2] == 0.)
        {
            irrational_vertices += 1;
            assert!(
                analytic
                    .iter()
                    .any(|x| enclosure[0][0] <= x + 1e-13 && x - 1e-13 <= enclosure[0][1])
            );
            assert!(enclosure[2][0] <= 0. && 0. <= enclosure[2][1]);
        }
    }
    assert_eq!(irrational_vertices, 4);
    graph.reverify(&a, &b).unwrap();
    // This preserves the global graph topology, so a structural-only checker
    // would miss the lie. Full source/trim reconstruction must reject it.
    let mut forged = c.clone();
    let inactive = forged
        .pairs
        .iter_mut()
        .flat_map(|p| &mut p.pieces)
        .find(|p| p.inside == [false, true])
        .unwrap();
    inactive.inside[1] = false;
    assert!(curved::verify_nurbs_intersection_graph(&a, &b, &forged).is_err());
    let mut forged = c.clone();
    forged.pairs.pop();
    assert!(curved::verify_nurbs_intersection_graph(&a, &b, &forged).is_err());
    let mut forged = c.clone();
    forged.cycles[0].arcs.pop();
    assert!(curved::verify_nurbs_intersection_graph(&a, &b, &forged).is_err());
    let mut forged = c.clone();
    forged.vertices[0].rank += 1;
    assert!(curved::verify_nurbs_intersection_graph(&a, &b, &forged).is_err());
    // A complete intersection graph is not yet an authored/material-proved
    // Boolean Model. Standard dispatch must retain that separate gate.
    let refused = brep_core::operations::boolean(&a, &b, "difference").unwrap_err();
    assert_eq!(refused.code, "BREP_NURBS_BOOLEAN_UNPROVED");
    assert_eq!((a, b), before);
}
#[test]
fn unresolved_geometry_and_budget_never_acquire_graph_authority() {
    let (a, b) = fixture::solids();
    for limits in [
        NurbsGraphLimits {
            max_cells: 1,
            ..Default::default()
        },
        NurbsGraphLimits {
            max_rectangles: 1,
            ..Default::default()
        },
        NurbsGraphLimits {
            max_exact_work: 0,
            ..Default::default()
        },
    ] {
        assert!(curved::certify_nurbs_intersection_graph(&a, &b, limits).is_err());
    }
    assert!(curved::certify_nurbs_intersection_graph(&a, &a, NurbsGraphLimits::default()).is_err());
}

#[test]
fn complete_disjoint_trim_graph_needs_no_root_or_exact_lift_work() {
    let (a, b) = fixture::solids();
    let b = brep_core::transform::affine(
        &b,
        [
            [1., 0., 0., 16.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let graph = curved::certify_nurbs_intersection_graph(
        &a,
        &b,
        NurbsGraphLimits {
            max_cells: 1,
            max_rectangles: 1,
            max_exact_work: 0,
            ..Default::default()
        },
    )
    .unwrap();
    let c = graph.certificate();
    assert_eq!(c.pairs.len(), 36);
    assert!(c.pairs.iter().all(|p| p.separating_axis.is_some()));
    assert!(c.vertices.is_empty() && c.arcs.is_empty() && c.cycles.is_empty());
    assert_eq!(c.work.queries, 0);
    assert_eq!(c.work.rectangles, 0);
    assert_eq!(c.work.exact_work, 0);
    graph.reverify(&a, &b).unwrap();
}

#[test]
fn either_carrier_order_can_supply_the_original_common_clock_proof() {
    let (a, b) = fixture::solids();
    let graph =
        curved::certify_nurbs_intersection_graph(&b, &a, NurbsGraphLimits::default()).unwrap();
    let c = graph.certificate();
    assert_eq!((c.vertices.len(), c.arcs.len(), c.cycles.len()), (8, 8, 2));
    assert!(
        c.pairs
            .iter()
            .flat_map(|p| &p.pieces)
            .any(|p| p.inside == [true, false])
    );
    graph.reverify(&b, &a).unwrap();
}
