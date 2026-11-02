use nurbs_core::{
    curve::Curve,
    intersection::certified_curve_surface::{
        affine_plane_contacts, original_affine_patch_contacts,
    },
    surface::Surface,
};
fn patch() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-2., -2., 2., 2.],
        knots_v: vec![10., 10., 14., 14.],
        control_points: vec![
            vec![vec![0.1, 0., 0.], vec![0.1, 1., 0.]],
            vec![vec![1.1, 0., 0.1], vec![1.1, 1., 0.1]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn curve(p: usize) -> Curve {
    Curve {
        degree: p,
        knots: [vec![0.; p + 1], vec![1.; p + 1]].concat(),
        control_points: (0..=p)
            .map(|i| {
                vec![
                    if i % 2 == 0 { 0.1 } else { 1.1 },
                    i as f64 / p as f64,
                    if i % 2 == 0 { 0.1 } else { 0. },
                ]
            })
            .collect(),
        weights: vec![1.; p + 1],
        periodic: false,
    }
}
#[test]
fn unrepresentable_plane_contacts_are_admitted_inside_the_original_patch() {
    for order in [2, 4] {
        let c = curve(order);
        let s = patch();
        assert!(affine_plane_contacts(&c, &s, 1e-9, 1000, 100000).is_err());
        let r = original_affine_patch_contacts(&c, &s, 1e-9, 10, 100000).unwrap();
        assert!(r.complete, "{r:?}");
        assert_eq!(r.contacts.len(), 1, "{r:?}");
        let x = &r.contacts[0];
        assert_eq!(x.multiplicities, vec![order]);
        for (i, t) in [0.5, 0., 12.].into_iter().enumerate() {
            assert!(
                x.parameter_box[i][0] <= t && t <= x.parameter_box[i][1],
                "{x:?}"
            );
        }
        assert!(r.exact_work <= 100000);
        assert_eq!(r.scalar_proofs.len(), 1);
    }
}
#[test]
fn finite_membership_separates_exterior_exact_boundary_and_interval_ambiguity() {
    let mut c = curve(2);
    for p in &mut c.control_points {
        p[1] = 2.;
    }
    let r = original_affine_patch_contacts(&c, &patch(), 1e-9, 10, 100000).unwrap();
    assert!(r.complete);
    assert!(r.contacts.is_empty());
    for p in &mut c.control_points {
        p[1] = 0.;
    }
    let r = original_affine_patch_contacts(&c, &patch(), 1e-9, 10, 100000).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.contacts.len(), 1);
    assert_eq!(r.contacts[0].uv_proof,nurbs_core::intersection::certified_curve_surface::OriginalPatchUvProof::ExactBoundaryProjective);
    assert_eq!(r.contacts[0].parameter_box[2], [10., 10.]);
    c.weights = vec![1., 2., 4.];
    let r = original_affine_patch_contacts(&c, &patch(), 1e-9, 10, 100000).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.contacts.len(), 1);
    assert_eq!(r.contacts[0].parameter_box[2], [10., 10.]);
    assert!(r.contacts[0].parameter_box[0][0] < r.contacts[0].parameter_box[0][1]);
    assert_eq!(r.contacts[0].uv_proof, nurbs_core::intersection::certified_curve_surface::OriginalPatchUvProof::OriginalAffineCoordinateBounds);
}
#[test]
fn one_global_exact_budget_and_span_budget_preserve_missing_coverage() {
    let mut c = curve(2);
    let row = c.control_points.clone();
    c.control_points.extend([row[1].clone(), row[2].clone()]);
    c.weights = vec![1.; 5];
    c.knots = vec![0., 0., 0., 1., 1., 2., 2., 2.];
    let r = original_affine_patch_contacts(&c, &patch(), 1e-9, 1, 100000).unwrap();
    assert!(!r.complete);
    assert_eq!(r.spans, 1);
    assert_eq!(r.contacts.len(), 1);
    assert_eq!(r.unresolved_spans.len(), 1);
    let r = original_affine_patch_contacts(&c, &patch(), 1e-9, 10, 0).unwrap();
    assert!(!r.complete);
    assert!(r.contacts.is_empty());
    assert_eq!(r.exact_work, 0);
    assert_eq!(r.unresolved_spans.len(), 2);
}
#[test]
fn coincident_span_and_nonaffine_target_do_not_authorize_isolated_contacts() {
    let mut c = curve(2);
    for p in &mut c.control_points {
        p[0] = 0.1;
        p[2] = 0.;
    }
    let r = original_affine_patch_contacts(&c, &patch(), 1e-9, 10, 100000).unwrap();
    assert!(!r.complete);
    assert!(r.contacts.is_empty());
    assert_eq!(r.unresolved_spans.len(), 1);
    let mut s = patch();
    s.control_points[1][1][0] = 1.1f64.next_up();
    assert!(original_affine_patch_contacts(&curve(2), &s, 1e-9, 10, 100000).is_err());
}

