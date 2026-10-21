use brep_core::profile_certificate as p;
use nurbs_core::curve::Curve;
use std::sync::Arc;
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
fn owner(a: &Curve, b: &Curve, self_pair: bool) -> Arc<p::CurveEventNodes> {
    let c = p::certify_span_intersections(a, b, self_pair, 1e-12).unwrap();
    Arc::new(p::own_curve_event_nodes(a, b, &c).unwrap())
}
fn branch(
    o: Arc<p::CurveEventNodes>,
    node: usize,
    source: usize,
    direction: i8,
) -> p::OwnedEventBranch {
    let g = p::own_event_branch(o, p::EventParameterRef { node, source }, direction, 192).unwrap();
    g.reverify_source().unwrap();
    g
}
#[test]
fn original_polynomial_and_rational_branches_have_exact_incoming_outgoing_signs() {
    for rational in [false, true] {
        let mut a = curve(vec![vec![-3., 0.], vec![0., 6.], vec![3., 0.]]);
        if rational {
            a.weights = vec![1., 2., 1.];
        }
        for k in &mut a.knots {
            *k = 2. + 4. * *k;
        }
        let b = curve(vec![vec![-4., 1.5], vec![4., 1.5]]);
        let o = owner(&a, &b, false);
        assert_eq!(o.nodes().len(), 2);
        assert_eq!(branch(o.clone(), 0, 0, 1).ray().signs, [1, 1]);
        assert_eq!(branch(o.clone(), 0, 0, -1).ray().signs, [-1, -1]);
        assert_eq!(branch(o.clone(), 1, 0, 1).ray().signs, [1, -1]);
        assert_eq!(branch(o.clone(), 1, 0, -1).ray().signs, [-1, 1]);
        assert_eq!(branch(o.clone(), 0, 1, 1).ray().signs, [1, 0]);
        assert_eq!(branch(o, 0, 1, -1).ray().signs, [-1, 0]);
    }
}
#[test]
fn higher_order_stationary_germs_are_not_replaced_by_arbitrary_tangents() {
    let c = curve(vec![
        vec![-1., 0.],
        vec![1., 0.],
        vec![-1., 0.],
        vec![1., 0.],
    ]);
    let o = owner(&c, &c, true);
    for (direction, sign) in [(-1, -1), (1, 1)] {
        let g = branch(o.clone(), 0, 0, direction);
        assert_eq!(g.ray().order, 3);
        assert_eq!(g.ray().signs, [sign, 0]);
    }
    let cusp = curve(vec![
        vec![3., -1.],
        vec![-1., 1.],
        vec![-1., -1.],
        vec![3., 1.],
    ]);
    let o = owner(&cusp, &cusp, true);
    let incoming = branch(o.clone(), 0, 0, -1);
    let outgoing = branch(o, 0, 0, 1);
    assert_eq!(incoming.ray().order, 2);
    assert_eq!(outgoing.ray().order, 2);
    assert_eq!(incoming.ray().signs, [1, 0]);
    assert_eq!(outgoing.ray().signs, [1, 0]);
    // Same leading ray is deliberately NOT a certificate of identical branches.
    assert_eq!(incoming.ray().numerator, outgoing.ray().numerator);
}
#[test]
fn knot_contact_incoming_uses_previous_original_cell_and_outgoing_uses_right() {
    let a = Curve::from_polyline(vec![vec![-1., 0.], vec![0., 0.], vec![0., 1.]]).unwrap();
    let b = curve(vec![vec![-1., -1.], vec![1., 1.]]);
    let o = owner(&a, &b, false);
    assert_eq!(o.nodes().len(), 1);
    assert_eq!(o.nodes()[0].source_cells[0], 1);
    let incoming = branch(o.clone(), 0, 0, -1);
    let outgoing = branch(o, 0, 0, 1);
    assert_eq!(incoming.source_cell(), 0);
    assert_eq!(incoming.ray().signs, [-1, 0]);
    assert_eq!(outgoing.source_cell(), 1);
    assert_eq!(outgoing.ray().signs, [0, 1]);
}
#[test]
fn natural_domain_ends_refuse_nonexistent_branches_even_when_stationary() {
    let c = curve(vec![vec![0., 0.], vec![0., 0.], vec![1., 1.]]);
    let o = owner(&c, &c, true);
    assert_eq!(branch(o.clone(), 0, 0, 1).ray().order, 2);
    assert!(p::own_event_branch(o, p::EventParameterRef { node: 0, source: 0 }, -1, 192).is_err());
}

#[test]
fn optional_branch_omits_only_actual_domain_ends_not_unproved_algebraic_signs() {
    let c = curve(vec![vec![0., 0.], vec![0., 0.], vec![1., 1.]]);
    let o = owner(&c, &c, true);
    assert!(
        p::try_own_event_branch(o, p::EventParameterRef { node: 0, source: 0 }, -1, 192)
            .unwrap()
            .is_none()
    );
    let a = curve(vec![vec![-3., 0.], vec![0., 6.], vec![3., 0.]]);
    let y = 3_f64.next_down();
    let b = curve(vec![vec![-4., y], vec![4., y]]);
    let census = p::certify_span_intersections(&a, &b, false, 1e-3).unwrap();
    let o = Arc::new(p::own_curve_event_nodes(&a, &b, &census).unwrap());
    let r = p::EventParameterRef { node: 0, source: 0 };
    assert!(p::try_own_event_branch(o.clone(), r, 1, 0).is_err());
    let branch = p::try_own_event_branch(o, r, 1, 192).unwrap().unwrap();
    branch.reverify_source().unwrap();
    assert_eq!(branch.ray().signs, [1, 1]);
}
