use nurbs_core::rack::{self, Spec};
#[test]
fn rack_has_exact_closed_segments_pitch_spacing_and_pressure_flanks() {
    for teeth in [1, 12, 50] {
        let spec = Spec {
            teeth,
            module: 2.,
            backlash: 0.1,
            addendum: 2.,
            dedendum: 2.5,
            backing_height: 3.,
            ..Spec::default()
        };
        let p = rack::profile(spec).unwrap();
        assert_eq!(p.boundary.degree, 1);
        assert_eq!(p.boundary.control_points.len(), 4 * teeth + 5);
        assert_eq!(
            p.boundary.control_points.first(),
            p.boundary.control_points.last()
        );
        assert!(!p.boundary.periodic);
        for pair in p.tooth_centers.windows(2) {
            assert!((pair[1] - pair[0] - p.pitch).abs() < 1e-12);
        }
        let cp = &p.boundary.control_points;
        let mut area = 0.;
        for pair in cp.windows(2) {
            area += pair[0][0] * pair[1][1] - pair[1][0] * pair[0][1];
        }
        assert!(area > 0.);
        let expected_area = p.width * spec.backing_height
            + teeth as f64
                * (spec.addendum + spec.dedendum)
                * (p.tooth_thickness_at_pitch
                    + (spec.dedendum - spec.addendum) * spec.pressure_angle_radians.tan());
        assert!((area / 2. - expected_area).abs() < 1e-9);
        for i in 0..teeth {
            let a = &cp[3 + 4 * i];
            let b = &cp[4 + 4 * i];
            let dx = a[0] - b[0];
            let dy = b[1] - a[1];
            assert!((dx / dy - spec.pressure_angle_radians.tan()).abs() < 1e-12);
            // Independently intersect both linear flanks with pitch line y=0.
            let left = &cp[6 + 4 * i];
            let top_left = &cp[5 + 4 * i];
            let right_x = a[0] + (b[0] - a[0]) * (-a[1]) / (b[1] - a[1]);
            let left_x = left[0] + (top_left[0] - left[0]) * (-left[1]) / (top_left[1] - left[1]);
            assert!((right_x - left_x - (std::f64::consts::PI * 2. / 2. - 0.1)).abs() < 1e-12);
        }
        // Degree-one NURBS evaluates exactly the authored polygon segments
        // up to evaluation rounding; no spline fit or hidden mesh is involved.
        for (i, pair) in cp.windows(2).enumerate() {
            let v = p.boundary.evaluate(i as f64 + 0.5).unwrap().point;
            for k in 0..3 {
                assert!((v[k] - (pair[0][k] + pair[1][k]) / 2.).abs() < 1e-12);
            }
        }
    }
}
#[test]
fn rack_refuses_collapsed_tips_overlap_nonfinite_and_invalid_dimensions() {
    let cases = [
        Spec {
            teeth: 0,
            ..Spec::default()
        },
        Spec {
            teeth: 51,
            ..Spec::default()
        },
        Spec {
            module: 0.,
            ..Spec::default()
        },
        Spec {
            backlash: -0.1,
            ..Spec::default()
        },
        Spec {
            backlash: 2.,
            ..Spec::default()
        },
        Spec {
            addendum: 10.,
            ..Spec::default()
        },
        Spec {
            dedendum: 10.,
            ..Spec::default()
        },
        Spec {
            backing_height: 0.,
            ..Spec::default()
        },
        Spec {
            pressure_angle_radians: f64::NAN,
            ..Spec::default()
        },
        Spec {
            module: 1e9,
            ..Spec::default()
        },
    ];
    for spec in cases {
        assert!(rack::profile(spec).is_err(), "{spec:?}");
    }
}
