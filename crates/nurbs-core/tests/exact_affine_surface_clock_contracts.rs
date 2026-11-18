use nurbs_core::{
    editing::exact_projective::{Refusal, affine_surface_clock},
    surface::Surface,
};

const WORK: u64 = 1_000_000;

fn patch() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-1., 0., 1., 3., 5.],
        knots_v: vec![0., 0., 2., 2.],
        control_points: [0., 1., 3.]
            .into_iter()
            .map(|u| vec![vec![u, 0., u], vec![u, 2., u * u - 1.]])
            .collect(),
        weights: vec![vec![1., 2.], vec![4., 1.], vec![2., 0.5]],
        periodic_u: false,
        periodic_v: false,
    }
}

#[test]
fn maps_independent_axes_and_preserves_a_rational_tensor_surface() {
    let source = patch();
    let result = affine_surface_clock(&source, [[12., 6.], [-2., -1.]], WORK).unwrap();
    let out = result
        .geometry
        .unwrap_or_else(|| panic!("{:?}", result.refusal));
    assert_eq!(out.knots_u, [2., 6., 10., 12., 14.]);
    assert_eq!(out.knots_v, [-2., -2., -1., -1.]);
    assert_eq!(out.control_points[0], source.control_points[2]);
    assert_eq!(out.weights[0], source.weights[2]);
    for u in [0., 0.5, 1., 2.75, 3.] {
        for v in [0., 0.5, 2.] {
            let a = source.evaluate(u, v).unwrap().point;
            let b = out.evaluate(12. - 2. * u, -2. + v / 2.).unwrap().point;
            assert!(a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-13));
        }
    }
}

#[test]
fn maps_periodic_knots_and_reverses_both_control_net_axes() {
    let mut source = patch();
    source.periodic_u = true;
    source.knots_u = vec![-1., 0., 1., 2., 3., 4.];
    source.control_points.push(source.control_points[0].clone());
    source.weights.push(source.weights[0].clone());
    let result = affine_surface_clock(&source, [[6., 0.], [4., 0.]], WORK).unwrap();
    let out = result
        .geometry
        .unwrap_or_else(|| panic!("{:?}", result.refusal));
    assert!(out.periodic_u);
    assert_eq!(out.knots_u, [-2., 0., 2., 4., 6., 8.]);
    assert_eq!(out.control_points[1][0], source.control_points[2][1]);
    assert_eq!(out.weights[1][0], source.weights[2][1]);
    for (u, v) in [(0., 0.), (0.25, 0.5), (1., 1.), (2.5, 2.), (3., 0.)] {
        let a = source.evaluate(u, v).unwrap().point;
        let b = out.evaluate(6. - 2. * u, 4. - 2. * v).unwrap().point;
        assert!(a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-13));
    }
}

#[test]
fn refuses_unrepresentable_knots_without_returning_a_partial_net() {
    let source = patch();
    let refusal = affine_surface_clock(&source, [[0., 1.], [2., 4.]], WORK).unwrap();
    assert_eq!(refusal.refusal, Some(Refusal::NotRepresentable));
    assert!(refusal.geometry.is_none());
    let limited = affine_surface_clock(&source, [[0., 3.], [2., 4.]], 1).unwrap();
    assert_eq!(limited.refusal, Some(Refusal::ResourceLimit));
    assert!(limited.geometry.is_none());
    assert!(limited.work_used <= 1);
}
