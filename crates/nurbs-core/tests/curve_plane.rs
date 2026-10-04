use nurbs_core::{
    curve::Curve,
    curve_plane::{self, Classification, Proof, StopReason},
    polynomial,
};
fn coverage(c: &Curve, r: &curve_plane::Report) {
    assert_eq!(r.cells.first().unwrap().domain[0], c.domain()[0]);
    assert_eq!(r.cells.last().unwrap().domain[1], c.domain()[1]);
    for pair in r.cells.windows(2) {
        assert_eq!(pair[0].domain[1], pair[1].domain[0]);
    }
}
fn single_linear_root(r: &curve_plane::Report, expected: f64, tolerance: f64) {
    assert!(r.all_roots_isolated, "{r:?}");
    assert_eq!(r.work, 1);
    assert_eq!(r.cells[0].proof, Some(Proof::DirectLinearFormula));
    let roots: Vec<_> = r
        .cells
        .iter()
        .filter_map(|x| x.root_parameter_bounds())
        .collect();
    assert_eq!(roots.len(), 1);
    assert!(
        roots[0][0] <= expected && expected <= roots[0][1],
        "{roots:?}"
    );
    assert!(roots[0][1] - roots[0][0] <= tolerance);
}
#[test]
fn rational_crossing_uses_original_parameter_and_plane_scale() {
    let c = Curve {
        degree: 1,
        knots: vec![2., 2., 7., 7.],
        control_points: vec![vec![0., 0., 0.], vec![4., 0., 0.]],
        weights: vec![1., 3.],
        periodic: false,
    };
    let before = c.clone();
    let r = curve_plane::inspect(&c, [2., 0., 0.], 2.2, 1e-6, 1000).unwrap();
    coverage(&c, &r);
    assert!(r.all_roots_isolated, "{r:?}");
    let roots: Vec<_> = r
        .cells
        .iter()
        .filter(|x| x.classification == Classification::UniqueRoot)
        .collect();
    assert_eq!(roots.len(), 1);
    // Cx=12q/(1+2q), q=(t-2)/5. Independent quotient oracle.
    let t = 2. + 5. * 1.1 / 9.8;
    let bounds = roots[0].root_parameter_bounds().unwrap();
    assert!(bounds[0] < t && t < bounds[1]);
    assert!(bounds[1] - bounds[0] <= 1e-6);
    assert_eq!(r.work, 1);
    assert_eq!(c, before);
}
#[test]
fn two_crossings_hidden_between_initial_samples_are_both_isolated() {
    let c = polynomial::parametric_curve([0., 1.], &[[0., 0., 0.06], [1., 0., -0.5], [0., 0., 1.]])
        .unwrap();
    let r = curve_plane::inspect(&c, [0., 0., 1.], 0., 1e-5, 2000).unwrap();
    coverage(&c, &r);
    assert!(r.all_roots_isolated, "{r:?}");
    let roots: Vec<_> = r
        .cells
        .iter()
        .filter(|x| x.classification == Classification::UniqueRoot)
        .collect();
    assert_eq!(roots.len(), 2);
    for (cell, t) in roots.iter().zip([0.2, 0.3]) {
        assert!(cell.domain[0] < t && t < cell.domain[1]);
    }
}
#[test]
fn tangency_overlap_endpoint_and_budget_keep_unresolved_coverage() {
    let tangent = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0., 0.25], vec![0.5, 0., -0.25], vec![1., 0., 0.25]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let r = curve_plane::inspect(&tangent, [0., 0., 1.], 0., 1e-4, 1).unwrap();
    coverage(&tangent, &r);
    assert!(r.all_roots_isolated);
    assert_eq!(
        r.cells[0].classification,
        Classification::TangencyAtMidpoint
    );
    assert_eq!(r.cells[0].root_parameter_bounds(), Some([0.5; 2]));
    assert_eq!(r.cells[0].stop_reason, StopReason::ExactTangency);
    let line = Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.]]).unwrap();
    for (normal, offset) in [([1., 0., 0.], 0.3)] {
        let r = curve_plane::inspect(&line, normal, offset, 1e-20, 1).unwrap();
        coverage(&line, &r);
        assert!(!r.all_roots_isolated);
        assert_eq!(r.cells[0].stop_reason, StopReason::WorkLimit);
        assert_eq!(r.work, 1);
    }
    let r = curve_plane::inspect(&line, [0., 0., 1.], 1., 1e-5, 1).unwrap();
    assert!(r.all_roots_isolated);
    assert_eq!(r.cells[0].classification, Classification::Excluded);
}
#[test]
fn oblique_rational_overlap_is_proved_but_rounded_cancellation_is_not() {
    let c = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![1., 2., 3.], vec![2., 4., 6.], vec![3., -1., 2.]],
        weights: vec![1., 2., 3.],
        periodic: false,
    };
    let r = curve_plane::inspect(&c, [1., 1., -1.], 0., 1e-12, 1).unwrap();
    coverage(&c, &r);
    assert!(r.coverage_resolved);
    assert!(!r.all_roots_isolated);
    assert_eq!(r.work, 1);
    assert_eq!(r.cells[0].classification, Classification::Coincident);
    assert_eq!(r.cells[0].root_parameter_bounds(), None);
    assert_eq!(r.cells[0].residual_bounds, [0., 0.]);
    let mut c = c;
    c.control_points = vec![vec![1e9, 2_f64.powi(-30), 1e9]; 3];
    let r = curve_plane::inspect(&c, [1., 1., -1.], 0., 1e-12, 1).unwrap();
    assert_ne!(r.cells[0].classification, Classification::Coincident);
}
#[test]
fn coincidence_is_local_to_the_owning_knot_span() {
    let c = Curve {
        degree: 1,
        knots: vec![2., 2., 4., 7., 7.],
        control_points: vec![vec![0., 0., 0.], vec![1., 0., 0.], vec![2., 0., 1.]],
        weights: vec![1., 2., 3.],
        periodic: false,
    };
    let before = c.clone();
    let r = curve_plane::inspect(&c, [0., 0., 1.], 0., 1e-6, 200).unwrap();
    coverage(&c, &r);
    let overlaps: Vec<_> = r
        .cells
        .iter()
        .filter(|x| x.classification == Classification::Coincident)
        .collect();
    assert_eq!(overlaps.len(), 1);
    assert_eq!(overlaps[0].domain, [2., 4.]);
    assert_eq!(overlaps[0].span, 1);
    assert_eq!(overlaps[0].stop_reason, StopReason::ExactCoincidence);
    assert!(!r.all_roots_isolated);
    assert!(r.coverage_resolved);
    assert_eq!(r.cells[1].classification, Classification::RootAtStart);
    assert_eq!(r.cells[1].domain, [4., 7.]);
    assert!(
        r.cells
            .iter()
            .filter(|x| x.domain[0] >= 4.)
            .all(|x| x.classification != Classification::Coincident)
    );
    assert_eq!(c, before);
}
#[test]
fn monotone_interpolated_endpoints_are_exact_even_with_one_cell() {
    let c = Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 1., 1.]]).unwrap();
    for (offset, expected) in [
        (0., Classification::RootAtStart),
        (6., Classification::RootAtEnd),
    ] {
        let r = curve_plane::inspect(&c, [2., -3., 7.], offset, 1e-12, 1).unwrap();
        assert!(r.all_roots_isolated, "{r:?}");
        assert!(r.coverage_resolved);
        assert_eq!(r.cells[0].classification, expected);
        assert_eq!(r.cells[0].stop_reason, StopReason::ExactEndpoint);
    }
    // Degree two with a C0 interior knot: both sides own the shared endpoint.
    let c = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
        control_points: vec![
            vec![0., 0., -1.],
            vec![0.25, 0., -0.5],
            vec![0.5, 0., 0.],
            vec![0.75, 0., 0.5],
            vec![1., 0., 1.],
        ],
        weights: vec![1.; 5],
        periodic: false,
    };
    let r = curve_plane::inspect(&c, [0., 0., 1.], 0., 1e-12, 2).unwrap();
    coverage(&c, &r);
    assert!(r.all_roots_isolated, "{r:?}");
    assert_eq!(r.cells[0].classification, Classification::RootAtEnd);
    assert_eq!(r.cells[1].classification, Classification::RootAtStart);
}
#[test]
fn noninterpolated_controls_are_not_roots_and_disconnected_curves_are_rejected() {
    let c = Curve {
        degree: 2,
        knots: vec![0., 1., 2., 3., 4., 5.],
        control_points: vec![vec![0., 0., 0.], vec![1., 0., 1.], vec![2., 0., 2.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let r = curve_plane::inspect(&c, [0., 0., 1.], 0., 1e-5, 100).unwrap();
    assert!(r.all_roots_isolated, "{r:?}");
    assert!(
        r.cells
            .iter()
            .all(|x| x.classification == Classification::Excluded)
    );
    let c = Curve {
        degree: 1,
        knots: vec![0., 0., 0.5, 0.5, 1., 1.],
        control_points: vec![
            vec![0., 0., 1.],
            vec![0.5, 0., 0.],
            vec![0.5, 0., 1.],
            vec![1., 0., 2.],
        ],
        weights: vec![1.; 4],
        periodic: false,
    };
    assert!(curve_plane::inspect(&c, [0., 0., 1.], 0., 1e-5, 100).is_err());
}
#[test]
fn direct_linear_bounds_preserve_parameter_and_weight_changes() {
    let c = Curve {
        degree: 1,
        knots: vec![2., 2., 7., 7.],
        control_points: vec![vec![0., 0., 0.], vec![1., 1., 1.]],
        weights: vec![2.; 2],
        periodic: false,
    };
    let r = curve_plane::inspect(&c, [2., -3., 7.], 3., 1e-8, 3).unwrap();
    coverage(&c, &r);
    single_linear_root(&r, 4.5, 1e-8);
    let mut weighted = c;
    weighted.weights[1] = 3.;
    let r = curve_plane::inspect(&weighted, [2., -3., 7.], 3., 1e-8, 1).unwrap();
    // 3q/(2+q)=1/2 gives q=2/5 and source parameter t=4.
    single_linear_root(&r, 4., 1e-8);
}
#[test]
fn direct_rational_bounds_cover_midpoint_and_nonmidpoint_roots() {
    let mut c = Curve {
        degree: 1,
        knots: vec![2., 2., 7., 7.],
        control_points: vec![vec![0., 0., 0.], vec![1., 1., 1.]],
        weights: vec![1., 3.],
        periodic: false,
    };
    let r = curve_plane::inspect(&c, [2., -3., 7.], 4.5, 1e-10, 3).unwrap();
    single_linear_root(&r, 4.5, 1e-10);
    c.weights = vec![1.; 2];
    let r = curve_plane::inspect(&c, [2., -3., 7.], 1.5, 1e-10, 1).unwrap();
    single_linear_root(&r, 3.25, 1e-10);
}
#[test]
fn decreasing_rational_line_is_monotone_for_both_plane_orientations() {
    let c = Curve {
        degree: 1,
        knots: vec![2., 2., 7., 7.],
        control_points: vec![vec![1., 1., 1.], vec![0., 0., 0.]],
        weights: vec![1., 3.],
        periodic: false,
    };
    // C(1/2)=1/4. Changing normal sign preserves the same exact root.
    for (normal, offset) in [([2., -3., 7.], 1.5), ([-2., 3., -7.], -1.5)] {
        let r = curve_plane::inspect(&c, normal, offset, 1e-10, 3).unwrap();
        single_linear_root(&r, 4.5, 1e-10);
    }
}
#[test]
fn oblique_projection_excludes_a_whole_rational_line_with_one_cell() {
    let c = Curve {
        degree: 1,
        knots: vec![2., 2., 7., 7.],
        control_points: vec![vec![0., 0., 1.], vec![2., 1., 2.]],
        weights: vec![1., 3.],
        periodic: false,
    };
    let r = curve_plane::inspect(&c, [1., -1., 1.], 0., 1e-8, 1).unwrap();
    // With q=(t-2)/5, residual = 1+6q/(1+2q), continuously in [1,3].
    assert!(r.all_roots_isolated, "{r:?}");
    assert_eq!(r.work, 1);
    assert_eq!(r.cells[0].classification, Classification::Excluded);
    let [lo, hi] = r.cells[0].residual_bounds;
    assert!(lo > 0. && lo <= 1. && hi >= 3. && hi < 3.000001);
}
#[test]
fn shifted_quadratic_tangency_preserves_parameter_and_plane_orientation() {
    let mut c = Curve {
        degree: 2,
        knots: vec![2., 2., 2., 7., 7., 7.],
        control_points: vec![vec![0., 0., 2.25], vec![0.5, 0., 1.75], vec![1., 0., 2.25]],
        weights: vec![2.; 3],
        periodic: false,
    };
    for (normal, offset) in [([0., 0., 2.], 4.), ([0., 0., -2.], -4.)] {
        let r = curve_plane::inspect(&c, normal, offset, 1e-12, 1).unwrap();
        assert!(r.all_roots_isolated, "{r:?}");
        assert_eq!(
            r.cells[0].classification,
            Classification::TangencyAtMidpoint
        );
        assert_eq!(r.cells[0].root_parameter_bounds(), Some([4.5; 2]));
    }
    c.weights[1] = 4.;
    let r = curve_plane::inspect(&c, [0., 0., 2.], 4., 1e-6, 1000).unwrap();
    assert!(
        !r.cells
            .iter()
            .any(|x| x.classification == Classification::TangencyAtMidpoint)
    );
}
#[test]
fn unequal_quadratic_weights_preserve_a_proved_double_root() {
    let c = Curve {
        degree: 2,
        knots: vec![2., 2., 2., 7., 7., 7.],
        control_points: vec![
            vec![0., 0., 0.25],
            vec![0.5, 0., -0.125],
            vec![1., 0., 0.125],
        ],
        weights: vec![1., 2., 2.],
        periodic: false,
    };
    let r = curve_plane::inspect(&c, [0., 0., 1.], 0., 1e-12, 1).unwrap();
    // Homogeneous numerator is (1-2q)^2/4; W=(1-q)^2+4q(1-q)+2q²>0.
    assert!(r.all_roots_isolated, "{r:?}");
    assert!(r.coverage_resolved);
    assert_eq!(r.work, 1);
    assert_eq!(
        r.cells[0].classification,
        Classification::TangencyAtMidpoint
    );
    assert_eq!(r.cells[0].root_parameter_bounds(), Some([4.5; 2]));
    assert_eq!(r.cells[0].stop_reason, StopReason::ExactTangency);
    let mut shifted = c;
    for p in &mut shifted.control_points {
        p[2] += 2.;
    }
    for (normal, offset) in [([0., 0., 2.], 4.), ([0., 0., -2.], -4.)] {
        let r = curve_plane::inspect(&shifted, normal, offset, 1e-12, 1).unwrap();
        assert!(r.all_roots_isolated, "{r:?}");
        assert_eq!(
            r.cells[0].classification,
            Classification::TangencyAtMidpoint
        );
        assert_eq!(r.cells[0].root_parameter_bounds(), Some([4.5; 2]));
    }
}
#[test]
fn rational_quadratic_endpoint_tangencies_have_exact_double_roots() {
    for (z, expected, t) in [
        ([0., 0., 1.], Classification::TangencyAtStart, 2.),
        ([1., 0., 0.], Classification::TangencyAtEnd, 7.),
    ] {
        let c = Curve {
            degree: 2,
            knots: vec![2., 2., 2., 7., 7., 7.],
            control_points: vec![
                vec![0., 0., z[0] + 2.],
                vec![0.5, 0., z[1] + 2.],
                vec![1., 0., z[2] + 2.],
            ],
            weights: vec![1., 2., 3.],
            periodic: false,
        };
        // Numerator of z-2 is 3q² or (1-q)²; positive W preserves multiplicity.
        let r = curve_plane::inspect(&c, [0., 0., 1.], 2., 1e-12, 1).unwrap();
        assert!(r.all_roots_isolated, "{r:?}");
        assert!(r.coverage_resolved);
        assert_eq!(r.cells[0].classification, expected);
        assert_eq!(r.cells[0].root_parameter_bounds(), Some([t; 2]));
        assert_eq!(r.cells[0].stop_reason, StopReason::ExactTangency);
        let mut perturbed = c;
        perturbed.control_points[1][2] += 2_f64.powi(-30);
        let r = curve_plane::inspect(&perturbed, [0., 0., 1.], 2., 1e-12, 1).unwrap();
        assert!(!r.cells.iter().any(|x| matches!(
            x.classification,
            Classification::TangencyAtStart | Classification::TangencyAtEnd
        )));
    }
}
#[test]
fn bernstein_endpoint_contact_reports_the_exact_higher_order_multiplicity() {
    for degree in [3, 4, 8, 25] {
        for at_start in [true, false] {
            let mut controls: Vec<_> = (0..=degree)
                .map(|i| vec![i as f64 / degree as f64, 0., 2.])
                .collect();
            controls[if at_start { degree } else { 0 }][2] = 3.;
            let mut knots = vec![2.; degree + 1];
            knots.extend(vec![7.; degree + 1]);
            let c = Curve {
                degree,
                knots,
                control_points: controls,
                weights: (0..=degree).map(|i| (i + 1) as f64).collect(),
                periodic: false,
            };
            // Numerator of z-2 is w_last*q^degree or w_first*(1-q)^degree.
            let r = curve_plane::inspect(&c, [0., 0., 1.], 2., 1e-12, 1).unwrap();
            assert!(r.all_roots_isolated, "{r:?}");
            assert_eq!(r.cells[0].root_multiplicity, Some(degree));
            assert_eq!(
                r.cells[0].classification,
                if at_start {
                    Classification::HigherOrderContactAtStart
                } else {
                    Classification::HigherOrderContactAtEnd
                }
            );
            assert_eq!(
                r.cells[0].root_parameter_bounds(),
                Some([if at_start { 2. } else { 7. }; 2])
            );
            let mut perturbed = c;
            perturbed.control_points[if at_start { 1 } else { degree - 1 }][2] += 2_f64.powi(-30);
            // This adds a nonzero linear endpoint term: the root is still
            // at the same end, but its multiplicity drops from degree to one.
            let r = curve_plane::inspect(&perturbed, [0., 0., 1.], 2., 1e-12, 1).unwrap();
            assert!(!r.cells.iter().any(|x| matches!(
                x.classification,
                Classification::HigherOrderContactAtStart | Classification::HigherOrderContactAtEnd
            )));
            assert!(r.cells.iter().all(|x| x.root_multiplicity != Some(degree)));
        }
    }
}
#[test]
fn endpoint_multiplicity_can_be_lower_than_the_bernstein_degree() {
    for order in [2, 3, 4] {
        for at_start in [true, false] {
            for sign in [1., -1.] {
                let degree = 6;
                let residual: Vec<_> = (0..=degree)
                    .map(|i| {
                        if (at_start && i < order) || (!at_start && i > degree - order) {
                            0.
                        } else {
                            sign
                        }
                    })
                    .collect();
                let mut knots = vec![2.; degree + 1];
                knots.extend(vec![7.; degree + 1]);
                let c = Curve {
                    degree,
                    knots,
                    control_points: residual
                        .iter()
                        .enumerate()
                        .map(|(i, z)| vec![i as f64, 0., 2. + z])
                        .collect(),
                    weights: (0..=degree).map(|i| (i + 1) as f64).collect(),
                    periodic: false,
                };
                let r = curve_plane::inspect(&c, [0., 0., 1.], 2., 1e-12, 1).unwrap();
                assert!(r.all_roots_isolated, "{r:?}");
                assert_eq!(r.cells[0].root_multiplicity, Some(order));
                assert_eq!(
                    r.cells[0].root_parameter_bounds(),
                    Some([if at_start { 2. } else { 7. }; 2])
                );
                let mut mixed = c;
                mixed.control_points[if at_start { degree } else { 0 }][2] = 2. - sign;
                let r = curve_plane::inspect(&mixed, [0., 0., 1.], 2., 1e-12, 1).unwrap();
                // Opposite signs imply an additional interior crossing.
                assert!(!r.all_roots_isolated, "{r:?}");
                assert!(!r.cells.iter().any(|x| x.root_multiplicity == Some(order)));
            }
        }
    }
}
#[test]
fn nonmonotone_bernstein_projection_has_one_proved_simple_endpoint_root() {
    for reverse in [false, true] {
        let mut c = Curve {
            degree: 3,
            knots: vec![2., 2., 2., 2., 7., 7., 7., 7.],
            control_points: vec![
                vec![0., 0., 0.],
                vec![1., 0., 1.],
                vec![2., 0., 0.],
                vec![3., 0., 1. / 64.],
            ],
            weights: vec![1., 2., 3., 4.],
            periodic: false,
        };
        if reverse {
            c.control_points.reverse();
            c.weights.reverse();
        }
        // Numerator 6q(1-q)^2+q^3/16 is positive on (0,1] and has order 1
        // at q=0. The rational projection rises, falls, then rises again.
        let r = curve_plane::inspect(&c, [0., 0., 1.], 0., 1e-12, 1).unwrap();
        assert!(r.all_roots_isolated, "{r:?}");
        assert_eq!(r.work, 1);
        assert_eq!(r.cells[0].root_multiplicity, Some(1));
        assert_eq!(
            r.cells[0].classification,
            if reverse {
                Classification::RootAtEnd
            } else {
                Classification::RootAtStart
            }
        );
        assert_eq!(
            r.cells[0].root_parameter_bounds(),
            Some([if reverse { 7. } else { 2. }; 2])
        );
    }
}
#[test]
fn bernstein_variation_proves_nonmonotone_interior_root_without_accepting_three() {
    let mut c = Curve {
        degree: 3,
        knots: vec![2., 2., 2., 2., 7., 7., 7., 7.],
        control_points: vec![
            vec![0., 0., -1.],
            vec![1., 0., 2.],
            vec![2., 0., 2.],
            vec![3., 0., 1.],
        ],
        weights: vec![1.; 4],
        periodic: false,
    };
    // F(q)=-1+9q-9q²+2q³ changes derivative sign, but its Bernstein
    // sequence has one sign change and its endpoints have opposite signs.
    let r = curve_plane::inspect(&c, [0., 0., 1.], 0., 5.1, 1).unwrap();
    assert!(r.all_roots_isolated, "{r:?}");
    assert_eq!(r.cells[0].classification, Classification::UniqueRoot);
    assert_eq!(r.cells[0].proof, Some(Proof::BernsteinOneVariation));
    assert_eq!(r.cells[0].root_multiplicity, Some(1));
    c.control_points[2][2] = -2.;
    // F(q)=(2q-1)(7q²-7q+1) has three distinct interior roots.
    let r = curve_plane::inspect(&c, [0., 0., 1.], 0., 5.1, 1).unwrap();
    assert!(!r.all_roots_isolated, "{r:?}");
    assert_eq!(r.cells[0].classification, Classification::Unresolved);
    assert_eq!(r.cells[0].root_multiplicity, None);
}
#[test]
fn zero_bernstein_variation_excludes_an_oblique_rational_curve_projection() {
    let c = Curve {
        degree: 3,
        knots: vec![2., 2., 2., 2., 7., 7., 7., 7.],
        control_points: vec![
            vec![0., 0., 1.],
            vec![100., 0., 101.],
            vec![0., 100., 101.],
            vec![100., 100., 201.],
        ],
        weights: vec![1., 2., 3., 4.],
        periodic: false,
    };
    // Each exact control residual -x-y+z is 1. Positive rational blending
    // keeps residual identically 1, despite large independent XYZ boxes.
    let r = curve_plane::inspect(&c, [-1., -1., 1.], 0., 1e-10, 1).unwrap();
    assert!(r.all_roots_isolated, "{r:?}");
    assert_eq!(r.work, 1);
    assert_eq!(r.cells[0].classification, Classification::Excluded);
    assert_eq!(r.cells[0].proof, Some(Proof::BernsteinZeroVariation));
    assert_eq!(r.cells[0].root_parameter_bounds(), None);
    assert!(r.cells[0].residual_bounds[0] < 0. && r.cells[0].residual_bounds[1] > 0.);
}
#[test]
fn same_sign_endpoints_do_not_exclude_two_hidden_roots() {
    let c = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            vec![0., 0., 3. / 16.],
            vec![0.5, 0., -5. / 16.],
            vec![1., 0., 3. / 16.],
        ],
        weights: vec![1.; 3],
        periodic: false,
    };
    // F(q)=q²-q+3/16=(q-1/4)(q-3/4); both endpoint residuals are positive.
    let r = curve_plane::inspect(&c, [0., 0., 1.], 0., 2., 1).unwrap();
    assert!(!r.all_roots_isolated, "{r:?}");
    assert_eq!(r.cells[0].classification, Classification::Unresolved);
    assert_eq!(r.cells[0].proof, None);
    assert_eq!(
        r.cells[0].stop_reason,
        StopReason::RootClassificationNotProven
    );
    assert_eq!(r.cells[0].root_parameter_bounds(), None);
    let r = curve_plane::inspect(&c, [0., 0., 1.], 0., 1e-5, 1000).unwrap();
    // Exact artificial split roots for degree two may remain unresolved,
    // but neither root may be dropped from the returned coverage.
    for root in [0.25, 0.75] {
        assert!(
            r.cells
                .iter()
                .any(|x| x.classification != Classification::Excluded
                    && x.domain[0] <= root
                    && root <= x.domain[1]),
            "{r:?}"
        );
    }
}
#[test]
fn local_linear_formula_avoids_overflow_from_large_parameter_origin() {
    let c = Curve {
        degree: 1,
        knots: vec![999999990., 999999990., 1e9, 1e9],
        control_points: vec![vec![0., 0., 0.], vec![1., 0., 0.]],
        weights: vec![1., 2.],
        periodic: false,
    };
    let r = curve_plane::inspect(&c, [1e300, 0., 0.], 4e299, 1e-5, 1).unwrap();
    // Cx=2q/(1+q)=0.4 gives q=1/4. Multiplying plane residuals by
    // source parameters near 1e9 would overflow; the local formula does not.
    single_linear_root(&r, 999999992.5, 1e-5);
    coverage(&c, &r);
}
#[test]
fn common_weight_scale_does_not_overflow_the_direct_linear_root() {
    for scale in [1e-12, 1., 5e11] {
        let c = Curve {
            degree: 1,
            knots: vec![2., 2., 7., 7.],
            control_points: vec![vec![0., 0., 0.], vec![1., 0., 0.]],
            weights: vec![scale, 2. * scale],
            periodic: false,
        };
        let r = curve_plane::inspect(&c, [1e300, 0., 0.], 4e299, 1e-10, 1).unwrap();
        // Common weight scaling leaves Cx=2q/(1+q), root q=1/4 unchanged.
        single_linear_root(&r, 3.25, 1e-10);
    }
}
#[test]
fn direct_linear_roots_preserve_all_spans_and_share_the_global_budget() {
    let c = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 2., 3., 3.],
        control_points: vec![
            vec![0., 0., -1.],
            vec![1., 0., 1.],
            vec![2., 0., -1.],
            vec![3., 0., 1.],
        ],
        weights: vec![1., 3., 2., 4.],
        periodic: false,
    };
    let before = c.clone();
    let r = curve_plane::inspect(&c, [0., 0., 1.], 0., 1e-10, 3).unwrap();
    coverage(&c, &r);
    assert!(r.all_roots_isolated, "{r:?}");
    assert_eq!(r.work, 3);
    assert_eq!(r.cells.len(), 3);
    for (cell, expected) in r.cells.iter().zip([0.25, 1.6, 2. + 1. / 3.]) {
        let [lo, hi] = cell.root_parameter_bounds().unwrap();
        // In each span the homogeneous linear numerator vanishes at
        // q=w_left/(w_left+w_right), irrespective of its sign orientation.
        assert!(lo <= expected && expected <= hi, "{cell:?}");
        assert!(hi - lo <= 1e-10);
        assert_eq!(cell.root_multiplicity, Some(1));
    }
    assert_eq!(c, before);
    assert!(curve_plane::inspect(&c, [0., 0., 1.], 0., 1e-10, 2).is_err());
}
#[test]
fn rational_quarter_circle_intersection_is_continuously_isolated() {
    let w = 0.5_f64.sqrt();
    let c = Curve {
        degree: 2,
        knots: vec![2., 2., 2., 7., 7., 7.],
        control_points: vec![vec![1., 0., 0.], vec![1., 1., 0.], vec![0., 1., 0.]],
        weights: vec![1., w, 1.],
        periodic: false,
    };
    let before = c.clone();
    let r = curve_plane::inspect(&c, [1., 0., 0.], 0.5, 1e-6, 1000).unwrap();
    coverage(&c, &r);
    assert!(r.all_roots_isolated, "{r:?}");
    let roots: Vec<_> = r
        .cells
        .iter()
        .filter_map(|x| x.root_parameter_bounds())
        .collect();
    assert_eq!(roots.len(), 1);
    let [lo, hi] = roots[0];
    assert!(hi - lo <= 1e-6);
    // Solve the original binary64 rational definition's numerator of x-1/2:
    // (1-q)^2+2wq(1-q)-q²=0, hence q=1/(1-w+sqrt(1+w²)).
    let q = 1. / (1. - w + (1. + w * w).sqrt());
    let parameter = 2. + 5. * q;
    assert!(lo < parameter && parameter < hi, "{roots:?}");
    assert_eq!(c, before);
    for (normal, expected, t) in [
        ([1., 0., 0.], Classification::TangencyAtStart, 2.),
        ([0., 1., 0.], Classification::TangencyAtEnd, 7.),
    ] {
        let r = curve_plane::inspect(&c, normal, 1., 1e-6, 1).unwrap();
        assert!(r.all_roots_isolated, "{r:?}");
        assert_eq!(r.cells[0].classification, expected);
        assert_eq!(r.cells[0].root_multiplicity, Some(2));
        assert_eq!(r.cells[0].root_parameter_bounds(), Some([t; 2]));
    }
    let r = curve_plane::inspect(&c, [1., 0., 0.], 1. + 2_f64.powi(-30), 1e-6, 1).unwrap();
    assert!(r.all_roots_isolated, "{r:?}");
    assert_eq!(r.cells[0].classification, Classification::Excluded);
}
#[test]
fn closed_rational_circle_merges_exact_knot_aliases_and_preserves_both_sides() {
    let w = 0.5_f64.sqrt();
    let c = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 0.25, 0.25, 0.5, 0.5, 0.75, 0.75, 1., 1., 1.],
        control_points: vec![
            vec![1., 0., 0.],
            vec![1., 1., 0.],
            vec![0., 1., 0.],
            vec![-1., 1., 0.],
            vec![-1., 0., 0.],
            vec![-1., -1., 0.],
            vec![0., -1., 0.],
            vec![1., -1., 0.],
            vec![1., 0., 0.],
        ],
        weights: vec![1., w, 1., w, 1., w, 1., w, 1.],
        periodic: false,
    };
    let r = curve_plane::inspect(&c, [1., 0., 0.], 0., 1e-6, 4).unwrap();
    assert!(r.all_roots_isolated, "{r:?}");
    let events = r.root_events();
    assert_eq!(events.len(), 2);
    for (event, t) in events.iter().zip([0.25, 0.75]) {
        assert_eq!(event.parameter_bounds, [t; 2]);
        assert_eq!(event.cells.len(), 2);
        for &i in &event.cells {
            assert_eq!(r.cells[i].root_multiplicity, Some(1));
            assert!(r.cells[i].proof.is_some());
        }
    }
    let r = curve_plane::inspect(&c, [0., 1., 0.], 0., 1e-6, 4).unwrap();
    assert!(r.all_roots_isolated, "{r:?}");
    assert_eq!(
        r.root_events()
            .iter()
            .map(|x| x.parameter_bounds)
            .collect::<Vec<_>>(),
        vec![[0.; 2], [0.5; 2], [1.; 2]]
    );
    let closed = curve_plane::inspect_closed(&c, [0., 1., 0.], 0., 1e-6, 4).unwrap();
    assert_eq!(
        closed
            .events
            .iter()
            .map(|x| x.parameter_bounds)
            .collect::<Vec<_>>(),
        vec![[0.; 2], [0.5; 2]]
    );
    assert_eq!(closed.events[0].cells.len(), 2);
    assert_eq!(
        closed.events[0]
            .cells
            .iter()
            .map(|&i| closed.report.cells[i].root_parameter_bounds().unwrap())
            .collect::<Vec<_>>(),
        vec![[0.; 2], [1.; 2]]
    );
}
#[test]
fn closed_event_aliasing_requires_exact_closure_and_clamped_endpoints() {
    let mut c =
        Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 1., 0.], vec![0., 0., 0.]]).unwrap();
    c.weights = vec![1., 3., 2.];
    let before = c.clone();
    let r = curve_plane::inspect_closed(&c, [1., 0., 0.], 0., 1e-8, 2).unwrap();
    assert_eq!(r.events.len(), 1);
    assert_eq!(r.events[0].parameter_bounds, [0.; 2]);
    assert_eq!(r.events[0].cells.len(), 2);
    assert_eq!(c, before);
    c.control_points[2][0] = 1e-12;
    assert!(curve_plane::inspect_closed(&c, [1., 0., 0.], 0., 1e-8, 2).is_err());
    let c = Curve {
        degree: 2,
        knots: vec![0., 1., 2., 3., 4., 5.],
        control_points: vec![vec![0.; 3], vec![1., 1., 0.], vec![0.; 3]],
        weights: vec![1.; 3],
        periodic: false,
    };
    assert!(curve_plane::inspect_closed(&c, [1., 0., 0.], 0., 1e-8, 100).is_err());
}
#[test]
fn closed_aliasing_keeps_unresolved_start_side_and_original_end_proof() {
    let c = Curve {
        degree: 3,
        knots: vec![0., 0., 0., 0., 0.5, 0.5, 0.5, 1., 1., 1., 1.],
        control_points: vec![
            vec![0.; 3],
            vec![1., 0., -1.],
            vec![2., 0., 1.],
            vec![3., 0., 1.],
            vec![2., 0., 1.],
            vec![1., 0., 1.],
            vec![0.; 3],
        ],
        weights: vec![1.; 7],
        periodic: false,
    };
    let r = curve_plane::inspect_closed(&c, [0., 0., 1.], 0., 1e-8, 2).unwrap();
    assert!(!r.report.all_roots_isolated);
    assert!(!r.report.coverage_resolved);
    assert_eq!(r.report.cells[0].classification, Classification::Unresolved);
    assert_eq!(r.events.len(), 1);
    assert_eq!(r.events[0].parameter_bounds, [0.; 2]);
    assert_eq!(r.events[0].cells, vec![1]);
    assert_eq!(r.report.cells[1].root_parameter_bounds(), Some([1.; 2]));
}
#[test]
fn shared_root_event_keeps_different_one_sided_multiplicities() {
    let c = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
        control_points: vec![
            vec![0., 0., 1.],
            vec![0.25, 0., 0.],
            vec![0.5, 0., 0.],
            vec![0.75, 0., 0.5],
            vec![1., 0., 1.],
        ],
        weights: vec![1.; 5],
        periodic: false,
    };
    let r = curve_plane::inspect(&c, [0., 0., 1.], 0., 1e-6, 2).unwrap();
    assert!(r.all_roots_isolated, "{r:?}");
    let events = r.root_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].parameter_bounds, [0.5; 2]);
    assert_eq!(
        events[0]
            .cells
            .iter()
            .map(|&i| r.cells[i].root_multiplicity)
            .collect::<Vec<_>>(),
        vec![Some(2), Some(1)]
    );
}
#[test]
fn positive_gap_near_a_tangent_is_excluded_without_creating_a_root() {
    let gap = 2_f64.powi(-30);
    let c = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            vec![0., 0., 0.25 + gap],
            vec![0.5, 0., -0.25 + gap],
            vec![1., 0., 0.25 + gap],
        ],
        weights: vec![1.; 3],
        periodic: false,
    };
    // Exact dyadic polynomial z=(t-1/2)^2+2^-30 is strictly positive.
    let r = curve_plane::inspect(&c, [0., 0., 1.], 0., 1e-6, 2000).unwrap();
    coverage(&c, &r);
    assert!(r.all_roots_isolated, "{r:?}");
    assert!(
        r.cells
            .iter()
            .all(|x| x.classification == Classification::Excluded)
    );
}
#[test]
fn invalid_inputs_and_insufficient_initial_coverage_are_errors() {
    let c =
        Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.], vec![2., 0., 0.]]).unwrap();
    assert!(curve_plane::inspect(&c, [1., 0., 0.], 0.3, 1e-5, 1).is_err());
    for n in [[0.; 3], [f64::NAN, 0., 0.]] {
        assert!(curve_plane::inspect(&c, n, 0., 1e-5, 100).is_err());
    }
    for t in [0., -1., f64::NAN] {
        assert!(curve_plane::inspect(&c, [1., 0., 0.], 0., t, 100).is_err());
    }
    for budget in [0, 100001] {
        assert!(curve_plane::inspect(&c, [1., 0., 0.], 0., 1e-5, budget).is_err());
    }
    assert!(curve_plane::inspect(&c, [1., 0., 0.], f64::INFINITY, 1e-5, 100).is_err());
    let mut bad = c;
    bad.weights[0] = 0.;
    assert!(curve_plane::inspect(&bad, [1., 0., 0.], 0., 1e-5, 100).is_err());
}

