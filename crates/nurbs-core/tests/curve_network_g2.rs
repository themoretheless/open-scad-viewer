use nurbs_core::{
    constrained_surface::{
        Constraint,
        network::{
            self, BoundaryLocks, CurvatureConstraint, CurvatureOptions, CurveConstraint,
            SupportTrace,
        },
    },
    curve::Curve,
    patches,
    surface::{Axis, Surface},
    surface_join::{Boundary, Decision},
};
fn plane(degree: usize, y: f64) -> Surface {
    patches::bezier(
        (0..=degree)
            .map(|i| {
                (0..=degree)
                    .map(|j| vec![i as f64 / degree as f64, y * j as f64 / degree as f64, 0.])
                    .collect()
            })
            .collect(),
        None,
    )
    .unwrap()
}
fn pc(v: f64) -> Curve {
    Curve::from_polyline(vec![vec![0., v], vec![1., v]]).unwrap()
}
fn transverse(b: f64) -> Surface {
    let mut s = plane(3, 1.);
    for row in &mut s.control_points {
        for (j, p) in row.iter_mut().enumerate() {
            p[2] = b * [0.25, -1. / 12., -1. / 12., 0.25][j];
        }
    }
    s
}
fn g2<'a>(c: &'a Curve, p: &'a Curve, s: &'a Surface, sp: &'a Curve) -> CurvatureConstraint<'a> {
    CurvatureConstraint {
        spatial: c,
        parameters: p,
        reverse: false,
        support: SupportTrace {
            surface: s,
            parameters: sp,
            reverse: false,
        },
    }
}
fn admitted(q: &network::Qualified) -> &nurbs_core::surface_g2_pcurves::Report {
    assert!(q.evidence().admitted);
    let network::Evidence::Curvature(r) = &q.evidence().evidence[0] else {
        panic!("No complete curvature proof")
    };
    assert!(r.positions.proven && r.geometric.regularity_proven);
    assert_eq!(r.geometric.decision, Decision::WithinTolerance);
    r
}
#[test]
fn g2_changes_transverse_curvature_that_whole_g1_does_not_constrain() {
    let seed = plane(3, 1.);
    let support = transverse(0.0625);
    let p = pc(0.5);
    let c = Curve::from_polyline(vec![vec![0., 0.5, 0.], vec![1., 0.5, 0.]]).unwrap();
    let before = (seed.clone(), support.clone(), c.clone(), p.clone());
    let g1 = network::solve(
        &seed,
        &[CurveConstraint {
            spatial: &c,
            parameters: &p,
            reverse: false,
            tangent: Some(SupportTrace {
                surface: &support,
                parameters: &p,
                reverse: false,
            }),
        }],
        &[],
        network::Options::default(),
    )
    .unwrap();
    assert!(g1.evidence().admitted);
    let not_g2 = nurbs_core::surface_g2_pcurves::inspect(
        nurbs_core::surface_g2_pcurves::PcurveSeam { curve: &c, pcurves: [&p, &p], surfaces: [g1.surface(), &support], reversed: [false; 2] },
        nurbs_core::surface_g2_pcurves::G2PcurveTolerance { position: 1e-8, normal: 1e-5, curvature: 1e-5 },
        100000,
        1000000,
    )
    .unwrap();
    assert_ne!(not_g2.geometric.decision, Decision::WithinTolerance);
    let out = network::solve_with_curvature(
        &seed,
        &[],
        &[g2(&c, &p, &support, &p)],
        &[Constraint::Derivative {
            uv: [0.5, 0.5],
            axis: nurbs_core::constrained_surface::Axis::V,
            value: [0., 1., 0.],
        }],
        None,
        CurvatureOptions::default(),
    )
    .unwrap();
    admitted(&out);
    assert!(out.iterations() > 1);
    let (_, _, vv) = out
        .surface()
        .evaluate(0.5, 0.5)
        .unwrap()
        .second_derivatives()
        .unwrap();
    assert!((vv[2] - 0.125).abs() < 1e-5);
    assert_eq!(out.original_curvature_guides().len(), 1);
    assert!(out.reverify(100000, 1000000).unwrap().admitted);
    assert!(out.work().0 <= 100000 && out.work().1 <= 1000000);
    assert_eq!((seed, support, c, p), before);
}
#[test]
fn geometric_g2_uses_world_curvature_with_different_uv_tangent_scales() {
    let (a, b) = (0.01, 0.02);
    let mut support = plane(3, 1.);
    for (i, row) in support.control_points.iter_mut().enumerate() {
        for (j, p) in row.iter_mut().enumerate() {
            p[2] = a * (i * i.saturating_sub(1)) as f64 / 6.
                + b * (j * j.saturating_sub(1)) as f64 / 6.;
        }
    }
    let seed = plane(3, 2.);
    let p = pc(0.25);
    let sp = pc(0.5);
    let c = support.iso(Axis::V, 0.5).unwrap();
    let mut options = CurvatureOptions::default();
    options.network.max_iterations = 8;
    let out = network::solve_with_curvature(
        &seed,
        &[],
        &[g2(&c, &p, &support, &sp)],
        &[Constraint::Derivative {
            uv: [0.5, 0.25],
            axis: nurbs_core::constrained_surface::Axis::V,
            value: [0., 2., 2. * b],
        }],
        None,
        options,
    )
    .unwrap();
    admitted(&out);
    let at = out.surface().evaluate(0.5, 0.25).unwrap();
    let (_, _, vv) = at.second_derivatives().unwrap();
    let n = support.evaluate(0.5, 0.5).unwrap().unit_normal().unwrap();
    let actual = (0..3).map(|k| vv[k] * n[k]).sum::<f64>();
    let expected = 8. * b / (1. + a * a + b * b).sqrt();
    assert!(
        (actual - expected).abs() < 5e-5,
        "{actual} versus {expected}"
    );
    assert!(out.reverify(100000, 1000000).unwrap().admitted);
}
#[test]
fn nonlinear_g2_can_be_combined_with_four_original_boundary_two_jets() {
    let seed = plane(6, 1.);
    let mut support = seed.clone();
    support.control_points[3][3][2] = 0.1;
    let p = pc(0.5);
    let c = support.iso(Axis::V, 0.5).unwrap();
    let sides = [
        Boundary::UMin,
        Boundary::UMax,
        Boundary::VMin,
        Boundary::VMax,
    ];
    let out = network::solve_with_curvature(
        &seed,
        &[],
        &[g2(&c, &p, &support, &p)],
        &[],
        Some(BoundaryLocks {
            reference: &seed,
            boundaries: &sides,
            tolerances: [1e-8, 1e-6, 1e-6],
        }),
        CurvatureOptions::default(),
    )
    .unwrap();
    admitted(&out);
    let replay = out.reverify(100000, 1000000).unwrap();
    assert!(replay.admitted && replay.boundary_seams.len() == 4);
    assert_eq!(out.surface().weights, seed.weights);
    for t in [0., 0.13, 0.5, 0.91, 1.] {
        for uv in [[0., t], [1., t], [t, 0.], [t, 1.]] {
            let a = seed.evaluate(uv[0], uv[1]).unwrap();
            let b = out.surface().evaluate(uv[0], uv[1]).unwrap();
            assert_eq!(a.point, b.point);
            assert_eq!(a.first_derivatives(), b.first_derivatives());
            assert_eq!(a.second_derivatives(), b.second_derivatives());
        }
    }
}
#[test]
fn incompatible_curvature_fields_and_exhaustion_publish_no_qualified_patch() {
    let seed = plane(3, 1.);
    let a = transverse(0.0625);
    let b = transverse(0.125);
    let p = pc(0.5);
    let c = Curve::from_polyline(vec![vec![0., 0.5, 0.], vec![1., 0.5, 0.]]).unwrap();
    let before = (seed.clone(), a.clone(), b.clone(), c.clone());
    let derivatives = [Constraint::Derivative {
        uv: [0.5, 0.5],
        axis: nurbs_core::constrained_surface::Axis::V,
        value: [0., 1., 0.],
    }];
    assert!(
        network::solve_with_curvature(
            &seed,
            &[],
            &[g2(&c, &p, &a, &p), g2(&c, &p, &b, &p)],
            &derivatives,
            None,
            CurvatureOptions::default()
        )
        .is_err()
    );
    let mut low = CurvatureOptions::default();
    low.network.max_iterations = 1;
    assert!(
        network::solve_with_curvature(&seed, &[], &[g2(&c, &p, &a, &p)], &derivatives, None, low)
            .is_err()
    );
    low.network.max_cells = 1;
    assert!(
        network::solve_with_curvature(&seed, &[], &[g2(&c, &p, &a, &p)], &[], None, low).is_err()
    );
    low = CurvatureOptions::default();
    low.curvature_tolerance = f64::NAN;
    assert!(
        network::solve_with_curvature(&seed, &[], &[g2(&c, &p, &a, &p)], &[], None, low).is_err()
    );
    assert_eq!((seed, a, b, c), before);
}

