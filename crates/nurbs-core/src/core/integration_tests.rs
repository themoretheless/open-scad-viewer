//! Tests of `nurbs-geometry` that need higher nurbs-core layers (paths, primitives, sweeps,
//! transport). They moved here unchanged when `core` became its own crate.

mod affine {
    #[allow(unused_imports)]
    use crate::core::affine::*;
    const M: Matrix = [
        [-2., 0.5, 0., 3.],
        [0., 1., 1., -4.],
        [1., 0., 0.25, 2.],
        [0., 0., 0., 1.],
    ];

    #[test]
    fn affine_weighted_curve_preserves_rational_data_and_commutes_with_evaluation() {
        let source = crate::paths::bezier(
            vec![vec![0., 0., 0.], vec![2., 3., 1.], vec![4., 0., 2.]],
            Some(vec![1., 2., 1.]),
        )
        .unwrap();
        let transformed = curve(&source, &M).unwrap();
        assert_eq!(transformed.weights, source.weights);
        assert_eq!(transformed.knots, source.knots);
        for i in 0..=100 {
            let t = i as f64 / 100.;
            let p = source.evaluate(t).unwrap().point;
            let expected = [
                -2. * p[0] + 0.5 * p[1] + 3.,
                p[1] + p[2] - 4.,
                p[0] + 0.25 * p[2] + 2.,
            ];
            let q = transformed.evaluate(t).unwrap().point;
            assert!(q.iter().zip(expected).all(|(x, y)| (x - y).abs() < 1e-11));
        }
    }

    #[test]
    fn surfaces_and_patch_sets_retain_basis_and_independent_plane_equation() {
        let line = crate::primitives::line([0.; 3], [2., 0., 0.]).unwrap();
        let source = crate::surface::extrude(&line, [0., 3., 4.]).unwrap();
        let transformed = surface(&source, &M).unwrap();
        assert_eq!(transformed.weights, source.weights);
        assert_eq!(transformed.knots_u, source.knots_u);
        for i in 0..=20 {
            for j in 0..=20 {
                let u = i as f64 / 20.;
                let v = j as f64 / 20.;
                let p = transformed.evaluate(u, v).unwrap().point;
                let expected = [-4. * u + 1.5 * v + 3., 7. * v - 4., 2. * u + v + 2.];
                assert!(p.iter().zip(expected).all(|(x, y)| (x - y).abs() < 1e-11));
            }
        }
        assert_eq!(
            patches(&[source.clone(), source], &M).unwrap(),
            vec![transformed.clone(), transformed]
        );
    }

    #[test]
    fn invalid_projective_nonfinite_dimension_and_output_range_are_refused() {
        let mut source = crate::primitives::line([0.; 3], [1., 0., 0.]).unwrap();
        let mut m = M;
        m[3][0] = 0.1;
        assert!(curve(&source, &m).is_err());
        m = M;
        m[0][0] = f64::NAN;
        assert!(curve(&source, &m).is_err());
        m = M;
        m[0][3] = 1e6 + 1.;
        assert!(curve(&source, &m).is_err());
        source.control_points.iter_mut().for_each(|p| {
            p.pop();
        });
        assert!(curve(&source, &M).is_err());
    }
}

mod exact_curve_segments {
    #[allow(unused_imports)]
    use crate::core::exact_curve_segments::*;
    use crate::core::curve::Curve;

