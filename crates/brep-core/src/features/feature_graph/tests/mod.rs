use super::*;
use crate::{ChangeKind, ChangeProvenance, TopologyChange};
use nurbs_core::foundation::guards::Budget;

fn topo() -> TopoId {
    TopoId::derive(TopoKind::Face, "extrude_2", "occurrence", "cap", b"cap")
}

fn naming_graph() -> FeatureGraph {
    let id = topo();
    let changes = ChangeSet {
        nodes: [(id, TopoKind::Face)].into(),
        changes: vec![TopologyChange {
            kind: ChangeKind::Generated,
            topo_kind: TopoKind::Face,
            parents: vec![],
            children: vec![id],
            provenance: ChangeProvenance {
                operation: "extrude_2".into(),
                operand: None,
                occurrence: "occurrence".into(),
            },
            role: "cap".into(),
            anchor: None,
        }],
    };
    FeatureGraph::from_change_set(
        vec![
            Feature::new("extrude_2", "extrude_2"),
            Feature::new("fillet_3", "fillet_3").with_inputs([id]),
        ],
        &changes,
    )
    .unwrap()
}

fn ten_feature_graph() -> FeatureGraph {
    let mut features = Vec::new();
    for index in 0..9 {
        let feature = Feature::new(format!("feature_{index}"), format!("feature_{index}"));
        features.push(if index == 0 {
            feature
        } else {
            feature.with_dependencies([format!("feature_{}", index - 1)])
        });
    }
    features.push(Feature::new("move_top", "move_top"));
    FeatureGraph::new(features).unwrap()
}

fn rebuild_move(
    graph: &FeatureGraph,
    order: &[String],
    initial: &Model,
) -> FeatureRebuildReport {
    graph
        .trial_rebuild(order, initial, |context| {
            if context.feature().id != "move_top" {
                return Ok(context.model().clone());
            }
            let top = context
                .model()
                .faces
                .iter()
                .position(|face| {
                    face.surface
                        .control_points
                        .iter()
                        .flatten()
                        .all(|point| (point[2] - 1.).abs() < 1e-8)
                })
                .unwrap();
            let budget = Budget::new(10_000_000, 8, 60_000).unwrap();
            Ok(
                crate::direct_edit::move_face(context.model(), top, [0., 0., 1.], 1., &budget)?
                    .model,
            )
        })
        .unwrap()
}

#[test]
fn all_ten_topological_orders_rebuild_to_identical_hash() {
    let graph = ten_feature_graph();
    let initial = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    let baseline = rebuild_move(&graph, graph.order(), &initial).result_hash;
    // Nine features form a chain and one is independent: exactly ten
    // topological permutations, one for each insertion position.
    for position in 0..10 {
        let (candidate, _) = graph.candidate_order("move_top", position).unwrap();
        graph.validate_order(&candidate).unwrap();
        let rebuilt = rebuild_move(&graph, &candidate, &initial);
        assert_eq!(rebuilt.rebuilt_features, 10);
        assert_eq!(rebuilt.result_hash, baseline, "order {candidate:?}");
    }
}

#[test]
fn invalid_reorder_names_consumer_and_topology_producer() {
    let graph = naming_graph();
    let error = graph.candidate_order("fillet_3", 0).unwrap_err();
    assert_eq!(error.code, FEATURE_REORDER_CONFLICT);
    assert!(error.message.contains("fillet_3"), "{}", error.message);
    assert!(error.message.contains("extrude_2"), "{}", error.message);
    assert!(
        error.message.contains(&topo().to_string()),
        "{}",
        error.message
    );
}

#[test]
fn rebuild_detects_missing_persistent_input_with_both_feature_names() {
    let graph = naming_graph();
    let initial = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    let error = graph
        .trial_rebuild(graph.order(), &initial, |context| {
            Ok(context.model().clone())
        })
        .unwrap_err();
    assert_eq!(error.code, FEATURE_REBUILD_NAMING);
    assert!(error.message.contains("fillet_3"), "{}", error.message);
    assert!(error.message.contains("extrude_2"), "{}", error.message);
}