#[test]
fn near_identical_high_degree_original_fields_have_common_basis_g2_evidence() {
    let mut support = plane(6, 1.);
    support.control_points[3][3][2] = 0.1;
    let mut actual = support.clone();
    actual.control_points[3][3][2] = 0.1f64.next_up();
    let p = pc(0.5);
    let c = support.iso(Axis::V, 0.5).unwrap();
    let proof = nurbs_core::surface_g2_pcurves::inspect(
        nurbs_core::surface_g2_pcurves::PcurveSeam { curve: &c, pcurves: [&p, &p], surfaces: [&actual, &support], reversed: [false; 2] },
        nurbs_core::surface_g2_pcurves::G2PcurveTolerance { position: 1e-8, normal: 1e-5, curvature: 1e-5 },
        100,
        0,
    )
    .unwrap();
    assert!(proof.positions.proven);
    assert_eq!(proof.geometric.decision, Decision::WithinTolerance);
    assert!(proof.geometric.regularity_proven && proof.geometric.cells <= 100);
}

#[test]
fn nonlinear_g2_retains_original_nonuniform_rational_weights() {
    let mut support = transverse(0.0625);
    for row in &mut support.weights {
        row.copy_from_slice(&[1., 1.5, 0.75, 1.]);
    }
    let mut seed = support.clone();
    for p in seed.control_points.iter_mut().flatten() {
        p[2] = 0.;
    }
    let p = pc(0.5);
    let c = support.iso(Axis::V, 0.5).unwrap();
    let source = support.evaluate(0.5, 0.5).unwrap();
    let (_, v) = source.first_derivatives().unwrap();
    let out = network::solve_with_curvature(
        &seed,
        &[],
        &[g2(&c, &p, &support, &p)],
        &[Constraint::Derivative {
            uv: [0.5, 0.5],
            axis: nurbs_core::constrained_surface::Axis::V,
            value: v,
        }],
        None,
        CurvatureOptions::default(),
    )
    .unwrap();
    admitted(&out);
    assert_eq!(out.surface().weights, seed.weights);
    let (_, _, original) = source.second_derivatives().unwrap();
    let (_, _, actual) = out
        .surface()
        .evaluate(0.5, 0.5)
        .unwrap()
        .second_derivatives()
        .unwrap();
    let normal = source.unit_normal().unwrap();
    let error = (0..3)
        .map(|k| (actual[k] - original[k]) * normal[k])
        .sum::<f64>()
        .abs();
    assert!(error < 1e-5);
    assert!(out.reverify(100000, 1000000).unwrap().admitted);
}

