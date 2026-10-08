use nurbs_core::{curve::Curve, engineering_profiles as profile};
fn area(c: &Curve) -> f64 {
    let origin = &c.control_points[0];
    c.control_points
        .windows(2)
        .map(|e| {
            let a = [e[0][0] - origin[0], e[0][1] - origin[1]];
            let b = [e[1][0] - origin[0], e[1][1] - origin[1]];
            a[0] * b[1] - a[1] * b[0]
        })
        .sum::<f64>()
        / 2.
}
fn check_profile(c: &Curve, expected_area: f64, expected_perimeter: f64, edges: usize) {
    assert_eq!(c.degree, 1);
    assert_eq!(c.control_points.len(), edges + 1);
    assert_eq!(c.control_points.first(), c.control_points.last());
    assert_eq!(c.domain(), [0., 1.]);
    assert!(!c.periodic);
    assert!((area(c) - expected_area).abs() < 1e-10);
    let l = nurbs_core::curve_measure::length(c, 1e-8, edges).unwrap();
    assert!(l.within_tolerance);
    assert!(
        l.bounds[0] <= expected_perimeter && expected_perimeter <= l.bounds[1],
        "{l:?}"
    );
    let wall = nurbs_core::surface::extrude(c, [0., 0., 7.]).unwrap();
    let a = nurbs_core::surface_measure::area(&wall, 1e-7, edges).unwrap();
    assert!(a.within_tolerance, "{a:?}");
    assert!(
        a.bounds[0] <= 7. * expected_perimeter && 7. * expected_perimeter <= a.bounds[1],
        "{a:?}"
    );
}
#[test]
fn keyway_dimensions_area_and_extruded_wall() {
    let c = profile::keyway([7., 2., 3.], 6., 5.).unwrap();
    check_profile(&c, 30., 22., 4);
    for i in 0..=400 {
        let p = c.evaluate(i as f64 / 400.).unwrap().point;
        let x = p[0] - 7.;
        let y = p[1] - 2.;
        assert!(x >= -3. - 1e-12 && x <= 3. + 1e-12 && y >= -5. - 1e-12 && y <= 1e-12);
        assert!((p[2] - 3.).abs() < 1e-12);
    }
}
#[test]
fn t_slot_piecewise_width_area_and_extruded_wall() {
    let c = profile::t_slot([7., 2., 3.], 4., 10., 3., 8.).unwrap();
    check_profile(&c, 62., 36., 8);
    for i in 0..=800 {
        let p = c.evaluate(i as f64 / 800.).unwrap().point;
        let x = p[0] - 7.;
        let y = p[1] - 2.;
        assert!(y >= -8. - 1e-12 && y <= 1e-12);
        let width = if y > -3. + 1e-12 { 2. } else { 5. };
        assert!(x.abs() <= width + 1e-12);
    }
}
#[test]
fn dovetail_slope_area_and_extruded_wall() {
    let c = profile::dovetail([7., 2., 3.], 4., 10., 3.).unwrap();
    check_profile(&c, 21., 14. + 2. * 3_f64.hypot(3.), 4);
    for i in 0..=400 {
        let p = c.evaluate(i as f64 / 400.).unwrap().point;
        let x = p[0] - 7.;
        let y = p[1] - 2.;
        assert!(y >= -3. - 1e-12 && y <= 1e-12);
        assert!(x.abs() <= 2. - y + 1e-12);
    }
}
#[test]
fn malformed_and_unrepresentable_dimensions_are_refused() {
    assert!(profile::keyway([0.; 3], 0., 5.).is_err());
    assert!(profile::keyway([0.; 3], 6., -1.).is_err());
    assert!(profile::keyway([f64::NAN, 0., 0.], 6., 5.).is_err());
    assert!(profile::t_slot([0.; 3], 4., 4., 3., 8.).is_err());
    assert!(profile::t_slot([0.; 3], 4., 10., 3., 3.).is_err());
    assert!(profile::t_slot([0.; 3], 4., 10., f64::INFINITY, 8.).is_err());
    assert!(profile::dovetail([0.; 3], 4., 3., 2.).is_err());
    assert!(profile::dovetail([0.; 3], 4., 10., f64::NAN).is_err());
    assert!(profile::keyway([1e8, 0., 0.], 1e-12, 1.).is_err());
    assert!(profile::dovetail([0., 1e8, 0.], 4., 10., 1e-12).is_err());
    assert!(profile::t_slot([1e8, 0., 0.], 10., 10_f64.next_up(), 3., 8.).is_err());
    assert!(profile::keyway([0.; 3], f64::from_bits(1), 1.).is_err());
}

