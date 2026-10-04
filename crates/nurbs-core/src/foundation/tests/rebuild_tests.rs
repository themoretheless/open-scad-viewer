#[cfg(feature = "codec")]
use value_codec::json;

use super::*;
fn curve() -> Curve {
    Curve {
        degree: 2,
        knots: vec![2., 2., 2., 5., 5., 5.],
        control_points: vec![vec![0., 0., 0.], vec![1., 2., 0.], vec![2., 0., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    }
}
#[test]
#[cfg(feature = "codec")]
fn rebuild_controls_and_degree_with_rollback() {
    let c = curve();
    let refused = rebuild_curve(&c, 1, 2, 0., None).unwrap();
    assert_eq!(refused["certificate"]["accepted"], json!(false));
    assert_eq!(refused["curve"], value_codec::to_value(&c).unwrap());
    let accepted = rebuild_curve(&c, 3, 6, 0.1, None).unwrap();
    assert_eq!(accepted["certificate"]["accepted"], json!(true));
    let rebuilt: Curve = value_codec::from_value(accepted["curve"].clone()).unwrap();
    assert_eq!(rebuilt.degree, 3);
    assert_eq!(rebuilt.control_points.len(), 6);
    assert_eq!(rebuilt.domain(), [2., 5.]);
    let bound = accepted["certificate"]["hausdorffErrorUpper"]
        .as_f64()
        .unwrap();
    for i in 0..=300 {
        let u = 2. + 3. * i as f64 / 300.;
        assert!(
            distance(
                &c.evaluate(u).unwrap().point,
                &rebuilt.evaluate(u).unwrap().point
            ) <= bound
        );
    }
}
#[test]
#[cfg(feature = "codec")]
fn rebuild_preserves_rational_shape_within_reported_bound() {
    let c = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![1., 0., 0.], vec![1., 1., 0.], vec![0., 1., 0.]],
        weights: vec![1., 0.5_f64.sqrt(), 1.],
        periodic: false,
    };
    let result = rebuild_curve(&c, 3, 6, 0.05, None).unwrap();
    assert_eq!(result["certificate"]["accepted"], json!(true));
    let rebuilt: Curve = value_codec::from_value(result["curve"].clone()).unwrap();
    assert!(rebuilt.weights.iter().any(|w| (*w - 1.).abs() > 0.01));
    let bound = result["certificate"]["hausdorffErrorUpper"]
        .as_f64()
        .unwrap();
    for i in 0..=500 {
        let u = i as f64 / 500.;
        let p = rebuilt.evaluate(u).unwrap().point;
        assert!(distance(&c.evaluate(u).unwrap().point, &p) <= bound);
        assert!((p[0].hypot(p[1]) - 1.).abs() < 1e-10);
    }
}
#[test]
#[cfg(feature = "codec")]
fn rebuild_rejects_invalid_budgets_and_basis() {
    let c = curve();
    for (d, n) in [(0, 4), (26, 30), (3, 3), (1, 257), (usize::MAX, 4)] {
        assert!(rebuild_curve(&c, d, n, 0.1, None).is_err());
    }
    for budget in [-1., f64::NAN, f64::INFINITY] {
        assert!(rebuild_curve(&c, 2, 3, budget, None).is_err());
    }
}