#[test]
fn periodic_storage_seam_has_one_event_with_original_side_proofs() {
    let c = Curve {
        degree: 2,
        knots: vec![-1., -1., 0., 0., 1., 1., 2., 2., 3., 3., 4.],
        control_points: vec![
            vec![-1., 0., 1.],
            vec![0.; 3],
            vec![1., 0., 1.],
            vec![2., 0., 2.],
            vec![1., 1., 1.],
            vec![-1., 1., 2.],
            vec![-1., 0., 1.],
            vec![0.; 3],
        ],
        weights: vec![1., 2., 3., 4., 5., 6., 1., 2.],
        periodic: true,
    };
    let before = c.clone();
    // Both seam-adjacent homogeneous numerators have one exact endpoint
    // zero and positive remaining coefficients: simple roots, no interior zero.
    let r = curve_plane::inspect_closed(&c, [0., 0., 1.], 0., 1e-8, 3).unwrap();
    coverage(&c, &r.report);
    assert!(r.report.all_roots_isolated, "{r:?}");
    assert_eq!(r.events.len(), 1);
    assert_eq!(r.events[0].parameter_bounds, [0.; 2]);
    assert_eq!(r.events[0].cells, vec![0, 2]);
    assert_eq!(r.report.cells[0].root_parameter_bounds(), Some([0.; 2]));
    assert_eq!(r.report.cells[2].root_parameter_bounds(), Some([3.; 2]));
    assert_eq!(c, before);
    // General validation allows tiny authored knot drift; exact closure must
    // refuse it, rather than aliasing endpoints based on a rounded equality.
    let mut drift = c.clone();
    drift.knots[0] = (-1_f64).next_down();
    drift.validate().unwrap();
    assert!(curve_plane::inspect_closed(&drift, [0., 0., 1.], 0., 1e-8, 3).is_err());
    let mut wrong_weight = c;
    wrong_weight.weights[6] = 1.0000000001;
    assert!(curve_plane::inspect_closed(&wrong_weight, [0., 0., 1.], 0., 1e-8, 3).is_err());
}
#[test]
fn smooth_periodic_storage_can_exclude_a_plane_without_seam_sampling() {
    let c = Curve {
        degree: 2,
        knots: vec![-2., -1., 0., 1., 2., 3., 4., 5., 6.],
        control_points: vec![
            vec![0., 0., 1.],
            vec![1., 0., 2.],
            vec![1., 1., 1.],
            vec![0., 1., 2.],
            vec![0., 0., 1.],
            vec![1., 0., 2.],
        ],
        weights: vec![1., 2., 3., 4., 1., 2.],
        periodic: true,
    };
    // Every original z control is >=1; positive rational weights keep the
    // entire closed curve strictly above z=0, including its noninterpolated seam.
    let r = curve_plane::inspect_closed(&c, [0., 0., 1.], 0., 1e-8, 4).unwrap();
    assert!(r.report.all_roots_isolated);
    assert!(r.events.is_empty());
    assert!(
        r.report
            .cells
            .iter()
            .all(|x| x.classification == Classification::Excluded)
    );
    coverage(&c, &r.report);
}