#[test]
fn fastener_wrench_sizes_and_walls() {
    let square = profile::square_fastener([7., 2., 3.], 6.).unwrap();
    check_profile(&square, 36., 24., 4);
    let hex = profile::hex_fastener([7., 2., 3.], 6.).unwrap();
    check_profile(&hex, 18. * 3_f64.sqrt(), 12. * 3_f64.sqrt(), 6);
    // Independent supporting-line distances verify all six wrench flats.
    for edge in hex.control_points.windows(2) {
        let a = [edge[0][0] - 7., edge[0][1] - 2.];
        let b = [edge[1][0] - 7., edge[1][1] - 2.];
        let distance = (a[0] * b[1] - a[1] * b[0]).abs()
            / ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
        assert!((distance - 3.).abs() < 1e-12);
    }
}
#[test]
fn fastener_invalid_and_collapsed_dimensions() {
    for make in [profile::square_fastener, profile::hex_fastener] {
        for size in [0., -1., f64::NAN, f64::INFINITY] {
            assert!(make([0.; 3], size).is_err());
        }
        assert!(make([f64::NAN, 0., 0.], 1.).is_err());
        assert!(make([1e8, 1e8, 0.], 1e-12).is_err());
    }
}

#[test]
fn v_belt_rib_dimensions_area_and_wall() {
    for ribs in [1, 3, 7] {
        let pitch = 4_f64;
        let back = 2_f64;
        let height = 3_f64;
        let width = ribs as f64 * pitch;
        let c = profile::poly_v_belt([7., 2., 3.], ribs, pitch, back, height).unwrap();
        let perimeter = width + 2. * back + 2. * ribs as f64 * (pitch / 2.).hypot(height);
        check_profile(
            &c,
            width * back + width * height / 2.,
            perimeter,
            2 * ribs + 3,
        );
        // Sample the independently specified triangular lower envelope.
        for i in 0..=1000 {
            let p = c.evaluate(i as f64 / 1000.).unwrap().point;
            let x = p[0] - 7. + width / 2.;
            let y = p[1] - 2.;
            let phase = x.rem_euclid(pitch) / pitch;
            let lower = -back - height * (1. - (2. * phase - 1.).abs());
            assert!(x >= -1e-12 && x <= width + 1e-12);
            assert!(y <= 1e-12 && y >= lower - 1e-12);
            assert!(
                (y.abs() < 1e-12)
                    || ((y - lower).abs() < 1e-12)
                    || x.abs() < 1e-12
                    || (x - width).abs() < 1e-12
            );
        }
    }
}
#[test]
fn v_belt_budget_and_invalid_geometry() {
    let c = profile::poly_v_belt([0.; 3], 126, 1., 2., 3.).unwrap();
    assert_eq!(c.control_points.len(), 256);
    for ribs in [0, 127, usize::MAX] {
        assert!(profile::poly_v_belt([0.; 3], ribs, 1., 2., 3.).is_err());
    }
    for dimensions in [
        [0., 2., 3.],
        [1., -2., 3.],
        [1., 2., f64::NAN],
        [f64::INFINITY, 2., 3.],
    ] {
        assert!(
            profile::poly_v_belt([0.; 3], 3, dimensions[0], dimensions[1], dimensions[2]).is_err()
        );
    }
    assert!(profile::poly_v_belt([1e8, 0., 0.], 3, 1e-12, 2., 3.).is_err());
    assert!(profile::poly_v_belt([0., 1e8, 0.], 3, 1., 2., 1e-12).is_err());
}

