#![cfg(feature = "codec")]
use brep_core::feature_ops::edge_partition::history::{
    Budget,
    sequence::{Applied, Command, Operation, Sequence},
};
use value_codec::Value;
#[path = "common/closed_periodic_source.rs"]
mod fixture;
fn request(parent: brep_core::TopoId, operation: Operation, key: &str) -> Command {
    Command {
        parent,
        operation,
        key: key.into(),
        uv_tolerance: 1e-8,
    }
}
fn chain() -> Sequence {
    let b = Budget::default();
    let mut root = fixture::source();
    for l in &mut root.loops {
        l.coedges[0] = l.coedges[0]
            .clone()
            .with_trim(brep_topology::CoedgeTrim {
                curve_parameter: [0., 4.],
                pcurve_parameter: [0., 4.],
                periodic_lift: [[0; 2]; 2],
            })
            .unwrap();
    }
    let mut h = Sequence::new(&root, b).unwrap();
    let s = h
        .apply(
            request(
                root.1.edges[0],
                Operation::Split {
                    cuts: vec![0.5, 1.5, 2.5],
                },
                "split",
            ),
            b,
        )
        .unwrap();
    let Applied::Split(s) = s.applied() else {
        panic!()
    };
    let selected = s.model().1.edges[s.edges()[1]];
    h.apply(
        request(selected, Operation::Relocate { parameter: 0.5 }, "seam"),
        b,
    )
    .unwrap();
    let s = h
        .apply(request(selected, Operation::Compact, "merge"), b)
        .unwrap();
    let Applied::Compact(s) = s.applied() else {
        panic!()
    };
    let selected = s.model().1.edges[s.edge()];
    let s = h
        .apply(
            request(selected, Operation::Reclock { domain: [-2., 10.] }, "clock"),
            b,
        )
        .unwrap();
    let Applied::Reclock(s) = s.applied() else {
        panic!()
    };
    let selected = s.model().1.edges[s.edge()];
    h.apply(
        request(
            selected,
            Operation::Split {
                cuts: vec![-0.875, 4.75],
            },
            "split-new-clock",
        ),
        b,
    )
    .unwrap();
    h
}
#[test]
fn a_mixed_native_chain_reloads_changed_world_clocks_and_original_authored_relations() {
    let b = Budget::default();
    let h = chain();
    let packet = h.to_json(b).unwrap();
    let loaded = Sequence::from_json(&packet, b).unwrap();
    assert_eq!(loaded.to_json(b).unwrap(), packet);
    assert_eq!(loaded.model(), h.model());
    assert_eq!(loaded.steps().len(), 5);
    let Applied::Reclock(r) = loaded.steps()[3].applied() else {
        panic!()
    };
    assert_eq!(r.domain(), [-2., 10.]);
    assert_eq!(r.relations().len(), 2);
    for relation in r.relations() {
        assert_eq!(relation.original.curve_parameter, [0., 4.]);
        assert_eq!(relation.transferred.curve_parameter, [-2., 10.]);
        assert_eq!(relation.transferred.pcurve_parameter, [0., 4.]);
    }
    assert!(
        loaded
            .steps()
            .iter()
            .all(|s| s.evidence().trim_regions_valid)
    );
}
#[test]
fn editing_a_world_clock_reconstructs_later_literal_parameter_requests_without_consuming_the_original()
 {
    let b = Budget::default();
    let h = chain();
    let before = h.to_json(b).unwrap();
    let mut command = h.commands()[3].clone();
    command.operation = Operation::Reclock { domain: [-4., 12.] };
    let edited = h.with_command(3, command, b).unwrap();
    assert_ne!(edited.model(), h.model());
    assert_eq!(edited.model().1.edges, h.model().1.edges);
    assert_eq!(edited.commands()[4], h.commands()[4]);
    let loaded = Sequence::from_json(&edited.to_json(b).unwrap(), b).unwrap();
    assert_eq!(loaded.model(), edited.model());
    assert_eq!(h.to_json(b).unwrap(), before);
    let mut partial = h.prefix(3, b).unwrap();
    let before = partial.to_json(b).unwrap();
    let mut bad = h.commands()[3].clone();
    bad.uv_tolerance = 0.;
    assert!(partial.apply(bad, b).is_err());
    assert_eq!(partial.to_json(b).unwrap(), before);
}
#[test]
fn saved_clock_images_and_changed_requests_cannot_authorize_an_old_output() {
    let b = Budget::default();
    let h = chain();
    let packet = h.to_json(b).unwrap();
    let v: Value = value_codec::from_str_strict(&packet).unwrap();
    let mut maps = v.clone();
    maps["steps"][3]["maps"]["relations"][0]["curve"][0] = Value::from("-2001/1000");
    assert!(Sequence::from_json(&maps.to_string(), b).is_err());
    let mut request = v.clone();
    request["steps"][3]["command"]["operation"]["domain"] = value_codec::json!([-4., 12.]);
    assert!(Sequence::from_json(&request.to_string(), b).is_err());
    let mut malformed = v.clone();
    malformed["steps"][3]["command"]["operation"]["domain"] = value_codec::json!([-2., 10., 11.]);
    assert!(Sequence::from_json(&malformed.to_string(), b).is_err());
    assert_eq!(h.to_json(b).unwrap(), packet);
}
