use brep_core::profile_certificate::original_restriction::wire::material::Limits;
use brep_core::{
    Model,
    feature_ops::{
        full_round_curved::admitted::history::{Budget, History, codec},
        full_round_exact,
    },
};
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
fn earlier_selection_rebuilds_source_descendants_and_preserves_hidden_redo() {
    let mut h = root();
    let source = h.current().source().clone();
    let original = h.selected_source_ids();
    let opposite = [3, 2, 5].map(|i| source.1.faces[i]);
    h.rebase_source(&scale(&source, 1.25)).unwrap();
    h.reselect(original).unwrap();
    h.rebase_source(&scale(&source, 1.5)).unwrap();
    let before = codec::export(&h).unwrap();
    let retained = h.revisions()[3].body().model().clone();
    h.undo().unwrap();
    h.undo().unwrap();
    h.regenerate_selection(0, opposite).unwrap();
    assert_eq!(h.cursor(), 1);
    assert_eq!(h.revisions().len(), 4);
    assert_eq!(h.selected_source_ids(), opposite);
    assert_ne!(
        h.revisions()[0].body().model(),
        h.revisions()[1].body().model()
    );
    assert_ne!(
        h.revisions()[0].body().model(),
        &codec::restore(&before, Budget::default())
            .unwrap()
            .revisions()[0]
            .body()
            .model()
            .clone()
    );
    h.redo().unwrap();
    assert_eq!(h.selected_source_ids(), original);
    h.redo().unwrap();
    assert_eq!(h.current().model(), &retained);
    let doc = codec::export(&h).unwrap();
    let replay = codec::restore(&doc, Budget::default()).unwrap();
    assert_eq!(codec::export(&replay).unwrap(), doc);
    assert_eq!(h.work(), replay.work());
}
#[test]
fn edits_to_reselection_regenerate_later_source_steps_and_failure_is_atomic() {
    let mut h = root();
    let source = h.current().source().clone();
    let selected = [3, 2, 5].map(|i| source.1.faces[i]);
    h.reselect(selected).unwrap();
    h.rebase_source(&scale(&source, 1.25)).unwrap();
    let root_ids = [5, 4, 3].map(|i| source.1.faces[i]);
    h.regenerate_selection(1, root_ids).unwrap();
    assert_eq!(h.selected_source_ids(), root_ids);
    let expected = full_round_exact(&scale(&source, 1.25), [5, 4, 3], Limits::default()).unwrap();
    assert_eq!(h.current().model(), expected.model());
    let before = codec::export(&h).unwrap();
    assert!(h.regenerate_selection(2, selected).is_err());
    assert!(h.regenerate_selection(99, selected).is_err());
    assert!(h.regenerate_selection(0, [selected[0]; 3]).is_err());
    assert_eq!(codec::export(&h).unwrap(), before);
}
#[test]
fn missing_inherited_face_on_a_later_source_refuses_the_entire_regeneration() {
    let mut h = root();
    let source = h.current().source().clone();
    let opposite = [3, 2, 5].map(|i| source.1.faces[i]);
    let mut later = scale(&source, 1.25);
    // Changing an unselected face identity is valid for the existing selection,
    // but the edited earlier selection must not guess its replacement.
    later.1.faces[2] = brep_core::TopoId::parse("f:ffffffffffffffffffffffffffffffff").unwrap();
    later.1.change_set.nodes.insert(later.1.faces[2], later.1.faces[2].kind());
    later.validate().unwrap();
    h.rebase_source(&later).unwrap();
    let before = codec::export(&h).unwrap();
    let work = h.work();
    assert!(h.regenerate_selection(0, opposite).is_err());
    assert_eq!(codec::export(&h).unwrap(), before);
    assert_eq!(h.work(), work);
}
