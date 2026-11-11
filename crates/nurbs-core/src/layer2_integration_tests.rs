//! Tests of `nurbs-foundation` that need higher nurbs-core layers. They moved here unchanged
//! when foundation, distance and certificates became their own crate.

mod foundation_knot_removal {
    #[allow(unused_imports)]
    use crate::foundation::knot_removal::*;
    #[allow(unused_imports)]
    use crate::{curve::Curve, surface::Surface};

    #[test]
    fn surface_offset_rejects_nonfinite_distance_and_tolerance() {
        use crate::surface::Surface;
        let plane = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 10., 0.]],
                vec![vec![10., 0., 0.], vec![10., 10., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        plane.validate().unwrap();
        for distance in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let Err(error) = crate::surface_offset::offset(&plane, distance, 0.1) else {
                panic!("non-finite distance {distance} must be rejected")
            };
            assert!(
                error.to_string().contains("distance must be finite"),
                "distance {distance}: unexpected error {error}"
            );
        }
        for tolerance in [f64::NAN, f64::INFINITY, 0., -0.1] {
            assert!(
                crate::surface_offset::offset(&plane, 0.5, tolerance).is_err(),
                "tolerance {tolerance} must be rejected"
            );
        }
    }

    #[test]
    fn surface_with_nan_coordinate_fails_validation_before_offset() {
        use crate::surface::Surface;
        let mut plane = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., f64::NAN], vec![0., 10., 0.]],
                vec![vec![10., 0., 0.], vec![10., 10., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        assert!(plane.validate().is_err(), "NaN coordinate must be rejected");
        plane.control_points[0][0][2] = f64::INFINITY;
        assert!(plane.validate().is_err(), "+∞ coordinate must be rejected");
        // …and offset therefore errors instead of propagating NaN.
        assert!(crate::surface_offset::offset(&plane, 0.5, 0.1).is_err());
    }
}

mod foundation_natural_loft {
    #[allow(unused_imports)]
    use crate::foundation::natural_loft::*;
    #[allow(unused_imports)]
    use crate::{curve::Curve, surface::Surface};

    fn cyclic_sections() -> Vec<Curve> {
        [[1., 0.], [0., 1.], [-1., 0.], [0., -1.], [1., 0.]]
            .into_iter()
            .map(|xy| {
                let mut c = section(0., 1.);
                for p in &mut c.control_points {
                    p[0] += xy[0];
                    p[1] += xy[1];
                }
                c
            })
            .collect()
    }

    fn section(z: f64, scale: f64) -> Curve {
        let mut c = crate::primitives::circle_arc([0., 0., z], [0., 0., 1.], 2., 0., 90.).unwrap();
        c.weights.iter_mut().for_each(|w| *w *= scale);
        c
    }

    #[cfg(feature = "transport")]
    #[test]
    fn json_control_tangent_loft_keeps_variable_boundary_derivative() {
        let curves = [section(0., 1.), section(7., 1.)];
        let t = [[0., 0., 1.], [0., 0., 3.], [0., 0., -2.]];
        let result = crate::transport::dispatch(value_codec::json!({
            "op":"surface_control_tangent_loft", "curves":curves,
            "parameters":[2.,7.], "start_tangents":t,"end_tangents":t
        }))
        .unwrap();
        let s: Surface = value_codec::from_value(result).unwrap();
        for (u, z) in [(0., 5.), (1., -10.)] {
            let (_, dv) = s.evaluate(u, 0.).unwrap().first_derivatives().unwrap();
            assert!((dv[2] - z).abs() < 1e-9);
        }
        assert!(
            crate::transport::dispatch(value_codec::json!({
                "op":"surface_control_tangent_loft", "curves":curves,
                "parameters":[2.,7.], "start_tangents":[],"end_tangents":t
            }))
            .is_err()
        );
    }

    #[cfg(feature = "transport")]
    #[test]
    fn json_boundary_retains_the_constructor_contract() {
        let sections = vec![section(0., 1.), section(2., 1.)];
        let v = crate::transport::dispatch(value_codec::json!({
            "op":"surface_natural_loft", "curves":sections, "parameters":[2.,5.]
        }))
        .unwrap();
        let s: Surface = value_codec::from_value(v).unwrap();
        assert!((s.evaluate(0.5, 0.3).unwrap().point[2] - 0.6).abs() < 1e-11);
    }

