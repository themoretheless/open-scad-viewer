use super::*;

fn budget() -> Budget {
    Budget::new(10_000_000, 8, 60_000).unwrap()
}

fn policy() -> RebindPolicy {
    RebindPolicy {
        linear_tolerance_mm: 2.,
        area_relative_tolerance: 0.25,
        angle_tolerance_rad: 1e-6,
    }
}

fn vertical_edge_at(model: &Model, xy: [f64; 2]) -> usize {
    model
        .edges
        .iter()
        .enumerate()
        .find_map(|(index, edge)| {
            let a = model.vertices[edge.vertices[0]].point;
            let b = model.vertices[edge.vertices[1]].point;
            ((a[0] - b[0]).abs() < 1e-9
                && (a[1] - b[1]).abs() < 1e-9
                && (a[2] - b[2]).abs() > 1e-9
                && ((a[0] + b[0]) * 0.5 - xy[0]).abs() < 1e-9
                && ((a[1] + b[1]) * 0.5 - xy[1]).abs() < 1e-9)
                .then_some(index)
        })
        .unwrap()
}

fn plane_face_at(model: &Model, axis: usize, offset: f64) -> usize {
    model
        .faces
        .iter()
        .enumerate()
        .find_map(|(index, _)| {
            let plane = face_plane_fit(model, index).ok()?;
            ((0..3).all(|other| {
                if other == axis {
                    plane.normal[other].abs() > 1. - 1e-9
                        && (plane.normal[other] * offset - plane.offset).abs() < 1e-8
                } else {
                    plane.normal[other].abs() < 1e-9
                }
            }))
            .then_some(index)
        })
        .unwrap()
}

fn fillet_move_history() -> FeatureHistory {
    let budget = budget();
    let block = cuboid([0.; 3], [10., 10., 10.]).unwrap();
    let edge = vertical_edge_at(&block, [10., 10.]);
    let edge_binding = TopologyBinding::from_edge(&block, edge, "block", &budget).unwrap();
    let filleted = analytic_fillet(&block, edge, 1.).unwrap().0;
    let wall = plane_face_at(&filleted, 0, 0.);
    let wall_binding = TopologyBinding::from_face(&filleted, wall, "fillet", &budget).unwrap();
    FeatureHistory::new(vec![
        HistoryNode::block("block", "block", [0.; 3], [10., 10., 10.]),
        HistoryNode::fillet("fillet", "fillet", "block", edge_binding, 1.),
        HistoryNode::move_face(
            "move_wall",
            "move_wall",
            "fillet",
            wall_binding,
            [1., 0., 0.],
            2.,
        ),
    ])
}

#[test]
fn extrude_fillet_move_rebinds_after_height_change() {
    let mut history = fillet_move_history();
    let budget = budget();
    let first = history.rebuild_and_commit(policy(), &budget).unwrap();
    assert!(
        first
            .bindings
            .iter()
            .all(|binding| binding.method == BindingResolutionMethod::Exact)
    );
    assert_eq!(first.result_hash, direct_edit_model_hash(&first.model));

    history
        .set_block_dimensions("block", [0.; 3], [10., 10., 12.])
        .unwrap();
    let second = history.rebuild_and_commit(policy(), &budget).unwrap();
    let moved = second
        .bindings
        .iter()
        .find(|binding| binding.feature_id == "move_wall")
        .unwrap();
    assert_eq!(moved.method, BindingResolutionMethod::Fuzzy);
    assert_ne!(moved.old_id, moved.new_id);
    let wall = second
        .model
        .1
        .faces
        .iter()
        .position(|id| id == &moved.new_id)
        .unwrap();
    assert_eq!(plane_face_at(&second.model, 0, 2.), wall);
    let committed = history
        .nodes()
        .iter()
        .find(|node| node.id == "move_wall")
        .and_then(HistoryNode::binding)
        .unwrap();
    assert_eq!(committed.id, second.model.1.faces[wall]);
    assert_eq!(second.result_hash, direct_edit_model_hash(&second.model));
}

#[test]
fn deleted_upstream_face_refuses_without_mutating_history() {
    let budget = budget();
    let block = cuboid([0.; 3], [10., 10., 10.]).unwrap();
    let wall = plane_face_at(&block, 0, 10.);
    let deleted = TopologyBinding::from_face(&block, wall, "block", &budget).unwrap();
    let moved = deleted.clone();
    let mut history = FeatureHistory::new(vec![
        HistoryNode::block("block", "block", [0.; 3], [10., 10., 10.]),
        HistoryNode::delete_face("delete_wall_node", "delete_wall", "block", deleted),
        HistoryNode::move_face(
            "move_wall",
            "move_wall",
            "delete_wall_node",
            moved,
            [1., 0., 0.],
            2.,
        ),
    ]);
    let before = history.clone();
    let error = history.rebuild_and_commit(policy(), &budget).unwrap_err();
    assert_eq!(error.code, FEATURE_HISTORY_BINDING_LOST);
    assert!(error.message.contains("move_wall"), "{}", error.message);
    assert!(
        error.message.contains("\"delete_wall\""),
        "{}",
        error.message
    );
    assert!(
        !error.message.contains("delete_wall_node"),
        "{}",
        error.message
    );
    assert_eq!(history, before);
}

#[test]
fn suppressed_history_rebuild_preserves_parameters_and_reports_cascade() {
    let mut history = fillet_move_history();
    let budget = budget();
    let baseline = history.rebuild(policy(), &budget).unwrap();
    let operations: Vec<HistoryOperation> = history
        .nodes()
        .iter()
        .map(|node| node.operation.clone())
        .collect();
    let preview = history.set_suppressed("fillet", true).unwrap();
    assert_eq!(preview.suppressed[0].feature_name, "fillet");
    assert_eq!(preview.dangling.len(), 1);
    assert_eq!(preview.dangling[0].feature_name, "move_wall");
    assert!(preview.dangling[0].reason.contains("fillet"));

    let suppressed = history.rebuild(policy(), &budget).unwrap();
    let block = cuboid([0.; 3], [10., 10., 10.]).unwrap();
    assert_eq!(suppressed.result_hash, direct_edit_model_hash(&block));
    assert!(suppressed.bindings.is_empty());
    assert_eq!(suppressed.suppression, preview);
    assert!(history.nodes().iter().all(|node| node.id == "fillet" || !node.suppressed));
    assert!(history.nodes().iter().find(|node| node.id == "fillet").unwrap().suppressed);
    assert_eq!(
        history
            .nodes()
            .iter()
            .map(|node| node.operation.clone())
            .collect::<Vec<_>>(),
        operations
    );

    history.set_suppressed("fillet", false).unwrap();
    let restored = history.rebuild(policy(), &budget).unwrap();
    assert_eq!(restored.result_hash, baseline.result_hash);
}
