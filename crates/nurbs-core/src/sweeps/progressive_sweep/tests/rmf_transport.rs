
    use super::*;
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
        let mut broken = path;
        broken.weights[1] = f64::from_bits(1.25_f64.to_bits() + 1);
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
        assert_eq!(report.cover.as_ref().unwrap().len(), 4096);
        assert_eq!(report.arc_corrected_normals.as_ref().unwrap().len(), 4096);
        assert_eq!(report.arc_corrected_binormals.as_ref().unwrap().len(), 4096);
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
            assert_eq!(strict.error_upper, bound.error_upper);
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
