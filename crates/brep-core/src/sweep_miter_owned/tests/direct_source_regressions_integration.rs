    use super::*;
    #[test]
    fn direct_explicit_circle_repair_preserves_original_bound_and_saved_profile_g2() {
        let outer = nurbs_core::primitives::circle([0.; 3], [1., 0., 0.], 0.5).unwrap();
        let hole = nurbs_core::primitives::circle([0.; 3], [1., 0., 0.], 0.2)
            .unwrap()
            .reverse()
            .unwrap();
        let loops = vec![vec![outer], vec![hole]];
        let points = [
            [0., 0., 0.],
            [10., 0., 0.],
            [13., 9., 4e-12],
            [-2., 11., 1e-12],
            [1., 5., -2e-12],
        ];
        let mut request = DirectRequest {
            loops: &loops,
            points: &points,
            normal: [0., 0., 1.],
            closed: true,
            miter_limit: 4.,
            max_deviation: 0.01,
            circle: None,
            caps: None,
            cap_endpoints: None,
        };
        let raw = construct_direct(&request, &limits()).unwrap();
        request.circle = Some(Budget {
            quantum: 2_f64.powi(-40),
            tolerance: 1e-9,
            max_work: Some(1000000.),
        });
        let corrected = construct_direct(&request, &limits()).unwrap();
        assert_eq!(raw.source_sections(), corrected.source_sections());
        assert!(
            corrected
                .correction()
                .unwrap()
                .wall_displacement_upper
                .unwrap()
                > 0.
        );
        assert!(corrected.complete_boundary().is_some());
        let smooth =
            crate::sweep_smoothness::inspect_profile(corrected.model(), &[], 2_000_000).unwrap();
        assert!(
            smooth.profile.g2_certified,
            "Saved corrected profile G2 unproved"
        );
        request.circle.as_mut().unwrap().max_work = Some(0.);
        assert!(construct_direct(&request, &limits()).is_err());
    }
    #[test]
    fn direct_owner_keeps_legacy_hollow_geometry_and_requires_filled_caps() {
        use nurbs_core::construct::curves::paths::{closed_miter_sections, miter_sections};
        for closed in [false, true] {
            let axis = if closed { [1., 0., 0.] } else { [0., 0., 1.] };
            let outer = nurbs_core::primitives::circle([0.; 3], axis, 0.5).unwrap();
            let mut hole = nurbs_core::primitives::circle([0.; 3], axis, 0.2).unwrap();
            hole.control_points.reverse();
            hole.weights.reverse();
            let lo = hole.knots[0];
            let hi = *hole.knots.last().unwrap();
            hole.knots = hole.knots.iter().rev().map(|u| lo + hi - u).collect();
            let loops = vec![vec![outer], vec![hole]];
            let before = loops.clone();
            let points = if closed {
                vec![[0., 0., 0.], [10., 0., 0.], [10., 10., 0.], [0., 10., 0.]]
            } else {
                vec![[0., 0., 0.], [0., 0., 10.]]
            };
            let request = DirectRequest {
                loops: &loops,
                points: &points,
                normal: if closed { [0., 0., 1.] } else { [1., 0., 0.] },
                closed,
                miter_limit: 2.,
                max_deviation: 0.01,
                circle: None,
                caps: None,
                cap_endpoints: None,
            };
            let profiles = loops.iter().flatten().cloned().collect::<Vec<_>>();
            let direct = if closed {
                closed_miter_sections(&profiles, &points, request.normal, 2.)
            } else {
                miter_sections(&profiles, &points, request.normal, 2.)
            }
            .unwrap();
            let sections = sweep_miter_layout::partition(&direct, &[1, 1]).unwrap();
            let legacy = if closed {
                crate::periodic_section_loft(&sections)
            } else {
                crate::rational_section_loft(&sections)
            }
            .unwrap();
            let body = construct_direct(&request, &limits()).unwrap();
            assert_eq!(
                body.model().0,
                legacy.0,
                "Direct owner changed retained geometry"
            );
            assert_eq!(body.source_sections(), direct);
            assert!(body.correction().is_none());
            assert!(
                body.complete_boundary().is_some(),
                "closed={closed}, wall={:?}, caps={:?}, within={:?}",
                body.boundary().wall_error_upper,
                body.boundary().filled_cap_error_upper,
                body.boundary().within_budget
            );
            assert_eq!(body.boundary().filled_cap_error_upper.is_some(), !closed);
            assert_eq!(loops, before);
            if !closed {
                let mut denied = limits();
                denied.domain_exact_work = 1;
                let unproved = construct_direct(&request, &denied).unwrap();
                assert_eq!(unproved.model().0, legacy.0);
                assert!(unproved.boundary().wall_error_upper.is_some());
                assert!(unproved.complete_boundary().is_none());
            }
        }
    }
    #[test]
    fn direct_spatial_owner_does_not_hide_nonplanar_caps_with_automatic_repair() {
        let outer = nurbs_core::primitives::circle([0.; 3], [0., 0., 1.], 0.5).unwrap();
        let hole = nurbs_core::primitives::circle([0.; 3], [0., 0., -1.], 0.2).unwrap();
        let loops = vec![vec![outer], vec![hole]];
        let points = [[0., 0., 0.], [0., 0., 10.], [10., 0., 10.], [10., 10., 15.]];
        let request = DirectRequest {
            loops: &loops,
            points: &points,
            normal: [1., 0., 0.],
            closed: false,
            miter_limit: 2.,
            max_deviation: 0.01,
            circle: None,
            caps: None,
            cap_endpoints: None,
        };
        let direct = nurbs_core::construct::curves::paths::miter_sections(
            &loops.iter().flatten().cloned().collect::<Vec<_>>(),
            &points,
            request.normal,
            2.,
        )
        .unwrap();
        let sections = sweep_miter_layout::partition(&direct, &[1, 1]).unwrap();
        let legacy = crate::rational_section_loft(&sections).unwrap();
        let body = construct_direct(&request, &limits()).unwrap();
        assert_eq!(body.model().0, legacy.0);
        assert_eq!(body.source_sections(), direct);
        assert!(body.correction().is_none());
        assert!(body.boundary().wall_error_upper.is_some());
        assert!(
            body.complete_boundary().is_none(),
            "Raw spatial cap gaps were accepted"
        );
    }
    #[test]
    fn annular_mode_owns_original_spatial_hollow_and_all_logical_cap_faces() {
        let loops = vec![
            vec![nurbs_core::primitives::circle([0.; 3], [0., 0., 1.], 0.5).unwrap()],
            vec![nurbs_core::primitives::circle([0.; 3], [0., 0., -1.], 0.2).unwrap()],
        ];
        let points = [[0., 0., 0.], [0., 0., 10.], [10., 0., 10.], [10., 10., 15.]];
        let request = DirectRequest {
            loops: &loops,
            points: &points,
            normal: [1., 0., 0.],
            closed: false,
            miter_limit: 2.,
            max_deviation: 0.01,
            circle: None,
            caps: None,
            cap_endpoints: None,
        };
        let body = annular::construct_direct(&request, &limits()).unwrap();
        assert!(body.source().complete_boundary().is_none());
        assert!(
            body.complete_boundary().is_some(),
            "{:?}",
            body.boundary().reason
        );
        assert_eq!(body.model().faces.len(), 32);
        assert_eq!(
            body.cap_face_groups(),
            &[vec![24, 25, 26, 27], vec![28, 29, 30, 31]]
        );
        assert_eq!(body.cap_faces().len(), 8);
        assert!(body.source().correction().is_none());
        assert_eq!(body.model().vertices, body.source().model().vertices);
        assert_eq!(body.model().faces[..24], body.source().model().faces[..24]);
        assert!(body.filled_caps().unwrap().source_filling_covered);
        let walls = crate::sweep_retained_charts::inspect(body.model(), &body.cap_faces(), 100_000)
            .unwrap();
        assert!(walls.certified);
        assert_eq!(walls.charts.len(), 24);
        let mut denied = limits();
        denied.domain_exact_work = 1;
        let unproved = annular::construct_direct(&request, &denied).unwrap();
        assert!(unproved.complete_boundary().is_none());
        assert!(unproved.boundary().wall_error_upper.is_some());
    }
    #[test]
    fn annular_explicit_circle_correction_keeps_source_bound_and_retained_profile_g2() {
        let loops = vec![
            vec![nurbs_core::primitives::circle([0.; 3], [0., 0., 1.], 0.5).unwrap()],
            vec![nurbs_core::primitives::circle([0.; 3], [0., 0., -1.], 0.2).unwrap()],
        ];
        let points = [[0., 0., 0.], [0., 0., 10.], [10., 0., 10.], [10., 10., 15.]];
        let request = DirectRequest {
            loops: &loops,
            points: &points,
            normal: [1., 0., 0.],
            closed: false,
            miter_limit: 2.,
            max_deviation: 0.01,
            circle: Some(Budget {
                quantum: 2_f64.powi(-40),
                tolerance: 1e-9,
                max_work: Some(1000000.),
            }),
            caps: None,
            cap_endpoints: None,
        };
        let body = annular::construct_direct(&request, &limits()).unwrap();
        let caps = body.filled_caps().unwrap();
        assert!(
            body.complete_boundary().is_some(),
            "{:?}: {:?}; {}",
            body.boundary().reason,
            body.boundary().error_upper,
            caps.reason
        );
        assert!(
            caps.filled_cap_error_upper
                .unwrap()
                .iter()
                .all(|&b| b < 1e-9)
        );
        assert!(
            body.source()
                .correction()
                .unwrap()
                .wall_displacement_upper
                .unwrap()
                < 1e-9
        );
        let smooth = body.inspect_smoothness(2_000_000).unwrap();
        assert!(
            smooth.walls.profile.g1_certified && smooth.walls.profile.g2_certified,
            "Retained corrected wall G1/G2 is unproved"
        );
        assert_eq!(smooth.walls.cap_continuity, "C0");
        assert_eq!(smooth.cap_seams.len(), 2);
        assert!(
            smooth
                .cap_seams
                .iter()
                .all(|r| r.g1_certified && r.g2_certified),
            "Retained internal cap G1/G2 is unproved"
        );
        assert!(smooth.exact_work <= 2_000_000);
        assert_eq!(smooth.cap_seams[0].max_work, 2_000_000);
        assert_eq!(
            smooth.cap_seams[1].max_work,
            smooth.cap_seams[0].max_work - smooth.cap_seams[0].exact_work
        );
        let cap_work = smooth.cap_seams.iter().map(|r| r.exact_work).sum::<u64>();
        assert_eq!(smooth.walls.profile.max_work, 2_000_000 - cap_work);
        assert_eq!(smooth.walls.station.max_work, smooth.walls.profile.max_work - smooth.walls.profile.exact_work);
        assert_eq!(smooth.exact_work, cap_work + smooth.walls.total_exact_work);
        let required_g2_work = cap_work + smooth.walls.profile.exact_work;
        let enough = body.inspect_smoothness(required_g2_work).unwrap();
        assert!(enough.walls.profile.g2_certified && enough.cap_seams.iter().all(|r| r.g2_certified));
        let short = body.inspect_smoothness(required_g2_work - 1).unwrap();
        assert!(short.exact_work <= required_g2_work - 1);
        assert!(
            !short.walls.profile.g2_certified || short.cap_seams.iter().any(|r| !r.g2_certified)
        );
    }
    #[test]
    fn annular_source_caps_cover_all_affine_authored_frame_guide_combinations() {
        let loops = vec![
            vec![nurbs_core::primitives::circle([0.; 3], [0., 0., 1.], 0.5).unwrap()],
            vec![nurbs_core::primitives::circle([0.; 3], [0., 0., -1.], 0.2).unwrap()],
        ];
        let points = [[0., 0., 0.], [0., 0., 10.]];
        let mut scale = scalar(1.);
        scale.control_points[1][0] = 1.125;
        let mut twist = scalar(0.);
        twist.control_points[1][0] = 0.1;
        let vector = |a: [f64; 3], b: [f64; 3]| Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![a.to_vec(), b.to_vec()],
            weights: vec![1.; 2],
            periodic: false,
        };
        let axes = vector([2., 1., 1.], [2., 1.25, 1.]);
        let center = vector([0.125, -0.25, 0.], [0.25, 0.125, 0.]);
        let axis = vector([0., 0., 2.], [0., 0.5, 2.]);
        let normal = vector([1., 0., 0.], [1., 0., 0.]);
        let guide = vector([1., 0., 0.], [1., 0., 10.]);
        for mask in 0..8 {
            let request = Request {
                loops: &loops,
                points: &points,
                scale: &scale,
                twist: &twist,
                options: progressive_miter::Options {
                    normal: [1., 0., 0.],
                    closed: false,
                    miter_limit: 4.,
                    initial_steps: 1,
                    max_steps: 16,
                    max_deviation: 0.25,
                },
                affine: (mask & 1 != 0).then_some((&axes, &center)),
                frames: (mask & 2 != 0).then_some((&axis, &normal)),
                guide: (mask & 4 != 0).then_some(&guide),
                circle: None,
                caps: None,
            };
            let body = annular::construct(&request, &limits()).unwrap();
            assert!(
                body.complete_boundary().is_some(),
                "mask={mask}: {:?}",
                body.boundary().reason
            );
            assert!(body.filled_caps().expect("explicit annular mode owns radial caps").within_budget, "mask={mask}");
            assert_eq!(body.complete_boundary().unwrap().within_budget,Some(true));
            assert_eq!(
                body.cap_face_groups()
                    .iter()
                    .map(Vec::len)
                    .collect::<Vec<_>>(),
                vec![4, 4]
            );
            let count = body.source().model().faces.len() - 2;
            assert_eq!(
                body.model().faces[..count],
                body.source().model().faces[..count]
            );
        }
    }
    #[test]
    fn direct_spatial_explicit_caps_keep_legacy_correction_and_complete_bound() {
        let loops = vec![
            vec![nurbs_core::primitives::circle([0.; 3], [0., 0., 1.], 0.5).unwrap()],
            vec![nurbs_core::primitives::circle([0.; 3], [0., 0., -1.], 0.2).unwrap()],
        ];
        let points = [[0., 0., 0.], [0., 0., 10.], [10., 0., 10.], [10., 10., 15.]];
        let correction = || CapCorrection {
            budget: Budget {
                quantum: 2_f64.powi(-40),
                tolerance: 1e-9,
                max_work: Some(1000000.),
            },
            authored_frame: false,
        };
        let direct = nurbs_core::construct::curves::paths::miter_sections(
            &loops.iter().flatten().cloned().collect::<Vec<_>>(),
            &points,
            [1., 0., 0.],
            2.,
        )
        .unwrap();
        let legacy_correction = miter_section_correction::correct(
            &direct,
            &points,
            false,
            None,
            Some(correction()),
            None,
        )
        .unwrap()
        .unwrap();
        let sections =
            sweep_miter_layout::partition(legacy_correction.sections.as_ref().unwrap(), &[1, 1])
                .unwrap();
        let legacy = crate::rational_section_loft(&sections).unwrap();
        let request = DirectRequest {
            loops: &loops,
            points: &points,
            normal: [1., 0., 0.],
            closed: false,
            miter_limit: 2.,
            max_deviation: 0.01,
            circle: None,
            caps: Some(correction()),
            cap_endpoints: None,
        };
        let body = construct_direct(&request, &limits()).unwrap();
        assert_eq!(body.model().0, legacy.0);
        assert_eq!(body.source_sections(), direct);
        assert_eq!(
            body.correction().unwrap().wall_displacement_upper,
            legacy_correction.wall_displacement_upper
        );
        assert!(
            body.complete_boundary().is_some(),
            "Explicit corrected cap bound still unproved"
        );
        assert!(body.boundary().filled_cap_error_upper.is_some());
        let mut denied = request;
        denied.caps.as_mut().unwrap().budget.max_work = Some(0.);
        assert!(construct_direct(&denied, &limits()).is_err());
    }
    #[test]
    fn direct_owned_affine_preserves_raw_legacy_models_and_bounds_filled_caps() {
        let quarters = |r: f64| {
            [
                [[r, 0.], [r, r], [0., r]],
                [[0., r], [-r, r], [-r, 0.]],
                [[-r, 0.], [-r, -r], [0., -r]],
                [[0., -r], [r, -r], [r, 0.]],
            ]
            .map(|p| Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                control_points: p.map(|[x, y]| vec![x, y, 0.]).to_vec(),
                weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
                periodic: false,
            })
            .to_vec()
        };
        let mut hole = quarters(0.25);
        hole.reverse();
        for c in &mut hole {
            c.control_points.reverse();
            c.weights.reverse();
        }
        let examples = [
            vec![
                vec![nurbs_core::primitives::circle([0.; 3], [0., 0., 1.], 0.5).unwrap()],
                vec![nurbs_core::primitives::circle([0.; 3], [0., 0., -1.], 0.2).unwrap()],
            ],
            vec![quarters(0.5), hole],
        ];
        let matrix = [
            [1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [1., 1., 1., 0.],
            [0., 0., 0., 1.],
        ];
        for loops in examples {
            let points = [[0., 0., 0.], [0., 0., 10.]];
            let source = construct_direct(
                &DirectRequest {
                    loops: &loops,
                    points: &points,
                    normal: [1., 0., 0.],
                    closed: false,
                    miter_limit: 2.,
                    max_deviation: 0.01,
                    circle: None,
                    caps: None,
                    cap_endpoints: None,
                },
                &limits(),
            )
            .unwrap();
            assert!(source.complete_boundary().is_some());
            let expected = crate::transform::affine(source.model(), matrix).unwrap();
            let placed = source
                .place(matrix, 2_f64.powi(-40), 100000, Some(0.01))
                .unwrap();
            let boundary = placed.boundary.unwrap();
            assert!(boundary.continuous_bound);
            assert_eq!(boundary.within_budget, Some(true));
            assert!(boundary.filled_cap_error_upper.is_some());
            assert_eq!(
                placed.placement.unwrap().model.unwrap().0,
                expected.0,
                "Affine certificate changed the raw legacy geometry"
            );
            assert!(
                source
                    .place(matrix, 2_f64.powi(-40), 0, Some(0.01))
                    .unwrap()
                    .boundary
                    .is_none()
            );
            let denied = source
                .place(matrix, 2_f64.powi(-40), 100000, Some(0.))
                .unwrap();
            assert!(denied.placement.unwrap().model.is_none());
        }
    }
    #[test]
    fn selected_end_cap_owns_the_original_external_correction_without_touching_start() {
        use nurbs_core::sweep_section_projection::{Correction, Target};
        let loops = vec![
            vec![nurbs_core::primitives::circle([0.; 3], [0., 0., 1.], 0.5).unwrap()],
            vec![nurbs_core::primitives::circle([0.; 3], [0., 0., -1.], 0.2).unwrap()],
        ];
        let points = [[0., 0., 0.], [0., 0., 10.], [10., 0., 10.], [10., 10., 15.]];
        let direct = nurbs_core::construct::curves::paths::miter_sections(
            &loops.iter().flatten().cloned().collect::<Vec<_>>(),
            &points,
            [1., 0., 0.],
            2.,
        )
        .unwrap();
        let legacy = nurbs_core::sweep_section_projection::project(
            &direct,
            &[Correction {
                section: direct.len() - 1,
                quantum: 2_f64.powi(-40),
                tolerance: 1e-9,
                target: Target::Plane {
                    axis: 1,
                    coefficients: [0., -0.5],
                    offset: 17.5,
                },
            }],
            Some(1000000.),
        )
        .unwrap();
        let sections =
            sweep_miter_layout::partition(legacy.sections.as_ref().unwrap(), &[1, 1]).unwrap();
        let expected = crate::rational_section_loft(&sections).unwrap();
        let mut request = DirectRequest {
            loops: &loops,
            points: &points,
            normal: [1., 0., 0.],
            closed: false,
            miter_limit: 2.,
            max_deviation: 0.01,
            circle: None,
            caps: Some(CapCorrection {
                budget: Budget {
                    quantum: 2_f64.powi(-40),
                    tolerance: 1e-9,
                    max_work: Some(1000000.),
                },
                authored_frame: false,
            }),
            cap_endpoints: Some([false, true]),
        };
        let body = construct_direct(&request, &limits()).unwrap();
        assert_eq!(body.model().0, expected.0);
        assert_eq!(
            body.sections()[0],
            sweep_miter_layout::partition(&direct, &[1, 1]).unwrap()[0]
        );
        assert_eq!(body.correction().unwrap().work, legacy.work);
        assert!(
            body.complete_boundary().is_some(),
            "Selected end cap complete bound unproved"
        );
        request.caps.as_mut().unwrap().budget.max_work = Some((legacy.work - 1) as f64);
        assert!(construct_direct(&request, &limits()).is_err());
        request.caps.as_mut().unwrap().budget.max_work = Some(1000000.);
        request.cap_endpoints = Some([false, false]);
        assert!(construct_direct(&request, &limits()).is_err());
    }
    #[test]
    fn rational_lattice_preserves_basis_and_charges_bounded_atomic_displacement() {
        let c = |z: f64| Curve {
            degree: 2,
            knots: (0..=8).map(|i| i as f64).collect(),
            control_points: [
                [1., 0., z],
                [0., 1., z],
                [-1., 0., z],
                [0., -1., z],
                [1., 0., z],
                [0., 1., z],
            ]
            .map(Vec::from)
            .to_vec(),
            weights: vec![1., 0.5, 1., 1., 1., 0.5],
            periodic: true,
        };
        let original = vec![vec![c(0.)], vec![c(5.)], vec![c(10.)]];
        let before = original.clone();
        let q = 2_f64.powi(-40);
        let (repaired, error, work) = rational_periodic_lattice(&original, q, 1e-9, 10000)
            .unwrap()
            .unwrap();
        assert!(error > 0. && error < 1e-9);
        assert_eq!(original, before);
        for (a, b) in original.iter().flatten().zip(repaired.iter().flatten()) {
            assert_eq!(a.weights, b.weights);
            assert_eq!(a.knots, b.knots);
            assert!(
                a.control_points
                    .iter()
                    .zip(&b.control_points)
                    .all(|(a, b)| a[2] == b[2])
            );
            assert!(nurbs_core::retained_wall_coefficients::exact_bezier_controls(b, 12).is_some());
            assert!(nurbs_core::retained_wall_coefficients::exact_bezier_controls(b, 11).is_none());
        }
        assert!(
            rational_periodic_lattice(&original, q, 0., 10000)
                .unwrap()
                .is_none()
        );
        assert!(rational_periodic_lattice(&original, q, 1e-9, 0).is_err());
        assert!(rational_periodic_lattice(&original, q, 1e-9, work - 1).is_err());
        assert_eq!(
            rational_periodic_lattice(&original, q, 1e-9, work)
                .unwrap()
                .unwrap()
                .0,
            repaired
        );
        let sections = sweep_miter_layout::partition(&repaired, &[1]).unwrap();
        let model = crate::rational_section_loft(&sections).unwrap();
        let caps = [model.faces.len() - 2, model.faces.len() - 1];
        assert!(
            crate::sweep_smoothness::inspect_profile(&model, &caps, 2_000_000)
                .unwrap()
                .profile
                .g1_certified
        );
    }
    #[test]
    fn complete_boundary_refines_after_cap_correction_and_keeps_shared_work() {
        let outer = nurbs_core::primitives::circle([0.; 3], [0., 0., 1.], 0.5).unwrap();
        let hole = nurbs_core::primitives::circle([0.; 3], [0., 0., -1.], 0.2).unwrap();
        let loops = vec![vec![outer], vec![hole]];
        let before = loops.clone();
        let mut scale = scalar(1.);
        scale.control_points[1][0] = 1.25;
        let mut twist = scalar(0.);
        twist.control_points[1][0] = std::f64::consts::PI / 12.;
        let vector = |a: [f64; 3], b: [f64; 3]| Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![a.to_vec(), b.to_vec()],
            weights: vec![1., 1.],
            periodic: false,
        };
        let axis = vector([0., 0., 2.], [0., 2., 2.]);
        let normal = vector([3., 0., 0.], [3., 0., 0.]);
        let axes = vector([2., 1., 1.], [2., 1., 1.]);
        let center = vector([0.125, -0.25, 0.], [0.125, -0.25, 0.]);
        let mut request = Request {
            loops: &loops,
            points: &[[0., 0., 0.], [0., 0., 10.]],
            scale: &scale,
            twist: &twist,
            options: progressive_miter::Options {
                normal: [1., 0., 0.],
                closed: false,
                miter_limit: 4.,
                initial_steps: 1,
                max_steps: 16,
                max_deviation: 2.,
            },
            affine: Some((&axes, &center)),
            frames: Some((&axis, &normal)),
            guide: None,
            circle: None,
            caps: Some(CapCorrection {
                budget: Budget {
                    quantum: 2_f64.powi(-40),
                    tolerance: 2.,
                    max_work: Some(1000000.),
                },
                authored_frame: false,
            }),
        };
        let body = construct(&request, &limits()).unwrap();
        assert!(body.complete_boundary().is_some());
        assert!(body.levels().len() > 1);
        assert!(!body.levels()[0].accepted);
        let rejected = body
            .levels()
            .iter()
            .find(|r| r.error_certificate_reason == Some("complete-boundary-refinement"))
            .expect("The cap budget must cause additional refinement");
        assert!(!rejected.accepted);
        assert!(body.levels().last().unwrap().steps > rejected.steps);
        assert!(body.levels().last().unwrap().accepted);
        let work = body.correction().unwrap().work;
        request.caps.as_mut().unwrap().budget.max_work = Some((work - 1) as f64);
        assert!(construct(&request, &limits()).is_err());
        request.caps.as_mut().unwrap().budget.max_work = Some(1000000.);
        request.options.max_steps = 1;
        assert!(construct(&request, &limits()).is_err());
        assert_eq!(loops, before);
        eprintln!(
            "complete miter refinement levels={} last steps={} total correction work={work} error={:?}",
            body.levels().len(),
            body.levels().last().unwrap().steps,
            body.boundary().error_upper
        );
        let guide = nurbs_core::primitives::line([1., 0., 0.], [1., 0., 10.]).unwrap();
        request.guide = Some(&guide);
        request.options.max_steps = 16;
        let guided = construct(&request, &limits()).unwrap();
        assert!(guided.complete_boundary().is_some());
    }

    fn limits() -> Limits {
        Limits {
            max_products: 100000,
            max_faces: 1024,
            correspondence_faces: 1024,
            cap_max_edges: 1024,
            wall_cells: 10000,
            exact_work: 1000000,
            domain_exact_work: 1000000,
            projection_exact_work: 1000000,
            domain_tolerance: 0.001,
            domain_pairs: 1000,
            domain_cells: 10000,
            projection_cells: 10000,
            cap_regions: sweep_cap_contacts::Budgets {
                max_walls: 1024,
                max_exact_work: 1000000,
                max_chart_cells: 10000,
                max_trim_pairs: 10000,
                max_trim_cells: 100000,
                max_trim_domain_cells: 1000000,
            },
        }
    }
    fn scalar(value: f64) -> Curve {
        Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![value, 0., 0.], vec![value, 0., 0.]],
            weights: vec![1., 1.],
            periodic: false,
        }
    }
    #[test]
    fn periodic_profile_lattice_has_bounded_atomic_displacement() {
        let curve = Curve {
            degree: 2,
            knots: vec![0., 1., 2., 3., 4., 5., 6., 7., 8.],
            control_points: vec![
                vec![0.1, 0., 0.],
                vec![0., 1., 0.],
                vec![-1., 0., 0.],
                vec![0., -1., 0.],
                vec![0.1, 0., 0.],
                vec![0., 1., 0.],
            ],
            weights: vec![1.; 6],
            periodic: true,
        };
        let sections = vec![vec![curve]];
        let before = sections.clone();
        let result = periodic_profile_lattice(&sections, 2_f64.powi(-40), 1e-9, 1000)
            .unwrap()
            .unwrap();
        assert!(result.1 > 0. && result.1 < 1e-9);
        assert_eq!(sections, before);
        assert!(periodic_profile_lattice(&sections, 2_f64.powi(-40), 1e-9, result.2 - 1).is_err());
        assert!(periodic_profile_lattice(&sections, 2_f64.powi(-40), 0., 1000).is_err());
        // A nearly symmetric profile must be repaired with a charged bound;
        // a genuinely asymmetric profile above retains independent rounding.
        let mut symmetric = before.clone();
        symmetric[0][0].control_points[0][0] = 1. + 1e-13;
        symmetric[0][0].control_points[4][0] = 1. + 1e-13;
        let coupled = periodic_profile_lattice(&symmetric, 2_f64.powi(-40), 1e-9, 1000)
            .unwrap()
            .unwrap();
        assert!(coupled.1 > 0. && coupled.1 < 1e-9);
        let p = &coupled.0[0][0].control_points;
        for k in 0..3 {
            assert_eq!(p[0][k] + p[2][k], p[1][k] + p[3][k]);
        }
        assert_eq!(p[0], p[4]);
        assert_eq!(p[1], p[5]);
        assert!(
            periodic_profile_lattice(&symmetric, 2_f64.powi(-40), 1e-9, coupled.2 - 1).is_err()
        );
        assert_ne!(result.0[0][0].control_points[0][0], 1.);
        let mut rational = sections;
        rational[0][0].weights[1] = 0.5;
        assert!(
            periodic_profile_lattice(&rational, 2_f64.powi(-40), 1e-9, 1000)
                .unwrap()
                .is_none()
        );
    }
    #[test]
    fn derives_owned_hollow_boundary_and_correction_from_original_request() {
        let outer = nurbs_core::primitives::circle([0., 0., 0.], [0., 0., 1.], 0.5).unwrap();
        let mut hole = nurbs_core::primitives::circle([0., 0., 0.], [0., 0., 1.], 0.2).unwrap();
        hole.control_points.reverse();
        hole.weights.reverse();
        let loops = vec![vec![outer], vec![hole]];
        let before = loops.clone();
        let scale = scalar(1.);
        let twist = scalar(0.);
        let request = Request {
            loops: &loops,
            points: &[[0., 0., 0.], [0., 0., 10.]],
            scale: &scale,
            twist: &twist,
            options: progressive_miter::Options {
                normal: [1., 0., 0.],
                closed: false,
                miter_limit: 4.,
                initial_steps: 1,
                max_steps: 1,
                max_deviation: 0.01,
            },
            affine: None,
            frames: None,
            guide: None,
            circle: Some(Budget {
                quantum: 2_f64.powi(-40),
                tolerance: 1e-9,
                max_work: Some(100000.),
            }),
            caps: None,
        };
        let body = construct(&request, &limits()).unwrap();
        assert!(body.complete_boundary().is_some());
        assert_eq!(body.model().faces.len(), 10);
        assert!(body.boundary().filled_cap_error_upper.is_some());
        let rebuilt = body
            .reconstruct_stations(2_f64.powi(-40), 1., 100000, Some(1.))
            .unwrap();
        assert_eq!(rebuilt.reason, "owned-station-complete-boundary");
        assert!(rebuilt.reconstruction.unwrap().model.is_some());
        assert_eq!(
            rebuilt.boundary.unwrap().filled_cap_error_upper,
            body.boundary().filled_cap_error_upper
        );
        let denied = body
            .reconstruct_stations(2_f64.powi(-40), 1., 100000, Some(0.))
            .unwrap();
        assert_eq!(denied.reason, "boundary-budget-unproved");
        assert!(denied.reconstruction.unwrap().model.is_none());
        let exhausted = body
            .reconstruct_stations(2_f64.powi(-40), 1., 0, Some(1.))
            .unwrap();
        assert!(exhausted.reconstruction.is_none());
        assert_eq!(loops, before);
        assert!(body.wall_charts().certified);
        let mut charts_limited = limits();
        charts_limited.wall_cells = 0;
        assert!(
            construct(&request, &charts_limited)
                .err()
                .unwrap()
                .to_string()
                .contains("retained wall regularity unproved")
        );
        assert_eq!(body.edges(), 1);
        assert_eq!(body.max_steps(), 1);
        assert!(body.correction().unwrap().wall_displacement_upper.is_some());

        // Cap audit work allowances do not change retained-wall work or each other.
        let mut domain_limited = limits();
        domain_limited.domain_exact_work = 1;
        let domain_denied = construct(&request, &domain_limited).unwrap();
        assert!(domain_denied.complete_boundary().is_none());
        assert_eq!(
            domain_denied.boundary().wall_error_upper,
            body.boundary().wall_error_upper
        );
        let mut projection_limited = limits();
        projection_limited.projection_exact_work = 1;
        let projection_denied = construct(&request, &projection_limited).unwrap();
        assert!(projection_denied.complete_boundary().is_none());
        assert_eq!(
            projection_denied.boundary().wall_error_upper,
            body.boundary().wall_error_upper
        );

        let shear = [
            [1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [1., 1., 1., 0.],
            [0., 0., 0., 1.],
        ];
        let placed = body
            .place(shear, 2_f64.powi(-40), 100000, Some(1e-9))
            .unwrap();
        assert!(placed.placement.unwrap().model.is_some());
        let denied = body
            .place(shear, 2_f64.powi(-40), 100000, Some(1e-15))
            .unwrap();
        assert!(denied.placement.unwrap().model.is_none());
        let vector = |value: [f64; 3]| Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![value.to_vec(), value.to_vec()],
            weights: vec![1., 1.],
            periodic: false,
        };
        let axes = vector([2., 1., 1.]);
        let center = vector([0.125, 0.25, 0.]);
        let axis = vector([0., 0., 2.]);
        let normal = vector([3., 0., 0.]);
        let guide = nurbs_core::primitives::line([1., 0., 0.], [1., 0., 10.]).unwrap();
        let joint = Request {
            affine: Some((&axes, &center)),
            frames: Some((&axis, &normal)),
            guide: Some(&guide),
            loops: request.loops,
            points: request.points,
            scale: request.scale,
            twist: request.twist,
            options: request.options,
            circle: request.circle,
            caps: None,
        };
        let body = construct(&joint, &limits()).unwrap();
        assert!(body.complete_boundary().is_some());
        let report = body.levels().last().unwrap();
        assert!(
            report.authored_frames_applied
                && report.orientation_guide_applied
                && report.affine_laws_applied
        );
        let denied = Request {
            circle: Some(Budget {
                max_work: Some(1.),
                ..request.circle.unwrap()
            }),
            ..request
        };
        assert!(construct(&denied, &limits()).is_err());
    }
    #[test]
    fn owns_closed_path_and_sharp_stations_without_filled_caps() {
        let profile = nurbs_core::primitives::circle([0., 0., 0.], [0., 0., 1.], 0.5).unwrap();
        let loops = vec![vec![profile]];
        let scale = scalar(1.);
        let twist = scalar(0.);
        let points = [[0., 0., 0.], [0., 0., 10.], [10., 0., 10.], [10., 0., 0.]];
        let request = Request {
            loops: &loops,
            points: &points,
            scale: &scale,
            twist: &twist,
            options: progressive_miter::Options {
                normal: [1., 0., 0.],
                closed: true,
                miter_limit: 4.,
                initial_steps: 1,
                max_steps: 1,
                max_deviation: 0.01,
            },
            affine: None,
            frames: None,
            guide: None,
            circle: None,
            caps: None,
        };
        let body = construct(&request, &limits()).unwrap();
        assert!(body.closed());
        assert!(body.complete_boundary().is_some());
        assert!(body.boundary().filled_cap_error_upper.is_none());
        assert_eq!(body.sharp_stations(), &[0, 1, 2, 3]);
        assert_eq!(body.model().faces.len(), 16);
        let denied = Request {
            caps: Some(CapCorrection {
                budget: Budget {
                    quantum: 0.125,
                    tolerance: 0.01,
                    max_work: Some(100000.),
                },
                authored_frame: false,
            }),
            ..request
        };
        assert!(construct(&denied, &limits()).is_err());
    }
    #[test]
    fn closed_periodic_profile_preserves_g2_without_caps() {
        let profile = |r: f64, sign: f64| Curve {
            degree: 2,
            knots: (0..9).map(|x| x as f64).collect(),
            control_points: vec![
                vec![0., r, 0.],
                vec![0., 0., sign * r],
                vec![0., -r, 0.],
                vec![0., 0., -sign * r],
                vec![0., r, 0.],
                vec![0., 0., sign * r],
            ],
            weights: vec![1.; 6],
            periodic: true,
        };
        let loops = vec![vec![profile(0.25, 1.)], vec![profile(0.1, -1.)]];
        let before = loops.clone();
        let vector = |values: Vec<Vec<f64>>| Curve {
            degree: 1,
            knots: vec![0., 0., 0.25, 0.5, 0.75, 1., 1.],
            control_points: values,
            weights: vec![1.; 5],
            periodic: false,
        };
        let axis = vector(vec![
            vec![1., -1., 0.],
            vec![1., 1., 0.],
            vec![-1., 1., 0.],
            vec![-1., -1., 0.],
            vec![1., -1., 0.],
        ]);
        let guide = vector(vec![
            vec![0., 0., 100.],
            vec![10., 0., 100.],
            vec![10., 10., 100.],
            vec![0., 10., 100.],
            vec![0., 0., 100.],
        ]);
        let normal = nurbs_core::progressive_sweep::constant_vector_law([0., 0., 1.]).unwrap();
        let axes = nurbs_core::progressive_sweep::constant_vector_law([2., 1., 1.]).unwrap();
        let center = nurbs_core::progressive_sweep::constant_vector_law([0.; 3]).unwrap();
        let scale = scalar(1.);
        let twist = scalar(0.);
        let request = Request {
            loops: &loops,
            points: &[[0., 0., 0.], [10., 0., 0.], [10., 10., 0.], [0., 10., 0.]],
            scale: &scale,
            twist: &twist,
            options: progressive_miter::Options {
                normal: [0., 0., 1.],
                closed: true,
                miter_limit: 4.,
                initial_steps: 1,
                max_steps: 8,
                max_deviation: 10.,
            },
            affine: Some((&axes, &center)),
            frames: Some((&axis, &normal)),
            guide: Some(&guide),
            circle: None,
            caps: None,
        };
        let body = construct(&request, &limits()).unwrap();
        assert!(body.complete_boundary().is_some());
        assert!(body.boundary().filled_cap_error_upper.is_none());
        assert!(body.wall_charts().certified);
        let report = crate::sweep_smoothness::inspect_profile(body.model(), &[], 2000000).unwrap();
        assert!(
            report.profile.g2_certified,
            "Closed actual profile G2 unproved"
        );
        assert_eq!(report.station_continuity, "C0");
        assert_eq!(
            body.correction().unwrap().reason,
            "bounded-periodic-profile-interpolation"
        );
        assert!(body.correction().unwrap().wall_displacement_upper.unwrap() <= 1e-9);
        assert_eq!(loops, before);
        let mut denied = limits();
        denied.projection_exact_work = 0;
        assert!(construct(&request, &denied).is_err());
    }

    #[test]
    fn derives_periodic_moving_frame_caps_from_original_laws() {
        let outer = Curve {
            degree: 2,
            knots: (0..9).map(|x| x as f64).collect(),
            control_points: vec![
                vec![1., 0., 0.],
                vec![0., 1., 0.],
                vec![-1., 0., 0.],
                vec![0., -1., 0.],
                vec![1., 0., 0.],
                vec![0., 1., 0.],
            ],
            weights: vec![1.; 6],
            periodic: true,
        };
        let mut hole = outer.clone();
        hole.control_points.reverse();
        for p in &mut hole.control_points {
            for x in p {
                *x *= 0.25;
            }
        }
        let loops = vec![vec![outer], vec![hole]];
        let scale = scalar(1.);
        let mut twist = scalar(0.);
        twist.control_points[1][0] = 0.25;
        let vector = |a: [f64; 3], b: [f64; 3]| Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![a.to_vec(), b.to_vec()],
            weights: vec![1., 1.],
            periodic: false,
        };
        let axis = vector([0., 0., 1.], [0., 0.5, 1.]);
        let normal = vector([3., 0., 0.], [3., 0., 0.]);
        let axes = vector([1., 1., 1.], [2., 1., 1.]);
        let center = vector([0., 0., 0.], [0., 0.125, 0.25]);
        let guide = nurbs_core::primitives::line([1., 0., 0.], [1., 0., 10.]).unwrap();
        let request = Request {
            loops: &loops,
            points: &[[0., 0., 0.], [0., 0., 10.]],
            scale: &scale,
            twist: &twist,
            options: progressive_miter::Options {
                normal: [1., 0., 0.],
                closed: false,
                miter_limit: 4.,
                initial_steps: 1,
                max_steps: 32,
                max_deviation: 0.01,
            },
            affine: Some((&axes, &center)),
            frames: Some((&axis, &normal)),
            guide: Some(&guide),
            circle: None,
            caps: None,
        };
        let raw = construct(&request, &limits()).unwrap();
        assert!(raw.complete_boundary().is_some());
        assert_eq!(
            raw.correction().unwrap().reason,
            "automatic-bounded-cap-planarity"
        );
        assert!(raw.boundary().error_upper.unwrap() < 0.01);
        assert!(raw.wall_charts().certified);
        let smooth = crate::sweep_smoothness::inspect_profile(
            raw.model(),
            &[raw.model().faces.len() - 2, raw.model().faces.len() - 1],
            2000000,
        )
        .unwrap();
        assert!(
            smooth.profile.g1_certified,
            "Periodic profile G1 not preserved"
        );
        assert!(
            smooth.profile.g2_certified,
            "Periodic profile G2 not preserved"
        );
        let mut no_repair_work = limits();
        no_repair_work.cap_regions.max_exact_work = 0;
        let unproved = construct(&request, &no_repair_work).unwrap();
        assert!(unproved.complete_boundary().is_none());
        let mut no_wall_work = limits();
        no_wall_work.wall_cells = 0;
        assert!(construct(&request, &no_wall_work).is_err());
        assert!(raw.boundary().filled_cap_error_upper.is_some());
        let request = Request {
            caps: Some(CapCorrection {
                budget: Budget {
                    quantum: 2_f64.powi(-40),
                    tolerance: 1e-9,
                    max_work: Some(1000000.),
                },
                authored_frame: true,
            }),
            ..request
        };
        let owned = construct(&request, &limits()).unwrap();
        assert!(owned.complete_boundary().is_some());
        assert_eq!(owned.levels().last().unwrap().steps, 16);
        assert!(owned.boundary().error_upper.unwrap() < 0.01);
        let smooth = crate::sweep_smoothness::inspect_profile(
            owned.model(),
            &[owned.model().faces.len() - 2, owned.model().faces.len() - 1],
            2000000,
        )
        .unwrap();
        assert!(
            smooth.profile.g2_certified,
            "Explicit authored cap profile G2 unproved"
        );
        assert!(owned.correction().unwrap().wall_displacement_upper.unwrap() <= 1e-9);
        let denied = Request {
            caps: Some(CapCorrection {
                budget: Budget {
                    max_work: Some(1.),
                    ..request.caps.as_ref().unwrap().budget
                },
                authored_frame: true,
            }),
            ..request
        };
        assert!(construct(&denied, &limits()).is_err());
    }