    #[cfg(feature = "transport")]
    #[test]
    fn json_clamped_boundary_uses_authored_parameter_derivatives() {
        let v = crate::transport::dispatch(value_codec::json!({
            "op":"surface_clamped_loft", "curves":[section(0.,1.),section(2.,1.2)],
            "parameters":[2.,5.], "start_tangent":[0.,0.,4.], "end_tangent":[0.,0.,-1.]
        }))
        .unwrap();
        let s: Surface = value_codec::from_value(v).unwrap();
        for u in [0., 0.37, 1.] {
            assert!(
                (s.evaluate(u, 0.).unwrap().first_derivatives().unwrap().1[2] - 12.).abs() < 1e-10
            );
            assert!(
                (s.evaluate(u, 1.).unwrap().first_derivatives().unwrap().1[2] + 3.).abs() < 1e-10
            );
        }
    }

    #[cfg(feature = "transport")]
    #[test]
    fn json_closed_loft_preserves_the_cycle_without_periodic_encoding() {
        let v = crate::transport::dispatch(value_codec::json!({
            "op":"surface_closed_loft", "curves":cyclic_sections(),
            "parameters":[0.,1.,2.,3.,4.]
        }))
        .unwrap();
        let s: Surface = value_codec::from_value(v).unwrap();
        assert!(!s.periodic_v);
        let a = s.evaluate(0.37, 0.).unwrap().point;
        let b = s.evaluate(0.37, 1.).unwrap().point;
        assert_eq!(a, b);
    }
}

mod distance_polar_surface_differential {
    #[allow(unused_imports)]
    use crate::distance::polar_surface_differential::*;
    #[allow(unused_imports)]
    use crate::{curve::Curve, surface::Surface};

    fn s() -> Surface {
        Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.]; 2],
                vec![vec![0.5, 0., 0.]; 2],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        }
    }

    #[test]
    fn original_polar_chord_bound_covers_collapsed_diagonal_and_refuses_missing_work() {
        for end in [false, true] {
            let mut source = s();
            if end {
                source.control_points.reverse();
            }
            let d = [if end { [0.999, 1.] } else { [0., 0.001] }, [0., 1.]];
            assert!(
                crate::surface_self_chord::qualify_rectangle(&source, d, 1e-6, 3)
                    .unwrap()
                    .certificate
                    .is_none()
            );
            let r = crate::surface_self_chord::qualify_polar_rectangle(&source, d, end, 1e-6, 3)
                .unwrap();
            let c = r.certificate.unwrap();
            assert!(std::ptr::eq(c.surface(), &source));
            assert_eq!(c.domain(), d);
            assert_eq!(r.spans, 3);
            assert!(
                crate::surface_self_chord::qualify_polar_rectangle(&source, d, end, 1e-6, 2)
                    .unwrap()
                    .certificate
                    .is_none()
            );
        }
        let mut source = s();
        source.weights[0][1] = 0.5;
        assert!(crate::surface_self_chord::qualify_polar_rectangle(
            &source,
            [[0., 1.], [0., 1.]],
            false,
            1e-6,
            3
        )
        .unwrap()
        .certificate
        .is_none());
    }
}

mod distance_rational_pole_differential {
    #[allow(unused_imports)]
    use crate::distance::rational_pole_differential::*;
    #[allow(unused_imports)]
    use crate::{curve::Curve, surface::Surface};

    fn source() -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..3)
            .map(|u| {
                (0..2)
                    .map(|v| vec![u as f64 / 2., if u == 2 { v as f64 } else { 0. }, 0.])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1., 2.], vec![1., 3.], vec![1., 2.]],
        periodic_u: false,
        periodic_v: false,
    }
}

    #[test]
