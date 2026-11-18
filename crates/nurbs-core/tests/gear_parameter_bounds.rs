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

#[test]
fn mounting_tolerance_encloses_continuous_signed_backlash() {
    use involute::{SpurMesh, spur_backlash_over_mounting_range as range};
    let a = 20_f64.to_radians();
    for (mesh, teeth, sign) in [
        (SpurMesh::External, [20, 40], 1.),
        (SpurMesh::Internal, [20, 80], -1.),
    ] {
        let bounds = range(1., teeth, 20., [0.03, 0.02], mesh, [29.8, 30.3]).unwrap();
        for i in 0..=100 {
            let d = 29.8 + 0.5 * f64::from(i) / 100.;
            let w = (30. * a.cos() / d).acos();
            contains(
                bounds,
                d / 30. * 0.05 + sign * 2. * d * ((w.tan() - w) - (a.tan() - a)),
            );
        }
        assert!(bounds[1] - bounds[0] < 0.5);
        let fixed = range(1., teeth, 20., [0.03, 0.02], mesh, [30.; 2]).unwrap();
        contains(fixed, 0.05);
        assert!(fixed[1] - fixed[0] < 1e-9);
        for invalid in [[31., 30.], [30., 40.], [0., 30.]] {
            assert!(range(1., teeth, 20., [0.; 2], mesh, invalid).is_err());
        }
    }
}

#[test]
fn contact_tolerance_captures_an_interior_peak_and_ring_overlap() {
    use involute::{SpurMesh, spur_contact_ratio_over_mounting_range as range};
    let cosine = 20_f64.to_radians().cos();
    let base = 4. * cosine;
    let extent = (25. - base * base).sqrt();
    let peak = extent / (std::f64::consts::PI * cosine);
    let bounds = range(1., [8, 8], 20., SpurMesh::External, [7.8, 8.4]).unwrap();
    contains(bounds, peak);
    assert!(bounds[1] - peak < 1e-9);
    for d in [7.8, 8.4] {
        let span = (d * d - 4. * base * base).sqrt();
        let end = (extent - (span - extent).abs()) / (std::f64::consts::PI * cosine);
        contains(bounds, end);
        assert!(end < peak - 0.05);
    }
    let ring = range(1., [20, 80], 20., SpurMesh::Internal, [30., 30.2]).unwrap();
    for i in 0..=20 {
        let d = 30. + f64::from(i) * 0.01;
        let b =
            involute::mounted_spur_contact_ratio_bounds(1., [20, 80], 20., SpurMesh::Internal, d)
                .unwrap();
        contains(ring, (b[0] + b[1]) / 2.);
    }
    assert!(range(1., [8, 8], 20., SpurMesh::External, [7., 8.]).is_err());
    assert!(range(1., [8, 8], 20., SpurMesh::External, [8., 12.]).is_err());
}

#[test]
fn inverse_contact_window_brackets_both_limiting_tangent_spans() {
    use involute::{SpurMesh, mounting_window_for_contact_ratio as window};
    let cosine = 20_f64.to_radians().cos();
    let base = 4. * cosine;
    let flank = (25. - base * base).sqrt();
    let length = std::f64::consts::PI * cosine;
    let [lo, hi] = window(1., [8, 8], 20., SpurMesh::External, 1.).unwrap();
    let first = (2. * base).hypot(length);
    let last = (2. * base).hypot(2. * flank - length);
    assert!(lo >= first && lo - first < 1e-10 && hi <= last && last - hi < 1e-10);
    for mesh in [SpurMesh::Internal, SpurMesh::External] {
        let teeth = if matches!(mesh, SpurMesh::Internal) {
            [20, 80]
        } else {
            [20, 40]
        };
        let [a, b] = window(1., teeth, 20., mesh, 1.25).unwrap();
        for d in [a + 1e-6, (a + b) / 2., b - 1e-6] {
            assert!(
                involute::mounted_spur_contact_ratio_bounds(1., teeth, 20., mesh, d).unwrap()[0]
                    >= 1.25
            );
        }
    }
    assert!(window(1., [8, 8], 20., SpurMesh::External, 2.).is_err());
    assert!(window(1., [80, 20], 20., SpurMesh::Internal, 1.).is_err());
}

