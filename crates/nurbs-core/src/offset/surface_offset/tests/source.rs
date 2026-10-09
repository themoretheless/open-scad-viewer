use super::*;
fn plane() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn folded_periodic_plane() -> Surface {
    let mut s = plane();
    s.periodic_u = true;
    s.knots_u = vec![-1., 0., 1., 2., 3.];
    s.control_points.push(s.control_points[0].clone());
    s.weights.push(s.weights[0].clone());
    s
}
fn cylinder() -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![[3., 0.], [3., 3.], [0., 3.]]
            .into_iter()
            .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 5.]])
            .collect(),
        weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.]
            .into_iter()
            .map(|w| vec![w; 2])
            .collect(),
        periodic_u: false,
        periodic_v: false,
    }
}
fn enclosed(image: [[f64; 2]; 3], point: [f64; 3]) {
    for k in 0..3 {
        assert!(
            image[k][0] <= point[k] && point[k] <= image[k][1],
            "{image:?} misses {point:?}"
        );
    }
}
#[test]
fn signed_plane_offsets_and_jets_preserve_authored_geometry() {
    let s = plane();
    let before = s.clone();
    for distance in [-2., 0., 2.] {
        let r = bounds(&s, [[0., 1.], [0., 1.]], distance, 1).unwrap();
        enclosed(r.image.unwrap(), [0., 0., distance]);
        enclosed(r.image.unwrap(), [1., 1., distance]);
        let e = evaluate(&s, [0.37, 0.62], distance).unwrap();
        assert_eq!(e.point, [0.37, 0.62, distance]);
        assert_eq!(e.du, [1., 0., 0.]);
        assert_eq!(e.dv, [0., 1., 0.]);
    }
    assert_eq!(s, before);
}
#[test]
fn rational_cylinder_offsets_cover_complete_parameter_cells() {
    let s = cylinder();
    for distance in [-2., 1.5] {
        for i in 0..16 {
            let lo = i as f64 / 16.;
            let hi = (i + 1) as f64 / 16.;
            let r = bounds(&s, [[lo, hi], [0., 1.]], distance, 1).unwrap();
            assert!(r.image.is_some(), "{r:?}");
            let jacobian = jacobian_bounds(&s, [[lo, hi], [0., 1.]], distance, 1).unwrap();
            assert!(jacobian.derivatives.is_some(), "{jacobian:?}");
            for u in [lo, (lo + hi) / 2., hi] {
                let e = evaluate(&s, [u, 0.37], distance).unwrap();
                enclosed(r.image.unwrap(), e.point);
                enclosed(jacobian.image.unwrap(), e.point);
                for axis in 0..2 {
                    enclosed(jacobian.derivatives.unwrap()[axis], [e.du, e.dv][axis]);
                }
                assert!((e.point[0].hypot(e.point[1]) - (3. + distance)).abs() < 1e-12);
                assert!((e.point[2] - 1.85).abs() < 1e-12);
                let h = 1e-5;
                if u > h && u < 1. - h {
                    let left = evaluate(&s, [u - h, 0.37], distance).unwrap();
                    let right = evaluate(&s, [u + h, 0.37], distance).unwrap();
                    for k in 0..3 {
                        assert!(
                            ((right.point[k] - left.point[k]) / (2. * h) - e.du[k]).abs()
                                < 1e-8
                        );
                    }
                }
            }
        }
    }
    // The inward radius-three offset collapses to the cylinder axis.
    // An image enclosure is not a proof of a regular offset surface.
    let collapsed = evaluate(&s, [0.37, 0.62], -3.).unwrap();
    assert!(collapsed.du.iter().all(|x| x.abs() < 1e-12));
}
#[test]
fn reversing_a_surface_parameter_requires_reversing_the_signed_offset() {
    let s = cylinder();
    let reversed = s
        .edit_axis(crate::surface::Axis::U, |c| c.reverse())
        .unwrap();
    let a = evaluate(&s, [0.37, 0.62], 1.5).unwrap();
    let b = evaluate(&reversed, [0.63, 0.62], -1.5).unwrap();
    for k in 0..3 {
        assert!((a.point[k] - b.point[k]).abs() < 1e-12);
        assert!((a.du[k] + b.du[k]).abs() < 1e-12);
        assert!((a.dv[k] - b.dv[k]).abs() < 1e-12);
    }
    let r = bounds(&reversed, [[0.6, 0.65], [0.6, 0.65]], -1.5, 1).unwrap();
    enclosed(r.image.unwrap(), b.point);
}
#[test]
fn general_spatial_rational_patch_offset_bounds_and_jets() {
    let s = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        let x = i as f64 / 2.;
                        let y = j as f64 / 2.;
                        let z = 0.2 * x * y + 0.1 * x * x;
                        vec![17. + z, -9. + x, 23. + y]
                    })
                    .collect()
            })
            .collect(),
        weights: (0..3)
            .map(|i| (0..3).map(|j| 1. + (i + j) as f64 / 5.).collect())
            .collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let before = s.clone();
    for distance in [-0.7, 0.7] {
        for i in 0..8 {
            for j in 0..8 {
                let u = [i as f64 / 8., (i + 1) as f64 / 8.];
                let v = [j as f64 / 8., (j + 1) as f64 / 8.];
                let r = bounds(&s, [u, v], distance, 1).unwrap();
                let uv = [(u[0] + u[1]) / 2., (v[0] + v[1]) / 2.];
                let e = evaluate(&s, uv, distance).unwrap();
                enclosed(r.image.unwrap(), e.point);
                let jacobian = jacobian_bounds(&s, [u, v], distance, 1).unwrap();
                enclosed(jacobian.image.unwrap(), e.point);
                enclosed(jacobian.derivatives.unwrap()[0], e.du);
                enclosed(jacobian.derivatives.unwrap()[1], e.dv);
                for axis in 0..2 {
                    let mut left = uv;
                    let mut right = uv;
                    left[axis] -= 1e-5;
                    right[axis] += 1e-5;
                    let left = evaluate(&s, left, distance).unwrap();
                    let right = evaluate(&s, right, distance).unwrap();
                    for k in 0..3 {
                        let derivative = [e.du, e.dv][axis][k];
                        assert!(
                            ((right.point[k] - left.point[k]) / 2e-5 - derivative).abs() < 1e-7
                        );
                    }
                }
            }
        }
    }
    assert_eq!(s, before);
}
#[test]
fn subnormal_regular_source_normals_do_not_require_an_overflowing_reciprocal() {
    let mut s = plane();
    for row in &mut s.control_points {
        for p in row {
            p[0] *= 1e-160;
            p[1] *= 1e-160;
        }
    }
    let r = bounds(&s, [[0., 1.], [0., 1.]], 0.2, 1).unwrap();
    enclosed(r.image.unwrap(), [0., 0., 0.2]);
    enclosed(r.image.unwrap(), [1e-160, 1e-160, 0.2]);
}
#[test]
fn periodic_endpoints_include_the_wrapped_parameter_branch() {
    let s = folded_periodic_plane();
    let d = [[2., 2.], [0.3, 0.3]];
    assert_eq!(s.evaluate(2., 0.3).unwrap().point[0], 0.);
    // The degree-one seam has two different limiting normals; a numerical
    // regular offset jet there is refused. Bounds retain both span sides.
    assert!(evaluate(&s, [2., 0.3], 0.2).is_err());
    let r = bounds(&s, d, 0.2, 2).unwrap();
    enclosed(r.image.unwrap(), [0., 0.3, -0.2]);
    enclosed(r.image.unwrap(), [0., 0.3, 0.2]);
    assert_eq!(r.spans, 2);
    assert!(bounds(&s, d, 0.2, 1).unwrap().image.is_none());
    let j = jacobian_bounds(&s, d, 0.2, 2).unwrap();
    enclosed(j.image.unwrap(), [0., 0.3, -0.2]);
    enclosed(j.image.unwrap(), [0., 0.3, 0.2]);
    enclosed(j.derivatives.unwrap()[0], [1., 0., 0.]);
    enclosed(j.derivatives.unwrap()[0], [-1., 0., 0.]);
    assert!(jacobian_bounds(&s, d, 0.2, 1).unwrap().image.is_none());
}
#[test]
fn periodic_start_bounds_include_the_incident_end_side() {
    let s = folded_periodic_plane();
    let r = bounds(&s, [[0., 1e-6], [0.3, 0.3]], 0.2, 2).unwrap();
    enclosed(r.image.unwrap(), [0., 0.3, 0.2]);
    enclosed(r.image.unwrap(), [0., 0.3, -0.2]);
    assert_eq!(r.spans, 2);
    let r = bounds(&s, [[0., 1e-6], [0.3, 0.3]], 0.2, 1).unwrap();
    assert!(r.image.is_none());
}
#[test]
fn crossing_offset_planes_have_a_unique_section_center() {
    use crate::surface_contact::Verdict;
    let a = plane();
    let mut b = a.clone();
    // B is the XZ plane at Y=0.5, with normal pointing toward -Y.
    for row in &mut b.control_points {
        for p in row {
            let z = p[1];
            p[1] = 0.5;
            p[2] = z;
        }
    }
    let r = certify_contact_section(
        [&a, &b],
        [0.2, 0.2],
        0,
        0.37,
        [0.25, 0.35],
        [[0.32, 0.42], [0.15, 0.25]],
        2,
    )
    .unwrap();
    let Verdict::Witness(w) = r else {
        panic!("{r:?}");
    };
    enclosed(w.point, [0.37, 0.3, 0.2]);
    assert!(w.contraction_upper < 0.5);
    assert!(w.first_uv[1][0] <= 0.3 && w.first_uv[1][1] >= 0.3);
    let coincident = certify_contact_section(
        [&a, &a],
        [0.2, 0.2],
        0,
        0.37,
        [0.25, 0.35],
        [[0.32, 0.42], [0.25, 0.35]],
        2,
    )
    .unwrap();
    assert!(matches!(coincident, Verdict::Unresolved));
    let separated = certify_contact_section(
        [&a, &b],
        [0.2, 0.2],
        0,
        0.37,
        [0.75, 0.85],
        [[0.32, 0.42], [0.15, 0.25]],
        2,
    )
    .unwrap();
    assert!(matches!(separated, Verdict::Excluded));
}
#[test]
fn contact_band_covers_every_driving_parameter_and_refuses_an_inadequate_tube() {
    let a = plane();
    let mut b = a.clone();
    for row in &mut b.control_points {
        for p in row {
            let z = p[1];
            p[1] = 0.5;
            p[2] = z;
        }
    }
    let r = certify_contact_band(
        [&a, &b],
        [0.2, 0.2],
        0,
        [0.35, 0.39],
        [0.25, 0.35],
        [[0.30, 0.44], [0.15, 0.25]],
        2,
    )
    .unwrap();
    let ContactBand::ContinuousBranch(w) = r else {
        panic!("{r:?}")
    };
    assert_eq!(w.first_uv[0], [0.35, 0.39]);
    // Samples check the analytic fixture; the actual proof uses the full
    // interval residual and Jacobian, including both driving endpoints.
    for t in [0.35, 0.351, 0.367, 0.389, 0.39] {
        enclosed(w.point, [t, 0.3, 0.2]);
    }
    let r = certify_contact_band(
        [&a, &b],
        [0.2, 0.2],
        0,
        [0.35, 0.39],
        [0.25, 0.35],
        [[0.36, 0.38], [0.15, 0.25]],
        2,
    )
    .unwrap();
    assert!(matches!(r, ContactBand::Unresolved));
    let r = certify_contact_band(
        [&a, &a],
        [0.2, 0.2],
        0,
        [0.35, 0.39],
        [0.25, 0.35],
        [[0.30, 0.44], [0.25, 0.35]],
        2,
    )
    .unwrap();
    assert!(matches!(r, ContactBand::Unresolved));
    assert!(
        certify_contact_band(
            [&a, &b],
            [0.2, 0.2],
            0,
            [0.39, 0.35],
            [0.25, 0.35],
            [[0.30, 0.44], [0.15, 0.25]],
            2
        )
        .is_err()
    );
}
#[test]
fn rational_cylinder_plane_offsets_have_a_certified_contact_section() {
    use crate::surface_contact::Verdict;
    let a = cylinder();
    let mut b = plane();
    for row in &mut b.control_points {
        for p in row {
            let y = 4. * p[0];
            let z = 5. * p[1];
            *p = vec![2., y, z];
        }
    }
    // Numerical Newton proposes the interval only; inclusion below is
    // established independently from the original interval source jets.
    let mut u = 0.5;
    for _ in 0..8 {
        let e = evaluate(&a, [u, 0.37], 0.2).unwrap();
        u -= (e.point[0] - 2.2) / e.du[0];
    }
    let y = (3.2_f64 * 3.2 - 2.2 * 2.2).sqrt();
    let b_u = y / 4.;
    let band = certify_contact_band(
        [&a, &b],
        [0.2, 0.2],
        1,
        [0.369, 0.371],
        [u - 1e-4, u + 1e-4],
        [[b_u - 1e-4, b_u + 1e-4], [0.367, 0.373]],
        2,
    )
    .unwrap();
    let ContactBand::ContinuousBranch(branch) = band else {
        panic!("{band:?}")
    };
    for v in [0.369, 0.3693, 0.37, 0.3707, 0.371] {
        enclosed(branch.point, [2.2, y, 5. * v]);
    }

    let r = certify_contact_section(
        [&a, &b],
        [0.2, 0.2],
        1,
        0.37,
        [u - 1e-4, u + 1e-4],
        [[b_u - 1e-4, b_u + 1e-4], [0.3699, 0.3701]],
        2,
    )
    .unwrap();
    let Verdict::Witness(w) = r else {
        panic!("{r:?}");
    };
    enclosed(w.point, [2.2, y, 1.85]);
    assert!(w.contraction_upper < 0.5);
    assert!(w.point.iter().all(|p| p[1] - p[0] < 1e-5));
    for side in 0..2 {
        let uv = [w.first_uv, w.second_uv][side].map(|r| (r[0] + r[1]) / 2.);
        let sample = evaluate([&a, &b][side], uv, 0.2).unwrap();
        assert!((sample.point[0] - 2.2).abs() < 1e-5);
        assert!((sample.point[1] - y).abs() < 1e-5);
    }
}
#[test]
fn collapsed_offset_carrier_cannot_certify_a_unique_contact() {
    use crate::surface_contact::Verdict;
    let a = cylinder();
    let mut b = plane();
    for row in &mut b.control_points {
        for p in row {
            *p = vec![2. * p[0] - 1., 2. * p[1] - 1., 1.85];
        }
    }
    let r = certify_contact_section(
        [&a, &b],
        [-3., 0.],
        1,
        0.37,
        [0.36, 0.38],
        [[0.49, 0.51], [0.49, 0.51]],
        2,
    )
    .unwrap();
    assert!(matches!(r, Verdict::Unresolved));
}
#[test]
fn offset_contact_does_not_infer_continuity_from_a_periodic_flag() {
    use crate::surface_contact::Verdict;
    let a = folded_periodic_plane();
    let mut b = plane();
    for row in &mut b.control_points {
        for p in row {
            let z = p[1];
            p[1] = 0.5;
            p[2] = z;
        }
    }
    let r = certify_contact_section(
        [&a, &b],
        [0.2, 0.2],
        0,
        2.,
        [0.25, 0.35],
        [[0., 0.1], [0.15, 0.25]],
        2,
    )
    .unwrap();
    assert!(matches!(r, Verdict::Unresolved));
}
#[test]
fn offset_jacobian_keeps_both_incident_knot_sides_and_budget_limits() {
    let mut s = plane();
    s.knots_u = vec![0., 0., 0.5, 1., 1.];
    s.control_points
        .insert(1, vec![vec![0.25, 0., 0.], vec![0.25, 1., 0.]]);
    s.weights.insert(1, vec![1.; 2]);
    let point = [[0.5, 0.5], [0.3, 0.3]];
    let r = jacobian_bounds(&s, point, 1., 2).unwrap();
    assert_eq!(r.spans, 2);
    // Source U derivative jumps from 0.5 to 1.5; both sides must be enclosed.
    enclosed(r.derivatives.unwrap()[0], [0.5, 0., 0.]);
    enclosed(r.derivatives.unwrap()[0], [1.5, 0., 0.]);
    assert!(
        jacobian_bounds(&s, point, 1., 1)
            .unwrap()
            .derivatives
            .is_none()
    );
    let mut singular = plane();
    singular.control_points[1] = singular.control_points[0].clone();
    assert!(
        jacobian_bounds(&singular, [[0., 1.], [0., 1.]], 1., 1)
            .unwrap()
            .derivatives
            .is_none()
    );
}
#[test]
fn singular_source_and_budget_exhaustion_keep_offset_image_unproven() {
    let mut singular = plane();
    singular.control_points[1] = singular.control_points[0].clone();
    assert!(
        bounds(&singular, [[0., 1.], [0., 1.]], 1., 1)
            .unwrap()
            .image
            .is_none()
    );
    assert!(evaluate(&singular, [0.4, 0.6], 1.).is_err());
    let mut s = plane();
    s.knots_u = vec![0., 0., 0.5, 1., 1.];
    s.control_points
        .insert(1, vec![vec![0.5, 0., 0.], vec![0.5, 1., 0.]]);
    s.weights.insert(1, vec![1.; 2]);
    let incomplete = bounds(&s, [[0., 1.], [0., 1.]], 1., 1).unwrap();
    assert!(incomplete.image.is_none());
    assert_eq!(incomplete.spans, 1);
    let complete = bounds(&s, [[0., 1.], [0., 1.]], 1., 2).unwrap();
    assert!(complete.image.is_some());
    assert_eq!(complete.spans, 2);
    enclosed(complete.image.unwrap(), [0.5, 0.3, 1.]);
    for distance in [f64::NAN, f64::INFINITY] {
        assert!(bounds(&s, [[0., 1.], [0., 1.]], distance, 2).is_err());
    }
    assert!(bounds(&s, [[-0.1, 1.], [0., 1.]], 1., 2).is_err());
}
#[test]
fn offset_intersection_excludes_only_complete_disjoint_carriers() {
    let a = plane();
    let mut b = a.clone();
    for row in &mut b.control_points {
        for p in row {
            p[2] = 3.;
        }
    }
    let domains = [[[0., 1.], [0., 1.]]; 2];
    let r = intersection_candidates([&a, &b], domains, [1., -1.], 0.1, 100, 1).unwrap();
    assert_eq!(r.reason, "all-excluded");
    assert_eq!(r.visited_boxes, 1);
    assert_eq!(r.excluded_boxes, 1);
    assert!(r.boxes.is_empty() && r.pending.is_empty());
    for row in &mut b.control_points {
        for p in row {
            p[2] = 2.;
        }
    }
    let before = b.clone();
    // Original carriers are separated, but their offset carriers coincide.
    let r = intersection_candidates([&a, &b], domains, [1., -1.], 0.5, 100, 1).unwrap();
    assert_eq!(r.reason, "candidate-boxes");
    assert!(!r.boxes.is_empty());
    assert!(r.pending.is_empty());
    for t in [0., 0.23, 0.5, 0.77, 1.] {
        let uv = [[t, t], [t, t]];
        assert!(r.boxes.iter().any(|cell| (0..2).all(|side| {
            (0..2).all(|axis| {
                cell[side][axis][0] <= uv[side][axis] && uv[side][axis] <= cell[side][axis][1]
            })
        })));
    }
    assert_eq!(b, before);
    let limited = intersection_candidates([&a, &b], domains, [1., -1.], 0.001, 1, 1).unwrap();
    assert_eq!(limited.visited_boxes, 1);
    assert_eq!(limited.reason, "work-limit");
    assert_eq!(limited.pending.len(), 2);
    assert_eq!(limited.excluded_boxes, 0);
}
#[test]
fn singular_offset_regions_remain_pending_and_invalid_inputs_are_not_hidden() {
    let a = plane();
    let mut b = a.clone();
    b.control_points[1] = b.control_points[0].clone();
    let domains = [[[0., 1.], [0., 1.]]; 2];
    let r = intersection_candidates([&a, &b], domains, [1., -1.], 1., 1, 1).unwrap();
    assert_eq!(r.reason, "source-normal-unresolved");
    assert_eq!(r.pending.len(), 1);
    assert_eq!(r.excluded_boxes, 0);
    let mut invalid = b.clone();
    invalid.weights[0][0] = 0.;
    assert!(intersection_candidates([&a, &invalid], domains, [0., 1000.], 1., 1, 1).is_err());
    assert!(intersection_candidates([&a, &b], domains, [f64::NAN, 1.], 1., 1, 1).is_err());
}
