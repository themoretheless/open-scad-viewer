#![cfg(feature = "codec")]
use brep_core::{
    Model,
    feature_ops::edge_partition::{
        self,
        history::{Budget, History},
    },
};
fn root() -> Model {
    let mut s = brep_core::cuboid([0.; 3], [2., 3., 4.]).unwrap();
    for w in &mut s.loops {
        for c in &mut w.coedges {
            if c.edge == 0 {
                *c = c
                    .clone()
                    .with_trim(brep_topology::CoedgeTrim {
                        curve_parameter: [0.13, 0.87],
                        pcurve_parameter: [0.13, 0.87],
                        periodic_lift: [[0; 2]; 2],
                    })
                    .unwrap();
            }
        }
    }
    s.validate().unwrap();
    s
}
fn chain() -> History {
    let mut h = History::new(&root(), Budget::default()).unwrap();
    let e = h
        .split(0, &[0.23, 0.71], "upstream", 1e-9, Budget::default())
        .unwrap()
        .edges()[1];
    h.split(e, &[0.47], "downstream", 1e-9, Budget::default())
        .unwrap();
    h
}
fn child_ids(h: &History, step: usize) -> Vec<brep_core::TopoId> {
    h.steps()[step]
        .edges()
        .iter()
        .map(|e| h.steps()[step].model().1.edges[*e])
        .collect()
}
#[test]
fn an_upstream_parameter_edit_regenerates_actual_dependent_geometry_and_original_relations() {
    let old = chain();
    let before = old.to_json(Budget::default()).unwrap();
    let edited = old.with_cuts(0, &[0.25, 0.75], Budget::default()).unwrap();
    assert_eq!(edited.root(), old.root());
    assert_eq!(child_ids(&edited, 0), child_ids(&old, 0));
    assert_eq!(child_ids(&edited, 1), child_ids(&old, 1));
    assert_ne!(edited.steps()[1].source(), old.steps()[1].source());
    assert_ne!(edited.steps()[0].relations(), old.steps()[0].relations());
    assert!(
        edited
            .steps()
            .iter()
            .all(|s| s.evidence().trim_regions_valid)
    );
    let loaded = History::from_json(
        &edited.to_json(Budget::default()).unwrap(),
        Budget::default(),
    )
    .unwrap();
    assert_eq!(loaded.model(), edited.model());
    assert_eq!(loaded.requests(), edited.requests());
    assert_eq!(old.to_json(Budget::default()).unwrap(), before);
    let same = History::from_requests(old.root(), &old.requests(), Budget::default()).unwrap();
    assert_eq!(same.to_json(Budget::default()).unwrap(), before);
}
#[test]
fn removed_descendants_and_out_of_domain_downstream_clocks_cannot_publish_partial_edits() {
    let mut h = chain();
    let before = h.to_json(Budget::default()).unwrap();
    assert!(h.replace_cuts(0, &[0.1, 0.2], Budget::default()).is_err());
    assert_eq!(h.to_json(Budget::default()).unwrap(), before);
    let mut h = History::new(&root(), Budget::default()).unwrap();
    let e = h
        .split(0, &[0.2, 0.5], "parent", 1e-9, Budget::default())
        .unwrap()
        .edges()[2];
    h.split(e, &[0.75], "selected-third", 1e-9, Budget::default())
        .unwrap();
    let before = h.to_json(Budget::default()).unwrap();
    let err = match h.replace_cuts(0, &[0.2], Budget::default()) {
        Ok(()) => panic!("missing parent must refuse"),
        Err(e) => e,
    };
    assert!(err.message.contains("parent edge identity"));
    assert_eq!(h.to_json(Budget::default()).unwrap(), before);
}
#[test]
fn semantic_parent_ids_survive_root_placement_and_edge_storage_permutation() {
    let h = chain();
    let mut moved = brep_core::transform::affine(
        h.root(),
        [
            [1., 0., 0., 10.],
            [0., 1., 0., 20.],
            [0., 0., 1., 30.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let n = moved.edges.len();
    moved.edges.reverse();
    moved.1.edges.reverse();
    for w in &mut moved.loops {
        for c in &mut w.coedges {
            c.edge = n - 1 - c.edge;
        }
    }
    moved.validate().unwrap();
    let rebuilt = h.rebased(&moved, Budget::default()).unwrap();
    assert_eq!(rebuilt.requests(), h.requests());
    assert_eq!(child_ids(&rebuilt, 0), child_ids(&h, 0));
    assert_eq!(child_ids(&rebuilt, 1), child_ids(&h, 1));
    assert_eq!(rebuilt.root(), &moved);
    assert_eq!(rebuilt.model().vertices[0].point, [10., 20., 30.]);
    rebuilt.reverify(Budget::default()).unwrap();
}
#[test]
fn a_prefix_can_branch_and_undo_without_consuming_the_original_future() {
    let h = chain();
    let original = h.to_json(Budget::default()).unwrap();
    let mut branch = h.prefix(1, Budget::default()).unwrap();
    assert_eq!(branch.model(), h.steps()[0].model());
    let e = branch.steps()[0].edges()[2];
    let d = branch.model().edges[e].curve.domain();
    branch
        .split(
            e,
            &[(d[0] + d[1]) * 0.5],
            "alternative",
            1e-9,
            Budget::default(),
        )
        .unwrap();
    assert_ne!(child_ids(&branch, 1), child_ids(&h, 1));
    branch.truncate(1, Budget::default()).unwrap();
    assert_eq!(branch.model(), h.steps()[0].model());
    branch.truncate(0, Budget::default()).unwrap();
    assert_eq!(branch.model(), h.root());
    assert_eq!(h.to_json(Budget::default()).unwrap(), original);
}
#[test]
fn explicit_regeneration_can_requalify_under_stricter_caps_without_trusting_old_receipts() {
    let h = chain();
    let lower = Budget {
        proof: edge_partition::Limits {
            max_cells: 50000,
            ..Default::default()
        },
        ..Budget::default()
    };
    assert!(h.to_json(lower).is_err());
    let rebuilt = h.rebased(h.root(), lower).unwrap();
    assert_eq!(rebuilt.model(), h.model());
    History::from_json(&rebuilt.to_json(lower).unwrap(), lower).unwrap();
    let before = h.to_json(Budget::default()).unwrap();
    assert!(
        h.with_cuts(
            0,
            &[0.25, 0.75],
            Budget {
                max_bytes: 1,
                ..Budget::default()
            }
        )
        .is_err()
    );
    assert_eq!(h.to_json(Budget::default()).unwrap(), before);
}
#[test]
fn requests_are_data_and_invalid_parent_or_parameter_edits_require_fresh_admission() {
    let h = chain();
    let before = h.to_json(Budget::default()).unwrap();
    let mut requests = h.requests();
    requests[0].parent = brep_core::TopoId::derive(
        brep_core::TopoKind::Edge,
        "absent",
        "root",
        "edge",
        b"foreign",
    );
    assert!(History::from_requests(h.root(), &requests, Budget::default()).is_err());
    let mut requests = h.requests();
    requests[1].cuts = vec![f64::NAN];
    assert!(History::from_requests(h.root(), &requests, Budget::default()).is_err());
    assert!(h.with_cuts(usize::MAX, &[0.5], Budget::default()).is_err());
    assert!(h.prefix(3, Budget::default()).is_err());
    assert_eq!(h.to_json(Budget::default()).unwrap(), before);
}

#[path = "common/closed_periodic_source.rs"]
mod closed_fixture;
#[test]
fn a_periodic_rational_ancestor_can_change_while_its_selected_descendant_is_requalified() {
    let root = closed_fixture::source();
    let mut h = History::new(&root, Budget::default()).unwrap();
    let e = h
        .split(0, &[0.37], "closed-a", 1e-8, Budget::default())
        .unwrap()
        .edges()[1];
    h.split(e, &[2.25], "closed-b", 1e-8, Budget::default())
        .unwrap();
    let before = h.to_json(Budget::default()).unwrap();
    let changed = h.with_cuts(0, &[0.6], Budget::default()).unwrap();
    assert_eq!(child_ids(&changed, 0), child_ids(&h, 0));
    assert_eq!(child_ids(&changed, 1), child_ids(&h, 1));
    assert_eq!(
        changed.steps()[1].source().edges[e].curve.domain(),
        [0.6, 4.]
    );
    assert_eq!(changed.model().vertices[0], root.vertices[0]);
    assert_eq!(changed.model().loops[0].coedges.len(), 3);
    changed.reverify(Budget::default()).unwrap();
    assert_eq!(h.to_json(Budget::default()).unwrap(), before);
}