fn rational_tangent_term_is_retained_and_original_frame_is_owned() {
    for end in [false, true] {
        let mut s = source();
        if end {
            s.control_points.reverse();
            s.weights.reverse();
        }
        let r = prepare(&s, end, 10_000_000).unwrap();
        let c = r.certificate.unwrap();
        assert!(std::ptr::eq(c.surface(), &s));
        assert_eq!(c.anchor(), 0);
        let d = [if end { [0.999, 1.] } else { [0., 0.001] }, [0., 0.1]];
        let j = c.bounds(d).unwrap().unwrap();
        assert!(j[0][0].contains(1.));
        assert!(j[1][1].lo > 0.);
        assert!(j[2][0].contains(0.) && j[2][1].contains(0.));
        let d = [d[0], [0., 0.0001]];
        let admitted =
            crate::surface_self_chord::qualify_rational_polar_rectangle(&s, d, &c, 1e-4, 5)
                .unwrap();
        assert!(admitted.certificate.is_some());
        assert_eq!(admitted.spans, 5);
        let limited =
            crate::surface_self_chord::qualify_rational_polar_rectangle(&s, d, &c, 1e-4, 4)
                .unwrap();
        assert!(limited.certificate.is_none() && !limited.complete);
        assert_eq!(limited.spans, 0);
        let foreign = s.clone();
        assert!(crate::surface_self_chord::qualify_rational_polar_rectangle(
            &foreign, d, &c, 1e-4, 5,
        )
        .is_err());
        let normal_report =
            crate::factored_surface_normals::prepare(&s, end, 10_000_000).unwrap();
        let normals = normal_report.certificate.unwrap();
        let near = if end { [1. - 1e-12, 1.] } else { [0., 1e-12] };
        let pair = [[near, [0., 1e-12]], [near, [0.245, 0.245 + 1e-12]]];
        let excluded = crate::surface_self_chord::rational_pole_pair_exclusion(
            &s, pair, &c, &normals, 1e-4, 7,
        )
        .unwrap();
        // Correlated numerator/denominator coefficients now keep the
        // wide angular pair nonsingular, unlike the old independent hulls.
        assert!(excluded.0 && excluded.1 <= 7);
        let classified = crate::surface_self_chord::rational_pole_pair_report(
            &s, pair, &c, &normals, 1e-4, 7,
        )
        .unwrap();
        assert!(
            classified.excluded
                && classified.coordinate_path_excluded
                && classified.coordinate_path_attempted
                && classified.coordinate_path_available
        );

        let narrow = [[near, [0., 1e-12]], [near, [0.1, 0.1 + 1e-12]]];
        let local = crate::surface_self_chord::rational_pole_pair_exclusion(
            &s, narrow, &c, &normals, 1e-4, 7,
        )
        .unwrap();
        assert!(local.0 && local.1 <= 7);
        assert_eq!(
            crate::surface_self_chord::rational_pole_pair_exclusion(
                &s, pair, &c, &normals, 1e-4, 5
            )
            .unwrap(),
            (false, 5)
        );
        assert!(crate::surface_self_chord::rational_pole_pair_exclusion(
            &foreign, pair, &c, &normals, 1e-4, 7
        )
        .is_err());

        let limited = crate::surface_self_chord_coverage::qualify_with_rational_pole(
            &s,
            0.1,
            1e-4,
            crate::surface_self_chord_coverage::Limits { cells: 1, spans: 1 },
            &normals,
            &c,
        )
        .unwrap();
        assert!(limited.certificate.is_none() && limited.pending > 0 && limited.spans <= 1);
        assert_eq!(limited.coordinate_path_attempts, 0);
        assert_eq!(limited.coordinate_path_exclusions, 0);
        assert!(
            crate::surface_self_chord_coverage::qualify_with_rational_pole(
                &foreign,
                0.1,
                1e-4,
                crate::surface_self_chord_coverage::Limits { cells: 1, spans: 1 },
                &normals,
                &c
            )
            .is_err()
        );
        assert!(c.bounds([[-0.1, 1.], [0., 1.]]).is_err());
    }
}

    #[test]