    #[test]
    fn unclamped_exact_blossoms_certify_hollow_original_domains() {
        let c = Curve {
            degree: 2,
            knots: (0..9).map(|i| i as f64).collect(),
            control_points: vec![
                vec![1., 0., 0.],
                vec![0., 1., 0.],
                vec![-1., 0., 0.],
                vec![0., -1., 0.],
                vec![1., 0., 0.],
                vec![0., 1., 0.],
            ],
            weights: vec![1.; 6],
            periodic: false,
        };
        let before = c.clone();
        let pieces = inspect(&c).unwrap();
        assert_eq!(pieces.len(), 4);
        for (span, piece) in (2..6).zip(pieces) {
            assert_eq!(piece.domain(), [span as f64, (span + 1) as f64]);
            let bound =
                crate::curve_decomposition_certificate::inspect(&c, span, &piece, 9).unwrap();
            assert!(bound.error_upper.unwrap() < 1e-10, "{bound:?}");
        }
        let mut hole = c.clone();
        for point in &mut hole.control_points {
            for x in point {
                *x *= 0.25;
            }
        }
        let report =
            crate::sweep_contour_audit::inspect(&[vec![c.clone()], vec![hole]], 1e-6, 10000, 10000)
                .unwrap();
        assert!(report.cap_domain_certified, "{report:?}");
        let scale = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![1., 0., 0.]; 2],
            weights: vec![1.; 2],
            periodic: false,
        };
        let twist = Curve {
            control_points: vec![vec![0., 0., 0.]; 2],
            ..scale.clone()
        };
        let points = [[0., 0., 0.], [0., 0., 10.]];
        let profiles = [c.clone()];
        let sweep = crate::sweeps::progressive_miter::Sweep::new(
            &profiles,
            &points,
            &scale,
            &twist,
            crate::sweeps::progressive_miter::Options {
                normal: [1., 0., 0.],
                closed: false,
                miter_limit: 4.,
                initial_steps: 1,
                max_steps: 1,
                max_deviation: 0.001,
            },
        )
        .unwrap();
        let sections = sweep.sections_at(1).unwrap();
        // Zero analysis budgets isolate the exact ownership gate, without
        // claiming wall injectivity or intersection classification.
        assert!(
            sweep
                .inspect_wall_geometry_with_loops(&sections, &[1], 0., 1e-6, 0, 0, 0)
                .is_ok()
        );
        let mut open = sections.clone();
        open[1][0].control_points[5][0] += 0.125;
        assert!(
            sweep
                .inspect_wall_geometry_with_loops(&open, &[1], 0., 1e-6, 0, 0, 0)
                .is_err()
        );
        let mut rounded = c.clone();
        // The endpoint's weighted mean now contains 1/3, not a dyadic.
        rounded.weights[1] = 2.;
        assert!(inspect(&rounded).is_none());
        assert!(translated_active_seam_closed(&rounded) == false);
        let mut nonrepresentable = c.clone();
        nonrepresentable.control_points[0][0] = 1.;
        nonrepresentable.control_points[1][0] = f64::from_bits(1f64.to_bits() + 1);
        nonrepresentable.control_points[4] = nonrepresentable.control_points[0].clone();
        nonrepresentable.control_points[5] = nonrepresentable.control_points[1].clone();
        assert!(inspect(&nonrepresentable).is_none());
        assert!(translated_active_seam_closed(&nonrepresentable));
        let mut nonrepresentable_sections = sections.clone();
        nonrepresentable_sections[1][0] = nonrepresentable.clone();
        assert!(
            sweep
                .inspect_wall_geometry_with_loops(
                    &nonrepresentable_sections,
                    &[1],
                    0.,
                    1e-6,
                    0,
                    0,
                    0
                )
                .is_ok()
        );

        let mut bad_knots = nonrepresentable.clone();
        bad_knots.knots[1] = 1f64.next_up();
        assert!(!translated_active_seam_closed(&bad_knots));
        let mut bad_weight = nonrepresentable.clone();
        bad_weight.weights[5] = 2.;
        assert!(!translated_active_seam_closed(&bad_weight));

        let mut periodic = c.clone();
        periodic.periodic = true;
        let periodic_pieces = inspect(&periodic).unwrap();
        assert_eq!(periodic_pieces.len(), 4);
        for (span, piece) in (2..6).zip(periodic_pieces) {
            let bound = crate::curve_decomposition_certificate::inspect(&periodic, span, &piece, 9)
                .unwrap();
            assert!(bound.error_upper.unwrap() < 1e-10, "{bound:?}");
            assert!(
                crate::curve_decomposition_certificate::inspect(&periodic, span, &piece, 8)
                    .unwrap()
                    .error_upper
                    .is_none()
            );
            let mut shifted = piece.clone();
            for point in &mut shifted.control_points {
                point[2] += 0.125;
            }
            assert!(
                crate::curve_decomposition_certificate::inspect(&periodic, span, &shifted, 9)
                    .unwrap()
                    .error_upper
                    .unwrap()
                    >= 0.125
            );
        }
        let mut hole = periodic.clone();
        for point in &mut hole.control_points {
            for x in point {
                *x *= 0.25;
            }
        }
        let domain = crate::sweep_contour_audit::inspect(
            &[vec![periodic.clone()], vec![hole]],
            1e-6,
            10000,
            10000,
        )
        .unwrap();
        assert!(domain.cap_domain_certified, "{domain:?}");
        let mut almost_periodic = periodic.clone();
        almost_periodic.knots[1] = 1f64.next_up();
        almost_periodic.validate().unwrap();
        assert!(inspect(&almost_periodic).is_none());
        // A periodic flag is not evidence of active-domain endpoint ownership.
        // Isolate that premise using zero downstream analysis budgets.
        for (profile, expected_closed) in [(periodic, true), (almost_periodic, false)] {
            let profiles = [profile];
            let sweep = crate::sweeps::progressive_miter::Sweep::new(
                &profiles,
                &points,
                &scale,
                &twist,
                crate::sweeps::progressive_miter::Options {
                    normal: [1., 0., 0.],
                    closed: false,
                    miter_limit: 4.,
                    initial_steps: 1,
                    max_steps: 1,
                    max_deviation: 0.001,
                },
            )
            .unwrap();
            let sections = sweep.sections_at(1).unwrap();
            let result = sweep.inspect_wall_geometry_with_loops(&sections, &[1], 0., 1e-6, 0, 0, 0);
            assert_eq!(result.is_ok(), expected_closed, "{result:?}");
        }
        assert_eq!(c, before);
    }

    #[test]
    fn refuses_rounded_arithmetic_and_certifies_exact_spans() {
        assert_eq!(exact_add(1., 2f64.powi(-54)), None);
        assert_eq!(exact_mul(f64::from_bits(1), 0.5), None);
        assert_eq!(exact_div(1., 3.), None);
        assert_eq!(exact_mul(f64::MAX, 2.), None);
        assert_eq!(exact_div(f64::from_bits(1), 2.), None);
        assert_eq!(exact_add(f64::MAX, f64::MAX), None);
        assert_eq!(exact_mul(f64::from_bits(1), 1.), Some(f64::from_bits(1)));
        assert_eq!(exact_div(f64::from_bits(1), 1.), Some(f64::from_bits(1)));
        assert_eq!(exact_add(-0., f64::from_bits(1)), Some(f64::from_bits(1)));
        assert_eq!(exact_add(1., -1.), Some(0.));
        let c = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 0.25, 0.5, 0.75, 1., 1., 1.],
            control_points: vec![
                vec![1., 0., 0.],
                vec![1., 1., 0.],
                vec![-1., 1., 0.],
                vec![-1., -1., 0.],
                vec![1., -1., 0.],
                vec![1., 0., 0.],
            ],
            weights: vec![1.; 6],
            periodic: false,
        };
        let domain =
            crate::sweep_contour_audit::inspect(&[vec![c.clone()]], 1e-6, 10000, 10000).unwrap();
        assert!(domain.cap_domain_certified, "{domain:?}");
        let mut hole = c.clone();
        for point in &mut hole.control_points {
            point[0] *= 0.25;
            point[1] *= 0.25;
        }
        let hollow =
            crate::sweep_contour_audit::inspect(&[vec![c.clone()], vec![hole]], 1e-6, 10000, 10000)
                .unwrap();
        assert!(hollow.cap_domain_certified, "{hollow:?}");
        let pieces = inspect(&c).unwrap();
        assert_eq!(pieces.len(), 4);
        for piece in pieces {
            let [a, b] = piece.domain();
            for t in [0., 0.25, 0.5, 0.75, 1.] {
                let u = a + (b - a) * t;
                let original = c.evaluate(u).unwrap().point;
                let retained = piece.evaluate(u).unwrap().point;
                assert!(
                    original
                        .iter()
                        .zip(retained)
                        .all(|(a, b)| (a - b).abs() < 1e-12)
                );
            }
        }
    }
}

