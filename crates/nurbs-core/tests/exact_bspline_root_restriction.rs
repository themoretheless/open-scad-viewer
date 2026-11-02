use cad_predicates::{BezierIdentity as B, Reason};
use nurbs_core::{
    curve::Curve, curve_partition_agreement::verify_exact_bspline_restriction as verify,
};
fn source() -> Curve {
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 0.5, 1., 1., 1.],
        control_points: vec![
            vec![0., 0., 0.],
            vec![3., 6., 3.],
            vec![9., 6., 9.],
            vec![12., 0., 12.],
        ],
        weights: vec![1., 2., 2., 1.],
        periodic: false,
    }
}
fn child(points: Vec<Vec<f64>>, weights: Vec<f64>, d: [f64; 2]) -> Curve {
    let degree = points.len() - 1;
    Curve {
        degree,
        knots: [vec![d[0]; degree + 1], vec![d[1]; degree + 1]].concat(),
        control_points: points,
        weights,
        periodic: false,
    }
}
#[test]
fn rational_c1_world_source_reproves_both_original_spans_and_global_weight_gauge() {
    let a = source();
    let before = a.clone();
    let left = child(
        vec![vec![0., 0., 0.], vec![3., 6., 3.], vec![6., 6., 6.]],
        vec![1., 2., 2.],
        [0., 0.5],
    );
    let right = child(
        vec![vec![6., 6., 6.], vec![9., 6., 9.], vec![12., 0., 12.]],
        vec![2., 2., 1.],
        [0.5, 1.],
    );
    for c in [&left, &right] {
        assert_eq!(verify(&a, c, 1000000).unwrap().unwrap().outcome, B::Equal);
    }
    let mut scaled = a.clone();
    scaled.weights.iter_mut().for_each(|w| *w *= 4.);
    assert_eq!(
        verify(&a, &scaled, 1000000).unwrap().unwrap().outcome,
        B::Equal
    );
    let mut bad = left.clone();
    bad.control_points[1][2] += 2f64.powi(-40);
    assert_eq!(
        verify(&a, &bad, 1000000).unwrap().unwrap().outcome,
        B::Different
    );
    assert_eq!(a, before);
}
#[test]
fn differently_graded_child_keeps_the_original_world_clock() {
    let mut a = source();
    a.weights = vec![1.; 4];
    let c = child(
        vec![
            vec![0., 0., 0.],
            vec![2., 4., 2.],
            vec![4., 6., 4.],
            vec![6., 6., 6.],
        ],
        vec![1.; 4],
        [0., 0.5],
    );
    assert_eq!(verify(&a, &c, 1000000).unwrap().unwrap().outcome, B::Equal);
    let mut moved = c.clone();
    moved.knots.iter_mut().for_each(|k| *k += 0.125);
    assert_ne!(
        verify(&a, &moved, 1000000).unwrap().unwrap().outcome,
        B::Equal
    );
}
#[test]
fn incomplete_work_and_outside_domain_do_not_qualify_a_child() {
    let a = source();
    assert!(matches!(
        verify(&a, &a, 1).unwrap().unwrap().outcome,
        B::Indeterminate(Reason::ResourceLimit)
    ));
    assert!(verify(&a, &a, 0).is_err());
    let mut outside = a.clone();
    outside.knots = vec![-1., -1., -1., 0.5, 1., 1., 1.];
    assert!(matches!(
        verify(&a, &outside, 1000000).unwrap().unwrap().outcome,
        B::Indeterminate(_)
    ));
}
#[test]
fn rational_uv_source_uses_the_same_original_span_proof() {
    let mut a = source();
    for p in &mut a.control_points {
        p.pop();
    }
    let c = child(
        vec![vec![0., 0.], vec![3., 6.], vec![6., 6.]],
        vec![1., 2., 2.],
        [0., 0.5],
    );
    assert_eq!(verify(&a, &c, 1000000).unwrap().unwrap().outcome, B::Equal);
}

#[test]
fn partial_root_interval_crosses_an_original_c1_knot_without_rounded_extraction() {
    let mut a = source();
    for p in &mut a.control_points {
        for x in p {
            *x *= 7.;
        }
    }
    let c = Curve {
        degree: 2,
        knots: vec![0.25, 0.25, 0.25, 0.5, 0.75, 0.75, 0.75],
        control_points: vec![
            vec![24., 36., 24.],
            vec![31.5, 42., 31.5],
            vec![52.5, 42., 52.5],
            vec![60., 36., 60.],
        ],
        weights: vec![1.75, 2., 2., 1.75],
        periodic: false,
    };
    assert_eq!(verify(&a, &c, 1000000).unwrap().unwrap().outcome, B::Equal);
    let mut bad = c;
    bad.control_points[1][1] += 2f64.powi(-35);
    assert_eq!(
        verify(&a, &bad, 1000000).unwrap().unwrap().outcome,
        B::Different
    );
}

#[test]
fn owned_original_root_partition_requires_complete_adjacent_children_and_fresh_work() {
    use nurbs_core::curve_partition_agreement::verify_exact_bspline_root_partition as partition;
    let a = source();
    let before = a.clone();
    let left = child(
        vec![vec![0., 0., 0.], vec![3., 6., 3.], vec![6., 6., 6.]],
        vec![1., 2., 2.],
        [0., 0.5],
    );
    let right = child(
        vec![vec![6., 6., 6.], vec![9., 6., 9.], vec![12., 0., 12.]],
        vec![2., 2., 1.],
        [0.5, 1.],
    );
    let pieces = vec![left, right];
    let owned = partition(&a, &pieces, [0., 1.], 1000000).unwrap().unwrap();
    assert_eq!(owned.source(), &a);
    assert_eq!(owned.pieces(), &pieces);
    assert_eq!(owned.range(), [0., 1.]);
    owned.reverify(1000000).unwrap();
    assert!(owned.reverify(owned.work_used() - 1).is_err());
    assert!(partition(&a, &pieces[..1], [0., 1.], 1000000).is_err());
    let mut broken = pieces.clone();
    broken.reverse();
    assert!(partition(&a, &broken, [0., 1.], 1000000).is_err());
    let mut broken = pieces.clone();
    broken[1].knots = vec![0.25, 0.25, 0.25, 1., 1., 1.];
    assert!(partition(&a, &broken, [0., 1.], 1000000).is_err());
    let mut broken = pieces.clone();
    broken[1].control_points[1][2] += 2f64.powi(-40);
    assert!(partition(&a, &broken, [0., 1.], 1000000).unwrap().is_none());
    assert!(partition(&a, &pieces, [0., 1.], 1).unwrap().is_none());
    assert_eq!(a, before);
}