#[test]
fn independent_carrier_pin_tolerances_enclose_planet_envelopes() {
    use involute::planet_tip_gap_bounds as gap;
    let b = gap(1., [20, 16], [[30., 31.], [29., 30.5]], [39., 41.]).unwrap();
    for a in [30., 30.5, 31.] {
        for r in [29., 30., 30.5] {
            for deg in [39_f64, 40., 41.] {
                let t = deg.to_radians();
                contains(b, (a - r * t.cos()).hypot(r * t.sin()) - 20.);
            }
        }
    }
    for (deg, expected) in [(0., -18.), (180., 40.)] {
        let exact = gap(1., [20, 16], [[31.; 2], [29.; 2]], [deg; 2]).unwrap();
        contains(exact, expected);
        assert!(exact[1] - exact[0] < 1e-9);
    }
    assert!(gap(1., [20, 16], [[0.; 2], [30.; 2]], [40.; 2]).is_err());
    assert!(gap(1., [20, 16], [[30.; 2]; 2], [40., 181.]).is_err());
}

#[test]
fn spinner_layout_search_combines_integer_phase_and_envelope_clearance() {
    use involute::planetary_layout_counts as layouts;
    assert_eq!(layouts(1., 32, 4, 0.).unwrap(), [2, 3, 4, 6, 8, 9, 12, 18]);
    assert_eq!(layouts(1., 32, 4, 4.).unwrap(), [2, 3, 4, 6, 8, 9]);
    assert_eq!(layouts(2., 32, 4, 8.).unwrap(), [2, 3, 4, 6, 8, 9]);
    assert!(layouts(1., 32, 4, 31.).unwrap().is_empty());
    assert!(layouts(1., usize::MAX, 4, 0.).is_err());
    assert!(layouts(1., 200, 40, 0.).is_err());
    assert!(layouts(1., 32, 4, f64::NAN).is_err());
}

#[test]
fn spinner_bore_stock_and_diameter_limit_include_eccentricity() {
    use involute::external_gear_bore_stock as stock;
    // The spinner's 32-tooth sun has pitch radius 24 and root radius 22.4 mm.
    for (offset, eccentricity) in [([0.,0.],0.), ([3.,-4.],5.), ([24.,7.],25.)] {
        let report = stock(1.5,32,0.1,offset,10.,0.375).unwrap();
        contains(report.root_web_mm, 22.4-eccentricity-5.);
        contains(report.diameter_limit_mm, 2.*(22.4-eccentricity-0.375));
        assert!(report.root_web_mm[1]-report.root_web_mm[0] < 1e-10);
    }
    assert!(stock(1.5,32,0.1,[0.,f64::NAN],10.,0.).is_err());
    assert!(stock(1.5,32,0.1,[0.;2],-1.,0.).is_err());
}

#[test]
fn spinner_ring_bolt_circle_distinguishes_all_three_ligaments() {
    use involute::ring_gear_bolt_stock as stock;
    // Ring root radius = 30 + 1.5 + 0.1 mm. Six holes give a chord equal to the radius.
    let report = stock(1.5,40,0.1,38.,35.,4.,6).unwrap();
    contains(report.root_web_mm, 1.4);
    contains(report.outer_web_mm, 1.);
    contains(report.adjacent_web_mm, 31.);
    assert!(stock(1.5,40,0.1,38.,33.,4.,6).unwrap().root_web_mm[1] < 0.);
    assert!(stock(1.5,40,0.1,38.,37.,4.,6).unwrap().outer_web_mm[1] < 0.);
    assert!(stock(1.5,40,0.1,38.,35.,4.,60).unwrap().adjacent_web_mm[1] < 0.);
    contains(stock(1.5,40,0.1,38.,35.,4.,2).unwrap().adjacent_web_mm, 66.);
    assert!(stock(1.5,40,0.1,38.,35.,4.,1).is_err());
    assert!(stock(1.5,40,0.1,38.,35.,f64::NAN,6).is_err());
}

