#[cfg(feature = "codec")]
use value_codec::json;

use super::*;
fn torus() -> Surface {
    let points = (0..6)
        .map(|i| {
            (0..6)
                .map(|j| {
                    let u = (i % 4) as f64 * std::f64::consts::FRAC_PI_2;
                    let v = (j % 4) as f64 * std::f64::consts::FRAC_PI_2;
                    vec![
                        (2. + 0.5 * v.cos()) * u.cos(),
                        (2. + 0.5 * v.cos()) * u.sin(),
                        0.5 * v.sin(),
                    ]
                })
                .collect()
        })
        .collect();
    Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: (0..9).map(|i| i as f64).collect(),
        knots_v: (0..9).map(|i| i as f64).collect(),
        control_points: points,
        weights: (0..6)
            .map(|i| {
                (0..6)
                    .map(|j| [1., 0.8, 1.2, 1.][i % 4] * [1., 0.9, 1.1, 1.][j % 4])
                    .collect()
            })
            .collect(),
        periodic_u: true,
        periodic_v: true,
    }
}
#[test]
#[cfg(feature = "codec")]
fn mixed_periodic_and_open_axes_preserve_cylinder_seam() {
    let base = torus();
    let cylinder = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: base.knots_u.clone(),
        knots_v: vec![0., 0., 1., 1.],
        control_points: base
            .control_points
            .iter()
            .map(|row| vec![row[0].clone(), vec![row[0][0], row[0][1], 3.]])
            .collect(),
        weights: base.weights.iter().map(|row| vec![row[0]; 2]).collect(),
        periodic_u: true,
        periodic_v: false,
    };
    for axis in [Axis::U, Axis::V] {
        let count = if matches!(axis, Axis::U) { 15 } else { 6 };
        let budget = if matches!(axis, Axis::U) { 0.2 } else { 0.001 };
        let result = rebuild_surface(&cylinder, axis, 3, count, budget, None).unwrap();
        assert_eq!(result["certificate"]["accepted"], json!(true), "{result:?}");
        let target: Surface = value_codec::from_value(result["surface"].clone()).unwrap();
        assert!(target.periodic_u && !target.periodic_v);
        let bound = result["certificate"]["hausdorffErrorUpper"]
            .as_f64()
            .unwrap();
        for i in 0..=32 {
            for j in 0..=16 {
                let u = 2. + 4. * i as f64 / 32.;
                let v = j as f64 / 16.;
                assert!(
                    distance(
                        &cylinder.evaluate(u, v).unwrap().point,
                        &target.evaluate(u, v).unwrap().point
                    ) <= bound
                );
            }
        }
    }
}
#[test]
#[cfg(feature = "codec")]
fn both_periodic_axes_keep_wrapping_and_certify_dense_error() {
    let source = torus();
    source.validate().unwrap();
    for axis in [Axis::U, Axis::V] {
        let value = rebuild_surface(&source, axis, 3, 15, 0.2, None).unwrap();
        assert_eq!(value["certificate"]["accepted"], json!(true), "{value:?}");
        let target: Surface = value_codec::from_value(value["surface"].clone()).unwrap();
        target.validate().unwrap();
        assert!(target.periodic_u && target.periodic_v);
        let bound = value["certificate"]["hausdorffErrorUpper"]
            .as_f64()
            .unwrap();
        for i in 0..=32 {
            for j in 0..=32 {
                let u = 2. + 4. * i as f64 / 32.;
                let v = 2. + 4. * j as f64 / 32.;
                assert!(
                    distance(
                        &source.evaluate(u, v).unwrap().point,
                        &target.evaluate(u, v).unwrap().point
                    ) <= bound
                );
            }
        }
        for i in 0..=32 {
            let t = 2. + 4. * i as f64 / 32.;
            assert!(
                distance(
                    &target.evaluate(2., t).unwrap().point,
                    &target.evaluate(6., t).unwrap().point
                ) < 1e-12
            );
            assert!(
                distance(
                    &target.evaluate(t, 2.).unwrap().point,
                    &target.evaluate(t, 6.).unwrap().point
                ) < 1e-12
            );
            for (a, b) in [
                (
                    target.evaluate(2., t).unwrap(),
                    target.evaluate(6., t).unwrap(),
                ),
                (
                    target.evaluate(t, 2.).unwrap(),
                    target.evaluate(t, 6.).unwrap(),
                ),
            ] {
                let (au, av) = a.first_derivatives().unwrap();
                let (bu, bv) = b.first_derivatives().unwrap();
                assert!(distance(&au, &bu) < 1e-11);
                assert!(distance(&av, &bv) < 1e-11);
            }
        }
        let refused = rebuild_surface(&source, axis, 1, 5, 0., None).unwrap();
        assert_eq!(refused["certificate"]["accepted"], json!(false));
        assert_eq!(refused["surface"], value_codec::to_value(&source).unwrap());
    }
}