fn tooth_shape(teeth: usize) -> profile::ToothedBelt {
    profile::ToothedBelt {
        teeth,
        pitch: 6.,
        base_width: 4.,
        tip_width: 2.,
        back_thickness: 2.,
        tooth_height: 3.,
    }
}
#[test]
fn toothed_belt_dimensions_area_perimeter_and_wall() {
    for teeth in [1, 3, 5] {
        let c = profile::toothed_belt([7., 2., 3.], tooth_shape(teeth)).unwrap();
        let n = teeth as f64;
        let area = 12. * n + 9. * n;
        let perimeter = 6. * n + 4. + n * (4. + 2. * 1_f64.hypot(3.));
        check_profile(&c, area, perimeter, 4 * teeth + 4);
        for i in 0..=1000 {
            let p = c.evaluate(i as f64 / 1000.).unwrap().point;
            let x = p[0] - 7. + 3. * n;
            let y = p[1] - 2.;
            let d = (x.rem_euclid(6.) - 3.).abs();
            let lower = if d <= 1. {
                -5.
            } else if d < 2. {
                -5. + 3. * (d - 1.)
            } else {
                -2.
            };
            assert!(x >= -1e-12 && x <= 6. * n + 1e-12);
            assert!(y >= lower - 1e-12 && y <= 1e-12);
            assert!(
                y.abs() < 1e-12
                    || (y - lower).abs() < 1e-12
                    || x.abs() < 1e-12
                    || (x - 6. * n).abs() < 1e-12
            );
        }
    }
}
#[test]
fn toothed_belt_resource_and_precision_limits() {
    let c = profile::toothed_belt([0.; 3], tooth_shape(62)).unwrap();
    assert_eq!(c.control_points.len(), 253);
    for teeth in [0, 63, usize::MAX] {
        assert!(profile::toothed_belt([0.; 3], tooth_shape(teeth)).is_err());
    }
    let base = tooth_shape(3);
    for invalid in [
        profile::ToothedBelt { pitch: 4., ..base },
        profile::ToothedBelt {
            tip_width: 4.,
            ..base
        },
        profile::ToothedBelt {
            tip_width: 0.,
            ..base
        },
        profile::ToothedBelt {
            tooth_height: f64::NAN,
            ..base
        },
        profile::ToothedBelt {
            back_thickness: -1.,
            ..base
        },
        profile::ToothedBelt {
            base_width: f64::INFINITY,
            ..base
        },
    ] {
        assert!(profile::toothed_belt([0.; 3], invalid).is_err());
    }
    assert!(profile::toothed_belt([f64::NAN, 0., 0.], base).is_err());
    assert!(profile::toothed_belt(
        [0., 1e8, 0.],
        profile::ToothedBelt {
            tooth_height: 1e-12,
            ..base
        }
    )
    .is_err());
    assert!(profile::toothed_belt(
        [1e8, 0., 0.],
        profile::ToothedBelt {
            tip_width: 1e-12,
            ..base
        }
    )
    .is_err());
    assert!(profile::toothed_belt(
        [1e8, 0., 0.],
        profile::ToothedBelt {
            base_width: 2. + 1e-12,
            ..base
        }
    )
    .is_err());
}

