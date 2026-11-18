use brep_core::{Model, feature_ops::edge_partition::{self, Limits, seam::compact}};
use brep_topology::CoedgeTrim;
#[cfg(feature = "codec")]
use value_codec::{Deserialize, Serialize};
#[path = "common/closed_periodic_source.rs"]
mod fixture;
fn source() -> Model {
    edge_partition::split(
        &fixture::source(),
        0,
        &[0.5, 1.5, 2.5],
        "parts",
        Limits::default(),
    )
    .unwrap()
    .model()
    .clone()
}
fn bind(m: &mut Model, w: usize, slot: usize, c: [f64; 2], p: [f64; 2]) {
    let old = m.loops[w].coedges[slot].clone();
    m.loops[w].coedges[slot] = old
        .with_trim(CoedgeTrim {
            curve_parameter: c,
            pcurve_parameter: p,
            periodic_lift: [[0; 2]; 2],
        })
        .unwrap();
}
#[test]
fn full_authored_cycles_with_independent_uv_clocks_and_directed_requests_survive_merge_and_split() {
    let mut s = source();
    for w in 0..s.loops.len() {
        for slot in 0..s.loops[w].coedges.len() {
            let e = s.loops[w].coedges[slot].edge;
            let mut c = s.edges[e].curve.domain();
            for k in &mut s.loops[w].coedges[slot].pcurve.knots {
                *k = 2. * *k + 7.;
            }
            let mut p = s.loops[w].coedges[slot].pcurve.domain();
            if (slot + w) % 2 == 1 {
                c.reverse();
                p.reverse();
            }
            bind(&mut s, w, slot, c, p);
        }
    }
    let saved = s.clone();
    let r = compact::compact(&s, 0, "authored-full", Limits::default()).unwrap();
    assert_eq!(r.relations().len(), 8);
    for record in r.relations() {
        assert_eq!(
            record.original,
            s.loops[record.wire].coedges[record.coedge].trim().unwrap()
        );
        assert!(record.original_curve_distance_upper <= s.tolerance_mm);
        assert!(record.original_lift_distance_upper <= s.tolerance_mm);
    }
    for l in &r.model().loops {
        let t = l.coedges[0].trim().unwrap();
        assert_eq!(t.curve_parameter, [0., 4.]);
        assert_eq!(t.pcurve_parameter, [0., 4.]);
    }
    assert!(r.reverify().unwrap().trim_regions_valid);
    #[cfg(feature = "codec")]
    {
        assert_eq!(Model::from_value(r.model().to_value()).unwrap(), *r.model());
    }
    let next = edge_partition::split(
        r.model(),
        r.edge(),
        &[0.375, 2.25],
        "later",
        Limits::default(),
    )
    .unwrap();
    assert!(next.reverify().unwrap().boundary_complete);
    assert_eq!(next.relations().len(), 2);
    assert_eq!(s, saved);
}
#[test]
fn connected_partial_requests_remain_partial_in_the_saved_coedge() {
    let mut s = source();
    bind(&mut s, 0, 0, [0.25, 0.5], [0.25, 0.5]);
    bind(&mut s, 0, 1, [0.5, 1.], [0.5, 1.]);
    let saved = s.clone();
    let r = compact::compact(&s, 0, "authored-partial", Limits::default()).unwrap();
    assert_eq!(r.relations().len(), 2);
    let t = r.model().loops[0].coedges[0].trim().unwrap();
    assert_eq!(t.curve_parameter, [0.5, 1.5]);
    assert_eq!(t.pcurve_parameter, [0.5, 1.5]);
    assert!(r.model().loops[1].coedges[0].trim().is_none());
    assert!(r.reverify().unwrap().trim_regions_valid);
    let next = edge_partition::split(
        r.model(),
        r.edge(),
        &[0.75, 1.25],
        "partial-again",
        Limits::default(),
    )
    .unwrap();
    assert_eq!(next.relations().len(), 1);
    assert_eq!(next.relations()[0].fragments.len(), 3);
    assert_eq!(s, saved);
}
#[test]
fn authored_clocks_follow_mixed_world_directions_without_losing_original_request_orientation() {
    let mut s = source();
    for w in 0..s.loops.len() {
        for slot in 0..s.loops[w].coedges.len() {
            let e = s.loops[w].coedges[slot].edge;
            let c = s.edges[e].curve.domain();
            let p = s.loops[w].coedges[slot].pcurve.domain();
            bind(&mut s, w, slot, [c[1], c[0]], [p[1], p[0]]);
        }
    }
    s.edges[1].curve = s.edges[1].curve.reverse().unwrap();
    s.edges[1].vertices.swap(0, 1);
    for l in &mut s.loops {
        for c in &mut l.coedges {
            if c.edge == 1 {
                c.reversed = !c.reversed;
            }
        }
    }
    let saved = s.clone();
    let r = compact::compact(&s, 1, "authored-mixed", Limits::default()).unwrap();
    assert_eq!(r.relations().len(), 8);
    assert!(r.model().loops.iter().all(|l| l.coedges[0].reversed));
    assert!(
        r.relations()
            .iter()
            .all(|r| r.original.curve_parameter[0] > r.original.curve_parameter[1])
    );
    assert!(r.reverify().unwrap().boundary_complete);
    assert_eq!(s, saved);
}
#[test]
fn disconnected_authored_ranges_cannot_be_filled_by_a_single_aggregate_relation() {
    let mut s = source();
    bind(&mut s, 0, 0, [0.125, 0.25], [0.125, 0.25]);
    bind(&mut s, 0, 2, [1.75, 2.], [1.75, 2.]);
    let saved = s.clone();
    let err = compact::compact(&s, 0, "gap", Limits::default())
        .err()
        .unwrap();
    assert!(
        err.to_string().contains("Disconnected authored clock"),
        "{err}"
    );
    assert_eq!(s, saved);
}
#[test]
fn geometrically_close_but_nonaffine_requested_clocks_cannot_be_flattened() {
    let mut s = source();
    bind(&mut s, 0, 0, [0., 0.5], [1e-10, 0.5]);
    bind(&mut s, 0, 1, [0.5, 1.5], [0.5, 1.5]);
    let saved = s.clone();
    let err = compact::compact(&s, 0, "nonaffine", Limits::default())
        .err()
        .unwrap();
    assert!(
        err.to_string().contains("Nonaffine authored clocks"),
        "{err}"
    );
    assert_eq!(s, saved);
}
#[test]
fn nonrepresentable_partial_output_clocks_keep_exact_images_and_fresh_drift_bounds() {
    use num_rational::BigRational as Q;
    let mut s = source();
    bind(&mut s, 0, 3, [2.75, 3.75], [2.75, 3.75]);
    let saved = s.clone();
    let r = compact::compact(&s, 0, "rounded-partial", Limits::default()).unwrap();
    let record = &r.relations()[0];
    assert!(
        record
            .curve_parameters
            .iter()
            .zip(record.transferred.curve_parameter)
            .any(|(exact, saved)| *exact != Q::from_float(saved).unwrap())
    );
    assert!(record.original_curve_distance_upper > 0.);
    assert!(record.original_curve_distance_upper <= s.tolerance_mm);
    assert!(r.reverify().unwrap().boundary_complete);
    let next = edge_partition::split(
        r.model(),
        r.edge(),
        &[3.5],
        "rounded-again",
        Limits::default(),
    )
    .unwrap();
    assert_eq!(next.relations().len(), 1);
    assert_eq!(next.relations()[0].fragments.len(), 2);
    assert_eq!(s, saved);
}
#[test]
fn output_rounding_cannot_close_an_exact_gap_between_authored_requests() {
    let mut s = edge_partition::split(
        &fixture::source(),
        0,
        &[0.125, 0.25, 3.75],
        "uneven",
        Limits::default(),
    )
    .unwrap()
    .model()
    .clone();
    bind(
        &mut s,
        0,
        2,
        [0.25, 3.75f64.next_down()],
        [0.25, 3.75f64.next_down()],
    );
    bind(&mut s, 0, 3, [3.75, 4.], [3.75, 4.]);
    // Both proposed stored ranges meet at output clock 3, but their exact
    // original-clock images do not. The source request's gap must survive.
    assert_eq!(2. + (3.75f64.next_down() - 0.25) / 3.5, 3.);
    let saved = s.clone();
    let err = compact::compact(&s, 0, "rounded-gap", Limits::default())
        .err()
        .unwrap();
    assert!(
        err.to_string().contains("Disconnected authored clock"),
        "{err}"
    );
    assert_eq!(s, saved);
}
