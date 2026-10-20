use brep_core::profile_certificate as p;
use nurbs_core::curve::Curve;
use std::{cmp::Ordering, sync::Arc};
fn curve(points: Vec<Vec<f64>>) -> Curve {
    let degree = points.len() - 1;
    Curve {
        degree,
        knots: [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
        weights: vec![1.; points.len()],
        control_points: points,
        periodic: false,
    }
}
fn owner(a: &Curve, b: &Curve, width: f64) -> Arc<p::CurveEventNodes> {
    let c = p::certify_span_intersections(a, b, false, width).unwrap();
    Arc::new(p::own_curve_event_nodes(a, b, &c).unwrap())
}
fn compare(
    a: Arc<p::CurveEventNodes>,
    i: usize,
    sa: usize,
    b: Arc<p::CurveEventNodes>,
    j: usize,
    sb: usize,
    limits: p::ParameterOrderLimits,
) -> p::QualifiedParameterOrder {
    let q = p::compare_event_parameters(
        a,
        p::EventParameterRef {
            node: i,
            source: sa,
        },
        b,
        p::EventParameterRef {
            node: j,
            source: sb,
        },
        limits,
    )
    .unwrap();
    q.reverify_source().unwrap();
    q
}
#[test]
fn different_primitive_extensions_identify_the_same_original_irrational_parameter() {
    let source = curve(vec![vec![-3., 0.], vec![0., 6.], vec![3., 0.]]);
    let horizontal = curve(vec![vec![-4., 1.5], vec![4., 1.5]]);
    // x=6v²-3, y=x²/3. Original parameter u=v² at the
    // intersections y=3/2, so this pair needs a different primitive extension.
    let nonlinear = curve(vec![
        vec![-3., 3.],
        vec![-3., 3.],
        vec![-2., 1.],
        vec![0., -3.],
        vec![3., 3.],
    ]);
    let a = owner(&source, &horizontal, 1e-12);
    let b = owner(&source, &nonlinear, 1e-12);
    assert_eq!(a.nodes().len(), 2);
    assert_eq!(b.nodes().len(), 2);
    assert!(b.census().pairs.iter().any(|p| p.shear > 0));
    assert_ne!(a.nodes()[0].exact.primitive, b.nodes()[0].exact.primitive);
    let limits = p::ParameterOrderLimits::default();
    assert_eq!(
        compare(a.clone(), 0, 0, b.clone(), 0, 0, limits).ordering(),
        Ordering::Equal
    );
    assert_eq!(
        compare(a.clone(), 1, 0, b.clone(), 1, 0, limits).ordering(),
        Ordering::Equal
    );
    assert_eq!(
        compare(a.clone(), 0, 0, b.clone(), 1, 0, limits).ordering(),
        Ordering::Less
    );
    assert_eq!(
        compare(a, 1, 0, b, 0, 0, limits).ordering(),
        Ordering::Greater
    );
}
#[test]
fn overlapping_enclosures_do_not_merge_one_ulp_different_source_events() {
    let source = curve(vec![vec![-3., 0.], vec![0., 6.], vec![3., 0.]]);
    let a = owner(&source, &curve(vec![vec![-4., 1.5], vec![4., 1.5]]), 1e-3);
    let y = 1.5_f64.next_up();
    let b = owner(&source, &curve(vec![vec![-4., y], vec![4., y]]), 1e-3);
    assert_eq!(
        a.nodes()[0].exact.isolating_interval,
        b.nodes()[0].exact.isolating_interval
    );
    let limits = p::ParameterOrderLimits::default();
    assert_eq!(
        compare(a.clone(), 0, 0, b.clone(), 0, 0, limits).ordering(),
        Ordering::Less
    );
    assert_eq!(
        compare(a.clone(), 1, 0, b.clone(), 1, 0, limits).ordering(),
        Ordering::Greater
    );
    let insufficient = p::ParameterOrderLimits {
        max_refinements: 0,
        ..limits
    };
    assert!(
        p::compare_event_parameters(
            a,
            p::EventParameterRef { node: 0, source: 0 },
            b,
            p::EventParameterRef { node: 0, source: 0 },
            insufficient
        )
        .is_err()
    );
}
#[test]
fn one_shared_point_keeps_different_traversal_parameters_separate() {
    let a = curve(vec![vec![0., 0.], vec![0., 1.]]);
    let b = curve(vec![vec![0.75, 0.5], vec![-1.25, 0.5], vec![0.75, 0.5]]);
    let owner = owner(&a, &b, 1e-12);
    assert_eq!(owner.nodes().len(), 2);
    let limits = p::ParameterOrderLimits::default();
    assert_eq!(
        compare(owner.clone(), 0, 0, owner.clone(), 1, 0, limits).ordering(),
        Ordering::Equal
    );
    assert_eq!(
        compare(owner.clone(), 0, 1, owner.clone(), 1, 1, limits).ordering(),
        Ordering::Less
    );
    assert!(
        p::compare_event_parameters(
            owner.clone(),
            p::EventParameterRef { node: 0, source: 0 },
            owner.clone(),
            p::EventParameterRef { node: 0, source: 1 },
            limits
        )
        .is_err()
    );
    assert!(
        p::compare_event_parameters(
            owner.clone(),
            p::EventParameterRef {
                node: 99,
                source: 0
            },
            owner,
            p::EventParameterRef { node: 0, source: 0 },
            limits
        )
        .is_err()
    );
}