#[test]
fn noninterpolated_periodic_quadratic_knot_roots_are_exact_and_seam_owned() {
    let c = Curve {
        degree: 2,
        knots: vec![-2., -1., 0., 1., 2., 3., 4., 5., 6.],
        control_points: vec![
            vec![0., 0., -2.],
            vec![1., 0., 1.],
            vec![1., 1., 1.],
            vec![0., 1., -2.],
            vec![0., 0., -2.],
            vec![1., 0., 1.],
        ],
        weights: vec![1., 2., 2., 1., 1., 2.],
        periodic: true,
    };
    // At t=0 and t=4 the exact homogeneous residual is (-2+2)/2=0.
    // At t=2 it is (2-2)/2=0. Other simple knots have strict signs.
    // Each arc between these knots has strictly signed Bernstein coefficients.
    let r = curve_plane::inspect_closed(&c, [0., 0., 1.], 0., 1e-6, 2000).unwrap();
    coverage(&c, &r.report);
    assert!(r.report.all_roots_isolated, "{r:?}");
    assert_eq!(r.events.len(), 2, "{r:?}");
    assert_eq!(r.events[0].parameter_bounds, [0.; 2]);
    assert_eq!(r.events[1].parameter_bounds, [2.; 2]);
    assert_eq!(r.events[0].cells.len(), 2);
    assert_eq!(r.events[1].cells.len(), 2);
    for e in &r.events {
        for &i in &e.cells {
            assert_eq!(r.report.cells[i].root_multiplicity, Some(1));
            assert_eq!(
                r.report.cells[i].proof,
                Some(Proof::EndpointWithMonotonicity)
            );
        }
    }
    // A nonzero offset cannot be erased by the new exact knot predicate.
    let shifted = curve_plane::inspect(&c, [0., 0., 1.], 2_f64.powi(-30), 1e-6, 2000).unwrap();
    assert!(!shifted.root_events().iter().any(|e| {
        e.parameter_bounds == [0.; 2]
            || e.parameter_bounds == [2.; 2]
            || e.parameter_bounds == [4.; 2]
    }));
}

