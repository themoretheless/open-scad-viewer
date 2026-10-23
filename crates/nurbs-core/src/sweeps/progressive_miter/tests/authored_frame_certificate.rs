use super::*;
fn law(points: Vec<Vec<f64>>, domain: [f64; 2]) -> Curve {
    Curve {
        degree: 1,
        knots: vec![domain[0], domain[0], domain[1], domain[1]],
        weights: vec![1.; points.len()],
        control_points: points,
        periodic: false,
    }
}
#[test]
fn rotating_frame_has_normalized_traversal_jets_and_shared_budget() {
    let a = law(vec![vec![1., 0., 0.], vec![1., 1., 0.]], [2., 5.]);
    let b = law(vec![vec![0., 0., 2.], vec![0., 0., 2.]], [7., 9.]);
    let before = (a.clone(), b.clone());
    let r = certify(&a, &b, [0., 1.], 6).unwrap();
    assert_eq!(r.status, Status::Certified);
    assert_eq!(r.cells, 6);
    for i in 0..=20 {
        let f = i as f64 / 20.;
        let h = (1. + f * f).sqrt();
        let v = [1. / h, f / h, 0.];
        let d = [-f / h.powi(3), 1. / h.powi(3), 0.];
        let dd = [(2. * f * f - 1.) / h.powi(5), -3. * f / h.powi(5), 0.];
        let t = r.longitudinal.as_ref().unwrap();
        for k in 0..3 {
            for (range, x) in [(t.value[k], v[k]), (t.first[k], d[k]), (t.second[k], dd[k])] {
                assert!(range[0] <= x && x <= range[1], "{range:?} misses {x}");
            }
        }
    }
    for budget in [0, 3, 5] {
        let r = certify(&a, &b, [0., 1.], budget).unwrap();
        assert_eq!(r.status, Status::Unresolved);
        assert!(r.cells <= budget);
        assert!(r.longitudinal.is_none() && r.transverse.is_none() && r.binormal.is_none());
    }
    assert_eq!(a.control_points, before.0.control_points);
    assert_eq!(b.knots, before.1.knots);
}
#[test]
fn rational_frame_encloses_all_axes_and_chain_rule_derivatives() {
    let mut a = law(vec![vec![1., 0., 0.], vec![1., 1., 0.]], [2., 5.]);
    a.weights = vec![1., 0.5];
    let b = law(vec![vec![0., 0., 2.], vec![0., 0., 2.]], [7., 9.]);
    let r = certify(&a, &b, [0., 1.], 6).unwrap();
    assert_eq!(r.status, Status::Certified);
    for i in 0..=20 {
        let f = i as f64 / 20.;
        let denominator = 1. - 0.5 * f;
        let g = 0.5 * f / denominator;
        let gd = 0.5 / denominator.powi(2);
        let gdd = 0.5 / denominator.powi(3);
        let h = (1. + g * g).sqrt();
        let v = [1. / h, g / h, 0.];
        let d = [-g / h.powi(3) * gd, gd / h.powi(3), 0.];
        let dd = [
            (2. * g * g - 1.) / h.powi(5) * gd * gd - g / h.powi(3) * gdd,
            -3. * g / h.powi(5) * gd * gd + gdd / h.powi(3),
            0.,
        ];
        for (axis, values, first, second) in [
            (r.longitudinal.as_ref().unwrap(), v, d, dd),
            (
                r.transverse.as_ref().unwrap(),
                [0., 0., 1.],
                [0.; 3],
                [0.; 3],
            ),
            (
                r.binormal.as_ref().unwrap(),
                [v[1], -v[0], 0.],
                [d[1], -d[0], 0.],
                [dd[1], -dd[0], 0.],
            ),
        ] {
            for k in 0..3 {
                for (range, x) in [
                    (axis.value[k], values[k]),
                    (axis.first[k], first[k]),
                    (axis.second[k], second[k]),
                ] {
                    assert!(range[0] <= x && x <= range[1], "{range:?} misses {x}");
                }
            }
        }
    }
}
#[test]
fn twist_uses_independent_domain_and_shared_frame_budget() {
    let a = law(vec![vec![0., 0., 1.], vec![0., 0., 1.]], [2., 5.]);
    let b = law(vec![vec![1., 0., 0.], vec![1., 0., 0.]], [7., 9.]);
    let mut twist = law(vec![vec![0., 0., 0.], vec![1., 0., 0.]], [11., 15.]);
    twist.weights = vec![1., 0.5];
    let r = certify_twisted(&a, &b, &twist, [0., 1.], 7).unwrap();
    assert_eq!(r.status, Status::Certified);
    assert_eq!(r.cells, 7);
    for i in 0..=20 {
        let f = i as f64 / 20.;
        let denominator = 1. - 0.5 * f;
        let theta = 0.5 * f / denominator;
        let td = 0.5 / denominator.powi(2);
        let tdd = 0.5 / denominator.powi(3);
        let (s, c) = theta.sin_cos();
        let values = certify_twisted_values(&a, &b, &twist, [f, f], 7).unwrap();
        assert_eq!(values.status, Status::Certified);
        assert_eq!(values.cells, 7);
        for (range, x) in values.transverse.unwrap().into_iter().zip([c, s, 0.]) {
            assert!(range[0] <= x && x <= range[1]);
        }
        let v = [c, s, 0.];
        let d = [-s * td, c * td, 0.];
        let dd = [-c * td * td - s * tdd, -s * td * td + c * tdd, 0.];
        for (axis, values, first, second) in [
            (r.transverse.as_ref().unwrap(), v, d, dd),
            (
                r.binormal.as_ref().unwrap(),
                [-v[1], v[0], 0.],
                [-d[1], d[0], 0.],
                [-dd[1], dd[0], 0.],
            ),
            (
                r.longitudinal.as_ref().unwrap(),
                [0., 0., 1.],
                [0.; 3],
                [0.; 3],
            ),
        ] {
            for k in 0..3 {
                for (range, x) in [
                    (axis.value[k], values[k]),
                    (axis.first[k], first[k]),
                    (axis.second[k], second[k]),
                ] {
                    assert!(range[0] <= x && x <= range[1], "{range:?} misses {x}");
                }
            }
        }
    }
    let refused = certify_twisted(&a, &b, &twist, [0., 1.], 6).unwrap();
    assert_eq!(refused.status, Status::Unresolved);
    assert_eq!(refused.cells, 6);
    assert!(
        refused.longitudinal.is_none()
            && refused.transverse.is_none()
            && refused.binormal.is_none()
    );
    assert_eq!(refused.reason, Some("twist-law-enclosure-unresolved"));
    let values = certify_twisted_values(&a, &b, &twist, [0., 0.], 6).unwrap();
    assert_eq!(values.status, Status::Unresolved);
    assert_eq!(values.cells, 6);
    assert!(values.transverse.is_none() && values.binormal.is_none());
}
#[test]
fn guide_projection_jets_follow_the_original_rail_and_shared_budget() {
    let guide = law(vec![vec![1., 0., 0.], vec![1., 1., 10.]], [7., 9.]);
    let twist = law(vec![vec![0., 0., 0.]; 2], [2., 5.]);
    let tangent = [[0., 0.], [0., 0.], [1., 1.]];
    let path = [[0., 0.], [0., 0.], [0., 10.]];
    let velocity = [[0., 0.], [0., 0.], [10., 10.]];
    let r = certify_guide(&guide, &twist, [0., 1.], tangent, path, velocity, 4).unwrap();
    assert_eq!(r.status, Status::Certified);
    assert_eq!(r.cells, 4);
    let n = r.transverse.as_ref().unwrap();
    for i in 0..=20 {
        let f = i as f64 / 20.;
        let h = (1. + f * f).sqrt();
        let position = [[0., 0.], [0., 0.], [10. * f, 10. * f]];
        let values =
            certify_guide_values(&guide, &twist, [f, f], tangent, position, 4).unwrap();
        assert_eq!(values.status, Status::Certified);
        assert_eq!(values.cells, 4);
        for (range, x) in values
            .transverse
            .unwrap()
            .into_iter()
            .zip([1. / h, f / h, 0.])
        {
            assert!(range[0] <= x && x <= range[1]);
        }

        let v = [1. / h, f / h, 0.];
        let d = [-f / h.powi(3), 1. / h.powi(3), 0.];
        let dd = [(2. * f * f - 1.) / h.powi(5), -3. * f / h.powi(5), 0.];
        for k in 0..3 {
            for (range, x) in [(n.value[k], v[k]), (n.first[k], d[k]), (n.second[k], dd[k])] {
                assert!(range[0] <= x && x <= range[1]);
            }
        }
    }
    let r = certify_guide(&guide, &twist, [0., 1.], tangent, path, velocity, 3).unwrap();
    assert_eq!(r.status, Status::Unresolved);
    assert_eq!(r.cells, 3);
    assert!(r.transverse.is_none());
    let crossing = law(vec![vec![1., 0., 0.], vec![-1., 0., 10.]], [7., 9.]);
    let r = certify_guide(&crossing, &twist, [0., 1.], tangent, path, velocity, 4).unwrap();
    assert_eq!(r.reason, Some("guide-transverse-direction-unproved"));
    assert!(r.longitudinal.is_none());
}
#[test]
fn singular_and_parallel_directions_refuse_without_partial_frame() {
    let a = law(vec![vec![1., 0., 0.], vec![-1., 0., 0.]], [0., 1.]);
    let b = law(vec![vec![0., 1., 0.], vec![0., 1., 0.]], [0., 1.]);
    let r = certify(&a, &b, [0., 1.], 6).unwrap();
    assert_eq!(r.reason, Some("longitudinal-nonzero-unproved"));
    assert!(r.longitudinal.is_none());
    let a = law(vec![vec![1., 0., 0.], vec![1., 0., 0.]], [0., 1.]);
    let r = certify(&a, &a, [0., 1.], 6).unwrap();
    assert_eq!(r.reason, Some("transverse-nonparallel-unproved"));
    assert_eq!(r.status, Status::Unresolved);
}