#[test]
fn a_shared_exact_knot_keeps_both_one_sided_multiplicities() {
    let c = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 2., 2., 2.],
        control_points: [1., 0., 0., 0.5, 1.]
            .into_iter()
            .enumerate()
            .map(|(i, x)| vec![x, 0.25 + i as f64 * 0.125, 1.])
            .collect(),
        weights: vec![1.; 5],
        periodic: false,
    };
    let s = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 0., 2.]],
            vec![vec![0., 1., 0.], vec![0., 1., 2.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let r = original_affine_patch_contacts(&c, &s, 1e-9, 10, 100000).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.contacts.len(), 1, "{r:?}");
    assert_eq!(r.contacts[0].parameter_box[0], [1., 1.]);
    assert_eq!(r.contacts[0].source_spans, vec![2, 4]);
    assert_eq!(r.contacts[0].multiplicities, vec![2, 1]);
}
#[test]
fn discontinuous_source_storage_is_not_given_a_continuous_patch_contact_proof() {
    let mut c = curve(2);
    let copy = c.control_points.clone();
    c.control_points.extend(copy);
    c.weights = vec![1.; 6];
    c.knots = vec![0., 0., 0., 1., 1., 1., 2., 2., 2.];
    assert!(original_affine_patch_contacts(&c, &patch(), 1e-9, 10, 100000).is_err());
}

#[test]
fn separable_target_weights_use_the_original_projective_uv_clocks() {
    for transpose in [false, true] {
        let mut s = patch();
        s.weights = vec![vec![1., 3.], vec![2., 6.]];
        if transpose {
            s.control_points = (0..2)
                .map(|j| (0..2).map(|i| s.control_points[i][j].clone()).collect())
                .collect();
            s.weights = (0..2)
                .map(|j| (0..2).map(|i| s.weights[i][j]).collect())
                .collect();
            std::mem::swap(&mut s.knots_u, &mut s.knots_v);
        }
        for order in [2, 4] {
            let r = original_affine_patch_contacts(&curve(order), &s, 1e-9, 10, 100000).unwrap();
            assert!(r.complete, "{r:?}");
            assert_eq!(r.contacts.len(), 1);
            assert_eq!(r.contacts[0].multiplicities, vec![order]);
            let uv = if transpose {
                [11., -2. / 3.]
            } else {
                [-2. / 3., 11.]
            };
            for (axis, t) in uv.into_iter().enumerate() {
                let b = r.contacts[0].parameter_box[axis + 1];
                assert!(b[0] <= t && t <= b[1], "transpose={transpose}: {r:?}");
            }
        }
    }
}
#[test]
fn one_ulp_nonseparable_weights_are_not_approximated_by_rank_one() {
    let mut s = patch();
    s.weights = vec![vec![1., 3.], vec![2., 6f64.next_up()]];
    let r = original_affine_patch_contacts(&curve(2), &s, 1e-9, 10, 100000).unwrap();
    assert!(!r.target_weights_separable);
    assert!(r.complete, "{r:?}");
    assert_eq!(r.contacts[0].uv_proof,nurbs_core::intersection::certified_curve_surface::OriginalPatchUvProof::InjectiveBilinearKrawczyk);
    let (a, b) = (1f64.next_up(), 1. - f64::EPSILON);
    assert_eq!(a * b, 1.); // Rounded determinant vanishes; the exact one does not.
    s.weights = vec![vec![a, 1.], vec![1., b]];
    let r = original_affine_patch_contacts(&curve(2), &s, 1e-9, 10, 100000).unwrap();
    assert!(!r.target_weights_separable);
    assert!(r.complete, "{r:?}");
    assert_eq!(r.contacts[0].uv_proof,nurbs_core::intersection::certified_curve_surface::OriginalPatchUvProof::InjectiveBilinearKrawczyk);
}