mod geometry {
    #[allow(unused_imports)]
    use crate::core::geometry::*;

    #[test]
    fn curve_method_agrees_with_basis_and_de_boor_on_rational_bezier() {
        let curve = crate::paths::bezier(
            vec![
                vec![0., 0., 0.],
                vec![1., 2., 0.],
                vec![3., 2., 1.],
                vec![4., 0., 1.],
            ],
            Some(vec![1., 2., 0.5, 1.5]),
        )
        .unwrap();
        for i in 0..=20 {
            let u = i as f64 / 20.;
            let direct = curve.evaluate_bezier_de_casteljau(u).unwrap().0;
            let basis = curve.evaluate(u).unwrap().point;
            let de_boor = curve.evaluate_de_boor(u).unwrap();
            for axis in 0..3 {
                assert!((direct[axis] - basis[axis]).abs() < 1e-12);
                assert!((direct[axis] - de_boor[axis]).abs() < 1e-12);
            }
        }
        let line = crate::primitives::line([0.; 3], [1., 0., 0.]).unwrap();
        assert!(line.evaluate_bezier_de_casteljau(0.5).is_ok());
        let multispan =
            crate::primitives::polyline(&[[0., 0., 0.], [1., 0., 0.], [2., 0., 0.]], false)
                .unwrap();
        assert!(multispan.evaluate_bezier_de_casteljau(0.5).is_err());
    }
}

