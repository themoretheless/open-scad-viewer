use cad_predicates::{BezierIdentity as B, Reason};
use nurbs_core::{
    curve::Curve, curve_surface_agreement::verify_exact_trimmed as verify, surface::Surface,
};
fn fixture() -> (Curve, Curve, Surface) {
    let points = vec![
        vec![0., 0., 0.],
        vec![3., 6., 3.],
        vec![9., 6., 9.],
        vec![12., 0., 12.],
    ];
    let weights = vec![1., 2., 2., 1.];
    let c = Curve {
        degree: 2,
        knots: vec![-2., -2., -2., 1., 4., 4., 4.],
        control_points: points.clone(),
        weights: weights.clone(),
        periodic: false,
    };
    let p = Curve {
        degree: 1,
        knots: vec![-4., -4., 8., 8.],
        control_points: vec![vec![0., 0.], vec![1., 0.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let s = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 0.5, 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: points
            .iter()
            .map(|p| {
                let mut q = p.clone();
                q[2] += 2.;
                vec![p.clone(), q]
            })
            .collect(),
        weights: weights.iter().map(|w| vec![*w; 2]).collect(),
        periodic_u: false,
        periodic_v: false,
    };
    (c, p, s)
}
#[test]
fn original_rational_surface_trace_crosses_knots_under_independent_trim_clocks() {
    let (c, p, s) = fixture();
    let before = (c.clone(), p.clone(), s.clone());
    assert_eq!(
        verify(&c, &p, &s, false, [0., 2.], [0., 4.], 1000000)
            .unwrap()
            .unwrap()
            .outcome,
        B::Equal
    );
    assert_eq!(
        verify(&c, &p, &s, true, [0., 2.], [4., 0.], 1000000)
            .unwrap()
            .unwrap()
            .outcome,
        B::Equal
    );
    assert_eq!(
        verify(&c, &p, &s, true, [0., 2.], [0., 4.], 1000000)
            .unwrap()
            .unwrap()
            .outcome,
        B::Different
    );
    assert_eq!((c, p, s), before);
}
#[test]
fn actual_world_or_surface_changes_are_not_hidden_by_trace_ownership() {
    let (mut c, p, mut s) = fixture();
    c.control_points[1][2] += 2f64.powi(-40);
    assert_eq!(
        verify(&c, &p, &s, false, [0., 2.], [0., 4.], 1000000)
            .unwrap()
            .unwrap()
            .outcome,
        B::Different
    );
    c = fixture().0;
    s.weights[1][0] += 0.125;
    assert_eq!(
        verify(&c, &p, &s, false, [0., 2.], [0., 4.], 1000000)
            .unwrap()
            .unwrap()
            .outcome,
        B::Different
    );
}
#[test]
fn outside_original_uv_poles_are_allowed_only_when_the_declared_trim_stays_in_chart() {
    let (c, mut p, s) = fixture();
    p.control_points = vec![vec![-1., 0.], vec![3., 0.]];
    p.knots = vec![2., 2., 6., 6.];
    assert_eq!(
        verify(&c, &p, &s, false, [-2., 4.], [3., 4.], 1000000)
            .unwrap()
            .unwrap()
            .outcome,
        B::Equal
    );
    assert!(matches!(
        verify(&c, &p, &s, false, [-2., 4.], [2., 6.], 1000000)
            .unwrap()
            .unwrap()
            .outcome,
        B::Indeterminate(Reason::MissingProof)
    ));
    assert!(matches!(
        verify(&c, &p, &s, false, [-2., 4.], [3., 4.], 1)
            .unwrap()
            .unwrap()
            .outcome,
        B::Indeterminate(Reason::ResourceLimit)
    ));
}
#[test]
fn curved_uv_trace_does_not_borrow_an_isoparametric_certificate() {
    let (c, mut p, s) = fixture();
    p.degree = 2;
    p.knots = vec![-4., -4., -4., 8., 8., 8.];
    p.control_points = vec![vec![0., 0.], vec![0.5, 0.25], vec![1., 0.]];
    p.weights = vec![1.; 3];
    assert!(
        verify(&c, &p, &s, false, [0., 2.], [0., 4.], 1000000)
            .unwrap()
            .is_none()
    );
}

#[test]
fn transposed_tensor_axis_and_global_world_weight_gauge_keep_the_same_source_trace() {
    let (mut c, mut p, s) = fixture();
    let transposed = Surface {
        degree_u: s.degree_v,
        degree_v: s.degree_u,
        knots_u: s.knots_v.clone(),
        knots_v: s.knots_u.clone(),
        control_points: (0..2)
            .map(|v| (0..4).map(|u| s.control_points[u][v].clone()).collect())
            .collect(),
        weights: (0..2)
            .map(|v| (0..4).map(|u| s.weights[u][v]).collect())
            .collect(),
        periodic_u: false,
        periodic_v: false,
    };
    for point in &mut p.control_points {
        point.swap(0, 1);
    }
    c.weights.iter_mut().for_each(|w| *w *= 4.);
    assert_eq!(
        verify(&c, &p, &transposed, false, [0., 2.], [0., 4.], 1000000)
            .unwrap()
            .unwrap()
            .outcome,
        B::Equal
    );
}
