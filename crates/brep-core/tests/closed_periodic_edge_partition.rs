use brep_core::feature_ops::edge_partition::{self, Limits};
#[path = "common/closed_periodic_source.rs"]
mod fixture;
use fixture::source;
#[test]
fn a_closed_periodic_edge_shared_by_two_single_curve_disk_faces_is_refined() {
    let s = source();
    let before = s.clone();
    let p =
        edge_partition::split(&s, 0, &[0.3, 1.4, 2.6], "closed-period", Limits::default()).unwrap();
    assert_eq!(p.edges().len(), 4);
    assert_eq!(p.vertices().len(), 3);
    assert_eq!(p.model().edges[p.edges()[0]].vertices[0], 0);
    assert_eq!(p.model().edges[*p.edges().last().unwrap()].vertices[1], 0);
    assert_eq!(p.model().faces, s.faces);
    assert_eq!(p.model().1.faces, s.1.faces);
    assert!(p.model().loops.iter().all(|w| w.coedges.len() == 4));
    assert!(p.reverify().unwrap().trim_regions_valid);
    let volume = brep_core::analysis::mass_properties(p.model(), 1e-6, 2000000)
        .unwrap()
        .signed_volume_mm3;
    assert!((volume - std::f64::consts::PI).abs() < 1e-6, "{volume}");
    assert_eq!(s, before);
}
#[test]
fn exhausted_closed_source_proof_preserves_original_periodic_storage() {
    let s = source();
    let before = s.clone();
    assert!(
        edge_partition::split(
            &s,
            0,
            &[0.5],
            "scarce",
            Limits {
                max_domain_cells: 1,
                ..Limits::default()
            }
        )
        .is_err()
    );
    assert_eq!(s, before);
}

#[test]
fn reversed_periodic_original_and_independent_partial_clocks_keep_the_closed_vertex() {
    let mut s = source();
    for k in &mut s.edges[0].curve.knots {
        *k = -3. + 2.5 * *k;
    }
    s.edges[0].curve = s.edges[0].curve.reverse().unwrap();
    for (i, w) in s.loops.iter_mut().enumerate() {
        for c in &mut w.coedges {
            c.reversed = true;
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
    s.validate().unwrap();
    let before = s.clone();
    let p = edge_partition::split(
        &s,
        0,
        &[-1., 1., 4.],
        "closed-reversed-clock",
        Limits::default(),
    )
    .unwrap();
    assert_eq!(p.relations().len(), 2);
    assert_eq!(p.model().vertices[0], s.vertices[0]);
    assert!(p.reverify().unwrap().boundary_complete);
    assert_eq!(s, before);
    let e = p.edges()[1];
    let d = p.model().edges[e].curve.domain();
    let repeat = edge_partition::split(
        p.model(),
        e,
        &[(d[0] + d[1]) * 0.5],
        "closed-child",
        Limits::default(),
    )
    .unwrap();
    assert!(repeat.reverify().unwrap().trim_regions_valid);
}

#[test]
fn one_cut_keeps_two_actual_long_curves_with_exact_original_uv_span_qualification() {
    let s = source();
    let p = edge_partition::split(&s, 0, &[0.37], "one-cut", Limits::default()).unwrap();
    assert_eq!(p.edges().len(), 2);
    assert_eq!(p.vertices().len(), 1);
    assert!(p.model().loops.iter().all(|w| w.coedges.len() == 2));
    assert!(p.reverify().unwrap().trim_regions_valid);
}