#[test]
fn nonuniform_quadratic_endpoint_uses_original_knot_lengths_and_offset() {
    let c = Curve {
        degree: 2,
        knots: vec![-2., -1., 0., 2., 5., 6., 7.],
        control_points: vec![
            vec![0., 0., 5.],
            vec![1., 0., 8.],
            vec![2., 0., 9.],
            vec![3., 0., 10.],
        ],
        weights: vec![1., 4., 2., 3.],
        periodic: false,
    };
    // At t=0 the nonzero basis values are 2/3 and 1/3.
    // With plane z=7 the homogeneous residual is 1*2*(-2)+4*1*(1)=0.
    // All following original residuals are positive: no further root.
    for sign in [-1., 1.] {
        let r = curve_plane::inspect(&c, [0., 0., sign], 7. * sign, 1e-6, 2000).unwrap();
        coverage(&c, &r);
        assert!(r.all_roots_isolated, "{r:?}");
        let events = r.root_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].parameter_bounds, [0.; 2]);
        assert_eq!(r.cells[events[0].cells[0]].root_multiplicity, Some(1));
    }
}

#[test]
fn exact_weighted_midpoint_powers_have_degree_multiplicity() {
    for degree in [3, 4, 7, 8, 25] {
        let mut knots = vec![2.; degree + 1];
        knots.extend(vec![7.; degree + 1]);
        let weights: Vec<_> = (0..=degree)
            .map(|i| if i % 3 == 0 { 2. } else { 4. })
            .collect();
        let controls: Vec<_> = (0..=degree)
            .map(|i| {
                // With offset 3 each exact weighted residual is (-1)^i.
                vec![
                    i as f64,
                    0.,
                    3. + if i % 2 == 0 { 1. } else { -1. } / weights[i],
                ]
            })
            .collect();
        let c = Curve {
            degree,
            knots,
            control_points: controls,
            weights,
            periodic: false,
        };
        let before = c.clone();
        for sign in [-1., 1.] {
            // Binomial identity: homogeneous N=sign*(1-2q)^degree;
            // W>0, hence there is precisely one root q=1/2 of that order.
            let r = curve_plane::inspect(&c, [0., 0., sign], 3. * sign, 1e-10, 1).unwrap();
            coverage(&c, &r);
            assert!(r.all_roots_isolated, "{r:?}");
            assert_eq!(r.work, 1);
            assert_eq!(
                r.cells[0].classification,
                Classification::HigherOrderContactAtMidpoint
            );
            assert_eq!(r.cells[0].proof, Some(Proof::BernsteinMidpointPower));
            assert_eq!(r.cells[0].root_multiplicity, Some(degree));
            assert_eq!(r.cells[0].root_parameter_bounds(), Some([4.5; 2]));
        }
        assert_eq!(c, before);
        let mut perturbed = c;
        perturbed.control_points[degree / 2][2] += 2_f64.powi(-30);
        let r = curve_plane::inspect(&perturbed, [0., 0., 1.], 3., 1e-10, 1).unwrap();
        assert!(
            r.cells
                .iter()
                .all(|x| x.proof != Some(Proof::BernsteinMidpointPower))
        );
    }
}

