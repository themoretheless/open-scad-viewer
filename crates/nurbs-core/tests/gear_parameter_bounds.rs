use nurbs_core::involute::{self, derive_gear_parameter_bounds as derive};
fn contains(bound: [f64; 2], value: f64) {
    assert!(
        bound[0] <= value && value <= bound[1],
        "{value} outside {bound:?}"
    )
}
#[test]
fn full_parameter_boxes_enclose_independent_nominal_gear_equations() {
    for (module, teeth, pressure, internal) in [
        (1., 16, 20., false),
        (2.5, 40, 20., true),
        (0.1, 100, 35., false),
        (100., 100, 10., false),
    ] {
        let p = derive(
            module,
            teeth,
            pressure,
            0.25 * module,
            0.02 * module,
            internal,
        )
        .unwrap();
        let alpha: f64 = pressure * std::f64::consts::PI / 180.;
        let pitch = module * teeth as f64 / 2.;
        let base = pitch * alpha.cos();
        let (tip, root) = if internal {
            (pitch - module, pitch + 1.25 * module)
        } else {
            (pitch + module, pitch - 1.25 * module)
        };
        contains(p.pitch_radius, pitch);
        contains(p.base_radius, base);
        contains(p.tip_radius, tip);
        contains(p.root_radius, root);
        let low = if internal { tip } else { root };
        let high = if internal { root } else { tip };
        contains(
            p.roll_start,
            ((low.max(base) / base).powi(2) - 1.).max(0.).sqrt(),
        );
        contains(p.roll_end, ((high / base).powi(2) - 1.).sqrt());
        let half = std::f64::consts::PI / (2. * teeth as f64)
            + if internal {
                0.02 * module / (2. * pitch)
            } else {
                -0.02 * module / (2. * pitch)
            }
            + alpha.tan()
            - alpha;
        contains(p.half_base_angle, half);
        contains(p.tooth_step, std::f64::consts::TAU / teeth as f64);
    }
}
#[test]
fn parameter_family_certificate_bounds_actual_curve_against_all_box_corners() {
    let curve = involute::approximate_quintic([0.; 3], 20., 0.1, 0.7, 1e-6)
        .unwrap()
        .curve;
    let radius = [20. - 1e-8, 20. + 1e-8];
    let start = [0.1 - 1e-8, 0.1 + 1e-8];
    let end = [0.7 - 1e-8, 0.7 + 1e-8];
    let r =
        involute::certify_quintic_parameter_box(&curve, [0.; 3], radius, start, end, [0., 0.], 1.)
            .unwrap();
    assert!(r.rounding_included);
    for rb in radius {
        for a in start {
            for b in end {
                for i in 0..=50 {
                    let u = i as f64 / 50.;
                    let t = a + (b - a) * u;
                    let rho = rb * t.hypot(1.);
                    let phi = t - t.atan();
                    let p = curve.evaluate(u).unwrap().point;
                    assert!(
                        (p[0] - rho * phi.cos()).hypot(p[1] - rho * phi.sin())
                            <= r.error_upper + 1e-12
                    );
                }
            }
        }
    }
}
#[test]
fn invalid_parameter_derivations_return_errors_without_point_panics() {
    let curve = involute::approximate_quintic([0.; 3], 20., 0.1, 0.7, 1e-6)
        .unwrap()
        .curve;
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(involute::certify_quintic(&curve, [0.; 3], bad, 0.1, 0.7).is_err());
        assert!(involute::certify_quintic(&curve, [0.; 3], 20., bad, 0.7).is_err());
        assert!(involute::certify_quintic(&curve, [0.; 3], 20., 0.1, bad).is_err());
        assert!(
            involute::certify_quintic_parameter_box(
                &curve,
                [0.; 3],
                [20., bad],
                [0.1; 2],
                [0.7; 2],
                [0.; 2],
                1.
            )
            .is_err()
        );
    }
    for (m, n, p, c, b) in [
        (f64::NAN, 16, 20., 0., 0.),
        (1., 0, 20., 0., 0.),
        (1., 16, 90., 0., 0.),
        (1., 16, 20., f64::NAN, 0.),
        (1., 16, 20., 0., f64::INFINITY),
        (1., 16, 20., -1., 0.),
    ] {
        assert!(derive(m, n, p, c, b, false).is_err());
    }
}

