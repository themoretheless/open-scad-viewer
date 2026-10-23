    use super::*;
    #[test]
    fn joint_original_frame_phase_preserves_complete_images_and_refuses_partial_work() {
        let path = crate::primitives::circle([0.; 3], [0., 0., 1.], 1.).unwrap();
        let proof =
            certify_original_rmf_transport_shared(&path, [1., 0., 1.], 512, 10000, true).unwrap();
        for interval in [[0.3125; 2], [0.2, 0.3], [0., 1.]] {
            let old = original_frame_axes(&proof, interval, 10000, None).unwrap();
            let (phase_work, old_phase) = correction_image(&proof, interval, 10000).unwrap();
            let (joint, phase) =
                original_frame_with_correction(&proof, interval, 10000, None).unwrap();
            assert_eq!(joint.status, Status::Certified);
            assert_eq!(joint.transverse, old.transverse);
            assert_eq!(joint.longitudinal, old.longitudinal);
            let (a, b) = phase.unwrap();
            let (c, d) = old_phase.unwrap();
            assert_eq!([a.lo, a.hi, b.lo, b.hi], [c.lo, c.hi, d.lo, d.hi]);
            assert!(joint.cells < old.cells + phase_work);
            for budget in [0, joint.cells - 1] {
                let (short, phase) =
                    original_frame_with_correction(&proof, interval, budget, None).unwrap();
                assert_eq!(short.status, Status::Unresolved);
                assert!(phase.is_none() && short.cells <= budget);
            }
        }
    }

    #[test]
    fn higher_degree_original_plane_requires_exact_coplanarity_and_complete_work() {
        // The six authored poles lie on y=z. This premise is independent of
        // their degree, rational weights and subsequent transport sampling.
        let poles = vec![
            vec![0., 0., 0.],
            vec![0.2, 0., 0.],
            vec![0.4, 0.1, 0.1],
            vec![0.6, 0.3, 0.3],
            vec![0.8, 0.6, 0.6],
            vec![1., 1., 1.],
        ];
        let (work, plane) = exact_pole_plane(&poles, 10000).unwrap();
        let normal = plane.unwrap();
        let r = 2_f64.sqrt();
        assert!(normal[0].lo <= 0. && normal[0].hi >= 0.);
        for (k, value) in [(1, -1. / r), (2, 1. / r)] {
            assert!(normal[k].lo <= value && normal[k].hi >= value);
        }
        for budget in [0, work - 1] {
            let (used, plane) = exact_pole_plane(&poles, budget).unwrap();
            assert!(plane.is_none() && used <= budget);
        }
        let mut perturbed = poles;
        perturbed[5][2] = 1_f64.next_up();
        assert!(exact_pole_plane(&perturbed, 10000).unwrap().1.is_none());
    }

    #[test]
    fn corrected_original_circle_queries_restrict_source_frame_before_rotation() {
        let path = crate::primitives::circle([0.; 3], [0., 0., 1.], 1.).unwrap();
        let proof =
            certify_original_rmf_transport_shared(&path, [1., 0., 1.], 512, 10000, true).unwrap();
        assert_eq!(proof.status, Status::Certified, "{proof:?}");
        assert!(proof.correction_phases.is_some());
        let twist = super::super::constant_vector_law([0.; 3]).unwrap();
        for i in 0..=64 {
            let t = i as f64 / 64.;
            let span = (2..path.control_points.len())
                .find(|&j| {
                    path.knots[j] < path.knots[j + 1]
                        && t >= path.knots[j]
                        && t <= path.knots[j + 1]
                })
                .unwrap();
            let f = (t - path.knots[span]) / (path.knots[span + 1] - path.knots[span]);
            let b = [(1. - f).powi(2), 2. * f * (1. - f), f * f];
            let db = [-2. * (1. - f), 2. - 4. * f, 2. * f];
            let weights = &path.weights[span - 2..=span];
            let poles = &path.control_points[span - 2..=span];
            let w = (0..3).map(|j| b[j] * weights[j]).sum::<f64>();
            let dw = (0..3).map(|j| db[j] * weights[j]).sum::<f64>();
            let v = std::array::from_fn::<_, 3, _>(|k| {
                (0..3)
                    .map(|j| db[j] * weights[j] * poles[j][k])
                    .sum::<f64>()
                    * w
                    - (0..3).map(|j| b[j] * weights[j] * poles[j][k]).sum::<f64>() * dw
            });
            let length = math_core::norm(v);
            let root = 2_f64.sqrt();
            let expected = [v[1] / length / root, -v[0] / length / root, 1. / root];
            let image = frame_values(&proof, &twist, [t; 2], [t; 2], 10000).unwrap();
            assert_eq!(image.status, Status::Certified);
            let normal = image.transverse.unwrap();
            for k in 0..3 {
                assert!(normal[k][0] <= expected[k] && expected[k] <= normal[k][1]);
                assert!(
                    normal[k][1] - normal[k][0] < 1e-7,
                    "station={i} component={k} normal={normal:?} angle={:?} end_radius={}",
                    proof.closing_angle,
                    proof
                        .cover
                        .as_ref()
                        .unwrap()
                        .last()
                        .unwrap()
                        .endpoint_radius_upper
                );
            }
            for budget in [0, image.cells - 1] {
                let short = frame_values(&proof, &twist, [t; 2], [t; 2], budget).unwrap();
                assert_eq!(short.status, Status::Unresolved);
                assert!(short.transverse.is_none() && short.cells <= budget);
            }
        }
    }
    #[test]
    fn tilted_original_quadratic_plane_has_independent_bishop_normal() {
        let path =
            crate::paths::bezier(vec![vec![0.; 3], vec![0.5, 0., 0.], vec![1., 1., 1.]], None)
                .unwrap();
        let proof =
            certify_original_rmf_transport_shared(&path, [0., 0., 1.], 512, 10000, false).unwrap();
        assert_eq!(proof.status, Status::Certified, "{proof:?}");
        assert!(proof.cover.as_ref().unwrap().iter().any(|cell| {
            cell.planar_basis
                .as_ref()
                .is_some_and(|basis| basis.axis.is_none())
        }));
        let end = proof.cover.as_ref().unwrap().last().unwrap();
        assert!(end.endpoint_radius_upper < 1e-8);
        let twist = super::super::constant_vector_law([0.; 3]).unwrap();
        // P=(t,t²,t²), fixed original plane normal (0,-1,1)/sqrt(2).
        // Bishop N=(B+B x T)/sqrt(2), from the original seed N(0)=Z.
        for i in 0..=64 {
            let t = i as f64 / 64.;
            let r = (1. + 8. * t * t).sqrt();
            let expected = [-2. * t / r, (-1. + 1. / r) / 2., (1. + 1. / r) / 2.];
            let image = frame_values(&proof, &twist, [t; 2], [t; 2], 10000).unwrap();
            assert_eq!(image.status, Status::Certified);
            let n = image.transverse.unwrap();
            for k in 0..3 {
                assert!(n[k][0] <= expected[k] && expected[k] <= n[k][1]);
                assert!(n[k][1] - n[k][0] < 1e-8);
            }
        }
        let work = proof.cells + proof.exact_work as usize;
        for budget in [0, work - 1] {
            let short =
                certify_original_rmf_transport_shared(&path, [0., 0., 1.], 512, budget, false)
                    .unwrap();
            assert_eq!(short.status, Status::Unresolved);
            assert!(short.cover.is_none());
            assert!(short.cells + short.exact_work as usize <= budget);
        }
    }
    #[test]
    fn closed_original_plane_speed_uses_normalized_nonunit_domain() {
        let mut path = crate::primitives::circle([0.; 3], [0., 0., 1.], 1.).unwrap();
        for knot in &mut path.knots {
            *knot = 2. + 3. * *knot;
        }
        let proof =
            certify_original_rmf_transport_shared(&path, [0., 0., 1.], 512, 10000, true).unwrap();
        assert_eq!(proof.status, Status::Certified, "{proof:?}");
        let work = proof.cells + proof.exact_work as usize;
        assert!(work <= 10000 && proof.arc_corrected_normals.is_some());
        for cell in proof.cover.as_ref().unwrap() {
            let t = (cell.traversal[0] + cell.traversal[1]) / 2.;
            let u = 2. + 3. * t;
            let span = (2..path.control_points.len())
                .find(|&i| {
                    path.knots[i] < path.knots[i + 1]
                        && u >= path.knots[i]
                        && u <= path.knots[i + 1]
                })
                .unwrap();
            let h = path.knots[span + 1] - path.knots[span];
            let f = (u - path.knots[span]) / h;
            let b = [(1. - f).powi(2), 2. * f * (1. - f), f * f];
            let db = [-2. * (1. - f), 2. - 4. * f, 2. * f];
            let weights = &path.weights[span - 2..=span];
            let poles = &path.control_points[span - 2..=span];
            let w = (0..3).map(|j| b[j] * weights[j]).sum::<f64>();
            let dw = (0..3).map(|j| db[j] * weights[j]).sum::<f64>();
            let velocity = std::array::from_fn(|k| {
                let p = (0..3).map(|j| b[j] * weights[j] * poles[j][k]).sum::<f64>();
                let dp = (0..3)
                    .map(|j| db[j] * weights[j] * poles[j][k])
                    .sum::<f64>();
                // Independent original rational Bernstein derivative per
                // normalized traversal, not per the [2,5] source parameter.
                3. * (dp * w - p * dw) / (h * w * w)
            });
            let speed = math_core::norm(velocity);
            assert!(
                cell.speed[0] <= speed + 1e-12 && speed <= cell.speed[1] + 1e-12,
                "{:?} misses {speed}",
                cell.speed
            );
        }
        for budget in [0, work - 1] {
            let short =
                certify_original_rmf_transport_shared(&path, [0., 0., 1.], 512, budget, true)
                    .unwrap();
            assert_eq!(short.status, Status::Unresolved);
            assert!(short.cover.is_none());
            assert!(short.cells + short.exact_work as usize <= budget);
        }
    }
    #[test]
    fn normalized_component_bounds_keep_shared_speed_and_signed_boxes() {
        for bounds in [
            [[1., 1000.], [0., 0.], [0., 0.]],
            [[-1000., -1.], [0., 0.], [0., 0.]],
            [[0.5, 2.], [-3., 4.], [-0.1, 0.2]],
            [[-2., -0.5], [-4., -3.], [0.1, 0.2]],
            [[-1., 1.], [2., 3.], [-4., 4.]],
        ] {
            let box_value = bounds.map(|v| I::new(v[0], v[1]).unwrap());
            let proved = unit(box_value).unwrap();
            let old = div(box_value, norm(box_value).unwrap()).unwrap();
            for k in 0..3 {
                assert!(proved[k].lo >= old[k].lo && proved[k].hi <= old[k].hi);
            }
            for x in [0., 0.125, 0.5, 0.875, 1.] {
                for y in [0., 0.5, 1.] {
                    for z in [0., 0.5, 1.] {
                        let actual = std::array::from_fn(|k| {
                            bounds[k][0] + [x, y, z][k] * (bounds[k][1] - bounds[k][0])
                        });
                        let length = math_core::norm(actual);
                        for k in 0..3 {
                            let value = actual[k] / length;
                            assert!(proved[k].lo - 1e-15 <= value && value <= proved[k].hi + 1e-15);
                        }
                    }
                }
            }
        }
        let axis = unit([I::new(1., 1000.).unwrap(), I::point(0.), I::point(0.)]).unwrap();
        assert!(axis[0].lo > 0.999999999999 && axis[0].hi <= 1.);
        assert!(unit([I::point(0.); 3]).is_err());
    }
    #[test]
    fn piecewise_planar_source_transport_preserves_independent_quarter_turn_frame() {
        let path = crate::paths::round_polyline(
            &[[0., 0., 0.], [0., 0., 10.], [10., 0., 10.], [10., 10., 10.]],
            2.,
        )
        .unwrap();
        let proof =
            certify_original_rmf_transport_shared(&path, [1., 0., 0.], 512, 10000, false).unwrap();
        assert_eq!(proof.status, Status::Certified, "{proof:?}");
        assert!(proof.cells + proof.exact_work as usize <= 10000);
        let end = proof.cover.as_ref().unwrap().last().unwrap();
        // Integer quarter turns: Z->X about +Y maps initial X to -Z;
        // X->Y about +Z then preserves -Z, independently of the ODE cover.
        for (axis, expected) in [0., 0., -1.].into_iter().enumerate() {
            assert!(
                end.endpoint_normal[axis][0] <= expected
                    && expected <= end.endpoint_normal[axis][1]
            );
        }
        assert!(
            end.endpoint_radius_upper < 1e-8,
            "{}",
            end.endpoint_radius_upper
        );
        let profile = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
        let scale = super::super::constant_vector_law([1., 0., 0.]).unwrap();
        let twist = super::super::constant_vector_law([0.; 3]).unwrap();
        let sweep = super::super::Sweep::new(
            &profile,
            &path,
            &scale,
            &twist,
            super::super::Options {
                normal: [1., 0., 0.],
                orientation: super::super::Orientation::RotationMinimizing,
                spacing: super::super::Spacing::Parameter,
                initial_sections: 5,
                max_sections: 1025,
                max_deviation: 0.01,
            },
        )
        .unwrap();
        assert_eq!(path.degree, 2);
        let reference = |t: f64| {
            let domain = path.domain();
            let u = domain[0] + (domain[1] - domain[0]) * t;
            let span = (2..path.control_points.len())
                .find(|&i| {
                    path.knots[i] < path.knots[i + 1]
                        && u >= path.knots[i]
                        && u <= path.knots[i + 1]
                })
                .unwrap();
            let f = (u - path.knots[span]) / (path.knots[span + 1] - path.knots[span]);
            let bernstein = [(1. - f) * (1. - f), 2. * f * (1. - f), f * f];
            let derivative = [-2. * (1. - f), 2. - 4. * f, 2. * f];
            let weights = &path.weights[span - 2..=span];
            let poles = &path.control_points[span - 2..=span];
            let w = (0..3).map(|j| bernstein[j] * weights[j]).sum::<f64>();
            let dw = (0..3).map(|j| derivative[j] * weights[j]).sum::<f64>();
            let numerator: [f64; 3] = std::array::from_fn(|k| {
                (0..3)
                    .map(|j| bernstein[j] * weights[j] * poles[j][k])
                    .sum()
            });
            let dn: [f64; 3] = std::array::from_fn(|k| {
                (0..3)
                    .map(|j| derivative[j] * weights[j] * poles[j][k])
                    .sum()
            });
            let velocity: [f64; 3] = std::array::from_fn(|k| dn[k] * w - numerator[k] * dw);
            let speed = math_core::norm(velocity);
            let arc_end = (path.knots[5] - domain[0]) / (domain[1] - domain[0]);
            let normal = if t <= arc_end {
                [velocity[2] / speed, 0., -velocity[0] / speed]
            } else {
                [0., 0., -1.]
            };
            (numerator.map(|x| x / w), normal)
        };
        let sections = sweep.sections_at(1025).unwrap();
        let mut deviation = 0_f64;
        for (i, section) in sections.iter().enumerate() {
            let t = i as f64 / 1024.;
            let (source, normal) = reference(t);
            for (j, q) in [1., 2.].into_iter().enumerate() {
                let delta = std::array::from_fn(|k| {
                    section.control_points[j][k] - source[k] - q * normal[k]
                });
                deviation = deviation.max(math_core::norm(delta));
            }
        }
        assert!(
            deviation < 0.01,
            "Original quarter-turn versus retained frame deviation={deviation}"
        );
        let bound = sweep
            .rmf_spatial_patch_error_bound(1025, 512, 10000, 1000000)
            .unwrap();
        assert_eq!(bound.status, Status::Certified, "{bound:?}");
        assert!(
            bound.within_budget && bound.error_upper.unwrap() <= 0.01,
            "{bound:?}"
        );
        for i in 0..1024 {
            for fraction in [0.125, 0.375, 0.5, 0.625, 0.875] {
                let (source, normal) = reference((i as f64 + fraction) / 1024.);
                for (j, q) in [1., 2.].into_iter().enumerate() {
                    let delta = std::array::from_fn(|k| {
                        (1. - fraction) * sections[i].control_points[j][k]
                            + fraction * sections[i + 1].control_points[j][k]
                            - source[k]
                            - q * normal[k]
                    });
                    assert!(math_core::norm(delta) <= bound.error_upper.unwrap());
                }
            }
        }
        for budget in [0, bound.cells - 1] {
            let short = sweep
                .rmf_spatial_patch_error_bound(1025, 512, budget, 1000000)
                .unwrap();
            assert!(
                !short.within_budget,
                "Short original wall budget unexpectedly admitted: {short:?}"
            );
            assert!(short.cells <= budget);
        }
        eprintln!(
            "Rounded continuous wall: upper={:?} cells={}",
            bound.error_upper, bound.cells
        );
        let mut worst = (0_f64, 0_usize);
        for i in 0..129 {
            let t = i as f64 / 128.;
            let (_, expected) = reference(t);
            let frame = frame_values(&proof, &twist, [t; 2], [t; 2], 10000).unwrap();
            assert_eq!(frame.status, Status::Certified);
            for budget in [0, frame.cells - 1] {
                let short = frame_values(&proof, &twist, [t; 2], [t; 2], budget).unwrap();
                assert_eq!(short.status, Status::Unresolved);
                assert!(short.cells <= budget && short.transverse.is_none());
            }
            let normal = frame.transverse.unwrap();
            for k in 0..3 {
                assert!(normal[k][0] <= expected[k] && expected[k] <= normal[k][1]);
            }
            let radius = normal
                .iter()
                .zip(expected)
                .map(|(v, x)| (v[0] - x).abs().max((v[1] - x).abs()).powi(2))
                .sum::<f64>()
                .sqrt();
            if radius > worst.0 {
                worst = (radius, i);
            }
        }
        assert!(
            worst.0 < 1e-8,
            "Restricted original station frame {worst:?}"
        );
        for i in 1..128 {
            let interval = [(i as f64 - 0.125) / 128., (i as f64 + 0.125) / 128.];
            let image = frame_values(&proof, &twist, interval, interval, 10000).unwrap();
            assert_eq!(image.status, Status::Certified);
            let normal = image.transverse.unwrap();
            for j in 0..=8 {
                let t = interval[0] + (interval[1] - interval[0]) * j as f64 / 8.;
                let (_, expected) = reference(t);
                for k in 0..3 {
                    assert!(normal[k][0] <= expected[k] && expected[k] <= normal[k][1]);
                }
            }
        }
        eprintln!("Rounded frame diagnostic: retained1025={deviation} worst129normal={worst:?}");
        for cell in proof.cover.as_ref().unwrap() {
            for fraction in [0.125, 0.5, 0.875] {
                let t = cell.traversal[0] + fraction * (cell.traversal[1] - cell.traversal[0]);
                let (_, expected) = reference(t);
                for k in 0..3 {
                    assert!(
                        cell.normal[k][0] - 1e-9 <= expected[k]
                            && expected[k] <= cell.normal[k][1] + 1e-9,
                        "t={t} axis={k}"
                    );
                }
            }
        }
        for budget in [0, (proof.cells + proof.exact_work as usize) - 1] {
            let refused =
                certify_original_rmf_transport_shared(&path, [1., 0., 0.], 512, budget, false)
                    .unwrap();
            assert_eq!(refused.status, Status::Unresolved);
            assert!(refused.cover.is_none());
            assert!(refused.cells + refused.exact_work as usize <= budget);
        }
    }
    #[test]
    fn original_non_grid_knots_keep_complete_cover_and_actual_arc_cell_widths() {
        let mut path = spatial_loop();
        path.knots = vec![
            0., 0., 0., 0., 0.2, 0.2, 0.2, 0.45, 0.45, 0.45, 0.7, 0.7, 0.7, 1., 1., 1., 1.,
        ];
        let proof = certify_original_rmf_transport(&path, [1., 0., 0.], 512, 100000, 1000000, true)
            .unwrap();
        assert_eq!(proof.status, Status::Certified, "{proof:?}");
        let cover = proof.cover.as_ref().unwrap();
        assert!(cover.len() > 512);
        assert_eq!(cover.first().unwrap().traversal[0], 0.);
        assert_eq!(cover.last().unwrap().traversal[1], 1.);
        assert!(cover.iter().all(|c| c.traversal[0] < c.traversal[1]));
        assert!(
            cover
                .windows(2)
                .all(|c| c[0].traversal[1] == c[1].traversal[0])
        );
        assert_eq!(
            proof.arc_corrected_normals.as_ref().unwrap().len(),
            cover.len()
        );
        assert_eq!(
            proof.arc_corrected_binormals.as_ref().unwrap().len(),
            cover.len()
        );
        for budget in [0, proof.cells - 1] {
            let refused =
                certify_original_rmf_transport(&path, [1., 0., 0.], 512, budget, 1000000, true)
                    .unwrap();
            assert_eq!(refused.status, Status::Unresolved);
            assert!(refused.cover.is_none() && refused.closing_angle.is_none());
            assert!(refused.cells <= budget);
        }
    }
    fn spatial_loop() -> Curve {
        let vertices = [[1., 0., 0.], [0., 1., 1.], [-1., 0., 0.], [0., -1., 0.5]];
        let tangents = [
            [0., 0.25, 0.25],
            [-0.25, 0., -0.25],
            [0., -0.25, 0.25],
            [0.25, 0.125, -0.25],
        ];
        let mut poles = vec![vertices[0].to_vec()];
        for i in 0..4 {
            let j = (i + 1) % 4;
            poles.push((0..3).map(|k| vertices[i][k] + tangents[i][k]).collect());
            poles.push((0..3).map(|k| vertices[j][k] - tangents[j][k]).collect());
            poles.push(vertices[j].to_vec());
        }
        Curve {
            degree: 3,
            knots: vec![
                0., 0., 0., 0., 0.25, 0.25, 0.25, 0.5, 0.5, 0.5, 0.75, 0.75, 0.75, 1., 1., 1., 1.,
            ],
            control_points: poles,
            weights: vec![1.; 13],
            periodic: false,
        }
    }
    #[test]
    fn closed_rational_nonuniform_spans_require_original_c1_and_complete_transport() {
        let mut path = spatial_loop();
        let fractions = [0., 0.125, 0.375, 0.75, 1.];
        // Equal interior/end weight ratios and offsets proportional to each
        // span preserve the original Cartesian first jet across every knot.
        for segment in 0..4 {
            let first = 3 * segment;
            let ratio = 4. * (fractions[segment + 1] - fractions[segment]);
            for k in 0..3 {
                path.control_points[first + 1][k] = path.control_points[first][k]
                    + ratio * (path.control_points[first + 1][k] - path.control_points[first][k]);
                path.control_points[first + 2][k] = path.control_points[first + 3][k]
                    + ratio
                        * (path.control_points[first + 2][k] - path.control_points[first + 3][k]);
            }
            path.weights[first + 1] = 1.25;
            path.weights[first + 2] = 1.25;
        }
        path.knots = vec![
            0., 0., 0., 0., 0.125, 0.125, 0.125, 0.375, 0.375, 0.375, 0.75, 0.75, 0.75, 1., 1., 1.,
            1.,
        ];
        let before = path.clone();
        let report =
            certify_original_rmf_transport(&path, [1., 0., 0.], 4096, 100000, 1000000, true)
                .unwrap();
        assert_eq!(report.status, Status::Certified, "{report:?}");
        assert_eq!(report.cover.as_ref().unwrap().len(), 4096);
        let angle = report.closing_angle.unwrap();
        assert!(angle.iter().all(|x| x.is_finite()) && angle[0] <= angle[1]);
        assert!(report.arc_corrected_normals.is_some() && report.arc_corrected_binormals.is_some());
        assert_eq!(path, before);
        use crate::progressive_sweep::{Options, Orientation, Spacing, Sweep, constant_vector_law};
        let total = report.cells + report.exact_work as usize;
        let shared =
            certify_original_rmf_transport_shared(&path, [1., 0., 0.], 4096, total, true).unwrap();
        assert_eq!(shared.status, Status::Certified);
        assert!(shared.cells + shared.exact_work as usize <= total);
        for budget in [0, total - 1] {
            let limited =
                certify_original_rmf_transport_shared(&path, [1., 0., 0.], 4096, budget, true)
                    .unwrap();
            assert_eq!(limited.status, Status::Unresolved);
            assert!(limited.cells + limited.exact_work as usize <= budget);
            assert!(
                limited.cover.is_none()
                    && limited.closing_angle.is_none()
                    && limited.arc_corrected_normals.is_none()
                    && limited.arc_corrected_binormals.is_none()
            );
        }
        let profile = crate::primitives::line([1.1, 0., 0.], [1.2, 0., 0.]).unwrap();
        let scale = constant_vector_law([1.1, 0., 0.]).unwrap();
        let twist = constant_vector_law([0.125, 0., 0.]).unwrap();
        let axes = constant_vector_law([1., 1.25, 0.75]).unwrap();
        let center = constant_vector_law([0.01, -0.02, 0.03]).unwrap();
        let contours = [
            crate::primitives::circle([1., 0., 0.], [0., 1., 1.], 0.05).unwrap(),
            crate::primitives::circle([1., 0., 0.], [0., -1., -1.], 0.02).unwrap(),
        ];
        let multi = crate::progressive_sweep::MultiSweep::new(
            &contours,
            &path,
            &scale,
            &twist,
            Options {
                normal: [1., 0., 0.],
                orientation: Orientation::RotationMinimizing,
                spacing: Spacing::ArcLength {
                    tolerance: 0.001,
                    max_cells: 100000,
                },
                initial_sections: 65,
                max_sections: 65,
                max_deviation: 0.4,
            },
        )
        .unwrap()
        .with_affine_laws(&axes, &center)
        .unwrap()
        .with_spatial_rmf_error_limits(4096, 100000, 1000000)
        .unwrap()
        .preview_at(65)
        .unwrap();
        assert!(
            multi.report.accepted && multi.report.continuous_bound,
            "{:?}",
            multi.report
        );
        assert!(multi.report.error_certificate_cells <= 100000);
        let refused_budget = crate::progressive_sweep::MultiSweep::new(
            &contours,
            &path,
            &scale,
            &twist,
            Options {
                normal: [1., 0., 0.],
                orientation: Orientation::RotationMinimizing,
                spacing: Spacing::ArcLength {
                    tolerance: 0.001,
                    max_cells: 100000,
                },
                initial_sections: 65,
                max_sections: 65,
                max_deviation: 0.25,
            },
        )
        .unwrap()
        .with_affine_laws(&axes, &center)
        .unwrap()
        .with_spatial_rmf_error_limits(4096, 100000, 1000000)
        .unwrap()
        .preview_at(65)
        .unwrap();
        assert!(refused_budget.report.continuous_bound && !refused_budget.report.accepted);
        assert_eq!(
            refused_budget.report.continuous_error_upper,
            multi.report.continuous_error_upper
        );
        eprintln!(
            "rational shared contours cells={} upper={:?}",
            multi.report.error_certificate_cells, multi.report.continuous_error_upper
        );
        for spacing in [
            Spacing::Parameter,
            Spacing::ArcLength {
                tolerance: 0.001,
                max_cells: 100000,
            },
        ] {
            let sweep = Sweep::new(
                &profile,
                &path,
                &scale,
                &twist,
                Options {
                    normal: [1., 0., 0.],
                    orientation: Orientation::RotationMinimizing,
                    spacing,
                    initial_sections: 65,
                    max_sections: 65,
                    max_deviation: 2.,
                },
            )
            .unwrap()
            .with_affine_laws(&axes, &center)
            .unwrap();
            let retained = sweep
                .rmf_spatial_patch_error_bound(65, 4096, 100000, 1000000)
                .unwrap();
            assert_eq!(
                retained.status,
                Status::Certified,
                "{spacing:?}: {retained:?}"
            );
            assert!(retained.within_budget && retained.error_upper.is_some());
        }
        let short = certify_original_rmf_transport(
            &path,
            [1., 0., 0.],
            4096,
            report.cells - 1,
            1000000,
            true,
        )
        .unwrap();
        assert_eq!(short.status, Status::Unresolved);
        assert!(
            short.cover.is_none()
                && short.closing_angle.is_none()
                && short.arc_corrected_normals.is_none()
        );
        let mut changed_speed = path.clone();
        changed_speed.weights[1] = f64::from_bits(1.25_f64.to_bits() + 1);
        assert!(
            !endpoint_jets::certify(&changed_speed, 1, 1000000)
                .unwrap()
                .certified
        );
        assert!(
            endpoint_jets::certify_direction(&changed_speed, 0, true, false, 1000000)
                .unwrap()
                .certified
        );
        let changed = certify_original_rmf_transport(
            &changed_speed,
            [1., 0., 0.],
            4096,
            100000,
            1000000,
            true,
        )
        .unwrap();
        assert_eq!(changed.status, Status::Certified);
        // A speed jump preserves unit tangent; an actual tangent perturbation
        // must still discard the complete transport and holonomy certificate.
        let mut broken = path;
        broken.control_points[1][2] = broken.control_points[1][2].next_up();
        let refused =
            certify_original_rmf_transport(&broken, [1., 0., 0.], 4096, 100000, 1000000, true)
                .unwrap();
        assert_eq!(refused.status, Status::Unresolved);
        assert!(refused.cover.is_none() && refused.closing_angle.is_none());
        eprintln!(
            "rational nonuniform closed RMF cells={} angle={angle:?}",
            report.cells
        );
    }
    #[test]
    fn antipodal_closed_bishop_transport_encloses_both_principal_branches() {
        let directions = [
            [1., 0., 0.],
            [0., 1., 0.],
            [0., 0., 1.],
            [1., 0., 0.],
            [0., 1., 0.],
            [-1., 0., 0.],
            [0., 0., -1.],
            [0., -1., 0.],
            [-1., 0., 0.],
            [0., -1., 0.],
            [1., 0., 0.],
        ];
        let mut points = vec![vec![0.; 3]];
        let mut position = [0.; 3];
        let mut independent_normal = [0., 0., 1.];
        for pair in directions.windows(2) {
            points.push((0..3).map(|k| position[k] + pair[0][k]).collect());
            position = std::array::from_fn(|k| position[k] + pair[0][k] + pair[1][k]);
            points.push(position.to_vec());
            // Each planar quadratic has positive curvature and turns its
            // tangent by exactly a quarter turn. This integer Rodrigues
            // calculation is independent of interval integration.
            let axis = math_core::cross(pair[0], pair[1]);
            let cross = math_core::cross(axis, independent_normal);
            let parallel = math_core::dot(axis, independent_normal);
            independent_normal = std::array::from_fn(|k| cross[k] + axis[k] * parallel);
        }
        assert_eq!(position, [0.; 3]);
        assert_eq!(independent_normal, [0., 0., -1.]);
        let mut knots = vec![0.; 3];
        for i in 1..10 {
            knots.extend([i as f64; 2]);
        }
        knots.extend([10.; 3]);
        let path = Curve {
            degree: 2,
            knots,
            weights: vec![1.; points.len()],
            control_points: points,
            periodic: false,
        };
        let report =
            certify_original_rmf_transport(&path, [0., 0., 1.], 4096, 100000, 1000000, true)
                .unwrap();
        assert_eq!(report.status, Status::Certified);
        let angle = report.closing_angle.unwrap();
        assert!(angle[0] < -std::f64::consts::PI && angle[1] > std::f64::consts::PI);
        let cover = report.cover.as_ref().unwrap();
        assert!(cover.len() >= 4096);
        assert_eq!(cover.first().unwrap().traversal[0], 0.);
        assert_eq!(cover.last().unwrap().traversal[1], 1.);
        assert!(
            cover
                .windows(2)
                .all(|c| c[0].traversal[1] == c[1].traversal[0])
        );
        assert_eq!(
            report.arc_corrected_normals.as_ref().unwrap().len(),
            cover.len()
        );
        assert_eq!(
            report.arc_corrected_binormals.as_ref().unwrap().len(),
            cover.len()
        );
        // The branch-cut enclosure does not bypass the correction work budget
        // or publish a partial normal/binormal image.
        let refused = certify_original_rmf_transport(
            &path,
            [0., 0., 1.],
            4096,
            report.cells - 1,
            1000000,
            true,
        )
        .unwrap();
        assert_eq!(refused.status, Status::Unresolved);
        assert!(refused.cover.is_none() && refused.closing_angle.is_none());
        assert!(
            refused.arc_corrected_normals.is_none() && refused.arc_corrected_binormals.is_none()
        );
        assert!(!report.continuous_bound && !report.solid_certified);
        use crate::progressive_sweep::{Options, Orientation, Spacing, Sweep, constant_vector_law};
        let profile = crate::primitives::line([0.01, 0., 0.], [0.02, 0., 0.]).unwrap();
        let scale = constant_vector_law([1., 0., 0.]).unwrap();
        let twist = constant_vector_law([0., 0., 0.]).unwrap();
        for spacing in [
            Spacing::Parameter,
            Spacing::ArcLength {
                tolerance: 0.001,
                max_cells: 100000,
            },
        ] {
            let sweep = Sweep::new(
                &profile,
                &path,
                &scale,
                &twist,
                Options {
                    normal: [0., 0., 1.],
                    orientation: Orientation::RotationMinimizing,
                    spacing,
                    initial_sections: 65,
                    max_sections: 65,
                    max_deviation: 1.,
                },
            )
            .unwrap();
            let bound = sweep
                .rmf_spatial_patch_error_bound(65, 4096, 100000, 1000000)
                .unwrap();
            assert_eq!(bound.status, Status::Certified, "{spacing:?}: {bound:?}");
            assert!(bound.within_budget && bound.error_upper.is_some());
            // A finite source enclosure is not permission to exceed tolerance.
            let strict = Sweep::new(
                &profile,
                &path,
                &scale,
                &twist,
                Options {
                    normal: [0., 0., 1.],
                    orientation: Orientation::RotationMinimizing,
                    spacing,
                    initial_sections: 65,
                    max_sections: 65,
                    max_deviation: 1e-6,
                },
            )
            .unwrap()
            .rmf_spatial_patch_error_bound(65, 4096, 100000, 1000000)
            .unwrap();
            assert_eq!(strict.status, Status::Certified);
            assert!(!strict.within_budget);
            // The first-defect owner optionally refines against the requested
            // tolerance; a stricter request may tighten its complete enclosure.
            assert!(strict.error_upper.unwrap() <= bound.error_upper.unwrap());
            assert!(strict.error_upper.unwrap() > 1e-6);
        }
        let mut broken = path;
        broken.control_points[1][0] = f64::from_bits(1f64.to_bits() + 1);
        let refused =
            certify_original_rmf_transport(&broken, [0., 0., 1.], 4096, 100000, 1000000, true)
                .unwrap();
        assert_eq!(refused.status, Status::Unresolved);
        assert!(refused.cover.is_none() && refused.closing_angle.is_none());
    }
    // Independent numerical comparison only: explicit cubic Bernstein jets
    // and RK4 do not participate in the production enclosure/admission.
    pub(crate) fn oracle(path: &Curve, steps: usize) -> Vec<[f64; 3]> {
        let mut n = [1., 0., 0.];
        let mut result = vec![n];
        let per_segment = 16384usize;
        let h = 1. / per_segment as f64;
        let per_cell = per_segment / (steps / 4);
        let rhs = |segment: usize, t: f64, n: [f64; 3]| {
            let p = &path.control_points[3 * segment..3 * segment + 4];
            let v = std::array::from_fn(|k| {
                3. * (1. - t).powi(2) * (p[1][k] - p[0][k])
                    + 6. * t * (1. - t) * (p[2][k] - p[1][k])
                    + 3. * t * t * (p[3][k] - p[2][k])
            });
            let a = std::array::from_fn(|k| {
                6. * (1. - t) * (p[2][k] - 2. * p[1][k] + p[0][k])
                    + 6. * t * (p[3][k] - 2. * p[2][k] + p[1][k])
            });
            let omega = math_core::cross(v, a).map(|x| x / math_core::dot(v, v));
            math_core::cross(omega, n)
        };
        for segment in 0..4 {
            for i in 0..per_segment {
                let t = i as f64 * h;
                let k1 = rhs(segment, t, n);
                let k2 = rhs(
                    segment,
                    t + h * 0.5,
                    std::array::from_fn(|k| n[k] + h * 0.5 * k1[k]),
                );
                let k3 = rhs(
                    segment,
                    t + h * 0.5,
                    std::array::from_fn(|k| n[k] + h * 0.5 * k2[k]),
                );
                let k4 = rhs(segment, t + h, std::array::from_fn(|k| n[k] + h * k3[k]));
                n = std::array::from_fn(|k| {
                    n[k] + h * (k1[k] + 2. * k2[k] + 2. * k3[k] + k4[k]) / 6.
                });
                if (i + 1) % per_cell == 0 {
                    result.push(n);
                }
            }
        }
        result
    }
    #[test]
    fn closed_spatial_source_encloses_nonzero_holonomy_and_discards_partial_work() {
        let path = spatial_loop();
        let before = path.clone();
        let r = certify_original_rmf_transport(&path, [1., 0., 0.], 4096, 100000, 1000000, true)
            .unwrap();
        assert_eq!(r.status, Status::Certified, "{r:?}");
        let coarse =
            certify_original_rmf_transport(&path, [1., 0., 0.], 512, 100000, 1000000, true)
                .unwrap();
        assert_eq!(coarse.status, Status::Certified);
        let coarse_radius = coarse
            .cover
            .as_ref()
            .unwrap()
            .last()
            .unwrap()
            .endpoint_radius_upper;
        let fine_radius = r
            .cover
            .as_ref()
            .unwrap()
            .last()
            .unwrap()
            .endpoint_radius_upper;
        assert!(
            fine_radius < coarse_radius * 0.5,
            "source refinement must reduce certified accumulated uncertainty"
        );
        eprintln!("spatial RMF endpoint radii: 512={coarse_radius}, 4096={fine_radius}");
        let cover = r.cover.as_ref().unwrap();
        assert_eq!(cover.len(), 4096);
        let independent = oracle(&path, 4096);
        for (i, cell) in cover.iter().enumerate() {
            for k in 0..3 {
                let x = independent[i + 1][k];
                assert!(
                    cell.endpoint_normal[k][0] - 1e-10 <= x
                        && x <= cell.endpoint_normal[k][1] + 1e-10
                );
                assert!(
                    cell.normal[k][0] - 1e-10 <= independent[i][k]
                        && independent[i][k] <= cell.normal[k][1] + 1e-10
                );
            }
        }
        let end = independent.last().unwrap();
        let angle = ((end[2] - end[1]) / 2f64.sqrt()).atan2(end[0]);
        assert!(
            angle.abs() > 1e-3,
            "spatial fixture must have nonzero holonomy: {angle}"
        );
        let section = certify_original_rmf_section_images(
            &path,
            [1., 0., 0.],
            &[[[2., 2.], [-3., -3.], [0.5, 0.5]]],
            4096,
            100000,
            1000000,
            true,
        )
        .unwrap();
        assert_eq!(section.transport.status, Status::Certified);
        let corrected = r.arc_corrected_normals.as_ref().unwrap();
        assert_eq!(corrected.len(), cover.len());
        assert_eq!(
            r.arc_corrected_binormals.as_ref().unwrap().len(),
            cover.len()
        );
        // Independent numerical arc quadrature and Rodrigues comparison.
        // These sampled checks do not participate in production admission.
        let derivative = |u: f64| {
            let segment = ((u * 4.).floor() as usize).min(3);
            let t = u * 4. - segment as f64;
            let p = &path.control_points[3 * segment..3 * segment + 4];
            std::array::from_fn::<_, 3, _>(|k| {
                4. * (3. * (1. - t).powi(2) * (p[1][k] - p[0][k])
                    + 6. * t * (1. - t) * (p[2][k] - p[1][k])
                    + 3. * t * t * (p[3][k] - p[2][k]))
            })
        };
        let speeds: Vec<_> = (0..4096)
            .map(|i| {
                let u = (i as f64 + 0.5) / 4096.;
                let d = derivative(u);
                d.iter().map(|x| x * x).sum::<f64>().sqrt()
            })
            .collect();
        let total: f64 = speeds.iter().sum();
        let mut prefix = 0.;
        for i in 0..4096 {
            prefix += speeds[i];
            let phase = angle * prefix / total;
            let d = derivative((i + 1) as f64 / 4096.);
            let speed = d.iter().map(|x| x * x).sum::<f64>().sqrt();
            let t = [d[0] / speed, d[1] / speed, d[2] / speed];
            let n = independent[i + 1];
            let cross = math_core::cross(t, n);
            let dot = math_core::dot(t, n);
            let rotated: [f64; 3] = std::array::from_fn(|k| {
                n[k] * phase.cos() + cross[k] * phase.sin() + t[k] * dot * (1. - phase.cos())
            });
            let binormal = math_core::cross(t, rotated);
            for k in 0..3 {
                let image = 2. * rotated[k] - 3. * binormal[k] + 0.5 * t[k];
                let enclosed = section.images.as_ref().unwrap()[i][0][k];
                assert!(enclosed[0] - 1e-8 <= image && image <= enclosed[1] + 1e-8);
                let value =
                    n[k] * phase.cos() + cross[k] * phase.sin() + t[k] * dot * (1. - phase.cos());
                assert!(corrected[i][k][0] - 1e-8 <= value && value <= corrected[i][k][1] + 1e-8);
            }
        }
        let bound = r.closing_angle.unwrap();
        assert!(bound[0] <= angle && angle <= bound[1]);
        assert!(bound[1] - bound[0] < 1.);
        assert!(!r.continuous_bound && !r.solid_certified);
        let short =
            certify_original_rmf_transport(&path, [1., 0., 0.], 4096, r.cells - 1, 1000000, true)
                .unwrap();
        assert!(
            short.cover.is_none()
                && short.closing_angle.is_none()
                && short.arc_corrected_normals.is_none()
                && short.cells <= r.cells - 1
        );
        assert!(
            certify_original_rmf_transport(
                &path,
                [1., 0., 0.],
                4096,
                100000,
                r.exact_work - 1,
                true
            )
            .unwrap()
            .cover
            .is_none()
        );
        let mut broken = path.clone();
        broken.control_points[1][0] = f64::from_bits(broken.control_points[1][0].to_bits() + 1);
        assert!(
            certify_original_rmf_transport(&broken, [1., 0., 0.], 4096, 100000, 1000000, true)
                .unwrap()
                .cover
                .is_none()
        );
        assert_eq!(path, before);
    }
    #[test]
    fn section_images_cover_all_axes_and_refuse_partial_coordinates() {
        let path = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![0., 0., 0.], vec![0., 0., 3.]],
            weights: vec![1., 1.],
            periodic: false,
        };
        let coordinates = [
            [[2., 2.], [-3., -3.], [4., 4.]],
            [[0., 1.], [2., 3.], [-1., 0.]],
        ];
        let r = certify_original_rmf_section_images(
            &path,
            [1., 0., 0.],
            &coordinates,
            8,
            1000,
            1000000,
            false,
        )
        .unwrap();
        assert_eq!(r.transport.status, Status::Certified);
        for cell in r.images.as_ref().unwrap() {
            for (k, value) in [2., -3., 4.].into_iter().enumerate() {
                assert!(cell[0][k][0] <= value && value <= cell[0][k][1]);
                assert!(
                    cell[1][k][0] <= coordinates[1][k][0] && coordinates[1][k][1] <= cell[1][k][1]
                );
            }
        }
        let short = certify_original_rmf_section_images(
            &path,
            [1., 0., 0.],
            &coordinates,
            8,
            r.transport.cells - 1,
            1000000,
            false,
        )
        .unwrap();
        assert!(short.images.is_none() && short.transport.cover.is_none());
        assert!(!r.transport.continuous_bound && !r.transport.solid_certified);
    }
    #[test]
    fn straight_original_cover_keeps_full_domain_budget_and_seed_scope() {
        let path = crate::primitives::line([0., 0., 0.], [0., 0., 4.]).unwrap();
        let r =
            certify_original_rmf_transport(&path, [1., 0., 0.], 16, 10000, 100000, false).unwrap();
        assert_eq!(r.status, Status::Certified, "{r:?}");
        assert!(!r.continuous_bound && !r.solid_certified && r.closing_angle.is_none());
        for c in r.cover.as_ref().unwrap() {
            for k in 0..3 {
                let x = if k == 0 { 1. } else { 0. };
                assert!(c.normal[k][0] <= x && x <= c.normal[k][1]);
            }
            assert!(c.endpoint_radius_upper < 1e-8);
        }
        let short =
            certify_original_rmf_transport(&path, [1., 0., 0.], 16, r.cells - 1, 100000, false)
                .unwrap();
        assert!(short.cover.is_none() && short.cells <= r.cells - 1);
        assert!(
            certify_original_rmf_transport(&path, [1., 0., 0.], 16, 10000, r.exact_work - 1, false)
                .unwrap()
                .cover
                .is_none()
        );
        assert!(
            certify_original_rmf_transport(&path, [0., 0., 1.], 16, 10000, 100000, false)
                .unwrap()
                .cover
                .is_none()
        );
    }
