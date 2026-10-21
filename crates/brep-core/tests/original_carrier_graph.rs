use brep_core::profile_certificate as p;
use nurbs_core::curve::Curve;
use std::sync::Arc;
fn graph(sources: &[Curve]) -> p::OwnedOriginalCarrierGraph {
    let a = Arc::new(
        p::assemble_original_contact_atlas(sources, 1e-12, p::ContactAtlasLimits::default())
            .unwrap(),
    );
    p::own_original_carrier_graph(Arc::new(p::own_original_carriers(a).unwrap())).unwrap()
}
fn line(a: [f64; 2], b: [f64; 2]) -> Curve {
    Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap()
}
#[test]
fn shared_original_endpoint_owns_two_distinct_half_edges() {
    let g = graph(&[line([0., 0.], [1., 1.]), line([1., 1.], [2., 0.])]);
    g.reverify_source().unwrap();
    assert_eq!(g.vertices().len(), 3);
    assert_eq!(g.ends()[0][1], g.ends()[1][0]);
    let ports = &g.incidence()[g.ends()[0][1]];
    assert_eq!(ports.len(), 2);
    assert_eq!(
        ports[0],
        p::OriginalCarrierIncidence {
            carrier: 0,
            direction: -1
        }
    );
    assert_eq!(
        ports[1],
        p::OriginalCarrierIncidence {
            carrier: 1,
            direction: 1
        }
    );
}
#[test]
fn nearby_endpoints_are_never_merged_by_display_tolerance() {
    let next = f64::from_bits(1f64.to_bits() + 1);
    let g = graph(&[line([0., 0.], [1., 1.]), line([next, 1.], [2., 0.])]);
    g.reverify_source().unwrap();
    assert_eq!(g.vertices().len(), 4);
    assert_ne!(g.ends()[0][1], g.ends()[1][0]);
}
#[test]
fn original_closed_traversal_retains_both_loop_incidences() {
    let c = Curve {
        degree: 3,
        knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        weights: vec![1.; 4],
        control_points: vec![vec![0., 0.], vec![3., 0.], vec![0., 3.], vec![0., 0.]],
        periodic: false,
    };
    let g = graph(&[c]);
    g.reverify_source().unwrap();
    let ends = g.ends();
    assert_eq!(ends.first().unwrap()[0], ends.last().unwrap()[1]);
    let ports = &g.incidence()[ends[0][0]];
    assert!(ports.contains(&p::OriginalCarrierIncidence {
        carrier: 0,
        direction: 1
    }));
    assert!(ports.contains(&p::OriginalCarrierIncidence {
        carrier: ends.len() - 1,
        direction: -1
    }));
}
#[test]
fn irrational_crossing_graph_covers_every_half_edge_once() {
    let c = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        weights: vec![1.; 3],
        control_points: vec![vec![-3., 0.], vec![0., 6.], vec![3., 0.]],
        periodic: false,
    };
    let g = graph(&[c, line([-4., 1.5], [4., 1.5])]);
    g.reverify_source().unwrap();
    assert_eq!(g.ends().len(), 6);
    assert_eq!(g.vertices().len(), 6);
    assert_eq!(g.incidence().iter().filter(|p| p.len() == 4).count(), 2);
    for carrier in 0..g.ends().len() {
        for side in 0..2 {
            let port = p::OriginalCarrierIncidence {
                carrier,
                direction: if side == 0 { 1 } else { -1 },
            };
            assert_eq!(
                g.incidence()
                    .iter()
                    .flatten()
                    .filter(|p| **p == port)
                    .count(),
                1
            );
            assert!(g.incidence()[g.ends()[carrier][side]].contains(&port));
        }
    }
}
#[test]
fn crossing_tree_walk_visits_all_half_edges_and_bridge_sides() {
    let g = Arc::new(graph(&[
        line([-1., -1.], [1., 1.]),
        line([-1., 1.], [1., -1.]),
    ]));
    let c = p::own_original_carrier_cycles(g, p::ContactPortOrderLimits::default()).unwrap();
    c.reverify_source().unwrap();
    assert_eq!(c.cycles().len(), 1);
    assert_eq!(c.cycles()[0].len(), 8);
    let mut all = c.cycles()[0].clone();
    all.sort();
    assert_eq!(all, (0..8).collect::<Vec<_>>());
}
#[test]
fn closed_original_loop_owns_two_oppositely_directed_boundary_walks() {
    let source = Curve {
        degree: 3,
        knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        weights: vec![1.; 4],
        control_points: vec![vec![0., 0.], vec![3., 0.], vec![0., 3.], vec![0., 0.]],
        periodic: false,
    };
    let g = Arc::new(graph(&[source]));
    let c = p::own_original_carrier_cycles(g, p::ContactPortOrderLimits::default()).unwrap();
    c.reverify_source().unwrap();
    assert_eq!(c.cycles().len(), 2);
    assert_eq!(
        c.cycles().iter().map(Vec::len).sum::<usize>(),
        c.successor().len()
    );
}
#[test]
fn self_crossing_loop_walks_keep_each_original_traversal() {
    let source = Curve {
        degree: 3,
        knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        weights: vec![1.; 4],
        control_points: vec![
            vec![3., -1.5],
            vec![-1., 3.5],
            vec![-1., -3.5],
            vec![3., 1.5],
        ],
        periodic: false,
    };
    let c = p::own_original_carrier_cycles(
        Arc::new(graph(&[source])),
        p::ContactPortOrderLimits::default(),
    )
    .unwrap();
    c.reverify_source().unwrap();
    assert_eq!(c.cycles().len(), 2);
    let mut all = c.cycles().iter().flatten().copied().collect::<Vec<_>>();
    all.sort();
    assert_eq!(all, (0..6).collect::<Vec<_>>());
}
