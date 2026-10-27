use nurbs_core::{
    curve::Curve,
    surface::{Axis, Surface},
    surface_collar, surface_edit,
    surface_join::Decision,
};
fn original() -> Surface {
    let s = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                [0., 1.]
                    .iter()
                    .map(|&v| vec![i as f64 / 2., v, v + if i == 2 { 0.25 } else { 0. }])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 2], vec![0.7; 2], vec![1.; 2]],
        periodic_u: false,
        periodic_v: false,
    };
    let s = surface_edit::insert(&s, Axis::U, 0.5, 1).unwrap();
    surface_edit::insert(&s, Axis::V, 0.5, 1).unwrap()
}
#[test]
fn original_multispan_rational_coefficients_admit_collar_without_extracted_reference() {
    let s = original();
    let before = s.clone();
    let pc = Curve::from_polyline(vec![vec![0.1, 0.2], vec![0.3, 0.3]]).unwrap();
    let direction = [0.01, 0.02];
    let out = surface_collar::construct(&s, &pc, direction, 1e-7, 2000000).unwrap();
    let proof = surface_collar::inspect_g2(
        &s,
        &pc,
        direction,
        &out.surface,
        [1e-7, 1e-5, 1e-4],
        100000,
        2000000,
    )
    .unwrap();
    assert_eq!(proof.decision, Decision::WithinTolerance);
    for t in [0., 0.13, 0.5, 0.81, 1.] {
        for v in [0., 0.37, 1.] {
            let u = 0.1 + 0.2 * t + v * direction[0];
            let y = 0.2 + 0.1 * t + v * direction[1];
            let w = 1. - 0.6 * u + 0.6 * u * u;
            let expected = [(0.7 * u + 0.3 * u * u) / w, y, y + 0.25 * u * u / w];
            let actual = out.surface.evaluate(t, v).unwrap().point;
            assert!(
                actual.iter().zip(expected).all(
                    |(a, b)| (a - b).abs() <= out.position_error_upper_mm + 512. * f64::EPSILON
                )
            );
        }
    }
    let mut broken = out.surface.clone();
    for row in &mut broken.control_points {
        row[2][2] += 0.01;
    }
    let invalid = surface_collar::inspect_g2(
        &s,
        &pc,
        direction,
        &broken,
        [1e-7, 1e-5, 1e-4],
        100000,
        2000000,
    )
    .unwrap();
    assert_ne!(invalid.decision, Decision::WithinTolerance);
    assert_eq!(s, before);
}
#[test]
fn crossing_original_knot_interval_is_refused() {
    let pc = Curve::from_polyline(vec![vec![0.4, 0.2], vec![0.6, 0.3]]).unwrap();
    assert!(surface_collar::construct(&original(), &pc, [0., 0.01], 1e-7, 2000000).is_err());
}

#[test]
fn exact_collar_endpoint_on_an_original_knot_is_not_a_crossing() {
    let pc = Curve::from_polyline(vec![vec![0.125, 0.25], vec![0.25, 0.25]]).unwrap();
    let out = surface_collar::construct(&original(), &pc, [0.25, 0.25], 1e-7, 2000000)
        .unwrap();
    assert!(out.position_error_upper_mm <= 1e-7);
}