#[test]
fn external_spur_pair_active_contact_ratio() {
    use involute::external_spur_contact_ratio_bounds as ratio;
    let a = 20_f64.to_radians();
    let expected = (((11_f64).powi(2) - (10. * a.cos()).powi(2)).sqrt()
        + ((21_f64).powi(2) - (20. * a.cos()).powi(2)).sqrt()
        - 30. * a.sin())
        / (std::f64::consts::PI * a.cos());
    for (module, teeth) in [(1., [20, 40]), (3., [40, 20])] {
        let bound = ratio(module, teeth, 20.).unwrap();
        contains(bound, expected);
        assert!(bound[1] - bound[0] < 1e-9);
    }
    // For two small gears the active involutes both terminate at their bases.
    contains(
        ratio(1., [8, 8], 20.).unwrap(),
        8. * a.tan() / std::f64::consts::PI,
    );
    assert!(ratio(1., [20, usize::MAX], 20.).is_err());
}

#[test]
fn internal_spur_contact_uses_ring_tip_and_difference_of_pitch_radii() {
    use involute::internal_spur_contact_ratio_bounds as ratio;
    let alpha = 20_f64.to_radians();
    let tip_roll_length =
        |pitch: f64, tip: f64| (tip.powi(2) - (pitch * alpha.cos()).powi(2)).sqrt();
    let expected = (tip_roll_length(10., 11.) - tip_roll_length(40., 39.) + 30. * alpha.sin())
        / (std::f64::consts::PI * alpha.cos());
    for module in [0.25, 1., 4.] {
        let bounds = ratio(module, 20, 80, 20.).unwrap();
        contains(bounds, expected);
        assert!(bounds[1] - bounds[0] < 1e-9);
    }
    assert!(ratio(1., 20, 20, 20.).is_err());
    assert!(ratio(1., 21, 20, 20.).is_err());
    assert!(ratio(1., 20, usize::MAX, 20.).is_err());
}

#[test]
fn mounted_spur_mesh_uses_actual_line_of_action() {
    use involute::{SpurMesh, mounted_spur_contact_ratio_bounds as ratio};
    let a = 20_f64.to_radians();
    let tip = |r: f64, t: f64| (t.powi(2) - (r * a.cos()).powi(2)).sqrt();
    for (mesh, teeth, d, projected) in [
        (
            SpurMesh::External,
            [20, 40],
            30.3_f64,
            tip(10., 11.) + tip(20., 21.),
        ),
        (
            SpurMesh::Internal,
            [20, 80],
            30.1_f64,
            tip(10., 11.) - tip(40., 39.),
        ),
    ] {
        let tangent = d * ((30. * a.cos() / d).acos()).sin();
        let expected = match mesh {
            SpurMesh::External => projected - tangent,
            // For this mounting the ring tip passes the pinion base tangent
            // point. The pinion involute cannot extend below its base.
            SpurMesh::Internal => (projected + tangent).min(tip(10., 11.)),
        } / (std::f64::consts::PI * a.cos());
        let bound = ratio(1., teeth, 20., mesh, d).unwrap();
        contains(bound, expected);
        assert!(bound[1] - bound[0] < 1e-9);
    }
    for distance in [0., f64::NAN, 25., 40.] {
        assert!(ratio(1., [20, 40], 20., SpurMesh::External, distance).is_err());
    }
    assert!(ratio(1., [40, 20], 20., SpurMesh::Internal, 10.).is_err());
}

#[test]
fn tip_to_root_margins_distinguish_both_roots_and_mesh_directions() {
    use involute::{SpurMesh, spur_tip_root_clearance_bounds as margins};
    for (mesh, teeth, distance) in [
        (SpurMesh::External, [20, 40], 29.75),
        (SpurMesh::Internal, [20, 80], 30.25),
    ] {
        let [first, second] = margins(1., teeth, [0.125, 0.5], mesh, distance).unwrap();
        contains(first, 0.25);
        contains(second, -0.125);
        assert!(first[0] > 0. && second[1] < 0.);
        for bound in margins(1., teeth, [0.; 2], mesh, 30.).unwrap() {
            contains(bound, 0.);
            assert!(bound[1] - bound[0] < 1e-10);
        }
    }
    assert!(margins(1., [20, 40], [-0.1, 0.], SpurMesh::External, 30.).is_err());
    assert!(margins(1., [40, 20], [0.; 2], SpurMesh::Internal, 10.).is_err());
    assert!(margins(1., [20, 40], [0.; 2], SpurMesh::External, f64::NAN).is_err());
}