#[test]
fn coupled_bilinear_uv_inverse_has_global_uniqueness_and_original_root_boxes() {
    for weights in [
        vec![vec![1., 1.], vec![1., 2.]],
        vec![vec![1., 1.], vec![2., 2.5]],
    ] {
        for transpose in [false, true] {
            let mut s = patch();
            s.weights = weights.clone();
            if transpose {
                s.control_points = (0..2)
                    .map(|j| (0..2).map(|i| s.control_points[i][j].clone()).collect())
                    .collect();
                s.weights = (0..2)
                    .map(|j| (0..2).map(|i| s.weights[i][j]).collect())
                    .collect();
                std::mem::swap(&mut s.knots_u, &mut s.knots_v);
            }
            for order in [2, 4] {
                let r =
                    original_affine_patch_contacts(&curve(order), &s, 1e-9, 10, 100000).unwrap();
                assert!(r.complete, "{r:?}");
                assert!(!r.target_weights_separable);
                let proof = r.target_uv_injectivity.as_ref().unwrap();
                assert!(proof.proven && proof.determinant_factorization_verified);
                assert_eq!(r.contacts.len(), 1);
                assert_eq!(r.contacts[0].multiplicities, vec![order]);
                let (a, b, c, d) = (weights[0][0], weights[1][0], weights[0][1], weights[1][1]);
                let u = (a * c).sqrt() / ((a * c).sqrt() + (b * d).sqrt());
                let v = (a * b).sqrt() / ((a * b).sqrt() + (c * d).sqrt());
                let uv = if transpose {
                    [10. + 4. * v, -2. + 4. * u]
                } else {
                    [-2. + 4. * u, 10. + 4. * v]
                };
                for (axis, x) in uv.into_iter().enumerate() {
                    let b = r.contacts[0].parameter_box[axis + 1];
                    assert!(b[0] <= x && x <= b[1], "{r:?}");
                }
                assert_eq!(r.contacts[0].uv_proof,nurbs_core::intersection::certified_curve_surface::OriginalPatchUvProof::InjectiveBilinearKrawczyk);
            }
        }
    }
}

#[test]
fn a_global_bijection_proof_preserves_contact_when_a_tight_numeric_uv_box_fails() {
    let mut s = patch();
    s.weights = vec![vec![1., 1.], vec![1., 1e12]];
    let r = original_affine_patch_contacts(&curve(2), &s, 1e-9, 10, 100000).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.contacts.len(), 1);
    assert!(r.target_uv_injectivity.as_ref().unwrap().proven);
    assert_eq!(r.contacts[0].uv_proof,nurbs_core::intersection::certified_curve_surface::OriginalPatchUvProof::PositiveBilinearGlobalInverse);
    assert_eq!(r.contacts[0].parameter_box[1], [-2., 2.]);
    assert_eq!(r.contacts[0].parameter_box[2], [10., 14.]);
}