#[test]
fn opposite_support_orientation_preserves_the_signed_world_tensor_branch() {
    let seed = plane(3, 1.);
    let original = transverse(0.0625);
    let support = nurbs_core::surface_edit::reverse(&original, Axis::U).unwrap();
    let p = pc(0.5);
    let sp = p.reverse().unwrap();
    let c = Curve::from_polyline(vec![vec![0., 0.5, 0.], vec![1., 0.5, 0.]]).unwrap();
    let out = network::solve_with_curvature(
        &seed,
        &[],
        &[g2(&c, &p, &support, &sp)],
        &[Constraint::Derivative {
            uv: [0.5, 0.5],
            axis: nurbs_core::constrained_surface::Axis::V,
            value: [0., 1., 0.],
        }],
        None,
        CurvatureOptions::default(),
    )
    .unwrap();
    let report = admitted(&out);
    assert_eq!(
        report.geometric.orientation,
        Some(nurbs_core::surface_g2::Orientation::Opposite)
    );
    assert!(out.reverify(100000, 1000000).unwrap().admitted);
}

#[test]
fn reversed_original_g2_clocks_preserve_the_complete_curve_constraint() {
    let seed = plane(3, 1.);
    let support = transverse(0.0625);
    let p = pc(0.5);
    let c = Curve::from_polyline(vec![vec![0., 0.5, 0.], vec![1., 0.5, 0.]]).unwrap();
    for reverse in [false, true] {
        for support_reverse in [false, true] {
            let candidate_pc = if reverse {
                p.reverse().unwrap()
            } else {
                p.clone()
            };
            let support_pc = if support_reverse {
                p.reverse().unwrap()
            } else {
                p.clone()
            };
            let out = network::solve_with_curvature(
                &seed,
                &[],
                &[CurvatureConstraint {
                    spatial: &c,
                    parameters: &candidate_pc,
                    reverse,
                    support: SupportTrace {
                        surface: &support,
                        parameters: &support_pc,
                        reverse: support_reverse,
                    },
                }],
                &[Constraint::Derivative {
                    uv: [0.5, 0.5],
                    axis: nurbs_core::constrained_surface::Axis::V,
                    value: [0., 1., 0.],
                }],
                None,
                CurvatureOptions::default(),
            )
            .unwrap();
            admitted(&out);
            assert!(out.reverify(100000, 1000000).unwrap().admitted);
            assert_eq!(out.original_curvature_guides()[0].reverse, reverse);
        }
    }
}