mod retained_wall_coefficients {
    #[allow(unused_imports)]
    use crate::core::retained_wall_coefficients::*;
    use crate::core::{curve::Curve, surface::Surface};
    // Copy of the fixture the remaining nurbs-geometry tests also use; test items are not
    // visible across crates.
    fn fixture()->(Surface,Curve,Curve) {
        let a=Curve{degree:2,knots:vec![7.,7.,7.,19.,19.,19.],
            control_points:vec![vec![0.,0.,0.],vec![1.,1.,0.],vec![2.,0.,0.]],
            weights:vec![1.,0.5,1.],periodic:false};
        let mut b=a.clone(); b.knots=vec![2.,2.,2.,5.,5.,5.];
        for point in &mut b.control_points {point[2]=4.;}
        let s=Surface{degree_u:2,degree_v:1,knots_u:vec![0.,0.,0.,1.,1.,1.],
            knots_v:vec![0.,0.,1.,1.],control_points:(0..3).map(|i|vec![a.control_points[i].clone(),b.control_points[i].clone()]).collect(),
            weights:(0..3).map(|i|vec![a.weights[i],b.weights[i]]).collect(),periodic_u:false,periodic_v:false};
        (s,a,b)
    }

    #[cfg(feature="transport")]
    #[test]
    fn contour_json_boundary_does_not_claim_a_filled_region() {
        let (_,mut a,_)=fixture();a.knots=vec![0.,0.,0.,1.,1.,1.];
        let r=crate::transport::dispatch(value_codec::json!({
            "op":"sweep_retained_cap_contour_audit","expected":[a.clone()],
            "actual":[a],"reversed":[false],"maxWork":3})).unwrap();
        assert_eq!(r["contourIdentity"].as_bool(),Some(true));
        assert_eq!(r["work"].as_u64(),Some(3));
        assert_eq!(r["filledRegionCertified"].as_bool(),Some(false));
    }

    #[cfg(feature="transport")]
    #[test]
    fn family_and_segmentation_json_boundaries_preserve_proof_scope() {
        let(s,a,b)=fixture();let r=crate::transport::dispatch(value_codec::json!({
            "op":"curve_segmented_bezier_controls","curve":a.clone(),"maxControlRows":3})).unwrap();
        let parts:value_codec::Value=r;assert_eq!(parts.as_array().unwrap().len(),1);
        let r=crate::transport::dispatch(value_codec::json!({"op":"sweep_retained_wall_family_audit",
            "surfaces":[s.clone(),s.clone(),s],"sections":[[[a]],[[b]]],"closed":false,"maxFaces":1})).unwrap();
        assert_eq!(r["coefficientFamilyIdentity"].as_bool(),Some(true));
        assert_eq!(r["globalEmbeddingCertified"].as_bool(),Some(false));
    }

    #[cfg(feature="transport")]
    #[test]
    fn json_boundary_reports_only_coefficient_identity() {
        let(s,a,b)=fixture();let r=crate::transport::dispatch(value_codec::json!({
            "op":"sweep_retained_wall_coefficients_audit","surface":s,"start":a,"end":b,"maxControls":3})).unwrap();
        assert_eq!(r["coefficientIdentity"].as_bool(),Some(true));
        assert_eq!(r["globalEmbeddingCertified"].as_bool(),Some(false));
    }
}
