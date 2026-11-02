use brep_core::{
    Model,
    feature_ops::edge_partition::{self, Limits},
};
#[path = "common/closed_periodic_source.rs"]
mod fixture;
fn source() -> Model {
    let mut s = fixture::source();
    let d = s.edges[0].curve.domain();
    s.edges[0].curve = s.edges[0].curve.trim(d[0], d[1]).unwrap();
    for w in &mut s.loops {
        for c in &mut w.coedges {
            let d = c.pcurve.domain();
            c.pcurve = c.pcurve.trim(d[0], d[1]).unwrap();
        }
    }
    assert!(!s.edges[0].curve.periodic);
    s.validate().unwrap();
    s
}
#[test]
fn clamped_closed_storage_admits_real_two_face_refinement_without_a_periodic_flag() {
    let s = source();
    let before = s.clone();
    let audit = brep_core::solid_audit::audit_solid(&s).unwrap();
    assert!(audit.sew.complete);
    let p = edge_partition::split(&s, 0, &[0.37], "clamped-cycle", Limits::default()).unwrap();
    assert_eq!(p.edges().len(), 2);
    assert_eq!(p.vertices().len(), 1);
    assert_eq!(p.model().edges[p.edges()[0]].vertices[0], 0);
    assert_eq!(p.model().edges[p.edges()[1]].vertices[1], 0);
    assert_eq!(p.model().faces, s.faces);
    assert!(p.reverify().unwrap().trim_regions_valid);
    let v = brep_core::analysis::mass_properties(p.model(), 1e-6, 2000000)
        .unwrap()
        .signed_volume_mm3;
    assert!((v - std::f64::consts::PI).abs() < 1e-6);
    assert_eq!(s, before);
}
#[test]
fn independent_partial_clocks_and_repeated_clamped_child_refinement_keep_source_ownership() {
    let mut s = source();
    for k in &mut s.edges[0].curve.knots {
        *k = -3. + 2.5 * *k;
    }
    for (i, w) in s.loops.iter_mut().enumerate() {
        for c in &mut w.coedges {
            for k in &mut c.pcurve.knots {
                *k = -11. + 10. * *k;
            }
            let mut curve = [-1., 5.];
            let mut pcurve = [-3., 21.];
            if i == 1 {
                curve.reverse();
                pcurve.reverse();
            }
            *c = c
                .clone()
                .with_trim(brep_topology::CoedgeTrim {
                    curve_parameter: curve,
                    pcurve_parameter: pcurve,
                    periodic_lift: [[0; 2]; 2],
                })
                .unwrap();
        }
    }
    let before = s.clone();
    let p =
        edge_partition::split(&s, 0, &[-1., 1., 4.], "clamped-clock", Limits::default()).unwrap();
    assert_eq!(p.relations().len(), 2);
    assert!(p.reverify().unwrap().boundary_complete);
    let e = p.edges()[1];
    let d = p.model().edges[e].curve.domain();
    let child = edge_partition::split(
        p.model(),
        e,
        &[(d[0] + d[1]) * 0.5],
        "clamped-child",
        Limits::default(),
    )
    .unwrap();
    assert!(child.reverify().unwrap().trim_regions_valid);
    assert_eq!(s, before);
}
#[cfg(feature = "codec")]
#[test]
fn clamped_closed_roots_survive_history_transport_and_parameter_regeneration() {
    use edge_partition::history::{Budget, History};
    let root = source();
    let mut h = History::new(&root, Budget::default()).unwrap();
    let e = h
        .split(0, &[0.37], "first-clamped", 1e-8, Budget::default())
        .unwrap()
        .edges()[1];
    h.split(e, &[2.25], "later-clamped", 1e-8, Budget::default())
        .unwrap();
    let edited = h.with_cuts(0, &[0.6], Budget::default()).unwrap();
    let loaded = History::from_json(
        &edited.to_json(Budget::default()).unwrap(),
        Budget::default(),
    )
    .unwrap();
    assert_eq!(loaded.root(), &root);
    assert_eq!(loaded.model(), edited.model());
    assert!(!loaded.root().edges[0].curve.periodic);
}

#[test]
fn an_ordinary_unclamped_original_uses_its_actual_closed_active_interval() {
    let mut s = fixture::source();
    s.edges[0].curve.periodic = false;
    for w in &mut s.loops {
        for c in &mut w.coedges {
            c.pcurve.periodic = false;
        }
    }
    s.validate().unwrap();
    let before = s.clone();
    let p = edge_partition::split(
        &s,
        0,
        &[0.37, 2.25],
        "unclamped-ordinary",
        Limits::default(),
    )
    .unwrap();
    assert_eq!(p.edges().len(), 3);
    assert!(p.reverify().unwrap().trim_regions_valid);
    assert_eq!(s, before);
}