#[test]
fn degree_elevated_midpoint_contacts_keep_actual_multiplicity() {
    // Hand-derived Bernstein coefficients, independent of the runtime's
    // binomial conversion: A(1-2q)^k elevated to p.
    for (degree, order, residuals) in [
        (3, 2, vec![3., -1., -1., 3.]),
        (4, 2, vec![3., 0., -1., 0., 3.]),
        (4, 3, vec![2., -1., 0., 1., -2.]),
        (6, 3, vec![5., 0., -1., 0., 1., 0., -5.]),
    ] {
        let mut knots = vec![2.; degree + 1];
        knots.extend(vec![7.; degree + 1]);
        let weights: Vec<_> = (0..=degree)
            .map(|i| if i % 2 == 0 { 2. } else { 4. })
            .collect();
        let controls: Vec<_> = residuals
            .iter()
            .enumerate()
            .map(|(i, &h)| vec![i as f64, 0., 7. + h / weights[i]])
            .collect();
        let c = Curve {
            degree,
            knots,
            control_points: controls,
            weights,
            periodic: false,
        };
        let r = curve_plane::inspect(&c, [0., 0., 1.], 7., 1e-10, 1).unwrap();
        assert!(r.all_roots_isolated, "{r:?}");
        assert_eq!(r.cells[0].root_parameter_bounds(), Some([4.5; 2]));
        assert_eq!(r.cells[0].root_multiplicity, Some(order));
        assert_eq!(r.cells[0].proof, Some(Proof::BernsteinMidpointPower));
        assert_eq!(
            r.cells[0].classification,
            if order == 2 {
                Classification::TangencyAtMidpoint
            } else {
                Classification::HigherOrderContactAtMidpoint
            }
        );
        let mut changed = c;
        changed.control_points[degree / 2][2] += 2_f64.powi(-30);
        let r = curve_plane::inspect(&changed, [0., 0., 1.], 7., 1e-10, 1).unwrap();
        assert!(
            r.cells
                .iter()
                .all(|x| x.proof != Some(Proof::BernsteinMidpointPower))
        );
    }
}
