use nurbs_core::{
    curve::Curve,
    skin_radius,
    surface::{Axis, Surface},
};
fn control(r: [f64; 2]) -> (Surface, Curve, Curve) {
    let weights = [1., 0.5_f64.sqrt(), 1.];
    let directions = [[1., 0.], [1., 1.], [0., 1.]];
    let skin = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..2)
                    .map(|j| vec![r[j] * directions[i][0], r[j] * directions[i][1], j as f64])
                    .collect()
            })
            .collect(),
        weights: weights.iter().map(|w| vec![*w; 2]).collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let center = Curve::from_polyline(vec![vec![0., 0., 0.], vec![0., 0., 1.]]).unwrap();
    let radius = Curve::from_polyline(vec![vec![r[0], 0., 0.], vec![r[1], 0., 0.]]).unwrap();
    (skin, center, radius)
}
#[test]
fn whole_rational_skin_confirms_constant_and_linear_radius_laws() {
    for r in [[0.1, 0.1], [0.1, 0.3]] {
        let (skin, center, radius) = control(r);
        let saved = skin.clone();
        let proof = skin_radius::inspect(&skin, &center, &radius, Axis::V, 1e-7, 2000000).unwrap();
        assert!(
            proof.within_tolerance,
            "upper={:?}",
            proof.radial_error_upper_mm
        );
        assert!(proof.coefficient_work <= 2000000);
        println!(
            "law={r:?} work={} error={:?}",
            proof.coefficient_work, proof.radial_error_upper_mm
        );
        let mut transposed = skin.clone();
        std::mem::swap(&mut transposed.degree_u, &mut transposed.degree_v);
        std::mem::swap(&mut transposed.knots_u, &mut transposed.knots_v);
        transposed.control_points = (0..skin.control_points[0].len())
            .map(|j| {
                skin.control_points
                    .iter()
                    .map(|row| row[j].clone())
                    .collect()
            })
            .collect();
        transposed.weights = (0..skin.weights[0].len())
            .map(|j| skin.weights.iter().map(|row| row[j]).collect())
            .collect();
        assert!(
            skin_radius::inspect(&transposed, &center, &radius, Axis::U, 1e-7, 2000000)
                .unwrap()
                .within_tolerance
        );
        let mut nonunit_center = center.clone();
        for k in &mut nonunit_center.knots {
            *k = 2. + 3. * *k;
        }
        let mut nonunit_radius = radius.clone();
        for k in &mut nonunit_radius.knots {
            *k = -3. + 7. * *k;
        }
        assert!(
            skin_radius::inspect(
                &skin,
                &nonunit_center,
                &nonunit_radius,
                Axis::V,
                1e-7,
                2000000
            )
            .unwrap()
            .within_tolerance
        );
        let scarce = skin_radius::inspect(
            &skin,
            &center,
            &radius,
            Axis::V,
            1e-7,
            proof.coefficient_work - 1,
        )
        .unwrap();
        assert!(!scarce.within_tolerance && scarce.radial_error_upper_mm.is_none());
        for u in [0., 0.13, 0.51, 0.9, 1.] {
            for v in [0., 0.21, 0.8, 1.] {
                let p = skin.evaluate(u, v).unwrap().point;
                assert!((p[0].hypot(p[1]) - (r[0] + v * (r[1] - r[0]))).abs() < 1e-14);
            }
        }
        assert_eq!(skin, saved);
        let mut damaged = skin.clone();
        damaged.control_points[1][1][0] += 0.01;
        let p = damaged.evaluate(0.5, 1.).unwrap().point;
        assert!((p[0].hypot(p[1]) - r[1]).abs() > 0.001);
        assert!(
            !skin_radius::inspect(&damaged, &center, &radius, Axis::V, 1e-7, 2000000)
                .unwrap()
                .within_tolerance
        );
        let other =
            Curve::from_polyline(vec![vec![r[0] + 0.01, 0., 0.], vec![r[1] + 0.01, 0., 0.]])
                .unwrap();
        assert!(
            !skin_radius::inspect(&skin, &center, &other, Axis::V, 1e-7, 2000000)
                .unwrap()
                .within_tolerance
        );
    }
}