fn flat_patch() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-2., -2., 2., 2.],
        knots_v: vec![10., 10., 14., 14.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 0., 1.]],
            vec![vec![0., 1., 0.], vec![0., 1., 1.]],
        ],
        weights: vec![vec![1., 2.], vec![3., 5.]],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn exact_edge_and_corner_contacts_preserve_coupled_projective_clocks() {
    for (a, b, uv) in [
        (0., 0.5, [-2., 10. + 4. / 3.]),
        (1., 0.5, [2., 11.5]),
        (0.5, 0., [-1., 10.]),
        (0.5, 1., [-6. / 7., 14.]),
        (0., 0., [-2., 10.]),
        (1., 1., [2., 14.]),
    ] {
        let c = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: [0.25, -0.25, 0.25]
                .into_iter()
                .map(|x| vec![x, a, b])
                .collect(),
            weights: vec![1.; 3],
            periodic: false,
        };
        let r = original_affine_patch_contacts(&c, &flat_patch(), 1e-9, 10, 100000).unwrap();
        assert!(r.complete, "{r:?}");
        assert_eq!(r.contacts.len(), 1);
        let p = &r.contacts[0];
        assert_eq!(p.uv_proof,nurbs_core::intersection::certified_curve_surface::OriginalPatchUvProof::ExactBoundaryProjective);
        assert_eq!(p.parameter_box[0], [0.5, 0.5]);
        for (axis, x) in uv.into_iter().enumerate() {
            let d = p.parameter_box[axis + 1];
            assert!(d[0] <= x && x <= d[1], "{r:?}");
        }
    }
}
#[test]
fn exact_membership_does_not_snap_tiny_offsets_to_the_boundary() {
    for sign in [-1., 1.] {
        let mut c = curve(2);
        let delta = sign * f64::from_bits(1);
        for p in &mut c.control_points {
            p[1] = delta;
        }
        let r = original_affine_patch_contacts(&c, &patch(), 1e-9, 10, 100000).unwrap();
        assert!(r.complete, "{r:?}");
        if sign < 0. {
            assert!(r.contacts.is_empty());
        } else {
            assert_eq!(r.contacts.len(), 1);
            assert_eq!(r.contacts[0].position_bounds[1], [delta, delta]);
            assert_eq!(r.contacts[0].uv_proof,nurbs_core::intersection::certified_curve_surface::OriginalPatchUvProof::PositiveBilinearGlobalInverse);
        }
    }
}
#[test]
fn singleton_root_uses_an_exact_nonbinary_natural_parameter() {
    let mut c = curve(2);
    c.knots = vec![0.1, 0.1, 0.1, 3.1, 3.1, 3.1];
    for p in &mut c.control_points {
        p[1] = 0.;
    }
    let r = original_affine_patch_contacts(&c, &patch(), 1e-9, 10, 100000).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.contacts.len(), 1);
    let t = r.contacts[0].parameter_box[0];
    assert!(t[0] < t[1]);
    assert!(t[0] <= 1.6 && 1.6 <= t[1]);
    assert_eq!(r.contacts[0].parameter_box[2], [10., 10.]);
}
#[test]
fn degree_25_corner_contact_retains_the_exact_order() {
    let c = Curve {
        degree: 25,
        knots: [vec![0.; 26], vec![1.; 26]].concat(),
        control_points: (0..26)
            .map(|i| vec![if i % 2 == 0 { 1. } else { -1. }, 0., 0.])
            .collect(),
        weights: vec![1.; 26],
        periodic: false,
    };
    let r = original_affine_patch_contacts(&c, &flat_patch(), 1e-9, 10, 1000000).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.contacts.len(), 1);
    assert_eq!(r.contacts[0].multiplicities, vec![25]);
    assert_eq!(
        r.contacts[0].parameter_box,
        [[0.5, 0.5], [-2., -2.], [10., 10.]]
    );
}

#[test]
fn exact_boundary_point_uses_original_rational_curve_weights() {
    let c = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: [1., -0.5, 0.25]
            .into_iter()
            .map(|x| vec![x, 0.5, 0.])
            .collect(),
        weights: vec![1., 2., 4.],
        periodic: false,
    };
    let r = original_affine_patch_contacts(&c, &flat_patch(), 1e-9, 10, 100000).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.contacts.len(), 1);
    assert_eq!(r.contacts[0].uv_proof,nurbs_core::intersection::certified_curve_surface::OriginalPatchUvProof::ExactBoundaryProjective);
    assert_eq!(
        r.contacts[0].parameter_box,
        [[0.5, 0.5], [-1., -1.], [10., 10.]]
    );
    assert_eq!(
        r.contacts[0].position_bounds,
        [[0., 0.], [0.5, 0.5], [0., 0.]]
    );
}

