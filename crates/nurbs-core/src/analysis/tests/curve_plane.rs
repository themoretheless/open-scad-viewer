use super::*;
#[test]
fn small_bernstein_coefficients_cannot_be_discarded_as_exact_zero() {
    let tiny = 2_f64.powi(-100);
    let c = Curve {
        degree: 3,
        knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points: vec![
            vec![0., 0., -1.],
            vec![1., 0., tiny],
            vec![2., 0., -tiny],
            vec![3., 0., 1.],
        ],
        weights: vec![1.; 4],
        periodic: false,
    };
    // The exact source signs are -,+,-,+. Extraction cancellation can
    // widen the tiny coefficients, but must retain the upper bound 3.
    assert_eq!(
        bernstein_variation_upper(&c, 3, [0., 1.], [0., 0., 1.], 0.).unwrap(),
        3
    );
}
#[test]
fn four_factor_products_retain_extreme_exponents_and_low_residuals() {
    let tiny = f64::from_bits(1);
    assert!(!exact_four_products_zero(&[[tiny; 4]]));
    assert!(exact_four_products_zero(&[
        [tiny; 4],
        [-tiny, tiny, tiny, tiny]
    ]));
    let huge = f64::MAX;
    assert!(exact_four_products_zero(&[
        [huge; 4],
        [-huge, huge, huge, huge]
    ]));
    assert!(!exact_four_products_zero(&[
        [huge; 4],
        [-huge, huge, huge, huge],
        [tiny; 4]
    ]));
}
#[test]
fn exact_products_keep_underflow_overflow_and_cancellation_residuals() {
    let tiny = f64::from_bits(1);
    assert!(!exact_plane_zero(&[tiny, 0., 0.], [tiny, 0., 0.], 0.));
    assert!(exact_plane_zero(&[tiny, tiny, 0.], [tiny, -tiny, 0.], 0.));
    assert!(exact_plane_zero(
        &[f64::MAX, f64::MAX, 0.],
        [f64::MAX, -f64::MAX, 0.],
        0.
    ));
    assert!(!exact_plane_zero(&[1e16, 1., 1e16], [1., 1., -1.], 0.));
    assert!(exact_plane_zero(&[1., 1., 1.], [1e308, -1e308, tiny], tiny));
}
