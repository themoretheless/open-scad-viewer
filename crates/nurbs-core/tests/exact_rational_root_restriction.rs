use cad_predicates::{BezierIdentity as B, Reason};
use nurbs_core::{
    curve::Curve, curve_partition_agreement::verify_exact_bezier_restriction as verify,
};
fn curve(points: Vec<Vec<f64>>, weights: Vec<f64>, domain: [f64; 2]) -> Curve {
    let degree = points.len() - 1;
    Curve {
        degree,
        knots: [vec![domain[0]; degree + 1], vec![domain[1]; degree + 1]].concat(),
        control_points: points,
        weights,
        periodic: false,
    }
}
#[test]
fn exact_rational_child_preserves_original_nonunit_clock_and_global_weight_gauge() {
    // Restriction to normalized [0,1/2] has exactly representable poles,
    // unlike many numerically extracted rational children.
    let source = curve(
        vec![vec![0., 0., 0.], vec![2., 1., 0.]],
        vec![1., 3.],
        [-2., 6.],
    );
    let child = curve(
        vec![vec![0., 0., 0.], vec![1.5, 0.75, 0.]],
        vec![2., 4.],
        [11., 19.],
    );
    assert_eq!(
        verify(&source, &child, [-2., 2.], false, 100000)
            .unwrap()
            .unwrap()
            .outcome,
        B::Equal
    );
    let elevated = curve(
        vec![vec![0., 0., 0.], vec![1., 0.5, 0.], vec![1.5, 0.75, 0.]],
        vec![2., 3., 4.],
        [0., 1.],
    );
    assert_eq!(
        verify(&source, &elevated, [-2., 2.], false, 100000)
            .unwrap()
            .unwrap()
            .outcome,
        B::Equal
    );
    let mut backward = child.clone();
    backward.control_points.reverse();
    backward.weights.reverse();
    assert_eq!(
        verify(&source, &backward, [-2., 2.], true, 100000)
            .unwrap()
            .unwrap()
            .outcome,
        B::Equal
    );
    assert_eq!(
        verify(&source, &backward, [-2., 2.], false, 100000)
            .unwrap()
            .unwrap()
            .outcome,
        B::Different
    );
    let mut wrong = child.clone();
    wrong.control_points[1][1] += 2f64.powi(-45);
    assert_eq!(
        verify(&source, &wrong, [-2., 2.], false, 100000)
            .unwrap()
            .unwrap()
            .outcome,
        B::Different
    );
    assert!(matches!(
        verify(&source, &child, [-2., 2.], false, 1)
            .unwrap()
            .unwrap()
            .outcome,
        B::Indeterminate(Reason::ResourceLimit)
    ));
    assert!(verify(&source, &child, [-2., 2.], false, 0).is_err());
}
#[test]
fn restriction_identity_checks_interiors_and_refuses_outside_original_domain() {
    let source = curve(
        vec![vec![0., 0., 0.], vec![1., 2., 0.], vec![2., 0., 0.]],
        vec![1., 2., 1.],
        [0., 1.],
    );
    // Entire range and a different positive global weight scale are exact too.
    let mut child = source.clone();
    child.weights.iter_mut().for_each(|w| *w *= 4.);
    assert_eq!(
        verify(&source, &child, [0., 1.], false, 100000)
            .unwrap()
            .unwrap()
            .outcome,
        B::Equal
    );
    child.control_points[1][1] += 0.125;
    assert_eq!(
        verify(&source, &child, [0., 1.], false, 100000)
            .unwrap()
            .unwrap()
            .outcome,
        B::Different
    );
    assert!(matches!(
        verify(&source, &source, [-0.25, 1.], false, 100000)
            .unwrap()
            .unwrap()
            .outcome,
        B::Indeterminate(_)
    ));
}

#[test]
fn exact_uv_children_use_the_same_root_proof_without_fabricating_a_third_coordinate() {
    let source = curve(vec![vec![0., 0.], vec![2., 1.]], vec![1., 3.], [-2., 6.]);
    let child = curve(vec![vec![0., 0.], vec![1.5, 0.75]], vec![2., 4.], [0., 1.]);
    assert_eq!(
        verify(&source, &child, [-2., 2.], false, 100000)
            .unwrap()
            .unwrap()
            .outcome,
        B::Equal
    );
}

#[test]
fn curved_rational_child_matches_the_complete_restricted_quadratic() {
    let source = curve(
        vec![vec![0., 0., 0.], vec![2., 2., 0.], vec![4., 0., 0.]],
        vec![1., 3., 1.],
        [-3., 7.],
    );
    let child = curve(
        vec![vec![0., 0., 0.], vec![1.5, 1.5, 0.], vec![2., 1.5, 0.]],
        vec![1., 2., 2.],
        [10., 20.],
    );
    assert_eq!(
        verify(&source, &child, [-3., 2.], false, 100000)
            .unwrap()
            .unwrap()
            .outcome,
        B::Equal
    );
    let mut damaged = child;
    damaged.control_points[1][2] = 2f64.powi(-50);
    assert_eq!(
        verify(&source, &damaged, [-3., 2.], false, 100000)
            .unwrap()
            .unwrap()
            .outcome,
        B::Different
    );
}

#[test]
fn non_dyadic_normalized_root_ends_are_not_rounded_to_binary64() {
    let source = curve(
        vec![vec![0., 0., 0.], vec![9., 0., 0.]],
        vec![1., 4.],
        [-2., 4.],
    );
    let child = curve(
        vec![vec![6., 0., 0.], vec![8., 0., 0.]],
        vec![2., 3.],
        [0., 1.],
    );
    assert_eq!(
        verify(&source, &child, [0., 2.], false, 100000)
            .unwrap()
            .unwrap()
            .outcome,
        B::Equal
    );
}