#[test]
fn o_ring_groove_circular_corners_and_perimeter() {
    let c = profile::o_ring_groove([7., 2., 3.], 8., 5., 1.).unwrap();
    assert_eq!(c.degree, 2);
    assert_eq!(c.domain(), [0., 1.]);
    near_point(
        &c.evaluate(0.).unwrap().point,
        &c.evaluate(1.).unwrap().point,
    );
    for i in 0..=1200 {
        let p = c.evaluate(i as f64 / 1200.).unwrap().point;
        let x = p[0] - 7.;
        let y = p[1] - 2.;
        assert!(x >= -4. - 1e-12 && x <= 4. + 1e-12 && y >= -5. - 1e-12 && y <= 1e-12);
        let boundary = if y < -4. && x.abs() > 3. {
            ((x.abs() - 3.).hypot(y + 4.) - 1.).abs()
        } else {
            y.abs().min((y + 5.).abs()).min((x.abs() - 4.).abs())
        };
        assert!(boundary < 1e-11, "{p:?}");
        assert!((p[2] - 3.).abs() < 1e-12);
    }
    let exact = 22. + std::f64::consts::PI;
    let length = nurbs_core::curve_measure::length(&c, 1e-4, 8192).unwrap();
    assert!(length.within_tolerance, "{length:?}");
    assert!(length.bounds[0] <= exact && exact <= length.bounds[1]);
    let wall = nurbs_core::surface::extrude(&c, [0., 0., 2.]).unwrap();
    let area = nurbs_core::surface_measure::area(&wall, 1e-3, 16384).unwrap();
    assert!(area.within_tolerance, "{area:?}");
    assert!(area.bounds[0] <= 2. * exact && 2. * exact <= area.bounds[1]);
}
fn near_point(a: &[f64], b: &[f64]) {
    for (x, y) in a.iter().zip(b) {
        assert!((x - y).abs() < 1e-12);
    }
}
#[test]
fn o_ring_groove_rejects_invalid_and_lost_corners() {
    for (width, depth, radius) in [
        (2., 5., 1.),
        (8., 1., 1.),
        (8., 5., 0.),
        (8., 5., f64::NAN),
        (f64::INFINITY, 5., 1.),
    ] {
        assert!(profile::o_ring_groove([0.; 3], width, depth, radius).is_err());
    }
    assert!(profile::o_ring_groove([1e8, 0., 0.], 8., 5., 1e-12).is_err());
    assert!(profile::o_ring_groove([0., 1e8, 0.], 8., 5., 1e-12).is_err());
}

#[test]
fn retaining_ring_annular_outline_and_perimeter() {
    let c = profile::retaining_ring([7., 2., 3.], 3., 5., 60.).unwrap();
    assert_eq!(c.degree, 2);
    assert_eq!(c.domain(), [0., 1.]);
    near_point(
        &c.evaluate(0.).unwrap().point,
        &c.evaluate(1.).unwrap().point,
    );
    for i in 0..=1200 {
        let p = c.evaluate(i as f64 / 1200.).unwrap().point;
        let x = p[0] - 7.;
        let y = p[1] - 2.;
        let r = x.hypot(y);
        assert!(r >= 3. - 1e-11 && r <= 5. + 1e-11);
        let angle = y.atan2(x).abs();
        assert!(angle >= std::f64::consts::PI / 6. - 1e-11);
        assert!(
            (r - 3.).abs() < 1e-11
                || (r - 5.).abs() < 1e-11
                || (angle - std::f64::consts::PI / 6.).abs() < 1e-11
        );
    }
    let exact = 8. * 5. * std::f64::consts::PI / 3. + 4.;
    let m = nurbs_core::curve_measure::length(&c, 1e-3, 16384).unwrap();
    assert!(m.within_tolerance, "{m:?}");
    assert!(m.bounds[0] <= exact && exact <= m.bounds[1]);
}
#[test]
fn retaining_ring_refuses_invalid_and_unrepresentable_sections() {
    for (inner, outer, gap) in [
        (0., 5., 60.),
        (5., 3., 60.),
        (3., 5., 0.),
        (3., 5., 360.),
        (3., 5., f64::NAN),
    ] {
        assert!(profile::retaining_ring([0.; 3], inner, outer, gap).is_err());
    }
    assert!(profile::retaining_ring([1e8, 1e8, 0.], 3., 3. + 1e-12, 60.).is_err());
    assert!(profile::retaining_ring([1e8, 1e8, 0.], 3., 5., 1e-12).is_err());
}
