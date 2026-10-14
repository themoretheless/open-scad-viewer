#[cfg(feature = "codec")]
use value_codec::json;

use super::*;
fn curve() -> Curve {
    Curve {
        degree: 3,
        knots: vec![2., 2., 2., 2., 5., 5., 5., 5.],
        control_points: vec![
            vec![0., 1., 2.],
            vec![1., 2., 3.],
            vec![3., 1., 4.],
            vec![4., 2., 5.],
        ],
        weights: vec![1., 0.7, 1.3, 0.9],
        periodic: false,
    }
}
#[test]
#[cfg(feature = "codec")]
fn matches_all_endpoint_pairs_and_preserves_weights_and_other_controls() {
    let a = curve();
    let mut b = curve();
    for p in &mut b.control_points {
        p[0] += 20.;
    }
    let before = b.clone();
    for r in ["start", "end"] {
        for e in ["start", "end"] {
            let result = checked(&a, &b, r, e, 1e-10).unwrap();
            assert_eq!(result["report"]["accepted"], json!(true));
            let out: Curve = value_codec::from_value(result["curve"].clone()).unwrap();
            let (ri, _) = endpoint(&a, r).unwrap();
            let (ei, ea) = endpoint(&b, e).unwrap();
            assert_eq!(out.control_points[ei], a.control_points[ri]);
            assert_eq!(out.weights, b.weights);
            assert_eq!(out.knots, b.knots);
            for i in 0..4 {
                if i != ei && i != ea {
                    assert_eq!(out.control_points[i], b.control_points[i]);
                }
            }
            let failed = checked(&a, &b, r, e, 0.).unwrap();
            assert_eq!(failed["report"]["accepted"], json!(false));
            assert_eq!(failed["curve"], value_codec::to_value(&b).unwrap());
        }
    }
    assert_eq!(
        value_codec::to_value(b).unwrap(),
        value_codec::to_value(before).unwrap()
    );
}
#[test]
#[cfg(feature = "codec")]
fn rejects_unclamped_knots_without_absolute_epsilon_and_degenerate_reference() {
    let mut a = curve();
    a.knots = vec![-2e-12, -1e-12, 0., 0., 1e-12, 1e-12, 1e-12, 1e-12];
    assert!(checked(&a, &curve(), "start", "start", 1e-10).is_err());
    let mut a = curve();
    a.control_points[1] = a.control_points[0].clone();
    assert!(checked(&a, &curve(), "start", "end", 1e-10).is_err());
}
#[test]
#[cfg(feature = "codec")]
fn refuses_a_handle_lost_to_coordinate_rounding() {
    let mut a = curve();
    a.control_points = vec![
        vec![1e8, 0., 0.],
        vec![1e8 + 4., 0., 0.],
        vec![1e8 + 8., 0., 0.],
        vec![1e8 + 12., 0., 0.],
    ];
    let mut b = curve();
    b.control_points[0] = vec![0.; 3];
    b.control_points[1] = vec![1e-10, 0., 0.];
    let result = checked(&a, &b, "end", "start", 1e-10).unwrap();
    assert_eq!(result["report"]["accepted"], json!(false));
    assert_eq!(result["curve"], value_codec::to_value(b).unwrap());
}