#[test]
fn ring_bolt_window_handles_both_lower_limits_and_empty_layouts() {
    use involute::{BoltRadiusWindow::*, ring_gear_bolt_radius_window as window};
    for holes in [6, 48] {
        let Feasible([lo, hi]) = window(1.5,40,0.1,45.,4.,holes,1.).unwrap() else { panic!("expected feasible rim") };
        let adjacent = 5. / (2. * (std::f64::consts::PI / holes as f64).sin());
        let lower = 34.6_f64.max(adjacent);
        assert!(lo >= lower && lo-lower < 1e-10 && hi <= 42. && 42.-hi < 1e-10);
    }
    assert_eq!(window(1.5,40,0.1,38.,4.,60,1.).unwrap(), Infeasible);
    assert_eq!(window(1.,40,0.,25.,2.,6,1.).unwrap(), Unresolved);
    assert!(window(1.,40,0.,40.,2.,6,-1.).is_err());
}

#[test]
fn circular_profile_instances_preserve_rational_shape_with_bounded_rotation_error() {
    use involute::CircularProfilePattern;
    let mut seed = nurbs_core::curve::Curve::from_polyline(vec![vec![4.,1.,3.],vec![6.,2.,4.]]).unwrap();
    seed.weights[1] = 2.;
    let pattern = CircularProfilePattern::new(seed.clone(),32).unwrap();
    let (quarter,error) = pattern.instance(8).unwrap();
    assert_eq!(quarter.control_points, vec![vec![-1.,4.,3.],vec![-2.,6.,4.]]);
    assert_eq!(error,0.);
    for i in [1, 7, 31] {
        let (copy,bound) = pattern.instance(i).unwrap();
        assert_eq!(copy.weights,seed.weights);
        assert_eq!(copy.knots,seed.knots);
        let (s,c) = (std::f64::consts::TAU * i as f64 / 32.).sin_cos();
        for (a,b) in seed.control_points.iter().zip(copy.control_points) {
            assert!((b[0]-(c*a[0]-s*a[1])).hypot(b[1]-(s*a[0]+c*a[1])) <= bound);
            assert_eq!(b[2],a[2]);
        }
        assert!(bound < 1e-9);
    }
    assert!(pattern.instance(32).is_err());
    assert!(CircularProfilePattern::new(seed,0).is_err());
}

#[test]
fn compresses_ordered_profile_segments_and_bounds_actual_reconstruction() {
    use involute::CircularProfilePattern as Pattern;
    let seed = nurbs_core::curve::Curve::from_polyline(vec![vec![3.,1.,0.], vec![4.,2.,0.]]).unwrap();
    let original = Pattern::new(seed, 7).unwrap();
    let mut segments: Vec<_> = (0..7).map(|i| original.instance(i).unwrap().0).collect();
    let (compressed, error) = Pattern::compress(&segments,0.).unwrap().unwrap();
    assert_eq!(error,0.);
    for (i,segment) in segments.iter().enumerate() {
        assert_eq!(compressed.instance(i).unwrap().0.control_points,segment.control_points);
    }
    segments[3].control_points[1][2] = 0.125;
    assert!(Pattern::compress(&segments,0.).unwrap().is_none());
    let (_, error) = Pattern::compress(&segments,0.126).unwrap().unwrap();
    assert!((0.125..=0.126).contains(&error));
    segments[3].weights[1] = 2.;
    assert!(Pattern::compress(&segments,1.).unwrap().is_none());
    assert!(Pattern::compress(&[],0.).is_err());
}