#[test]
fn actual_multispan_rolling_ball_skin_has_certified_radius_on_all_original_cells() {
    use nurbs_core::offset::fillet_surfaces::{FilletOptions, rolling_ball_fillet};
    let plane = |u: [f64; 3], v: [f64; 3]| Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..2)
            .map(|i| {
                (0..2)
                    .map(|j| {
                        (0..3)
                            .map(|k| {
                                [2., 3., 4.][k] + 10. * i as f64 * u[k] + 10. * j as f64 * v[k]
                            })
                            .collect()
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let a = plane([1., 0., 0.], [0., 1., 0.]);
    let b = plane([0., 1., 0.], [0., 0., 1.]);
    let built = rolling_ball_fillet(&a, &b, 1.25, &FilletOptions::default()).unwrap();
    assert!(built.failure.is_none());
    let skin = built.surface.unwrap();
    let center = [[0., 0., 0., 3.25], [0., 1., 0., 0.], [0., 0., 0., 5.25]];
    let radius = [0., 0., 0., 1.25];
    let proof = skin_radius::inspect_affine_field(&skin, center, radius, 1e-7, 2000000).unwrap();
    let mut sampled = 0_f64;
    for i in 0..101 {
        for j in 0..101 {
            let p = skin
                .evaluate(i as f64 / 100., j as f64 / 100.)
                .unwrap()
                .point;
            sampled = sampled.max(((p[0] - 3.25).hypot(p[2] - 5.25) - 1.25).abs());
        }
    }
    println!("diagnostic sampled radial error={sampled}");
    println!(
        "actual skin: degree=({},{}), controls=({},{}), work={} error={:?}",
        skin.degree_u,
        skin.degree_v,
        skin.control_points.len(),
        skin.control_points[0].len(),
        proof.coefficient_work,
        proof.radial_error_upper_mm
    );
    assert!(proof.within_tolerance);
    let scarce =
        skin_radius::inspect_affine_field(&skin, center, radius, 1e-7, proof.coefficient_work - 1)
            .unwrap();
    assert!(!scarce.within_tolerance && scarce.radial_error_upper_mm.is_none());
    let mut damaged = skin.clone();
    let last = damaged.control_points[0].len() - 1;
    damaged.control_points[1][last][0] += 0.01;
    assert!(
        !skin_radius::inspect_affine_field(&damaged, center, radius, 1e-7, 2000000)
            .unwrap()
            .within_tolerance
    );
    use nurbs_core::offset::fillet_surfaces::{RadiusLaw, variable_radius_fillet};
    let variable = variable_radius_fillet(
        &a,
        &b,
        RadiusLaw::Linear {
            start: 0.1,
            end: 0.3,
        },
        &FilletOptions::default(),
    )
    .unwrap();
    assert!(variable.failure.is_none());
    // Specify the complete immutable station correspondence from endpoint Y.
    // The proof receives these coefficients, never the builder's radius receipt.
    let y0 = variable.spine.first().unwrap().center[1];
    let y1 = variable.spine.last().unwrap().center[1];
    let slope = (0.3 - 0.1) / (y1 - y0);
    let intercept = 0.1 - slope * y0;
    let field = [0., slope, 0., intercept];
    let centers = [
        [0., slope, 0., 2. + intercept],
        [0., 1., 0., 0.],
        [0., slope, 0., 4. + intercept],
    ];
    let variable_skin = variable.surface.unwrap();
    let confirmed =
        skin_radius::inspect_affine_field(&variable_skin, centers, field, 1e-7, 2000000).unwrap();
    println!(
        "actual variable skin: work={} error={:?}",
        confirmed.coefficient_work, confirmed.radial_error_upper_mm
    );
    assert!(confirmed.within_tolerance);
}
