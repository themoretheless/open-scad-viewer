#![cfg(feature = "codec")]
use brep_core::{
    Model,
    feature_ops::edge_partition::history::{
        Budget, History,
        sequence::{Applied, Command, Operation, Sequence},
    },
};
use value_codec::{Serialize, Value};
#[path = "common/closed_periodic_source.rs"]
mod fixture;
fn root() -> Model {
    let mut s = fixture::source();
    for l in &mut s.loops {
        let c = l.coedges[0].clone();
        l.coedges[0] = c
            .with_trim(brep_topology::CoedgeTrim {
                curve_parameter: [0., 4.],
                pcurve_parameter: [0., 4.],
                periodic_lift: [[0; 2]; 2],
            })
            .unwrap();
    }
    s
}
fn command(parent: brep_core::TopoId, operation: Operation, key: &str) -> Command {
    Command {
        parent,
        operation,
        key: key.into(),
        uv_tolerance: 1e-8,
    }
}
fn chain() -> Sequence {
    let b = Budget::default();
    let s = root();
    let mut h = Sequence::new(&s, b).unwrap();
    let step = h
        .apply(
            command(
                s.1.edges[0],
                Operation::Split {
                    cuts: vec![0.5, 1.5, 2.5],
                },
                "split",
            ),
            b,
        )
        .unwrap();
    let Applied::Split(split) = step.applied() else {
        panic!()
    };
    let parent = split.model().1.edges[split.edges()[1]];
    h.apply(
        command(parent, Operation::Relocate { parameter: 0.5 }, "seam"),
        b,
    )
    .unwrap();
    let step = h
        .apply(command(parent, Operation::Compact, "compact"), b)
        .unwrap();
    let Applied::Compact(compact) = step.applied() else {
        panic!()
    };
    assert_eq!(compact.relations().len(), 8);
    let merged = compact.model().1.edges[compact.edge()];
    h.apply(
        command(
            merged,
            Operation::Split {
                cuts: vec![0.375, 2.25],
            },
            "after-merge",
        ),
        b,
    )
    .unwrap();
    h
}
#[test]
fn split_seam_compact_split_roundtrip_reconstructs_actual_models_and_authored_maps() {
    let b = Budget::default();
    let h = chain();
    let packet = h.to_json(b).unwrap();
    let loaded = Sequence::from_json(&packet, b).unwrap();
    assert_eq!(loaded.to_json(b).unwrap(), packet);
    assert_eq!(loaded.commands(), h.commands());
    assert_eq!(loaded.model(), h.model());
    assert_eq!(loaded.steps().len(), 4);
    assert!(
        loaded
            .steps()
            .iter()
            .all(|s| s.evidence().trim_regions_valid)
    );
    let Applied::Compact(a) = loaded.steps()[2].applied() else {
        panic!()
    };
    let Applied::Compact(original) = h.steps()[2].applied() else {
        panic!()
    };
    assert_eq!(a.relations(), original.relations());
    let Applied::Split(last) = loaded.steps()[3].applied() else {
        panic!()
    };
    assert_eq!(last.relations().len(), 2);
    assert_eq!(h.root(), &root());
}
#[test]
fn editing_an_upstream_split_rebuilds_seam_merge_and_later_split_with_persistent_parents() {
    let b = Budget::default();
    let h = chain();
    let before = h.to_json(b).unwrap();
    let mut change = h.commands()[0].clone();
    change.operation = Operation::Split {
        cuts: vec![0.5, 1.75, 2.75],
    };
    let edited = h.with_command(0, change, b).unwrap();
    assert_ne!(edited.steps()[1].model(), h.steps()[1].model());
    assert_ne!(edited.steps()[2].model(), h.steps()[2].model());
    assert_eq!(edited.model().1.edges, h.model().1.edges);
    assert_eq!(&edited.commands()[1..], &h.commands()[1..]);
    let restored = Sequence::from_json(&edited.to_json(b).unwrap(), b).unwrap();
    assert_eq!(restored.model(), edited.model());
    assert_eq!(h.to_json(b).unwrap(), before);
}
#[test]
fn missing_downstream_parents_and_invalid_commands_leave_the_original_history_atomic() {
    let b = Budget::default();
    let mut h = chain();
    let before = h.to_json(b).unwrap();
    let mut missing = h.commands()[0].clone();
    missing.operation = Operation::Split { cuts: vec![0.5] };
    assert!(h.replace_command(0, missing, b).is_err());
    assert_eq!(h.to_json(b).unwrap(), before);
    let parent = h.model().1.edges[0];
    assert!(
        h.apply(
            command(parent, Operation::Relocate { parameter: 999. }, "bad"),
            b
        )
        .is_err()
    );
    assert_eq!(h.to_json(b).unwrap(), before);
    let small = Budget {
        max_bytes: before.len() + 1,
        ..b
    };
    assert!(
        h.apply(
            command(parent, Operation::Split { cuts: vec![0.1] }, "oversized"),
            small
        )
        .is_err()
    );
    assert_eq!(h.to_json(b).unwrap(), before);
    assert!(
        h.apply(
            command(parent, Operation::Compact, "fifth"),
            Budget { max_steps: 4, ..b }
        )
        .is_err()
    );
    assert_eq!(h.to_json(b).unwrap(), before);
}
#[test]
fn mixed_output_maps_models_kinds_and_caller_limits_cannot_be_forged() {
    let b = Budget::default();
    let h = chain();
    let packet = h.to_json(b).unwrap();
    let v: Value = value_codec::from_str_strict(&packet).unwrap();
    let mut map = v.clone();
    map["steps"][2]["maps"]["relations"][0]["curve"][0] = Value::from("17/13");
    assert!(Sequence::from_json(&map.to_string(), b).is_err());
    let mut direction = v.clone();
    direction["steps"][2]["maps"]["cycle"][0]["reversed"] = Value::Bool(true);
    assert!(Sequence::from_json(&direction.to_string(), b).is_err());
    let mut model = v.clone();
    let old_x = model["steps"][2]["model"]["edges"][0]["curve"]["controlPoints"][0][0]
        .as_f64()
        .unwrap();
    model["steps"][2]["model"]["edges"][0]["curve"]["controlPoints"][0][0] =
        value_codec::json!(old_x + 0.001);
    assert!(Sequence::from_json(&model.to_string(), b).is_err());
    let mut kind = v.clone();
    kind["steps"][0]["command"]["operation"]["kind"] = Value::from("trust-saved");
    assert!(Sequence::from_json(&kind.to_string(), b).is_err());
    let mut receipt = v.clone();
    receipt["steps"][2]["proofAccepted"] = Value::Bool(true);
    assert!(Sequence::from_json(&receipt.to_string(), b).is_err());
    let mut caps = v.clone();
    caps["steps"][1]["limits"]["cells"] = value_codec::json!(100001);
    assert!(Sequence::from_json(&caps.to_string(), b).is_err());
    assert!(
        Sequence::from_json(
            &packet,
            Budget {
                proof: brep_core::feature_ops::edge_partition::Limits {
                    max_cells: 99999,
                    ..b.proof
                },
                ..b
            }
        )
        .is_err()
    );
    let replay = Sequence::from_commands(
        h.root(),
        &h.commands(),
        Budget {
            proof: brep_core::feature_ops::edge_partition::Limits {
                max_cells: 99999,
                ..b.proof
            },
            ..b
        },
    )
    .unwrap();
    assert_eq!(replay.model(), h.model());
    let duplicate = packet.replacen(
        "\"schema\":\"brep-edge-history/2\"",
        "\"schema\":\"brep-edge-history/2\",\"schema\":\"brep-edge-history/2\"",
        1,
    );
    assert_ne!(duplicate, packet);
    assert!(Sequence::from_json(&duplicate, b).is_err());
}
#[test]
fn a_prefix_branches_and_rebases_on_an_explicit_canonical_root_without_consuming_future_steps() {
    let b = Budget::default();
    let h = chain();
    let before = h.to_json(b).unwrap();
    let mut branch = h.prefix(2, b).unwrap();
    assert_eq!(branch.model(), h.steps()[1].model());
    let parent = branch.commands()[1].parent;
    branch
        .apply(command(parent, Operation::Compact, "branch-merge"), b)
        .unwrap();
    assert_ne!(branch.model().1.edges, h.steps()[2].model().1.edges);
    branch.truncate(1, b).unwrap();
    assert_eq!(branch.model(), h.steps()[0].model());
    let mut moved = h.root().clone();
    let translate = |p: &mut [f64]| {
        p[0] += 4.;
        p[1] -= 2.;
        p[2] += 1.;
    };
    for v in &mut moved.vertices {
        translate(&mut v.point);
    }
    for e in &mut moved.edges {
        for p in &mut e.curve.control_points {
            translate(p);
        }
    }
    for f in &mut moved.faces {
        for row in &mut f.surface.control_points {
            for p in row {
                translate(p);
            }
        }
    }
    moved.validate().unwrap();
    let rebased = h.rebased(&moved, b).unwrap();
    assert_eq!(rebased.commands(), h.commands());
    assert_eq!(rebased.model().1.edges, h.model().1.edges);
    assert_ne!(rebased.model(), h.model());
    assert_eq!(h.to_json(b).unwrap(), before);
}
#[test]
fn legacy_split_documents_migrate_by_replaying_once_and_can_continue_with_a_merge() {
    let b = Budget::default();
    let s = root();
    let mut old = History::new(&s, b).unwrap();
    old.split(0, &[0.5, 1.5, 2.5], "legacy", 1e-8, b).unwrap();
    let packet = old.to_json(b).unwrap();
    let mut migrated = Sequence::from_json(&packet, b).unwrap();
    assert_eq!(migrated.model(), old.model());
    assert_eq!(
        migrated.steps()[0].evidence().cells,
        old.steps()[0].evidence().cells
    );
    let parent = migrated.model().1.edges[1];
    migrated
        .apply(command(parent, Operation::Compact, "after-migration"), b)
        .unwrap();
    let v2 = migrated.to_json(b).unwrap();
    assert!(v2.contains("brep-edge-history/2"));
    assert!(History::from_json(&v2, b).is_err());
    assert_eq!(
        Sequence::from_json(&v2, b).unwrap().model(),
        migrated.model()
    );
    assert_eq!(old.to_json(b).unwrap(), packet);
    assert!(migrated.model().to_value().is_object());
}
