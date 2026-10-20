use brep_core::profile_certificate as p;
use nurbs_core::curve::Curve;
use std::sync::Arc;
fn chart(sources: &[Curve], wrapped: [bool; 2]) -> Arc<p::OwnedOriginalChartVertices> {
    let a =
        Arc::new(p::assemble_original_contact_atlas(sources, 1e-12, Default::default()).unwrap());
    let graph = Arc::new(
        p::own_original_carrier_graph(Arc::new(p::own_original_carriers(a).unwrap())).unwrap(),
    );
    Arc::new(
        p::own_original_chart_vertices(graph, [[0., 1.], [0., 1.]], wrapped, Default::default())
            .unwrap(),
    )
}
fn line() -> Curve {
    Curve::from_polyline(vec![vec![0., 0.], vec![0., 1.]]).unwrap()
}
fn quadratic(reverse: bool) -> Curve {
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        weights: vec![1.; 3],
        control_points: if reverse {
            vec![vec![1., 1.], vec![1., 1.], vec![1., 0.]]
        } else {
            vec![vec![1., 0.], vec![1., 0.], vec![1., 1.]]
        },
        periodic: false,
    }
}
#[test]
fn nonlinear_clock_and_endpoint_zero_jet_preserve_complete_shore_image() {
    let c = chart(&[line(), quadratic(false)], [true, false]);
    let a = p::own_original_shore_carrier(c.clone(), 0, 0, 0, Default::default()).unwrap();
    let b = p::own_original_shore_carrier(c, 1, 0, 1, Default::default()).unwrap();
    a.reverify_source().unwrap();
    b.reverify_source().unwrap();
    assert_eq!(a.direction(), 1);
    assert_eq!(b.direction(), 1);
    assert_eq!(a.endpoint_keys(), b.endpoint_keys());
    let pair = p::own_original_shore_pair(a, b).unwrap();
    pair.reverify_source().unwrap();
    assert!(!pair.reversed());
}
#[test]
fn doubly_wrapped_corners_keep_transverse_shore_distinctions_and_reversal() {
    let c = chart(&[line(), quadratic(true)], [true, true]);
    let a = p::own_original_shore_carrier(c.clone(), 0, 0, 0, Default::default()).unwrap();
    let b = p::own_original_shore_carrier(c, 1, 0, 1, Default::default()).unwrap();
    b.reverify_source().unwrap();
    assert_eq!(b.direction(), -1);
    let keys = a.endpoint_keys();
    assert_eq!(keys[0].0, keys[1].0);
    assert_ne!(keys[0].1, keys[1].1);
    assert_eq!(keys, [b.endpoint_keys()[1], b.endpoint_keys()[0]]);
    let pair = p::own_original_shore_pair(a, b).unwrap();
    pair.reverify_source().unwrap();
    assert!(pair.reversed());
}
#[test]
fn wrong_shore_axis_and_unwrapped_context_do_not_own_a_seam() {
    let c = chart(&[line()], [true, false]);
    assert!(p::own_original_shore_carrier(c.clone(), 0, 0, 1, Default::default()).is_err());
    assert!(p::own_original_shore_carrier(c, 0, 1, 0, Default::default()).is_err());
    let c = chart(&[line()], [false, false]);
    assert!(p::own_original_shore_carrier(c, 0, 0, 0, Default::default()).is_err());
}

#[test]
fn partial_shore_image_and_foreign_chart_context_are_not_paired() {
    let partial = Curve::from_polyline(vec![vec![1., 0.25], vec![1., 0.75]]).unwrap();
    let c = chart(&[line(), partial], [true, false]);
    let a = p::own_original_shore_carrier(c.clone(), 0, 0, 0, Default::default()).unwrap();
    let b = p::own_original_shore_carrier(c, 1, 0, 1, Default::default()).unwrap();
    assert!(p::own_original_shore_pair(a, b).is_err());
    let sources = [line(), quadratic(false)];
    let first = chart(&sources, [true, false]);
    let second = chart(&sources, [true, false]);
    let a = p::own_original_shore_carrier(first, 0, 0, 0, Default::default()).unwrap();
    let b = p::own_original_shore_carrier(second, 1, 0, 1, Default::default()).unwrap();
    assert!(p::own_original_shore_pair(a, b).is_err());
}
