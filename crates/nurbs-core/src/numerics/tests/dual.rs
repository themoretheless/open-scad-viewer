use super::*;

fn assert_close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance * (1. + expected.abs()),
        "actual {actual:e}, expected {expected:e}"
    );
}

fn rational_curve() -> Curve {
    // Multi-span cubic with nontrivial weights and simple interior knots
    // (C² there, so the analytic d1/d2 are available for comparison).
    Curve {
        degree: 3,
        knots: vec![0., 0., 0., 0., 1., 2., 3., 4., 4., 4., 4.],
        control_points: vec![
            vec![0., 0., 0.],
            vec![1., 2., 0.5],
            vec![2., -1., 1.],
            vec![3., 3., -0.5],
            vec![4., 0., 2.],
            vec![5., 1., 0.],
            vec![6., 2., 1.],
        ],
        weights: vec![1., 0.7, 1.8, 0.4, 2.2, 1.1, 0.9],
        periodic: false,
    }
}

fn rational_surface() -> Surface {
    // Cubic Bézier patch with varied weights.
    let control_points = vec![
        vec![
            vec![0., 0., 0.],
            vec![0., 1., 0.4],
            vec![0., 2., -0.3],
            vec![0., 3., 0.2],
        ],
        vec![
            vec![1., 0., 0.5],
            vec![1., 1., 1.0],
            vec![1., 2., 0.6],
            vec![1., 3., 0.9],
        ],
        vec![
            vec![2., 0., -0.4],
            vec![2., 1., 0.3],
            vec![2., 2., 0.8],
            vec![2., 3., -0.2],
        ],
        vec![
            vec![3., 0., 0.1],
            vec![3., 1., 0.7],
            vec![3., 2., 0.2],
            vec![3., 3., 0.6],
        ],
    ];
    let weights = vec![
        vec![1., 1.5, 0.8, 1.2],
        vec![0.9, 2., 1.1, 0.7],
        vec![1.3, 0.6, 1.8, 1.],
        vec![0.5, 1.4, 1., 1.6],
    ];
    Surface {
        degree_u: 3,
        degree_v: 3,
        knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points,
        weights,
        periodic_u: false,
        periodic_v: false,
    }
}

#[test]
fn dual_arithmetic_reference_values() {
    let x = Dual::variable(2.);
    let c = Dual::constant(3.);
    let sum = x + c;
    assert_eq!(sum, Dual { val: 5., der: 1. });
    let product = x * x * Dual::constant(0.5);
    assert_close(product.val, 2., 0.);
    assert_close(product.der, 2., 1e-15);
    let quotient = c / x;
    assert_close(quotient.val, 1.5, 1e-15);
    assert_close(quotient.der, -0.75, 1e-15);
    let neg = -x;
    assert_eq!(neg.val, -2.);
    assert_eq!(neg.der, -1.);
    let root = Dual::variable(2.).sqrt();
    assert_close(root.val, std::f64::consts::SQRT_2, 1e-15);
    assert_close(root.der, 0.25 * std::f64::consts::SQRT_2, 1e-15);
    // Chain rule: d/dx sin(x²) at 0.7 = 2x·cos(x²).
    let chained = (Dual::variable(0.7) * Dual::variable(0.7)).sin();
    assert_close(chained.der, 1.4 * (0.49_f64).cos(), 1e-14);
    // exp(ln x) = x; powi/powf agree at integer exponents.
    let recovered = Dual::variable(1.3).ln().exp();
    assert_close(recovered.val, 1.3, 1e-14);
    assert_close(recovered.der, 1., 1e-14);
    assert_close(Dual::variable(1.1).powi(3).der, 3. * 1.21, 1e-14);
    assert_close(
        Dual::variable(1.1).powf(Dual::constant(3.)).der,
        3. * 1.21,
        1e-14,
    );
    // d/dy atan2(y, x) = x/(x²+y²) at (y=1, x=2).
    let angle = Dual::variable(1.).atan2(Dual::constant(2.));
    assert_close(angle.der, 0.4, 1e-15);
}

