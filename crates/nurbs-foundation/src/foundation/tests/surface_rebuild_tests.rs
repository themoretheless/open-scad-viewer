#[cfg(feature = "codec")]
use value_codec::json;

use super::*;
fn surface(rational: bool) -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![2., 2., 2., 5., 5., 5.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| vec![i as f64, j as f64, if i == 1 && j == 1 { 1. } else { 0. }])
                    .collect()
            })
            .collect(),
        weights: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        if rational && i == 1 && j == 1 {
                            0.6
                        } else {
                            1.
                        }
                    })
                    .collect()
            })
            .collect(),
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
#[cfg(feature = "codec")]
fn rational_and_polynomial_rebuilds_bound_the_whole_patch() {
    for rational in [false, true] {
        for axis in [Axis::U, Axis::V] {
            let s = surface(rational);
            let result = rebuild_surface(&s, axis, 3, 6, 0.001, None).unwrap();
            assert_eq!(result["certificate"]["accepted"], json!(true));
            let rebuilt: Surface = value_codec::from_value(result["surface"].clone()).unwrap();
            assert_eq!(
                match axis {
                    Axis::U => rebuilt.control_points.len(),
                    Axis::V => rebuilt.control_points[0].len(),
                },
                6
            );
            let bound = result["certificate"]["hausdorffErrorUpper"]
                .as_f64()
                .unwrap();
            for i in 0..=20 {
                for j in 0..=20 {
                    let u = 2. + 3. * i as f64 / 20.;
                    let v = j as f64 / 20.;
                    let error = distance(
                        &s.evaluate(u, v).unwrap().point,
                        &rebuilt.evaluate(u, v).unwrap().point,
                    );
                    assert!(error <= bound, "{error} > {bound}");
                }
            }
        }
    }
}
#[test]
#[cfg(feature = "codec")]
fn surface_rebuild_never_accepts_an_overflowed_bound() {
    let mut s = surface(true);
    for row in &mut s.control_points {
        for point in row {
            for coordinate in point {
                *coordinate *= 1e290;
            }
        }
    }
    for row in &mut s.weights {
        for weight in row {
            *weight *= 1e10;
        }
    }
    let result = rebuild_surface(&s, Axis::U, 3, 6, 0.01, None);
    if let Ok(value) = result {
        assert_eq!(value["certificate"]["accepted"], json!(false));
    }
}
#[test]
#[cfg(feature = "codec")]
fn surface_bound_detects_a_peak_between_uniform_samples() {
    let s = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 0.0001, 0.0002, 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [0., 0.0001, 0.0002, 1.]
            .into_iter()
            .enumerate()
            .map(|(i, x)| {
                vec![
                    vec![x, 0., if i == 1 { 1. } else { 0. }],
                    vec![x, 1., if i == 1 { 1. } else { 0. }],
                ]
            })
            .collect(),
        weights: vec![vec![1.; 2]; 4],
        periodic_u: false,
        periodic_v: false,
    };
    let result = rebuild_surface(&s, Axis::U, 1, 2, 0.01, None).unwrap();
    assert_eq!(result["certificate"]["accepted"], json!(false));
    assert!(
        result["certificate"]["hausdorffErrorUpper"]
            .as_f64()
            .unwrap()
            >= 1.
    );
}
#[test]
#[cfg(feature = "codec")]
fn refuses_a_removed_bump_and_preserves_the_original() {
    let s = surface(false);
    let result = rebuild_surface(&s, Axis::U, 1, 2, 0.01, None).unwrap();
    assert_eq!(result["certificate"]["accepted"], json!(false));
    assert_eq!(result["surface"], value_codec::to_value(&s).unwrap());
    assert!(
        result["certificate"]["hausdorffErrorUpper"]
            .as_f64()
            .unwrap()
            >= 0.25
    );
    assert!(rebuild_surface(&s, Axis::U, 0, 2, 0.1, None).is_err());
    assert!(rebuild_surface(&s, Axis::U, 3, 3, 0.1, None).is_err());
}