#[test]
fn interior_singularity_between_preview_stations_is_not_a_regular_frame() {
    // z(t)=(t-3/8)^2: every quarter-grid station is nonzero, while
    // the original polynomial has an exact zero between those stations.
    let axis = Curve {
        degree: 2,
        knots: vec![2., 2., 2., 10., 10., 10.],
        control_points: vec![
            vec![0., 0., 9. / 64.],
            vec![0., 0., -15. / 64.],
            vec![0., 0., 25. / 64.],
        ],
        weights: vec![1.; 3],
        periodic: false,
    };
    let normal = law(vec![vec![1., 0., 0.]; 2], [31., 41.]);
    for k in 0..=4 {
        assert!(axis.evaluate(2. + 2. * k as f64).unwrap().point[2] > 0.);
    }
    assert_eq!(axis.evaluate(5.).unwrap().point, vec![0., 0., 0.]);
    let report = certify(&axis, &normal, [0., 1.], 100).unwrap();
    assert_eq!(report.status, Status::Unresolved);
    assert_eq!(report.reason, Some("longitudinal-nonzero-unproved"));
    assert!(report.longitudinal.is_none() && report.binormal.is_none());

    // A nonzero transverse vector can likewise become parallel to the
    // fixed axis between the same preview stations.
    let mut transverse = axis.clone();
    for pole in &mut transverse.control_points {
        pole[0] = pole[2];
        pole[2] = 1.;
    }
    let fixed_axis = law(vec![vec![0., 0., 1.]; 2], [7., 9.]);
    let report = certify(&fixed_axis, &transverse, [0., 1.], 100).unwrap();
    assert_eq!(report.status, Status::Unresolved);
    assert_eq!(report.reason, Some("transverse-nonparallel-unproved"));
    assert!(report.transverse.is_none() && report.binormal.is_none());
    let whole = certify_regularity(&fixed_axis, &transverse, 100).unwrap();
    assert_eq!(whole.status, Status::Unresolved);
    assert!(whole.cells <= 100);
}

