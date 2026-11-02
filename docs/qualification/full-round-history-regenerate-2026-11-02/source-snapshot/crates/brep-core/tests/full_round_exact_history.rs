use brep_core::profile_certificate::original_restriction::wire::material::Limits;
use brep_core::{
    Model,
    feature_ops::{
        full_round_curved::admitted::history::{Budget, Edit, History, codec},
        full_round_exact,
    },
};
use value_codec::json;
fn root() -> History {
    let source = brep_core::cuboid([0.; 3], [4., 8., 6.]).unwrap();
    History::new(
        full_round_exact(&source, [5, 4, 3], Limits::default()).unwrap(),
        Budget::default(),
    )
    .unwrap()
}
fn scale(source: &Model, z: f64) -> Model {
    brep_core::transform::affine(
        source,
        [
            [1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [0., 0., z, 0.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap()
}
#[test]
fn source_regeneration_binds_topology_ids_after_face_reordering() {
    let mut h = root();
    let ids = h.selected_source_ids();
    let mut source = scale(h.current().source(), 1.25);
    source.faces.reverse();
    source.1.faces.reverse();
    let count = source.faces.len();
    for shell in &mut source.shells {
        for use_ in &mut shell.faces {
            use_.face = count - 1 - use_.face;
        }
    }
    source.validate().unwrap();
    h.rebase_source(&source).unwrap();
    assert_eq!(h.selected_source_ids(), ids);
    assert_eq!(h.current().selected_faces(), [0, 1, 2]);
    assert_eq!(
        h.current().material().result().shell_geometry().height(),
        [
            ["0".to_string(), "1".to_string()],
            ["15".to_string(), "2".to_string()]
        ]
    );
    assert_eq!(h.revisions()[1].edit(), Edit::RebaseSource);
}
#[test]
fn undo_redo_and_branch_preserve_original_redo_and_owned_bodies() {
    let mut h = root();
    let source = h.current().source().clone();
    h.rebase_source(&scale(&source, 1.25)).unwrap();
    let second = h.current().model().clone();
    h.undo().unwrap();
    assert!(h.rebase_source(&scale(&source, 1.5)).is_err());
    let mut branch = h.branch().unwrap();
    branch.rebase_source(&scale(&source, 1.5)).unwrap();
    assert_ne!(branch.current().model(), &second);
    assert_eq!(h.redo().unwrap().model(), &second);
    assert_eq!(h.cursor(), 1);
    assert_eq!(branch.cursor(), 1);
    assert!(h.redo().is_err());
}
#[test]
fn selection_edit_rebuilds_the_other_end_on_the_same_source() {
    let mut h = root();
    let original = h.current().model().clone();
    let selected = [3, 2, 5].map(|i| h.current().source().1.faces[i]);
    h.reselect(selected).unwrap();
    assert_ne!(h.current().model(), &original);
    assert_eq!(h.revisions()[1].edit(), Edit::Reselect);
    let document = codec::export(&h).unwrap();
    assert_eq!(
        codec::export(&codec::restore(&document, Budget::default()).unwrap()).unwrap(),
        document
    );
}
#[test]
fn missing_ids_invalid_selection_and_resource_failures_leave_history_unchanged() {
    let mut h = root();
    let before = codec::export(&h).unwrap();
    let fresh = brep_core::cuboid([0.; 3], [4., 8., 7.5]).unwrap();
    assert!(h.rebase_source(&fresh).is_err());
    let id = h.selected_source_ids()[0];
    assert!(h.reselect([id; 3]).is_err());
    assert_eq!(codec::export(&h).unwrap(), before);
    let source = h.current().source();
    let work = h.current().material().certificate().relation().work();
    let limits = Limits {
        max_pair_projections: work.pair_projections,
        max_atoms: work.atoms,
        max_ray_cells: work.ray_cells,
        ..Limits::default()
    };
    let body = full_round_exact(source, [5, 4, 3], limits).unwrap();
    let mut exhausted = History::new(
        body,
        Budget {
            material: limits,
            ..Budget::default()
        },
    )
    .unwrap();
    let original = codec::export(&exhausted).unwrap();
    assert!(exhausted.rebase_source(&scale(source, 1.25)).is_err());
    assert_eq!(codec::export(&exhausted).unwrap(), original);
}
#[test]
fn frozen_history_replay_preserves_all_revisions_and_cursor() {
    let mut h = root();
    let source = h.current().source().clone();
    h.rebase_source(&scale(&source, 1.25)).unwrap();
    h.rebase_source(&scale(&source, 1.5)).unwrap();
    h.undo().unwrap();
    let document = codec::export(&h).unwrap();
    let restored = codec::restore(&document, Budget::default()).unwrap();
    assert_eq!(restored.cursor(), 1);
    assert_eq!(restored.revisions().len(), 3);
    assert_eq!(restored.work(), h.work());
    assert_eq!(codec::export(&restored).unwrap(), document);
}
#[test]
fn import_checks_hidden_redo_and_operation_semantics_under_caller_limits() {
    let mut h = root();
    let source = h.current().source().clone();
    h.rebase_source(&scale(&source, 1.25)).unwrap();
    h.undo().unwrap();
    let document = codec::export(&h).unwrap();
    let mut bad = document.clone();
    bad["revisions"][1]["artifact"]["geometry"]["vertices"][0]["point"][0] = json!(["999", "1"]);
    assert!(
        codec::restore(&bad, Budget::default())
            .unwrap_err()
            .message
            .contains("differs")
    );
    let mut bad = document.clone();
    bad["revisions"][1]["edit"] = json!("reselect");
    assert!(
        codec::restore(&bad, Budget::default())
            .unwrap_err()
            .message
            .contains("original source")
    );
    for budget in [
        Budget {
            max_revisions: 1,
            ..Budget::default()
        },
        Budget {
            max_bytes: 1,
            ..Budget::default()
        },
        Budget {
            material: Limits {
                max_pair_projections: 1,
                ..Limits::default()
            },
            ..Budget::default()
        },
    ] {
        assert!(codec::restore(&document, budget).is_err());
    }
}

#[test]
fn replay_rejects_an_independently_valid_body_with_wrong_source_id_ancestry() {
    let mut h = root();
    let source = h.current().source().clone();
    h.rebase_source(&scale(&source, 1.25)).unwrap();
    let mut document = codec::export(&h).unwrap();
    let independent = brep_core::cuboid([0.; 3], [4., 8., 9.]).unwrap();
    let limits = h.current().material().certificate().relation().limits();
    let valid = full_round_exact(&independent, [5, 4, 3], limits).unwrap();
    document["revisions"][1]["artifact"] =
        brep_core::feature_ops::full_round_curved::admitted::transport::export(&valid, 4 << 20)
            .unwrap();
    assert!(
        codec::restore(&document, Budget::default())
            .unwrap_err()
            .message
            .contains("authored face IDs")
    );
}

#[test]
fn revision_and_serialized_storage_limits_are_atomic_on_live_edits() {
    let original = root();
    let source = original.current().source().clone();
    let mut one = History::new(
        original.current().clone(),
        Budget {
            max_revisions: 1,
            ..Budget::default()
        },
    )
    .unwrap();
    let document = codec::export(&one).unwrap();
    assert!(one.rebase_source(&scale(&source, 1.25)).is_err());
    assert_eq!(codec::export(&one).unwrap(), document);
    let bytes = value_codec::encode_binary(&document).unwrap().len();
    let mut bounded = History::new(
        original.current().clone(),
        Budget {
            max_bytes: bytes + 1,
            ..Budget::default()
        },
    )
    .unwrap();
    let before = codec::export(&bounded).unwrap();
    assert!(bounded.rebase_source(&scale(&source, 1.25)).is_err());
    assert_eq!(codec::export(&bounded).unwrap(), before);
}
