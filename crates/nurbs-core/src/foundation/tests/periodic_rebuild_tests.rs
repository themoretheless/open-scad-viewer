#[cfg(feature = "codec")]
use value_codec::json;

use super::*;
fn source(rational: bool) -> Curve {
    Curve {
        degree: 2,
        knots: (0..9).map(|i| i as f64).collect(),
        control_points: vec![
            vec![1., 0., 0.],
            vec![0., 1., 0.],
            vec![-1., 0., 0.],
            vec![0., -1., 0.],
            vec![1., 0., 0.],
            vec![0., 1., 0.],
        ],
        weights: if rational {
            vec![1., 0.8, 1.2, 1., 1., 0.8]
        } else {
            vec![1.; 6]
        },
        periodic: true,
    }
}
#[test]
#[cfg(feature = "codec")]
fn rebuilds_periodic_degrees_with_wrapped_controls_and_bounded_error() {
    for rational in [false, true] {
        for degree in 1..=4 {
            let c = source(rational);
            let result = rebuild_curve(&c, degree, 12 + degree, 0.2, None).unwrap();
            assert_eq!(
                result["certificate"]["accepted"],
                json!(true),
                "degree={degree} rational={rational}: {result:?}"
            );
            let rebuilt: Curve = value_codec::from_value(result["curve"].clone()).unwrap();
            rebuilt.validate().unwrap();
            assert!(rebuilt.periodic);
            assert_eq!(rebuilt.control_points.len(), 12 + degree);
            let bound = result["certificate"]["hausdorffErrorUpper"]
                .as_f64()
                .unwrap();
            for i in 0..=2000 {
                let u = 2. + 4. * i as f64 / 2000.;
                assert!(
                    distance(
                        &c.evaluate(u).unwrap().point,
                        &rebuilt.evaluate(u).unwrap().point
                    ) <= bound
                );
            }
            let first = rebuilt.evaluate(2.).unwrap();
            let last = rebuilt.evaluate(6.).unwrap();
            assert!(distance(&first.point, &last.point) < 1e-12);
            if degree >= 2 {
                assert!(distance(&first.d1.unwrap(), &last.d1.unwrap()) < 1e-11);
            }
            if degree >= 3 {
                assert!(distance(&first.d2.unwrap(), &last.d2.unwrap()) < 1e-10);
            }
        }
    }
}
#[test]
#[cfg(feature = "codec")]
fn rollback_retains_periodic_source_and_invalid_counts_fail() {
    let c = source(true);
    let result = rebuild_curve(&c, 1, 5, 0., None).unwrap();
    assert_eq!(result["certificate"]["accepted"], json!(false));
    assert_eq!(result["curve"], value_codec::to_value(&c).unwrap());
    assert!(rebuild_curve(&c, 3, 6, 1., None).is_err());
    assert!(rebuild_curve(&c, 2, 257, 1., None).is_err());
}
