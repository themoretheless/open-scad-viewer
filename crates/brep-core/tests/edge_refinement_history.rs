#![cfg(feature = "codec")]
use brep_core::{Model, feature_ops::edge_partition::history::{Budget, History}};
use value_codec::Value;
fn source() -> Model {
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
fn history() -> History {
    let mut h = History::new(&source(), Budget::default()).unwrap();
    let e = h
        .split(0, &[0.23, 0.71], "first", 1e-9, Budget::default())
        .unwrap()
        .edges()[1];
    let d = h.model().edges[e].curve.domain();
    h.split(e, &[(d[0] + d[1]) * 0.5], "second", 1e-9, Budget::default())
        .unwrap();
    h
}
fn value(s: &str) -> Value {
    value_codec::from_str_strict(s).unwrap()
}
#[test]
fn roundtrip_keeps_root_partial_relations_every_tip_and_reconstructs_live_evidence() {
    let h = history();
    let wire = h.to_json(Budget::default()).unwrap();
    let restored = History::from_json(&wire, Budget::default()).unwrap();
    assert_eq!(restored.root(), &source());
    assert_eq!(restored.model(), h.model());
    assert_eq!(restored.steps().len(), 2);
    for (a, b) in h.steps().iter().zip(restored.steps()) {
        assert_eq!(a.relations(), b.relations());
        assert_eq!(a.source(), b.source());
        assert_eq!(a.edges(), b.edges());
        assert_eq!(a.vertices(), b.vertices());
        assert!(b.evidence().boundary_complete && b.evidence().trim_regions_valid);
    }
    assert_eq!(restored.to_json(Budget::default()).unwrap(), wire);
    restored.reverify(Budget::default()).unwrap();
}
#[test]
fn saved_model_requests_and_exact_fraction_records_cannot_authorize_tampered_results() {
    let wire = history().to_json(Budget::default()).unwrap();
    let mut bad = value(&wire);
    bad["steps"][0]["model"]["vertices"][0]["point"][0] = Value::from(99.);
    assert!(History::from_json(&bad.to_string(), Budget::default()).is_err());
    let mut bad = value(&wire);
    bad["steps"][0]["cuts"][0] = Value::from(0.24);
    assert!(History::from_json(&bad.to_string(), Budget::default()).is_err());
    let mut bad = value(&wire);
    bad["steps"][0]["maps"]["relations"][0]["fragments"][0]["range"][0] = Value::from("0/0");
    assert!(History::from_json(&bad.to_string(), Budget::default()).is_err());
    let mut bad = value(&wire);
    bad["steps"][1]["parent"] = Value::from("wrong-parent");
    assert!(History::from_json(&bad.to_string(), Budget::default()).is_err());
}
#[test]
fn missing_identity_tables_unknown_fields_and_reordered_steps_are_not_silently_repaired() {
    let wire = history().to_json(Budget::default()).unwrap();
    let mut bad = value(&wire);
    bad["root"].as_object_mut().unwrap().remove("topologyIds");
    assert!(History::from_json(&bad.to_string(), Budget::default()).is_err());
    let mut bad = value(&wire);
    bad["steps"].as_array_mut().unwrap().swap(0, 1);
    assert!(History::from_json(&bad.to_string(), Budget::default()).is_err());
    let mut bad = value(&wire);
    bad.as_object_mut()
        .unwrap()
        .insert("verified".into(), Value::from(true));
    assert!(History::from_json(&bad.to_string(), Budget::default()).is_err());
    let bad = wire.replacen("\"schema\":", "\"schema\":\"fake\",\"schema\":", 1);
    assert!(History::from_json(&bad, Budget::default()).is_err());
}
#[test]
fn lower_replay_allowances_cannot_be_raised_by_the_packet_and_failed_appends_are_atomic() {
    let mut h = history();
    let wire = h.to_json(Budget::default()).unwrap();
    for b in [
        Budget {
            max_steps: 1,
            ..Budget::default()
        },
        Budget {
            max_bytes: wire.len() - 1,
            ..Budget::default()
        },
        Budget {
            proof: brep_core::feature_ops::edge_partition::Limits {
                max_cells: 1,
                ..Default::default()
            },
            ..Budget::default()
        },
    ] {
        assert!(History::from_json(&wire, b).is_err());
        assert!(h.to_json(b).is_err());
    }
    let e = h.steps().last().unwrap().edges()[0];
    let d = h.model().edges[e].curve.domain();
    assert!(
        h.split(
            e,
            &[(d[0] + d[1]) * 0.5],
            "no-third",
            1e-9,
            Budget {
                max_steps: 2,
                ..Budget::default()
            }
        )
        .is_err()
    );
    assert_eq!(h.to_json(Budget::default()).unwrap(), wire);
    assert!(
        h.split(
            e,
            &[(d[0] + d[1]) * 0.5],
            "oversized",
            1e-9,
            Budget {
                max_bytes: wire.len(),
                ..Budget::default()
            }
        )
        .is_err()
    );
    assert_eq!(h.to_json(Budget::default()).unwrap(), wire);
}

#[test]
fn rational_partial_history_survives_reload_and_accepts_an_additional_native_edit() {
    let mut s = brep_core::cylinder(2., 3.).unwrap();
    let e = s
        .edges
        .iter()
        .position(|e| {
            e.curve.degree == 2 && e.curve.weights.iter().any(|w| *w != e.curve.weights[0])
        })
        .unwrap();
    for w in &mut s.loops {
        for c in &mut w.coedges {
            if c.edge == e {
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
    let mut h = History::new(&s, Budget::default()).unwrap();
    h.split(e, &[0.2, 0.6], "дуга\\\n", 1e-8, Budget::default())
        .unwrap();
    let text = h.to_json(Budget::default()).unwrap();
    let exact = Budget {
        max_bytes: text.len(),
        ..Budget::default()
    };
    assert_eq!(h.to_json(exact).unwrap(), text);
    let mut loaded = History::from_json(&text, exact).unwrap();
    assert_eq!(loaded.root(), &s);
    assert_eq!(loaded.steps()[0].relations(), h.steps()[0].relations());
    let e = loaded.steps()[0].edges()[1];
    let d = loaded.model().edges[e].curve.domain();
    loaded
        .split(
            e,
            &[(d[0] + d[1]) * 0.5],
            "after-load",
            1e-8,
            Budget::default(),
        )
        .unwrap();
    let second = History::from_json(
        &loaded.to_json(Budget::default()).unwrap(),
        Budget::default(),
    )
    .unwrap();
    assert_eq!(second.root(), &s);
    assert_eq!(second.model(), loaded.model());
    assert_eq!(second.steps().len(), 2);
}
#[test]
fn stored_caps_are_checked_even_when_actual_work_is_smaller_than_a_new_quota() {
    let h = history();
    let text = h.to_json(Budget::default()).unwrap();
    let used = h.steps().iter().map(|s| s.evidence().cells).sum::<usize>();
    assert!(used < 50000);
    let lower = Budget {
        proof: brep_core::feature_ops::edge_partition::Limits {
            max_cells: 50000,
            ..Default::default()
        },
        ..Budget::default()
    };
    assert!(h.to_json(lower).is_err());
    assert!(History::from_json(&text, lower).is_err());
    let lower_mass = Budget {
        proof: brep_core::feature_ops::edge_partition::Limits {
            mass_evaluations: 100,
            ..Default::default()
        },
        ..Budget::default()
    };
    assert!(h.to_json(lower_mass).is_err());
    let mut bad = value(&text);
    bad["steps"][0]["limits"]["cells"] = Value::from(200000usize);
    assert!(History::from_json(&bad.to_string(), Budget::default()).is_err());
}
#[test]
fn empty_history_and_literal_negative_zero_survive_the_versioned_packet() {
    let s = brep_core::cuboid([-0.0, 0., 0.], [1.; 3]).unwrap();
    assert!(s.vertices[0].point[0].is_sign_negative());
    let h = History::new(&s, Budget::default()).unwrap();
    let text = h.to_json(Budget::default()).unwrap();
    let loaded = History::from_json(&text, Budget::default()).unwrap();
    assert_eq!(loaded.steps().len(), 0);
    assert!(loaded.root().vertices[0].point[0].is_sign_negative());
    assert_eq!(loaded.to_json(Budget::default()).unwrap(), text);
    let h = History::new(&Model::empty(1e-7).unwrap(), Budget::default()).unwrap();
    assert!(
        History::from_json(&h.to_json(Budget::default()).unwrap(), Budget::default())
            .unwrap()
            .model()
            .is_empty()
    );
}

#[path = "common/closed_periodic_source.rs"]
mod closed_fixture;
#[test]
fn a_single_curve_periodic_root_reloads_with_its_original_seam_and_complete_saved_tip() {
    let s = closed_fixture::source();
    let mut h = History::new(&s, Budget::default()).unwrap();
    h.split(0, &[0.37], "closed-history", 1e-8, Budget::default())
        .unwrap();
    let restored =
        History::from_json(&h.to_json(Budget::default()).unwrap(), Budget::default()).unwrap();
    assert_eq!(restored.root(), &s);
    assert!(restored.root().edges[0].curve.periodic);
    assert_eq!(restored.root().loops[0].coedges.len(), 1);
    assert_eq!(restored.model().loops[0].coedges.len(), 2);
    assert_eq!(restored.model().vertices[0], s.vertices[0]);
    assert!(restored.steps()[0].evidence().audit.ok);
}
#[test]
fn a_root_requiring_legacy_identity_repair_is_refused_before_export() {
    let mut s = source();
    s.1.change_set = Default::default();
    assert!(History::new(&s, Budget::default()).is_err());
}

#[test]
fn existing_native_results_attach_in_order_under_remaining_history_allowances() {
    use brep_core::feature_ops::edge_partition::{self, Limits};
    let root = source();
    let a =
        edge_partition::split(&root, 0, &[0.23, 0.71], "existing-a", Limits::default()).unwrap();
    let e = a.edges()[1];
    let d = a.model().edges[e].curve.domain();
    let b = edge_partition::split(
        a.model(),
        e,
        &[(d[0] + d[1]) * 0.5],
        "existing-b",
        Limits::default(),
    )
    .unwrap();
    let mut h = History::new(&root, Budget::default()).unwrap();
    h.append(&a, Budget::default()).unwrap();
    let before = h.to_json(Budget::default()).unwrap();
    assert!(h.append(&a, Budget::default()).is_err());
    assert_eq!(h.to_json(Budget::default()).unwrap(), before);
    h.append(&b, Budget::default()).unwrap();
    let loaded =
        History::from_json(&h.to_json(Budget::default()).unwrap(), Budget::default()).unwrap();
    assert_eq!(loaded.root(), &root);
    assert_eq!(loaded.model(), b.model());
    assert!(a.reverify().unwrap().boundary_complete);
    assert!(b.reverify().unwrap().trim_regions_valid);
}