fn local_inverse_chart_encloses_independent_frames_without_global_monotonicity() {
    let mut s = source();
    for v in 0..2 {
        s.control_points[1][v][0] = -0.5;
    }
    let c = prepare(&s, false, 10_000_000).unwrap().certificate.unwrap();
    let d = [[0.8, 0.85], [0., 0.1]];
    let local = [[0.45, 0.55], [0., 0.01]];
    assert!(c.local_coordinate_path_bounds(local).unwrap().is_none());
    let refused = c.local_coordinate_path_report(local, 3.).unwrap();
    assert!(refused.0.is_none());
    assert_eq!(refused.1, 1);
    let prefix_refused = c.coordinate_path_report(local).unwrap();
    assert!(prefix_refused.0.is_none());
    assert_eq!(prefix_refused.1, 3);
    let uv_refused = c.bounds_report([[0., 0.8], [0., 0.1]]).unwrap();
    assert!(uv_refused.0.is_none());
    assert_eq!(uv_refused.1, 2);

    let accepted = c.local_coordinate_path_report(local, 0.25).unwrap();
    assert!(accepted.0.is_some());
    assert_eq!(accepted.1, 6);

    let narrow = c.local_coordinate_path_bounds_with_margin(local, 0.25)
        .unwrap().unwrap();
    for u in [0.45, 0.5, 0.55] {
        for v in [0., 0.005, 0.01] {
            let w = 1. + v + 2. * u * (1. - u) * v;
            let wu = 2. * (1. - 2. * u) * v;
            let wv = 1. + 2. * u * (1. - u);
            let x = -u * (1. + 2. * v) + u * u * (2. + 3. * v);
            let y = 2. * u * u * v;
            let xu = ((-1.-2.*v+2.*u*(2.+3.*v))*w-x*wu)/(w*w);
            let xv = ((-2.*u+3.*u*u)*w-x*wv)/(w*w);
            let yu = (4.*u*v*w-y*wu)/(w*w);
            let yv = (2.*u*u*w-y*wv)/(w*w);
            assert!(narrow[1][0].contains(yu/xu));
            assert!(narrow[1][1].contains((yv-yu*xv/xu)/(u*u)));
        }
    }
    assert!(c.local_coordinate_path_bounds_with_margin(local, 0.).is_err());

    assert!(c.coordinate_path_bounds(d).unwrap().is_none());
    let j = c.local_coordinate_path_bounds(d).unwrap().unwrap();
    let prepared = crate::factored_surface_normals::prepare(&s, false, 10_000_000).unwrap();
    let normals = prepared.certificate.unwrap();
    let pair = [[[0.8, 0.805], [0., 0.005]], [[0.845, 0.85], [0.095, 0.1]]];
    let report =
        crate::surface_self_chord::rational_pole_pair_report(&s, pair, &c, &normals, 1e-4, 10)
            .unwrap();
    assert!(
        report.excluded
            && report.local_path_excluded
            && report.local_path_available
            && report.local_path_attempted
            && !report.coordinate_path_available
    );
    assert_eq!(report.spans, 10);
    assert_eq!(report.local_path_margin, Some(3.));
    let alternatives = crate::surface_self_chord::rational_pole_pair_report(
        &s, pair, &c, &normals, 1e-4, 32).unwrap();
    assert!(alternatives.excluded && alternatives.local_path_excluded);
    assert!(alternatives.spans <= 32);

    let cached = normals.pair_bounds(pair).unwrap(); // Two paid restrictions.
    let reused = crate::surface_self_chord::rational_pole_pair_report_cached(
        &s, pair, &c, &normals, 1e-4, 9, Some(&cached)).unwrap();
    assert!(reused.excluded && reused.local_path_excluded);
    assert_eq!(reused.spans, 9);
    let no_matrix_budget = crate::surface_self_chord::rational_pole_pair_report_cached(
        &s, pair, &c, &normals, 1e-4, 8, Some(&cached)).unwrap();
    assert!(!no_matrix_budget.excluded && no_matrix_budget.spans <= 8);
    assert!(crate::surface_self_chord::rational_pole_pair_report_cached(
        &s, [pair[0]; 2], &c, &normals, 1e-4, 0, Some(&cached)).is_err());

    let limited =
        crate::surface_self_chord::rational_pole_pair_report(&s, pair, &c, &normals, 1e-4, 9)
            .unwrap();
    assert!(!limited.excluded && limited.spans == 9 && limited.local_path_available);

    let negative = c
        .local_coordinate_path_bounds([[0.01, 0.02], [0., 0.1]])
        .unwrap()
        .unwrap();
    for (j, parameters) in [(&j, [0.8, 0.825, 0.85]), (&negative, [0.01, 0.015, 0.02])] {
        for u in parameters {
            for v in [0., 0.05, 0.1] {
                let w = 1. + v + 2. * u * (1. - u) * v;
                let wu = 2. * (1. - 2. * u) * v;
                let wv = 1. + 2. * u * (1. - u);
                let x = -u * (1. + 2. * v) + u * u * (2. + 3. * v);
                let y = 2. * u * u * v;
                let xu = ((-1. - 2. * v + 2. * u * (2. + 3. * v)) * w - x * wu) / (w * w);
                let xv = ((-2. * u + 3. * u * u) * w - x * wv) / (w * w);
                let yu = (4. * u * v * w - y * wu) / (w * w);
                let yv = (2. * u * u * w - y * wv) / (w * w);
                assert_eq!(xu > 0., u > 0.25);
                assert!(j[1][0].contains(yu / xu));
                assert!(j[1][1].contains((yv - yu * xv / xu) / (u * u)));
            }
        }
    }
}
}

