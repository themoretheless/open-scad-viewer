use brep_core::{Model, feature_ops::edge_partition::{self, Limits, reclock}};
use brep_topology::CoedgeTrim;
#[cfg(feature = "codec")]
use value_codec::{Deserialize, Serialize};
#[path = "common/closed_periodic_source.rs"]
mod fixture;
fn cube() -> Model {
    let mut s = brep_core::cuboid([0.; 3], [2., 3., 4.]).unwrap();
    for l in &mut s.loops {
        for c in &mut l.coedges {
            if c.edge == 0 {
                for k in &mut c.pcurve.knots {
                    *k = 2. * *k + 3.;
                }
                *c = c
                    .clone()
                    .with_trim(CoedgeTrim {
                        curve_parameter: [0.13, 0.87],
                        pcurve_parameter: [3.26, 4.74],
                        periodic_lift: [[0; 2]; 2],
                    })
                    .unwrap();
            }
        }
    }
    s
}
#[test]
fn a_shared_world_clock_changes_without_replacing_independent_uv_clocks_or_vertex_geometry() {
    let mut s = cube();
    let parent = s.1.edges[0];
    s.1.local_tolerances_mm.insert(parent, 1e-9);
    let saved = s.clone();
    let r = reclock::reparameterize(&s, 0, [-3., 9.], "new-clock", Limits::default()).unwrap();
    assert_eq!(r.model().edges[0].curve.domain(), [-3., 9.]);
    assert_eq!(r.relations().len(), 2);
    assert_eq!(r.model().vertices, s.vertices);
    assert_eq!(r.model().1.vertices, s.1.vertices);
    assert_eq!(r.model().faces, s.faces);
    assert_eq!(r.model().1.faces, s.1.faces);
    assert_eq!(r.model().1.bodies, s.1.bodies);
    assert_eq!(
        r.model().1.local_tolerances_mm.get(&r.model().1.edges[0]),
        Some(&1e-9)
    );
    assert!(!r.model().1.local_tolerances_mm.contains_key(&parent));
    for relation in r.relations() {
        let old = &s.loops[relation.wire].coedges[relation.coedge];
        let new = &r.model().loops[relation.wire].coedges[relation.coedge];
        assert_eq!(old.pcurve, new.pcurve);
        assert_eq!(old.reversed, new.reversed);
        assert_eq!(relation.original, old.trim().unwrap());
        assert_eq!(relation.transferred, new.trim().unwrap());
        assert_eq!(
            relation.transferred.pcurve_parameter,
            relation.original.pcurve_parameter
        );
        assert!(relation.original_curve_distance_upper <= 1e-9);
    }
    assert!(r.reverify().unwrap().trim_regions_valid);
    assert!((r.evidence().numerical_volume_mm3 - 24.).abs() < 1e-7);
    let next =
        edge_partition::split(r.model(), 0, &[0., 6.], "split-clock", Limits::default()).unwrap();
    assert_eq!(next.relations().len(), 2);
    assert!(next.reverify().unwrap().boundary_complete);
    #[cfg(feature = "codec")]
    {
        assert_eq!(Model::from_value(r.model().to_value()).unwrap(), *r.model());
    }
    assert_eq!(s, saved);
}
#[test]
fn a_periodic_closed_world_edge_retains_its_seam_and_authored_partial_ranges() {
    let mut s = fixture::source();
    for l in &mut s.loops {
        let c = l.coedges[0].clone();
        l.coedges[0] = c
            .with_trim(CoedgeTrim {
                curve_parameter: [3.5, 0.5],
                pcurve_parameter: [3.5, 0.5],
                periodic_lift: [[0; 2]; 2],
            })
            .unwrap();
    }
    let saved = s.clone();
    let r =
        reclock::reparameterize(&s, 0, [-2., 10.], "periodic-clock", Limits::default()).unwrap();
    assert!(r.model().edges[0].curve.periodic);
    assert_eq!(r.model().edges[0].vertices, s.edges[0].vertices);
    assert_eq!(r.model().vertices, s.vertices);
    assert_eq!(r.relations()[0].transferred.curve_parameter, [8.5, -0.5]);
    assert!(r.reverify().unwrap().trim_regions_valid);
    let next = edge_partition::split(
        r.model(),
        0,
        &[1., 7.],
        "split-cycle-clock",
        Limits::default(),
    )
    .unwrap();
    assert!(next.reverify().unwrap().boundary_complete);
    assert_eq!(s, saved);
}
#[test]
fn ordinary_closed_edges_and_reversed_world_uses_preserve_actual_closed_boundaries() {
    let mut s = fixture::source();
    s.edges[0].curve.periodic = false;
    s.edges[0].curve = s.edges[0].curve.reverse().unwrap();
    for l in &mut s.loops {
        l.coedges[0].pcurve.periodic = false;
        l.coedges[0].reversed = true;
        l.coedges[0] = l.coedges[0]
            .clone()
            .with_trim(CoedgeTrim {
                curve_parameter: [0.5, 3.5],
                pcurve_parameter: [0.5, 3.5],
                periodic_lift: [[0; 2]; 2],
            })
            .unwrap();
    }
    let saved = s.clone();
    let r = reclock::reparameterize(&s, 0, [3., 11.], "ordinary-clock", Limits::default()).unwrap();
    assert!(!r.model().edges[0].curve.periodic);
    assert!(r.model().loops.iter().all(|l| l.coedges[0].reversed));
    assert_eq!(r.relations()[0].transferred.curve_parameter, [4., 10.]);
    assert!(r.reverify().unwrap().boundary_complete);
    assert_eq!(s, saved);
}
#[test]
fn knot_partition_collapse_and_rounded_authored_clock_drift_refuse_without_mutation() {
    let s = fixture::source();
    let saved = s.clone();
    let err = reclock::reparameterize(
        &s,
        0,
        [1., 1f64.next_up()],
        "collapsed-knots",
        Limits::default(),
    )
    .err()
    .unwrap();
    assert!(err.to_string().contains("knot partition"), "{err}");
    assert_eq!(s, saved);
    let mut s = cube();
    s.1.local_tolerances_mm.insert(s.1.edges[0], 1e-10);
    let saved = s.clone();
    let err = reclock::reparameterize(
        &s,
        0,
        [5e8, 5e8 + 16.],
        "lost-authored-clock",
        Limits::default(),
    )
    .err()
    .unwrap();
    assert!(
        err.to_string().contains("authored interval exceeds"),
        "{err}"
    );
    assert_eq!(s, saved);
}
#[test]
fn invalid_targets_and_exhausted_work_never_publish_an_unqualified_clock_change() {
    let s = cube();
    let saved = s.clone();
    for domain in [
        [0., 0.],
        [2., -1.],
        [f64::NAN, 1.],
        [0., f64::INFINITY],
        [-f64::MAX, f64::MAX],
    ] {
        assert!(reclock::reparameterize(&s, 0, domain, "invalid", Limits::default()).is_err());
    }
    assert!(reclock::reparameterize(&s, 999, [0., 1.], "missing", Limits::default()).is_err());
    assert!(reclock::reparameterize(&s, 0, [0., 1.], "", Limits::default()).is_err());
    assert!(
        reclock::reparameterize(
            &s,
            0,
            [-3., 9.],
            "exhausted",
            Limits {
                max_cells: 1,
                ..Limits::default()
            }
        )
        .is_err()
    );
    assert_eq!(s, saved);
}
#[test]
fn low_multiplicity_unclamped_endpoint_values_use_actual_native_enclosures() {
    let mut s = cube();
    let original = s.edges[0].curve.clone();
    let a = &original.control_points[0];
    let b = original.control_points.last().unwrap();
    s.edges[0].curve = nurbs_core::curve::Curve {
        degree: 2,
        knots: vec![-2., -1., 0., 1., 2., 3.],
        control_points: [-0.5, 0.5, 1.5]
            .iter()
            .map(|t| (0..3).map(|k| a[k] + t * (b[k] - a[k])).collect())
            .collect(),
        weights: vec![1.; 3],
        periodic: false,
    };
    assert_ne!(
        s.edges[0].curve.control_points[0].as_slice(),
        s.vertices[s.edges[0].vertices[0]].point.as_slice()
    );
    assert_eq!(
        s.edges[0].curve.knots.iter().filter(|k| **k == 0.).count(),
        1
    );
    let saved = s.clone();
    let r =
        reclock::reparameterize(&s, 0, [-3., 9.], "unclamped-clock", Limits::default()).unwrap();
    assert_eq!(
        r.model().edges[0].curve.control_points,
        s.edges[0].curve.control_points
    );
    assert_eq!(r.model().vertices, s.vertices);
    assert!(r.reverify().unwrap().boundary_complete);
    assert_eq!(s, saved);
}