#[test]
fn irrational_boundary_root_has_exact_conditional_coordinate() {
    let c = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            vec![-0.5, -0.5, 0.5],
            vec![-0.5, -0.5, 0.5],
            vec![0.5, 0.5, 0.5],
        ],
        weights: vec![1.; 3],
        periodic: false,
    };
    let r = original_affine_patch_contacts(&c, &flat_patch(), 1e-9, 10, 100000).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.contacts.len(), 1);
    let c = &r.contacts[0];
    assert_eq!(c.uv_proof, nurbs_core::intersection::certified_curve_surface::OriginalPatchUvProof::OriginalAffineCoordinateBounds);
    let t = 0.5f64.sqrt();
    assert!(c.parameter_box[0][0] <= t && t <= c.parameter_box[0][1]);
    assert!(c.parameter_box[0][0] < c.parameter_box[0][1]);
    assert_eq!(c.parameter_box[1], [-2., -2.]);
    let v = 10. + 4. / 3.;
    assert!(c.parameter_box[2][0] <= v && v <= c.parameter_box[2][1]);
    assert_eq!(c.position_bounds, [[0., 0.], [0., 0.], [0.5, 0.5]]);
    assert_eq!(c.multiplicities, [1]);
}

#[test]
fn nonproportional_common_root_proves_boundary_membership() {
    // X=(1-3t)^2, Y=1-3t. Their common root does not imply an affine
    // control identity; the exact polynomial common-root proof is required.
    let c = Curve {
        degree: 3,
        knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points: vec![
            vec![1., 1., 0.5],
            vec![-1., 0., 0.5],
            vec![0., -1., 0.5],
            vec![4., -2., 0.5],
        ],
        weights: vec![1.; 4],
        periodic: false,
    };
    let r = original_affine_patch_contacts(&c, &flat_patch(), 1e-9, 10, 100000).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.contacts.len(), 1);
    assert!(r.ambiguous_parameters.is_empty());
    assert_eq!(r.contacts[0].uv_proof,nurbs_core::intersection::certified_curve_surface::OriginalPatchUvProof::OriginalPolynomialCommonRoot);
    assert_eq!(r.contacts[0].parameter_box[1], [-2., -2.]);
    assert_eq!(
        r.contacts[0].position_bounds,
        [[0., 0.], [0., 0.], [0.5, 0.5]]
    );
    assert_eq!(r.contacts[0].multiplicities, [2]);
    assert!(r.exact_work <= 100000);
    assert_eq!(
        r.exact_work,
        r.scalar_proofs.iter().map(|(_, p)| p.work_used).sum()
    );
}

#[test]
fn nonsingleton_roots_preserve_all_edge_and_corner_clocks() {
    for (a, b, u, v) in [
        (0., 0.5, -2., 10. + 4. / 3.),
        (1., 0.5, 2., 11.5),
        (0.5, 0., -1., 10.),
        (0.5, 1., -6. / 7., 14.),
        (0., 0., -2., 10.),
        (1., 1., 2., 14.),
    ] {
        let c = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![1., a, b], vec![-1., a, b], vec![1., a, b]],
            weights: vec![1., 2., 4.],
            periodic: false,
        };
        let r = original_affine_patch_contacts(&c, &flat_patch(), 1e-9, 10, 100000).unwrap();
        assert!(r.complete, "{r:?}");
        assert_eq!(r.contacts.len(), 1);
        let c = &r.contacts[0];
        assert_eq!(c.uv_proof,nurbs_core::intersection::certified_curve_surface::OriginalPatchUvProof::OriginalAffineCoordinateBounds);
        assert_eq!(c.multiplicities, [2]);
        assert!(c.parameter_box[0][0] < c.parameter_box[0][1]);
        for (axis, x) in [(1, u), (2, v)] {
            assert!(
                c.parameter_box[axis][0] <= x && x <= c.parameter_box[axis][1],
                "{c:?}"
            );
        }
        assert_eq!(c.position_bounds, [[0., 0.], [a, a], [b, b]]);
    }
}