mod certificates_curve_decomposition_certificate {
    #[allow(unused_imports)]
    use crate::certificates::curve_decomposition_certificate::*;
    #[allow(unused_imports)]
    use crate::{curve::Curve, surface::Surface};

    #[test]
    fn nonuniform_rational_extraction_and_independent_translation() {
        let c = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 0.4, 1., 1., 1.],
            control_points: vec![
                vec![0., 0., 0.],
                vec![0.3, 0.8, 0.],
                vec![0.7, -0.2, 0.],
                vec![1., 0., 0.],
            ],
            weights: vec![1., 0.75, 1.25, 2.],
            periodic: false,
        };
        let before = c.clone();
        let parts = c.decompose().unwrap();
        for (span, part) in [2, 3].into_iter().zip(parts) {
            let r = inspect(&c, span, part.definition(), 1000).unwrap();
            assert!(r.error_upper.unwrap() < 1e-10);
            #[cfg(feature = "transport")]
            {
                let request = value_codec::json!({"op":"curve_decomposition_audit","curve":c,"span":span,"retained":part.definition(),"maxProducts":1000});
                let report = crate::transport::dispatch(request.clone()).unwrap();
                assert!(report["errorUpper"].as_f64().unwrap() < 1e-10);
                assert_eq!(report["continuousBound"], false);
                let mut cut = request.clone();
                cut["maxProducts"] = value_codec::json!(r.products - 1);
                assert!(crate::transport::dispatch(cut).unwrap()["errorUpper"].is_null());
                let mut invalid = request;
                invalid["maxProducts"] = value_codec::json!(-1);
                assert!(crate::transport::dispatch(invalid).is_err());
            }
            assert!(
                inspect(&c, span, part.definition(), r.products - 1)
                    .unwrap()
                    .error_upper
                    .is_none()
            );
            let mut shifted = part.definition().clone();
            for pole in &mut shifted.control_points {
                pole[2] += 0.125;
            }
            assert!(
                inspect(&c, span, &shifted, 1000)
                    .unwrap()
                    .error_upper
                    .unwrap()
                    >= 0.125
            );
            assert!(
                inspect(&c, 0, part.definition(), 1000)
                    .unwrap()
                    .error_upper
                    .is_none()
            );
        }
        assert_eq!(c, before);
    }

    #[test]
    fn last_span_exhaustion_discards_previous_bound_and_preserves_sources() {
        let c = Curve { degree: 1, knots: vec![0.,0.,1.,1.], control_points: vec![vec![0.,0.,0.],vec![1.,0.,0.]], weights: vec![1.,2.], periodic: false };
        let mut shifted=c.clone();
        for p in &mut shifted.control_points { p[2]=0.125; }
        let before=shifted.clone();
        let pairs=[(&c,1,&c),(&c,1,&shifted)];
        let complete=inspect_batch(&pairs,8).unwrap();
        assert!(complete.error_upper.unwrap() >= 0.125);
        assert_eq!((complete.products,complete.pairs_inspected),(8,2));
        let refused=inspect_batch(&pairs,7).unwrap();
        assert_eq!(refused.error_upper,None);
        assert_eq!(refused.reason,Some("work-limit"));
        assert_eq!((refused.products,refused.pairs_inspected),(4,2));
        assert_eq!(shifted,before);
        assert_eq!(inspect_batch(&[],8).unwrap().reason,Some("work-limit"));
        #[cfg(feature="transport")]
        {
            let request=value_codec::json!({"op":"curve_decomposition_batch_audit","pairs":[{"curve":c,"span":1,"retained":c},{"curve":c,"span":1,"retained":shifted}],"maxProducts":7});
            let result=crate::transport::dispatch(request).unwrap();
            assert!(result["errorUpper"].is_null());
            assert_eq!(result["pairsInspected"],2);
            assert_eq!(result["continuousBound"],false);
        }
    }

    #[test]
    fn batch_has_one_budget_and_never_returns_a_partial_maximum() {
        let c=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:vec![vec![0.,0.,0.],vec![1.,1.,0.],vec![2.,0.,0.]],weights:vec![1.;3],periodic:false};
        #[cfg(feature="transport")]
        {
            let request=value_codec::json!({"op":"curve_decomposition_batch_audit","pairs":[
                {"curve":c,"span":2,"retained":c},{"curve":c,"span":2,"retained":c}],"maxProducts":1000});
            let r=crate::transport::dispatch(request.clone()).unwrap();
            assert!(r["errorUpper"].as_f64().is_some());
            assert_eq!(r["pairsInspected"],2);
            let mut zero=request; zero["maxProducts"]=value_codec::json!(-1);
            let r=crate::transport::dispatch(zero).unwrap();
            assert!(r["errorUpper"].is_null()); assert_eq!(r["reason"],"work-limit");
        }
        let pairs=[(&c,2,&c),(&c,2,&c)];
        let r=inspect_batch(&pairs,1000).unwrap();
        assert!(r.error_upper.is_some() && r.pairs_inspected==2);
        let short=inspect_batch(&pairs,r.products-1).unwrap();
        assert!(short.error_upper.is_none() && short.pairs_inspected==2 && short.products<r.products);
        assert!(inspect_batch(&[],1000).unwrap().error_upper.is_none());
        assert!(inspect_batch(&[(&c,2,&c),(&c,0,&c)],1000).unwrap().error_upper.is_none());
    }
}