#[test]
fn whole_frame_regularity_subdivides_original_laws_with_shared_work() {
    let axis = law(vec![vec![1., 0., 0.], vec![-1., 1., 0.]], [2., 5.]);
    let normal = law(vec![vec![0., 0., 1.]; 2], [31., 41.]);
    assert_eq!(certify(&axis, &normal, [0., 1.], 100).unwrap().status, Status::Unresolved);
    let whole = certify_regularity(&axis, &normal, 1000).unwrap();
    assert_eq!(whole.status, Status::Certified);
    assert!(whole.certified_intervals > 1);
    assert!(whole.cells <= 1000);
    let cover = whole.intervals.as_ref().unwrap();
    assert_eq!(cover.len(), whole.certified_intervals);
    assert_eq!(cover.first().unwrap().traversal[0], 0.);
    assert_eq!(cover.last().unwrap().traversal[1], 1.);
    for pair in cover.windows(2) {
        assert_eq!(pair[0].traversal[1], pair[1].traversal[0]);
    }
    let limited = certify_regularity(&axis, &normal, whole.cells - 1).unwrap();
    assert_eq!(limited.status, Status::Unresolved);
    assert!(limited.cells < whole.cells);
    assert!(limited.intervals.is_none());
    assert_eq!(certify_regularity(&axis, &normal, 0).unwrap().status, Status::Unresolved);
    #[cfg(feature="transport")]
    for budget in [0, 1000] {
        let request = value_codec::json!({
            "op": "sweep_authored_frame_regularity",
            "longitudinal": axis,
            "transverse": normal,
            "maxCells": budget
        });
        let result = crate::transport::dispatch(request).unwrap();
        assert_eq!(result["regularityCertified"], value_codec::json!(budget > 0));
        assert_eq!(result["continuousBound"], value_codec::json!(false));
    }
}

#[test]
fn twisted_cover_retains_normalized_second_jets_and_shared_work() {
    let axis = law(vec![vec![0., 0., 1.]; 2], [2., 5.]);
    let normal = law(vec![vec![1., 0., 0.]; 2], [31., 41.]);
    let twist = law(vec![vec![0., 0., 0.], vec![1., 0., 0.]], [17., 19.]);
    let cover = certify_twisted_cover(&axis, &normal, &twist, 100).unwrap();
    assert_eq!(cover.status, Status::Certified);
    assert_eq!(cover.cells, 7);
    let intervals = cover.intervals.unwrap();
    assert_eq!(intervals.len(), 1);
    let jet = &intervals[0].transverse;
    for k in 0..=16 {
        let t = k as f64 / 16.;
        let (s, c) = t.sin_cos();
        for (bounds, expected) in [
            (jet.value, [c, s, 0.]),
            (jet.first, [-s, c, 0.]),
            (jet.second, [-c, -s, 0.]),
        ] {
            for i in 0..3 {
                assert!(bounds[i][0] <= expected[i] && expected[i] <= bounds[i][1]);
            }
        }
    }
    for budget in [0, 6] {
        let refused = certify_twisted_cover(&axis, &normal, &twist, budget).unwrap();
        assert_eq!(refused.status, Status::Unresolved);
        assert!(refused.cells <= budget);
        assert!(refused.intervals.is_none());
    }
}