#[test]
fn tooth_caliper_dimensions_follow_the_actual_measurement_circle() {
    use involute::spur_tooth_chord_bounds as measure;
    let pitch = 20.;
    let a = 20_f64.to_radians();
    for (radius, backlash) in [(pitch, 0.), (20.8, 0.125), (19.5, 0.25)] {
        let result = measure(2., 20, 20., 0.5, backlash, radius).unwrap();
        let working = (pitch * a.cos() / radius).acos();
        let angle = std::f64::consts::PI / 40. - backlash / (2. * pitch) + (a.tan() - a)
            - (working.tan() - working);
        contains(result.chord_length, 2. * radius * angle.sin());
        contains(result.depth_from_tip, 22. - radius * angle.cos());
        assert!(result.chord_length[1] - result.chord_length[0] < 1e-9);
        assert!(result.depth_from_tip[1] - result.depth_from_tip[0] < 1e-9);
    }
    for radius in [18., 23., f64::NAN] {
        assert!(measure(2., 20, 20., 0.5, 0., radius).is_err());
    }
}

#[test]
fn caliper_uncertainty_encloses_pitch_circle_tooth_thinning() {
    use involute::spur_tooth_thinning_bounds as infer;
    let width = |thinning: f64| 40. * (std::f64::consts::PI / 40. - thinning / 40.).sin();
    let bound = infer(2., 20, 20., 0.5, 20., [width(0.3), width(0.1)]).unwrap();
    contains(bound, 0.1);
    contains(bound, 0.3);
    assert!(bound[1] - bound[0] < 0.200000001);
    let chord = involute::spur_tooth_chord_bounds(2., 20, 20., 0.5, 0.42, 19.7).unwrap();
    contains(
        infer(2., 20, 20., 0.5, 19.7, chord.chord_length).unwrap(),
        0.42,
    );
    assert_eq!(
        infer(2., 20, 20., 0.5, 20., [0.1, 39.99]).unwrap(),
        [0., 1.]
    );
    for invalid in [[4., 4.1], [0.1, 0.2], [3., 2.], [0., 1.], [2., f64::NAN]] {
        assert!(infer(2., 20, 20., 0.5, 20., invalid).is_err());
    }
}

#[test]
fn mounting_backlash_matches_reference_and_working_circle_equations() {
    use involute::mounted_spur_backlash_bounds as gap;
    let a = 20_f64.to_radians();
    for d in [29.8, 30., 30.3] {
        let w = (30. * a.cos() / d).acos();
        for allowances in [[0., 0.], [0.125, 0.25]] {
            let expected = d / 30. * (allowances[0] + allowances[1])
                + 2. * d * ((w.tan() - w) - (a.tan() - a));
            let bound = gap(1., [20, 40], 20., allowances, d).unwrap();
            contains(bound, expected);
            assert!(bound[1] - bound[0] < 1e-9);
        }
    }
    assert!(gap(1., [20, 40], 20., [0.; 2], 29.8).unwrap()[1] < 0.);
    assert!(gap(1., [20, 40], 20., [0.; 2], 40.).is_err());
    assert!(gap(1., [usize::MAX, 40], 20., [0.; 2], 30.).is_err());
}

#[test]
fn mounted_ring_backlash_uses_ring_space_minus_pinion_tooth() {
    use involute::mounted_internal_spur_backlash_bounds as gap;
    let a = 20_f64.to_radians();
    for d in [29.8, 30., 30.3] {
        let w = (30. * a.cos() / d).acos();
        for allowances in [[0., 0.], [0.125, 0.25]] {
            let expected = d / 30. * (allowances[0] + allowances[1])
                - 2. * d * ((w.tan() - w) - (a.tan() - a));
            let bound = gap(1., [20, 80], 20., allowances, d).unwrap();
            contains(bound, expected);
            assert!(bound[1] - bound[0] < 1e-9);
        }
    }
    assert!(gap(1., [20, 80], 20., [0.; 2], 30.3).unwrap()[1] < 0.);
    for (teeth, d) in [([80, 20], 30.), ([20, 20], 0.), ([20, 80], 29.)] {
        assert!(gap(1., teeth, 20., [0.; 2], d).is_err());
    }
}