mod foundation_hermite {
    #[allow(unused_imports)]
    use crate::foundation::hermite::*;

    #[cfg(feature = "transport")]
    #[test]
    fn json_dispatch_preserves_the_same_authored_constraints() {
        let points = [[0., 0., 0.], [3., 2., 1.]];
        let tangents = [[1., 0., 0.], [0., 1., 0.]];
        let parameters = [-1., 2.];
        let direct = interpolate(&points, &tangents, &parameters).unwrap();
        let value=crate::dispatch(value_codec::json!({"op":"curve_hermite","points":points,"tangents":tangents,"parameters":parameters})).unwrap();
        assert_eq!(value_codec::to_value(direct).unwrap(), value);
    }
}

mod foundation_natural_spline {
    #[allow(unused_imports)]
    use crate::foundation::natural_spline::*;

    #[cfg(feature = "transport")]
    #[test]
    fn json_dispatch_matches_native_interpolation() {
        let points = [[0., 0., 0.], [1., 2., 0.], [3., 0., 1.]];
        let parameters = [0., 1., 3.];
        let direct = interpolate(&points, &parameters).unwrap();
        let value=crate::dispatch(value_codec::json!({"op":"curve_natural_spline","points":points,"parameters":parameters})).unwrap();
        assert_eq!(value_codec::to_value(direct).unwrap(), value);
    }
}

#[cfg(feature = "transport")]
mod foundation_reparameterization_materialization {
    use value_codec::{json, Value};
    use crate::curve::Curve;

    fn source() -> Curve {
        Curve {
            degree: 1,
            knots: vec![0., 0., 0.5, 1., 1.],
            control_points: vec![vec![0., 0.], vec![0.5, 1.], vec![1., 0.]],
            weights: vec![1., 0.75, 1.],
            periodic: false,
        }
    }

    fn mapping() -> Value {
        json!({"pieces":[{"domain":[0.,1.],"range":[0.,1.],"controlValues":[0.,0.25,1.],"weights":[1.,1.,1.]}]})
    }

    #[cfg(feature = "transport")]
    #[test]
    fn json_materialization_reports_numerical_retention_and_refuses_depleted_work() {
        let request = json!({"op":"curve_materialize_reparameterization_bounded","curve":source(),"mapping":mapping(),
            "errorBudget":1e-5,"maxCells":50000,"maxMapEvaluations":200000});
        let mapped = crate::dispatch(request.clone()).unwrap();
        assert_eq!(mapped["certificate"]["retention"]["accepted"], true);
        let mut depleted = request;
        depleted["maxCells"] = json!(1);
        assert!(crate::dispatch(depleted).is_err());
    }
}