#[test]
fn interval_root_signed_subnormal_offsets_are_never_snapped_to_edge() {
    for y in [-f64::from_bits(1), f64::from_bits(1)] {
        let mut c = curve(2);
        c.weights = vec![1., 2., 4.];
        for p in &mut c.control_points {
            p[1] = y;
        }
        let r = original_affine_patch_contacts(&c, &patch(), 1e-9, 10, 100000).unwrap();
        assert!(r.complete, "{r:?}");
        assert_eq!(r.contacts.len(), usize::from(y > 0.));
        if y > 0. {
            assert_eq!(r.contacts[0].position_bounds[1], [y, y]);
        }
    }
    let mut c = curve(2);
    c.weights = vec![1., 2., 4.];
    for p in &mut c.control_points {
        p[1] = 0.;
    }
    c.control_points[2][1] = -f64::from_bits(1);
    let r = original_affine_patch_contacts(&c, &patch(), 1e-9, 10, 100000).unwrap();
    assert!(r.contacts.is_empty(), "{r:?}");
    assert!(!r.complete);
}

#[test]
fn interval_edge_identity_uses_only_original_active_span_controls() {
    let mut c = curve(2);
    for p in &mut c.control_points {
        p[1] = 0.;
    }
    c.control_points
        .extend([vec![0.1, 1., 0.1], vec![0.1, -1., 0.1]]);
    c.weights = vec![1., 2., 4., 4., 4.];
    c.knots = vec![0., 0., 0., 1., 1., 2., 2., 2.];
    let r = original_affine_patch_contacts(&c, &patch(), 1e-9, 10, 100000).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.spans, 2);
    assert_eq!(r.contacts.len(), 1);
    assert_eq!(r.contacts[0].source_spans, [2]);
    assert_eq!(r.contacts[0].parameter_box[2], [10., 10.]);
}

#[test]
fn polynomial_common_roots_admit_each_edge_and_corner_without_affine_identity() {
    use nurbs_core::intersection::certified_curve_surface::OriginalPatchUvProof;
    for edge in 0..6 {
        let residual = [1., 0., -1., -2.];
        let controls = [1., -1., 0., 4.]
            .into_iter()
            .enumerate()
            .map(|(i, x)| {
                let (a, b) = match edge {
                    0 => (residual[i], 0.5),
                    1 => (1. + residual[i], 0.5),
                    2 => (0.5, residual[i]),
                    3 => (0.5, 1. + residual[i]),
                    4 => (residual[i], residual[i]),
                    _ => (1. + residual[i], 1. + residual[i]),
                };
                vec![x, a, b]
            })
            .collect();
        let c = Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: controls,
            weights: vec![1.; 4],
            periodic: false,
        };
        let r = original_affine_patch_contacts(&c, &flat_patch(), 1e-9, 10, 100000).unwrap();
        assert!(r.complete, "edge {edge}: {r:?}");
        assert_eq!(r.contacts.len(), 1);
        let c = &r.contacts[0];
        assert_eq!(
            c.uv_proof,
            OriginalPatchUvProof::OriginalPolynomialCommonRoot
        );
        let (u, v) = match edge {
            0 => (-2., 10. + 4. / 3.),
            1 => (2., 11.5),
            2 => (-1., 10.),
            3 => (-6. / 7., 14.),
            4 => (-2., 10.),
            _ => (2., 14.),
        };
        for (axis, x) in [(1, u), (2, v)] {
            assert!(
                c.parameter_box[axis][0] <= x && x <= c.parameter_box[axis][1],
                "{c:?}"
            );
        }
        assert_eq!(
            r.exact_work,
            r.scalar_proofs.iter().map(|(_, p)| p.work_used).sum()
        );
    }
}