#[test]
fn hyper_dual_arithmetic_reference_values() {
    // (x·y) with x = 2 + ε₁ + ε₂, y = 3 + 2ε₁ − ε₂:
    // d12 = x1·y2 + x2·y1 = 1·(−1) + 1·2 = 1.
    let x = HyperDual {
        val: 2.,
        d1: 1.,
        d2: 1.,
        d12: 0.,
    };
    let y = HyperDual {
        val: 3.,
        d1: 2.,
        d2: -1.,
        d12: 0.,
    };
    let product = x * y;
    assert_close(product.val, 6., 0.);
    assert_close(product.d1, 2. * 2. + 3., 1e-15);
    assert_close(product.d2, 2. * -1. + 3., 1e-15);
    assert_close(product.d12, 1., 1e-15);
    let quotient = x / y;
    assert_close(quotient.val, 2. / 3., 1e-15);
    // Second derivative through the both-seeded slot: d²(x³)/dx² at x=1.5 = 9.
    let cube = HyperDual::variable_second(1.5).powi(3);
    assert_close(cube.val, 3.375, 1e-14);
    assert_close(cube.d12, 9., 1e-13);
    // d²√x/dx² = −1/(4 x^(3/2)) at x = 4 → −1/32.
    let root = HyperDual::variable_second(4.).sqrt();
    assert_close(root.d12, -1. / 32., 1e-14);
    // d² sin x/dx² = −sin x.
    let sine = HyperDual::variable_second(0.9).sin();
    assert_close(sine.d12, -(0.9_f64).sin(), 1e-14);
    // d² cos x/dx² = −cos x; d² eˣ/dx² = eˣ; d² ln x = −1/x².
    assert_close(
        HyperDual::variable_second(0.9).cos().d12,
        -(0.9_f64).cos(),
        1e-14,
    );
    assert_close(
        HyperDual::variable_second(0.4).exp().d12,
        (0.4_f64).exp(),
        1e-14,
    );
    assert_close(HyperDual::variable_second(2.).ln().d12, -0.25, 1e-14);
    // Mixed partial: f(x,y) = x·sin(y), f_xy = cos(y).
    let mixed = HyperDual::variable_e1(1.2) * HyperDual::variable_e2(0.7).sin();
    assert_close(mixed.d12, (0.7_f64).cos(), 1e-14);
}

#[test]
fn dual_curve_matches_analytic_first_derivative() {
    let curve = rational_curve();
    for step in 0..=40 {
        let t = 0.05 + 3.9 * step as f64 / 40.;
        let analytic = curve.evaluate(t).unwrap();
        let lifted = evaluate_curve_dual(&curve, Dual::variable(t)).unwrap();
        let d1 = analytic.d1.as_ref().unwrap();
        for axis in 0..3 {
            assert_close(lifted[axis].val, analytic.point[axis], 1e-12);
            assert_close(lifted[axis].der, d1[axis], 1e-9);
        }
    }
}

#[test]
fn hyper_dual_curve_matches_analytic_second_derivative() {
    let curve = rational_curve();
    for step in 0..=40 {
        let t = 0.05 + 3.9 * step as f64 / 40.;
        let analytic = curve.evaluate(t).unwrap();
        let d2 = analytic.d2.as_ref().unwrap();
        // Both-ε seeding: the mixed slot of a lifted curve evaluation is
        // the unmixed second derivative.
        let lifted =
            evaluate_curve_scalar::<HyperDual>(&curve, &HyperDual::variable_second(t)).unwrap();
        for axis in 0..3 {
            assert_close(lifted[axis].val, analytic.point[axis], 1e-12);
            assert_close(lifted[axis].d1, analytic.d1.as_ref().unwrap()[axis], 1e-9);
            assert_close(lifted[axis].d12, d2[axis], 1e-9);
        }
    }
}

#[test]
fn surface_curvature_dual_matches_analytic_jet() {
    let surface = rational_surface();
    for (u, v) in [
        (0.13, 0.21),
        (0.37, 0.62),
        (0.55, 0.44),
        (0.79, 0.88),
        (0.91, 0.07),
    ] {
        let analytic = surface.evaluate(u, v).unwrap();
        let curvature = surface_curvature_dual(&surface, u, v).unwrap();
        let (du, dv) = analytic.first_derivatives().unwrap();
        let (duu, duv, dvv) = analytic.second_derivatives().unwrap();
        for axis in 0..3 {
            assert_close(curvature.point[axis], analytic.point[axis], 1e-12);
            assert_close(curvature.du[axis], du[axis], 1e-9);
            assert_close(curvature.dv[axis], dv[axis], 1e-9);
            assert_close(curvature.duu[axis], duu[axis], 1e-9);
            assert_close(curvature.duv[axis], duv[axis], 1e-9);
            assert_close(curvature.dvv[axis], dvv[axis], 1e-9);
        }
        let (gaussian, mean) = analytic.curvatures().unwrap();
        assert_close(curvature.gaussian, gaussian, 1e-9);
        assert_close(curvature.mean, mean, 1e-9);
    }
}

#[test]
fn horner_matches_polynomial_derivative() {
    // p(x) = 1 + 2x − x² + 0.5x³ → p'(x) = 2 − 2x + 1.5x² at x = 0.8.
    let p = horner::<Dual>(&[1., 2., -1., 0.5], &Dual::variable(0.8));
    assert_close(p.val, 1. + 1.6 - 0.64 + 0.256, 1e-14);
    assert_close(p.der, 2. - 1.6 + 0.96, 1e-14);
    let p2 = horner::<HyperDual>(&[1., 2., -1., 0.5], &HyperDual::variable_second(0.8));
    assert_close(p2.d12, -2. + 3. * 0.8, 1e-14);
}

#[test]
fn invalid_inputs_are_errors() {
    let curve = rational_curve();
    assert!(evaluate_curve_dual(&curve, Dual::variable(-0.1)).is_err());
    assert!(evaluate_curve_dual(&curve, Dual::variable(4.1)).is_err());
    let surface = rational_surface();
    assert!(surface_curvature_dual(&surface, 1.5, 0.5).is_err());
}
