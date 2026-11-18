#[cfg(feature = "codec")]
use value_codec::json;

use super::*;
fn plane() -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..3)
            .map(|i| (0..3).map(|j| vec![i as f64, j as f64, 0.]).collect())
            .collect(),
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
#[cfg(feature = "codec")]
fn rejects_degrees_before_unsigned_subtraction() {
    let s = plane();
    for degree in [0, 2, 3, usize::MAX] {
        assert!(reduce_surface_axis(&s, Axis::U, "reduce", 0., degree, 0., None).is_err());
    }
    assert!(reduce_surface_axis(&s, Axis::V, "other", 0., 1, 0., None).is_err());
}
#[test]
#[cfg(feature = "codec")]
fn reduces_both_axes_with_proven_budget_and_refuses_unproven_zero() {
    let s = plane();
    for axis in [Axis::U, Axis::V] {
        let zero = reduce_surface_axis(&s, axis, "reduce", 0., 1, 0., None).unwrap();
        assert_eq!(zero["certificate"]["accepted"], json!(false));
        assert_eq!(zero["certificate"]["rolledBack"], json!(true));
        let result = reduce_surface_axis(&s, axis, "reduce", 0., 1, 1e-3, None).unwrap();
        assert_eq!(result["certificate"]["accepted"], json!(true));
        assert!(result["certificate"]["hausdorffErrorUpper"].as_f64().unwrap() <= 1e-3);
    }
}