#[test]
fn nonproportional_irrational_boundary_root_is_proved_exactly() {
    let c = Curve {
        degree: 4,
        knots: vec![0., 0., 0., 0., 0., 1., 1., 1., 1., 1.],
        control_points: [3., 3., 1., -3., 3.]
            .into_iter()
            .zip([-3., -3., -2., 0., 3.])
            .map(|(x, y)| vec![x, y, 0.5])
            .collect(),
        weights: vec![1.; 5],
        periodic: false,
    };
    let r = original_affine_patch_contacts(&c, &flat_patch(), 1e-9, 10, 100000).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.contacts.len(), 1);
    let p = &r.contacts[0];
    let t = 0.5f64.sqrt();
    assert!(p.parameter_box[0][0] <= t && t <= p.parameter_box[0][1]);
    assert_eq!(p.position_bounds, [[0., 0.], [0., 0.], [0.5, 0.5]]);
    assert_eq!(p.multiplicities, [2]);
    assert_eq!(p.uv_proof,nurbs_core::intersection::certified_curve_surface::OriginalPatchUvProof::OriginalPolynomialCommonRoot);
}

#[test]
fn boundary_common_root_exhaustion_preserves_missing_coverage_and_work_sum() {
    let mut c = Curve {
        degree: 3,
        knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points: vec![
            vec![1., 2., 0.5],
            vec![-1., 2., 0.5],
            vec![0., 2., 0.5],
            vec![4., 2., 0.5],
        ],
        weights: vec![1.; 4],
        periodic: false,
    };
    let base = original_affine_patch_contacts(&c, &flat_patch(), 1e-9, 10, 100000).unwrap();
    assert!(base.complete);
    assert!(base.contacts.is_empty());
    for (p, y) in c.control_points.iter_mut().zip([1., 0., -1., -2.]) {
        p[1] = y;
    }
    let budget = base.exact_work + 10;
    let r = original_affine_patch_contacts(&c, &flat_patch(), 1e-9, 10, budget).unwrap();
    assert!(!r.complete, "{r:?}");
    assert!(r.contacts.is_empty());
    assert!(r.exact_work <= budget);
    assert_eq!(
        r.exact_work,
        r.scalar_proofs.iter().map(|(_, p)| p.work_used).sum()
    );
}

#[test]
fn common_root_boundary_uses_original_skew_patch_expressions() {
    let mut s = flat_patch();
    let o = [2., 3., 5.];
    let u = [1., 2., 3.];
    let v = [-1., 1., 2.];
    let n = [1., -5., 3.];
    s.control_points = (0..2)
        .map(|i| {
            (0..2)
                .map(|j| {
                    (0..3)
                        .map(|k| o[k] + i as f64 * u[k] + j as f64 * v[k])
                        .collect()
                })
                .collect()
        })
        .collect();
    let c = Curve {
        degree: 3,
        knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points: [1., -1., 0., 4.]
            .into_iter()
            .zip([1., 0., -1., -2.])
            .map(|(x, a)| {
                (0..3)
                    .map(|k| o[k] + a * u[k] + 0.5 * v[k] + x * n[k])
                    .collect()
            })
            .collect(),
        weights: vec![1.; 4],
        periodic: false,
    };
    let r = original_affine_patch_contacts(&c, &s, 1e-9, 10, 100000).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.contacts.len(), 1);
    let p = &r.contacts[0];
    assert_eq!(p.parameter_box[1], [-2., -2.]);
    for (axis, x) in [1.5, 3.5, 6.].into_iter().enumerate() {
        assert!(p.position_bounds[axis][0] <= x && x <= p.position_bounds[axis][1]);
    }
    assert_eq!(p.uv_proof,nurbs_core::intersection::certified_curve_surface::OriginalPatchUvProof::OriginalPolynomialCommonRoot);
}

#[test]
fn nearby_distinct_edge_root_never_admits_an_exterior_plane_contact() {
    let delta = 2f64.powi(-40);
    let c = Curve {
        degree: 3,
        knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points: [1., -1., 0., 4.]
            .into_iter()
            .zip([1., 0., -1., -2.])
            .map(|(x, y)| vec![x, y - delta, 0.5])
            .collect(),
        weights: vec![1.; 4],
        periodic: false,
    };
    let r = original_affine_patch_contacts(&c, &flat_patch(), 1e-9, 10, 100000).unwrap();
    assert!(r.contacts.is_empty(), "{r:?}");
    assert!(
        !r.complete,
        "Near but distinct roots require a sign proof to classify exterior"
    );
}