#[test]
fn allowed_position_search_is_one_edge_scan() {
    let graph = ten_feature_graph();
    let search = graph.allowed_positions("feature_5").unwrap();
    assert_eq!(search.checked_edges, graph.dependency_edge_count());
    assert_eq!(search.checked_edges, 8);
    assert_eq!(search.positions, 5..=5);
    let search = graph.allowed_positions("move_top").unwrap();
    assert_eq!(search.checked_edges, 8);
    assert_eq!(search.positions, 0..=9);
}

#[test]
fn suppressed_rebuild_skips_cascade_but_preserves_nodes() {
    let mut graph = FeatureGraph::new(vec![
        Feature::new("a", "root"),
        Feature::new("b", "blend").with_dependencies(["a".into()]),
        Feature::new("c", "move").with_dependencies(["b".into()]),
        Feature::new("d", "finish").with_dependencies(["c".into()]),
        Feature::new("e", "independent"),
    ])
    .unwrap();
    let cascade = graph.set_suppressed("b", true).unwrap();
    assert_eq!(cascade.suppressed[0].feature_name, "blend");
    assert_eq!(
        cascade
            .dangling
            .iter()
            .map(|feature| feature.feature_name.as_str())
            .collect::<Vec<_>>(),
        ["move", "finish"]
    );
    assert!(cascade.dangling[0].reason.contains("blend"));
    assert!(cascade.dangling[1].reason.contains("move"));
    let initial = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    let mut executed = Vec::new();
    let report = graph
        .trial_rebuild(graph.order(), &initial, |context| {
            executed.push(context.feature().id.clone());
            Ok(context.model().clone())
        })
        .unwrap();
    assert_eq!(executed, ["a", "e"]);
    assert_eq!(report.rebuilt_features, 2);
    assert_eq!(graph.feature("b").unwrap().name, "blend");
    assert!(graph.feature("b").unwrap().suppressed);
    assert!(!graph.feature("c").unwrap().suppressed);
}

#[test]
fn dependency_subgraph_and_deletion_preview_are_transitive_and_read_only() {
    let graph = FeatureGraph::new(vec![
        Feature::new("a", "root"),
        Feature::new("b", "left").with_dependencies(["a".into()]),
        Feature::new("c", "right").with_dependencies(["a".into()]),
        Feature::new("d", "left-detail").with_dependencies(["b".into()]),
        Feature::new("e", "right-detail").with_dependencies(["c".into()]),
        Feature::new("f", "join").with_dependencies(["d".into(), "e".into()]),
        Feature::new("g", "finish").with_dependencies(["f".into()]),
        Feature::new("h", "independent"),
    ])
    .unwrap();
    let before = graph.clone();
    fn ids(features: &[FeatureDescriptor]) -> Vec<&str> {
        features
            .iter()
            .map(|feature| feature.feature_id.as_str())
            .collect()
    }

    let subgraph = dependency_subgraph(&graph, "f").unwrap();
    assert_eq!(subgraph.feature.feature_id, "f");
    assert_eq!(ids(&subgraph.parents), ["a", "b", "c", "d", "e"]);
    assert_eq!(ids(&subgraph.children), ["g"]);

    let deletion = deletion_preview(&graph, "b").unwrap();
    assert_eq!(deletion.direct.feature_id, "b");
    assert_eq!(ids(&deletion.parents), ["a"]);
    assert_eq!(ids(&deletion.cascade), ["d", "f", "g"]);
    assert_eq!(graph, before);
}

#[test]
fn cycle_is_rejected() {
    let error = FeatureGraph::new(vec![
        Feature::new("a", "a").with_dependencies(["b".into()]),
        Feature::new("b", "b").with_dependencies(["a".into()]),
    ])
    .unwrap_err();
    assert_eq!(error.code, FEATURE_GRAPH_INVALID);
    assert!(error.message.contains("cycle"));
}