#[test]
fn between_site_curvature_is_not_admitted_from_matching_g0_g1_and_three_g2_sites() {
    let mut support = patches::bezier(
        (0..5)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        vec![
                            i as f64 / 4.,
                            j as f64 / 2.,
                            [0., 0.125, -1. / 6., 0.125, 0.][i] * [0.25, -0.25, 0.25][j],
                        ]
                    })
                    .collect()
            })
            .collect(),
        None,
    )
    .unwrap();
    let original = support.clone();
    let mut seed = support.clone();
    for p in seed.control_points.iter_mut().flatten() {
        p[2] = 0.;
    }
    let p = pc(0.5);
    let c = Curve::from_polyline(vec![vec![0., 0.5, 0.], vec![1., 0.5, 0.]]).unwrap();
    for u in [0., 0.5, 1.] {
        let (_, _, vv) = support
            .evaluate(u, 0.5)
            .unwrap()
            .second_derivatives()
            .unwrap();
        assert!(vv[2].abs() < 1e-14);
    }
    assert!(
        support
            .evaluate(0.25, 0.5)
            .unwrap()
            .second_derivatives()
            .unwrap()
            .2[2]
            > 0.04
    );
    let hard = [0., 0.25, 0.5, 0.75, 1.].map(|u| Constraint::Derivative {
        uv: [u, 0.5],
        axis: nurbs_core::constrained_surface::Axis::V,
        value: [0., 1., 0.],
    });
    let mut scarce = CurvatureOptions::default();
    scarce.network.max_iterations = 1;
    assert!(
        network::solve_with_curvature(&seed, &[], &[g2(&c, &p, &support, &p)], &hard, None, scarce)
            .is_err()
    );
    let out = network::solve_with_curvature(
        &seed,
        &[],
        &[g2(&c, &p, &support, &p)],
        &hard,
        None,
        CurvatureOptions::default(),
    )
    .unwrap();
    admitted(&out);
    assert!(out.iterations() > 1 && out.sites()[0].len() > 3);
    assert!(out.reverify(100000, 1000000).unwrap().admitted);
    assert_eq!(support, original);
    // Changing an external source after construction cannot change its proof.
    support.control_points[1][0][2] += 0.1;
    assert_eq!(
        out.original_curvature_guides()[0].support.surface,
        &original
    );
    assert!(out.reverify(100000, 1000000).unwrap().admitted);
}

#[test]
fn changed_rational_weights_do_not_inherit_common_basis_curvature() {
    let b = 0.0625;
    let source = transverse(b);
    let mut actual = source.clone();
    for row in &mut actual.weights {
        row.copy_from_slice(&[1., 2., 2., 1.]);
    }
    for row in &mut actual.control_points {
        row[1][2] = -b / 24.;
        row[2][2] = -b / 24.;
    }
    let p = pc(0.5);
    let c = Curve::from_polyline(vec![vec![0., 0.5, 0.], vec![1., 0.5, 0.]]).unwrap();
    // Both have this exact planar tangent trace; their transverse physical
    // curvatures are 2*b and (56/25)*b despite identical UV clocks.
    let at = actual.evaluate(0.5, 0.5).unwrap();
    let (_, v) = at.first_derivatives().unwrap();
    let (_, _, vv) = at.second_derivatives().unwrap();
    assert!((vv[2] / (v[1] * v[1]) - 56. * b / 25.).abs() < 1e-12);
    let proof = nurbs_core::surface_g2_pcurves::inspect(
        nurbs_core::surface_g2_pcurves::PcurveSeam { curve: &c, pcurves: [&p, &p], surfaces: [&actual, &source], reversed: [false; 2] },
        nurbs_core::surface_g2_pcurves::G2PcurveTolerance { position: 1e-8, normal: 1e-5, curvature: 1e-5 },
        100000,
        1000000,
    )
    .unwrap();
    assert!(proof.positions.proven);
    assert_eq!(
        proof.geometric.decision,
        Decision::CorrespondenceExceedsTolerance
    );
}
