use nurbs_core::{
    curve::Curve,
    intersection::certified_curve_surface::{
        original_quadric_graph_contacts, OriginalQuadricGraphContacts,
    },
    surface::Surface,
};
fn saddle() -> (Surface, [[f64; 4]; 4]) {
    (
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![-2., -2., 2., 2.],
            knots_v: vec![10., 10., 14., 14.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 1.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        },
        [
            [0., 0.5, 0., 0.],
            [0.5, 0., 0., 0.],
            [0., 0., 0., -0.5],
            [0., 0., -0.5, 0.],
        ],
    )
}
fn line(a: [f64; 3], b: [f64; 3]) -> Curve {
    Curve {
        degree: 1,
        knots: vec![2., 2., 6., 6.],
        control_points: vec![a.to_vec(), b.to_vec()],
        weights: vec![1., 1.],
        periodic: false,
    }
}
fn inspect(c: &Curve, s: &Surface, q: [[f64; 4]; 4]) -> OriginalQuadricGraphContacts {
    original_quadric_graph_contacts(c, s, q, &[], 1e-9, 10, 1_000_000).unwrap()
}
fn present(r: &OriginalQuadricGraphContacts, expected: &[(f64, f64, f64, usize)]) {
    assert!(r.target_graph_proven && r.complete, "{r:?}");
    assert_eq!(r.contacts.len(), expected.len());
    for (c, (t, u, v, m)) in r.contacts.iter().zip(expected) {
        for (axis, x) in [*t, *u, *v].into_iter().enumerate() {
            assert!(
                c.parameter_box[axis][0] <= x && x <= c.parameter_box[axis][1],
                "{c:?} vs {x}"
            );
        }
        assert_eq!(c.multiplicities[0], *m);
    }
    assert_eq!(
        r.exact_work,
        r.target_membership.work_used
            + r.carrier.as_ref().unwrap().exact_work
            + r.boundary_exact_work
    );
}
#[test]
fn saddle_transverse_and_tangent_roots_are_admitted_on_the_actual_patch() {
    let (s, q) = saddle();
    let r = inspect(&line([0., 0.5, 0.25], [1., 0.5, 0.25]), &s, q);
    present(&r, &[(4., 0., 12., 1)]);
    let r = inspect(&line([0., 0., -0.25], [1., 1., 0.75]), &s, q);
    present(&r, &[(4., 0., 12., 2)]);
    assert_eq!(
        r.contacts[0].position_bounds,
        [[0.5, 0.5], [0.5, 0.5], [0.25, 0.25]]
    );
}
#[test]
fn two_finite_contacts_and_original_rational_clock_are_preserved() {
    let (s, q) = saddle();
    let mut c = line([0., 0., -0.1875], [1., 1., 0.8125]);
    present(&inspect(&c, &s, q), &[(3., -1., 11., 1), (5., 1., 13., 1)]);
    c.weights = vec![1., 3.];
    present(&inspect(&c, &s, q), &[(2.4, -1., 11., 1), (4., 1., 13., 1)]);
}
#[test]
fn carrier_root_outside_projection_proves_finite_patch_absence() {
    let (s, q) = saddle();
    let r = inspect(&line([2., 0., 0.5], [2., 1., 0.5]), &s, q);
    assert!(r.target_graph_proven && r.complete);
    assert!(r.contacts.is_empty());
    assert_eq!(r.carrier.as_ref().unwrap().roots.len(), 1);
}
fn paraboloid() -> (Surface, [[f64; 4]; 4]) {
    let mut s = saddle().0;
    s.degree_u = 2;
    s.degree_v = 2;
    s.knots_u = vec![-2., -2., -2., 2., 2., 2.];
    s.knots_v = vec![10., 10., 10., 14., 14., 14.];
    s.control_points = (0..3)
        .map(|i| {
            (0..3)
                .map(|j| {
                    vec![
                        i as f64 / 2.,
                        j as f64 / 2.,
                        f64::from(i == 2) + f64::from(j == 2),
                    ]
                })
                .collect()
        })
        .collect();
    s.weights = vec![vec![1.; 3]; 3];
    (
        s,
        [
            [1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [0., 0., 0., -0.5],
            [0., 0., -0.5, 0.],
        ],
    )
}
#[test]
fn degree_two_paraboloid_has_irrational_interior_and_exact_boundary_tangent() {
    let (s, q) = paraboloid();
    let t = 0.5f64.sqrt();
    let r = inspect(&line([0., 0.5, 0.75], [1., 0.5, 0.75]), &s, q);
    present(&r, &[(2. + 4. * t, -2. + 4. * t, 12., 1)]);
    assert!(r.contacts[0].parameter_box[0][0] < r.contacts[0].parameter_box[0][1]);
    let r = inspect(&line([-0.5, 0.5, 0.25], [0.5, 0.5, 0.25]), &s, q);
    present(&r, &[(4., -2., 12., 2)]);
}
#[test]
fn rotated_dependent_axis_and_constant_weight_gauge_preserve_chart_ownership() {
    let (mut s, _) = saddle();
    for row in &mut s.control_points {
        for p in row {
            p.swap(0, 2);
        }
    }
    s.weights = vec![vec![7.; 2]; 2];
    let q = [
        [0., 0., 0., -0.5],
        [0., 0., 0.5, 0.],
        [0., 0.5, 0., 0.],
        [-0.5, 0., 0., 0.],
    ];
    let r = inspect(&line([0.25, 0.5, 0.], [0.25, 0.5, 1.]), &s, q);
    present(&r, &[(4., 0., 12., 1)]);
}
#[test]
fn perturbed_original_height_never_receives_the_unmodified_quadric_proof() {
    let (mut s, q) = saddle();
    s.control_points[1][1][2] = 1f64.next_up();
    let r = inspect(&line([0., 0.5, 0.25], [1., 0.5, 0.25]), &s, q);
    assert!(!r.target_graph_proven && !r.complete);
    assert!(r.carrier.is_none() && r.contacts.is_empty());
}
#[test]
fn unknown_sheets_collapsed_projection_and_nonseparable_weights_refuse_chart_admission() {
    let (mut s, q) = saddle();
    s.weights[1][1] = 2.;
    assert!(original_quadric_graph_contacts(
        &line([0., 0., 0.], [1., 1., 1.]),
        &s,
        q,
        &[],
        1e-9,
        10,
        100000
    )
    .is_err());
    let (mut s, _) = saddle();
    for row in &mut s.control_points {
        for p in row {
            p[2] = 1.;
        }
    }
    let q = [[0.; 4], [0.; 4], [0., 0., 1., 0.], [0., 0., 0., -1.]];
    assert!(original_quadric_graph_contacts(
        &line([0., 0., 0.], [1., 1., 1.]),
        &s,
        q,
        &[],
        1e-9,
        10,
        100000
    )
    .is_err());
    for row in &mut s.control_points {
        for p in row {
            p[1] = 0.;
        }
    }
    assert!(original_quadric_graph_contacts(
        &line([0., 0., 0.], [1., 1., 1.]),
        &s,
        saddle().1,
        &[],
        1e-9,
        10,
        100000
    )
    .is_err());
}
#[test]
fn target_and_source_predicates_share_one_total_work_budget() {
    let (s, q) = saddle();
    let c = line([0., 0., -0.25], [1., 1., 0.75]);
    let full = inspect(&c, &s, q);
    let zero = original_quadric_graph_contacts(&c, &s, q, &[], 1e-9, 10, 0).unwrap();
    assert!(!zero.target_graph_proven && !zero.complete);
    assert_eq!(zero.exact_work, 0);
    let budget = full.target_membership.work_used;
    let r = original_quadric_graph_contacts(&c, &s, q, &[], 1e-9, 10, budget).unwrap();
    assert!(r.target_graph_proven && !r.complete);
    assert!(r.contacts.is_empty());
    assert!(r.exact_work <= budget);
    assert_eq!(r.carrier.as_ref().unwrap().exact_work, 0);
}
#[test]
fn coincident_components_are_not_declared_complete_isolated_contact_coverage() {
    let (s, q) = saddle();
    let r = inspect(&line([0., 0., 0.], [1., 0., 0.]), &s, q);
    assert!(r.target_graph_proven && !r.complete);
    assert!(r.contacts.is_empty());
    assert!(!r.carrier.as_ref().unwrap().unresolved_spans.is_empty());
}

#[test]
fn nonsingleton_boundary_root_is_admitted_from_original_active_control_identity() {
    let (s, q) = saddle();
    let c = Curve {
        degree: 2,
        knots: vec![2., 2., 2., 6., 6., 6.],
        control_points: [1., -1., 1.]
            .into_iter()
            .map(|z| vec![0., 0.5, z])
            .collect(),
        weights: vec![1., 2., 4.],
        periodic: false,
    };
    let r = inspect(&c, &s, q);
    present(&r, &[(2. + 4. / 3., -2., 12., 2)]);
    assert!(r.contacts[0].parameter_box[0][0] < r.contacts[0].parameter_box[0][1]);
}

#[test]
fn nonproportional_algebraic_boundary_membership_has_an_exact_coordinate_proof() {
    let (s, q) = saddle();
    // F=X*Y-Z=(1-3t)^2, X=1-3t. At t=1/3 the point is on
    // the edge, but neither midpoint evaluation nor a control-hull clipping
    // can prove that. The exact quadric-root coordinate sign proof is required.
    let c = Curve {
        degree: 3,
        knots: vec![2., 2., 2., 2., 6., 6., 6., 6.],
        control_points: vec![
            vec![1., 0.5, -0.5],
            vec![0., 0.5, 1.],
            vec![-1., 0.5, -0.5],
            vec![-2., 0.5, -5.],
        ],
        weights: vec![1.; 4],
        periodic: false,
    };
    let r = inspect(&c, &s, q);
    present(&r, &[(2. + 4. / 3., -2., 12., 2)]);
    assert!(r.boundary_exact_work > 0);
    assert_eq!(r.contacts[0].uv_proof,nurbs_core::intersection::certified_curve_surface::OriginalQuadricGraphUvProof::QuadricCoordinateSign);
    assert_eq!(
        r.boundary_exact_work,
        r.boundary_proofs.iter().map(|p| p.work_delta).sum()
    );
}

#[test]
fn invalid_source_and_nonfinite_quadric_refuse_before_target_work() {
    let (s, q) = saddle();
    let mut c = line([0., 0., 0.], [1., 1., 1.]);
    c.periodic = true;
    assert!(original_quadric_graph_contacts(&c, &s, q, &[], 1e-9, 10, 0).is_err());
    c.periodic = false;
    for p in &mut c.control_points {
        p.pop();
    }
    assert!(original_quadric_graph_contacts(&c, &s, q, &[], 1e-9, 10, 0).is_err());
    let c = Curve {
        degree: 1,
        knots: vec![2., 2., 4., 4., 6., 6.],
        control_points: vec![
            vec![0., 0., 0.],
            vec![1., 1., 1.],
            vec![2., 2., 2.],
            vec![3., 3., 3.],
        ],
        weights: vec![1.; 4],
        periodic: false,
    };
    assert!(original_quadric_graph_contacts(&c, &s, q, &[], 1e-9, 10, 0).is_err());
    let mut q = q;
    q[0][0] = f64::INFINITY;
    assert!(original_quadric_graph_contacts(
        &line([0., 0., 0.], [1., 1., 1.]),
        &s,
        q,
        &[],
        1e-9,
        10,
        0
    )
    .is_err());
}

#[test]
fn finite_graph_proof_excludes_a_different_quadric_component_and_refuses_singular_sheet_selection()
{
    let (mut s, _) = saddle();
    for row in &mut s.control_points {
        for p in row {
            p[2] = p[1];
        }
    }
    // F=(X+1)*(Z-Y). The represented target Z=Y is the unique branch
    // above X in [0,1]. The extra carrier component X=-1 must not be
    // mistaken for a finite target point.
    let q = [
        [0., -1., 1., 0.],
        [0., 0., 0., -1.],
        [0., 0., 0., 1.],
        [0., 0., 0., 0.],
    ];
    let r = inspect(&line([-2., 0.5, -1.], [1., 0.5, 1.]), &s, q);
    present(&r, &[(5., -1., 12., 1)]);
    assert_eq!(r.carrier.as_ref().unwrap().roots.len(), 2);
    // Extending this target to X=-1 loses the unique-implicit-branch proof.
    for p in &mut s.control_points[0] {
        p[0] = -1.;
    }
    assert!(original_quadric_graph_contacts(
        &line([-2., 0.5, -1.], [1., 0.5, 1.]),
        &s,
        q,
        &[],
        1e-9,
        10,
        1_000_000
    )
    .is_err());
}

#[test]
fn quadric_coordinate_signs_prove_all_nonproportional_edges_and_corners() {
    for edge in 0..6 {
        let (s, q) = saddle();
        let l = [1., 0., -1., -2.];
        let f = [1., -1., 0., 4.];
        let points = (0..4)
            .map(|i| match edge {
                0 => vec![l[i], 0.5, 0.5 * l[i] - f[i]],
                1 => vec![1. + l[i], 0.5, 0.5 * (1. + l[i]) - f[i]],
                2 => vec![0.5, l[i], 0.5 * l[i] - f[i]],
                3 => vec![0.5, 1. + l[i], 0.5 * (1. + l[i]) - f[i]],
                4 => vec![l[i], l[i], 0.],
                _ => vec![1. + l[i], 1. + l[i], [3., 1., -1., -3.][i]],
            })
            .collect();
        let c = Curve {
            degree: 3,
            knots: vec![2., 2., 2., 2., 6., 6., 6., 6.],
            control_points: points,
            weights: vec![1.; 4],
            periodic: false,
        };
        let r = inspect(&c, &s, q);
        let (u, v) = match edge {
            0 => (-2., 12.),
            1 => (2., 12.),
            2 => (0., 10.),
            3 => (0., 14.),
            4 => (-2., 10.),
            _ => (2., 14.),
        };
        present(&r, &[(2. + 4. / 3., u, v, 2)]);
        assert_eq!(r.contacts[0].uv_proof,nurbs_core::intersection::certified_curve_surface::OriginalQuadricGraphUvProof::QuadricCoordinateSign);
        assert_eq!(
            r.boundary_exact_work,
            r.boundary_proofs.iter().map(|p| p.work_delta).sum()
        );
    }
}

#[test]
fn nearby_interior_exterior_and_rotated_projection_are_exactly_distinguished() {
    for rotated in [false, true] {
        for delta in [-2f64.powi(-40), 2f64.powi(-40)] {
            let (mut s, mut q) = saddle();
            let f = [1., -1., 0., 4.];
            let mut c = Curve {
                degree: 3,
                knots: vec![2., 2., 2., 2., 6., 6., 6., 6.],
                control_points: [1., 0., -1., -2.]
                    .into_iter()
                    .enumerate()
                    .map(|(i, l)| vec![l + delta, 0.5, 0.5 * (l + delta) - f[i]])
                    .collect(),
                weights: vec![1.; 4],
                periodic: false,
            };
            if rotated {
                for row in &mut s.control_points {
                    for p in row {
                        p.swap(0, 2);
                    }
                }
                for p in &mut c.control_points {
                    p.swap(0, 2);
                }
                q = [
                    [0., 0., 0., -0.5],
                    [0., 0., 0.5, 0.],
                    [0., 0.5, 0., 0.],
                    [-0.5, 0., 0., 0.],
                ];
            }
            let r = inspect(&c, &s, q);
            assert!(r.target_graph_proven && r.complete, "{r:?}");
            assert_eq!(r.contacts.len(), usize::from(delta > 0.));
            assert!(r.boundary_exact_work > 0);
        }
    }
}

#[test]
fn original_nonrepresentable_projection_frame_keeps_the_exact_edge() {
    let (mut s, q) = saddle();
    s.control_points = vec![
        vec![vec![0.1, 0., 0.], vec![0.1, 1., 0.1]],
        vec![vec![1.1, 0., 0.], vec![1.1, 1., 1.1]],
    ];
    let d = 2f64.powi(-8);
    let x = [0.1 + d, 0.1 - d, 0.1 + d];
    let f = [d, 0., -d];
    let c = Curve {
        degree: 2,
        knots: vec![2., 2., 2., 6., 6., 6.],
        control_points: (0..3).map(|i| vec![x[i], 0.5, 0.5 * x[i] - f[i]]).collect(),
        weights: vec![1., 2., 4.],
        periodic: false,
    };
    let r = inspect(&c, &s, q);
    present(&r, &[(2. + 4. / 3., -2., 12., 1)]);
    assert!(r.boundary_exact_work > 0);
}

#[test]
fn exhausted_coordinate_budget_does_not_promote_a_carrier_root_to_the_graph() {
    let (s, q) = saddle();
    let c = Curve {
        degree: 3,
        knots: vec![2., 2., 2., 2., 6., 6., 6., 6.],
        control_points: vec![
            vec![1., 0.5, -0.5],
            vec![0., 0.5, 1.],
            vec![-1., 0.5, -0.5],
            vec![-2., 0.5, -5.],
        ],
        weights: vec![1.; 4],
        periodic: false,
    };
    let full = inspect(&c, &s, q);
    assert!(full.complete && full.boundary_exact_work > 0);
    let base = full.target_membership.work_used + full.carrier.as_ref().unwrap().exact_work;
    for extra in [0, 1] {
        let r = original_quadric_graph_contacts(&c, &s, q, &[], 1e-9, 10, base + extra).unwrap();
        assert!(r.target_graph_proven && !r.complete);
        assert!(r.contacts.is_empty());
        assert_eq!(r.ambiguous_parameters.len(), 1);
        assert!(r.exact_work <= base + extra);
        assert_eq!(
            r.exact_work,
            r.target_membership.work_used
                + r.carrier.as_ref().unwrap().exact_work
                + r.boundary_exact_work
        );
    }
}

#[test]
fn irrational_quadric_root_has_exact_boundary_and_both_nearby_side_classifications() {
    for delta in [-2f64.powi(-40), 0., 2f64.powi(-40)] {
        let (s, q) = saddle();
        let x = [-3., -3., -2., 0., 3.];
        let f = [3., 3., 1., -3., 3.];
        // F=12*(t²-1/2)², X=6*t²-3+delta. Neither source
        // coordinate is proportional to the implicit residual polynomial.
        let c = Curve {
            degree: 4,
            knots: vec![2., 2., 2., 2., 2., 6., 6., 6., 6., 6.],
            control_points: (0..5)
                .map(|i| vec![x[i] + delta, 0.5, 0.5 * (x[i] + delta) - f[i]])
                .collect(),
            weights: vec![1.; 5],
            periodic: false,
        };
        let r = inspect(&c, &s, q);
        assert!(r.target_graph_proven && r.complete, "{r:?}");
        if delta < 0. {
            assert!(r.contacts.is_empty());
        } else {
            present(&r, &[(2. + 4. * 0.5f64.sqrt(), -2. + 4. * delta, 12., 2)]);
            assert_eq!(r.contacts[0].uv_proof,nurbs_core::intersection::certified_curve_surface::OriginalQuadricGraphUvProof::QuadricCoordinateSign);
        }
        assert!(r.boundary_exact_work > 0);
        assert_eq!(
            r.boundary_exact_work,
            r.boundary_proofs.iter().map(|p| p.work_delta).sum()
        );
    }
}

#[test]
fn original_geometric_target_weights_invert_both_projective_clocks() {
    use nurbs_core::intersection::certified_curve_surface::OriginalQuadricGraphParameterization as P;
    for (ru, rv, gauge) in [(2., 3., 1.), (2., 3., 7.), (1., 3., 1.), (0.25, 0.5, 4.)] {
        let (mut s, q) = saddle();
        s.weights = vec![vec![gauge, gauge * rv], vec![gauge * ru, gauge * ru * rv]];
        let r = inspect(&line([0., 0., -0.25], [1., 1., 0.75]), &s, q);
        present(&r, &[(4., -2. + 4. / (1. + ru), 10. + 4. / (1. + rv), 2)]);
        assert_eq!(r.target_parameterization, P::SeparableProjective);
    }
    let (s, q) = saddle();
    assert_eq!(
        inspect(&line([0., 0., -0.25], [1., 1., 0.75]), &s, q).target_parameterization,
        P::PolynomialAffine
    );
}
#[test]
fn rational_degree_two_graph_uses_neighbor_weights_not_endpoint_weight_ratio() {
    let (mut s, q) = paraboloid();
    s.weights = (0..3)
        .map(|i| (0..3).map(|j| 2f64.powi(i) * 3f64.powi(j)).collect())
        .collect();
    let a = 0.5f64.sqrt();
    let r = inspect(&line([0., 0.5, 0.75], [1., 0.5, 0.75]), &s, q);
    present(&r, &[(2. + 4. * a, -2. + 4. * a / (2. - a), 11., 1)]);
    let r = inspect(&line([-0.5, 0.5, 0.25], [0.5, 0.5, 0.25]), &s, q);
    present(&r, &[(4., -2., 11., 2)]);
}
#[test]
fn projective_target_clocks_preserve_exact_algebraic_edges_and_corners() {
    for edge in 0..6 {
        let (mut s, q) = saddle();
        s.weights = vec![vec![1., 3.], vec![2., 6.]];
        let l = [1., 0., -1., -2.];
        let f = [1., -1., 0., 4.];
        let c = Curve {
            degree: 3,
            knots: vec![2., 2., 2., 2., 6., 6., 6., 6.],
            control_points: (0..4)
                .map(|i| match edge {
                    0 => vec![l[i], 0.5, 0.5 * l[i] - f[i]],
                    1 => vec![1. + l[i], 0.5, 0.5 * (1. + l[i]) - f[i]],
                    2 => vec![0.5, l[i], 0.5 * l[i] - f[i]],
                    3 => vec![0.5, 1. + l[i], 0.5 * (1. + l[i]) - f[i]],
                    4 => vec![l[i], l[i], 0.],
                    _ => vec![1. + l[i], 1. + l[i], [3., 1., -1., -3.][i]],
                })
                .collect(),
            weights: vec![1.; 4],
            periodic: false,
        };
        let (u, v) = match edge {
            0 => (-2., 11.),
            1 => (2., 11.),
            2 => (-2. + 4. / 3., 10.),
            3 => (-2. + 4. / 3., 14.),
            4 => (-2., 10.),
            _ => (2., 14.),
        };
        present(&inspect(&c, &s, q), &[(2. + 4. / 3., u, v, 2)]);
    }
}
#[test]
fn one_ulp_factorization_errors_and_unproved_rational_geometry_never_admit() {
    let (mut s, q) = paraboloid();
    s.weights = vec![
        vec![1., 3., 9.],
        vec![2., 6., 18.],
        vec![4., 12., 36f64.next_up()],
    ];
    let c = line([0., 0.5, 0.75], [1., 0.5, 0.75]);
    assert!(original_quadric_graph_contacts(&c, &s, q, &[], 1e-9, 10, 1000000).is_err());
    // Still exactly separable, but no binomial projective clock identity.
    s.weights = vec![vec![1., 3., 9.], vec![2., 6., 18.], vec![5., 15., 45.]];
    let r = original_quadric_graph_contacts(&c, &s, q, &[], 1e-9, 10, 1000000).unwrap();
    assert!(!r.target_graph_proven && !r.complete);
    assert!(r.contacts.is_empty());
}

fn general_rational_saddle() -> (Surface, [[f64; 4]; 4]) {
    let (mut s, q) = saddle();
    s.degree_u = 2;
    s.degree_v = 2;
    s.knots_u = vec![-2., -2., -2., 2., 2., 2.];
    s.knots_v = vec![10., 10., 10., 14., 14., 14.];
    s.control_points = (0..3)
        .map(|i| {
            (0..3)
                .map(|j| vec![i as f64 / 2., j as f64 / 2., i as f64 * j as f64 / 4.])
                .collect()
        })
        .collect();
    s.weights = [1., 2., 5.]
        .into_iter()
        .map(|u| [1., 3., 7.].into_iter().map(|v| u * v).collect())
        .collect();
    (s, q)
}
#[test]
fn arbitrary_positive_separable_weights_have_certified_nongeometric_uv_inverses() {
    let (s, q) = general_rational_saddle();
    let r = inspect(&line([0., 0., -0.25], [1., 1., 0.75]), &s, q);
    let u = (5f64.sqrt() - 1.) / 4.;
    let v = (7f64.sqrt() - 1.) / 6.;
    present(&r, &[(4., -2. + 4. * u, 10. + 4. * v, 2)]);
    assert_eq!(r.target_parameterization,nurbs_core::intersection::certified_curve_surface::OriginalQuadricGraphParameterization::SeparableRational);
    for axis in 1..3 {
        let b = r.contacts[0].parameter_box[axis];
        assert!(b[0] < b[1]);
        assert!(
            b[1] - b[0] < 1e-12,
            "Expected a refined clock inverse, not a whole chart: {b:?}"
        );
    }
}
#[test]
fn general_rational_clocks_keep_global_edge_coverage_and_weight_gauge() {
    let (mut s, q) = general_rational_saddle();
    for row in &mut s.weights {
        for w in row {
            *w *= 8.;
        }
    }
    let c = Curve {
        degree: 3,
        knots: vec![2., 2., 2., 2., 6., 6., 6., 6.],
        control_points: vec![
            vec![1., 0.5, -0.5],
            vec![0., 0.5, 1.],
            vec![-1., 0.5, -0.5],
            vec![-2., 0.5, -5.],
        ],
        weights: vec![1.; 4],
        periodic: false,
    };
    let v = (7f64.sqrt() - 1.) / 6.;
    present(&inspect(&c, &s, q), &[(2. + 4. / 3., -2., 10. + 4. * v, 2)]);
}

#[test]
fn original_multispan_greville_graph_preserves_natural_uv_and_outer_edges() {
    let (mut s, q) = saddle();
    s.knots_u = vec![-2., -2., -1., 1., 2., 2.];
    s.knots_v = vec![10., 10., 11., 13., 14., 14.];
    let x = [0., 0.25, 0.75, 1.];
    s.control_points = x
        .iter()
        .map(|a| x.iter().map(|b| vec![*a, *b, a * b]).collect())
        .collect();
    s.weights = vec![vec![7.; 4]; 4];
    for (a, b) in [
        (0.5, 0.5),
        (0., 0.5),
        (1., 0.5),
        (0.5, 0.),
        (0.5, 1.),
        (0., 0.),
        (1., 1.),
    ] {
        let c = line([a, b, -1.], [a, b, 2.]);
        let report = inspect(&c, &s, q);
        present(
            &report,
            &[(2. + 4. * (a * b + 1.) / 3., -2. + 4. * a, 10. + 4. * b, 1)],
        );
    }
    // A changed interior pole must not be accepted as the same authored sheet.
    s.control_points[1][1][2] += 0.125;
    let r = inspect(&line([0.5, 0.5, -1.], [0.5, 0.5, 2.]), &s, q);
    assert!(!r.target_graph_proven && !r.complete && r.contacts.is_empty());
}

#[test]
fn quadratic_multispan_graph_uses_original_basis_and_refuses_tiny_projection_drift() {
    let (mut s, _) = saddle();
    s.degree_u = 2;
    s.knots_u = vec![-2., -2., -2., 0., 2., 2., 2.];
    let x = [0., 0.25, 0.75, 1.];
    let z = [0., 0., 0.5, 1.];
    s.control_points = (0..4)
        .map(|i| vec![vec![x[i], 0., z[i]], vec![x[i], 1., z[i]]])
        .collect();
    s.weights = vec![vec![1.; 2]; 4];
    let mut q = [[0.; 4]; 4];
    q[0][0] = 1.;
    q[2][3] = -0.5;
    q[3][2] = -0.5;
    let c = line([0.5, 0.5, -1.], [0.5, 0.5, 1.]);
    present(&inspect(&c, &s, q), &[(4.5, 0., 12., 1)]);
    s.control_points[1][0][0] = f64::from_bits(0.25f64.to_bits() + 1);
    assert!(original_quadric_graph_contacts(&c, &s, q, &[], 1e-9, 10, 1_000_000).is_err());
}

#[test]
fn separable_rational_multispan_clocks_have_global_exact_inverse_enclosures() {
    let (mut s, q) = saddle();
    s.knots_u = vec![-2., -2., -1., 1., 2., 2.];
    s.knots_v = vec![10., 10., 11., 13., 14., 14.];
    let x = [0., 0.25, 0.75, 1.];
    let a = [1., 3., 5., 7.];
    let b = [1., 2., 6., 8.];
    s.control_points = x
        .iter()
        .map(|u| x.iter().map(|v| vec![*u, *v, u * v]).collect())
        .collect();
    s.weights = a
        .iter()
        .map(|u| b.iter().map(|v| u * v).collect())
        .collect();
    let c = line([0., 0., -0.25], [1., 1., 0.75]);
    let r = inspect(&c, &s, q);
    present(&r, &[(4., -0.25, 11.5, 2)]);
    assert_eq!(r.target_parameterization,nurbs_core::intersection::certified_curve_surface::OriginalQuadricGraphParameterization::SeparableRational);
    for uv in &r.contacts[0].parameter_box[1..] {
        assert!(uv[1] - uv[0] < 1e-12);
    }
    for (u, v, nu, nv) in [
        (0., 0., -2., 10.),
        (1., 1., 2., 14.),
        (0., 0.5, -2., 11.5),
        (0.5, 1., -0.25, 14.),
    ] {
        present(
            &inspect(&line([u, v, -1.], [u, v, 2.]), &s, q),
            &[(2. + 4. * (u * v + 1.) / 3., nu, nv, 1)],
        );
    }
}

#[test]
fn nonseparable_planar_bilinear_target_has_joint_natural_uv_inverse() {
    let (mut s, _) = saddle();
    for row in &mut s.control_points {
        for p in row {
            p[2] = 0.;
        }
    }
    let mut q = [[0.; 4]; 4];
    q[2][3] = 0.5;
    q[3][2] = 0.5;
    for gauge in [1., 7.] {
        s.weights = vec![vec![gauge, 3. * gauge], vec![2. * gauge, 2. * gauge]];
        let r = inspect(&line([0.5, 0.625, -1.], [0.5, 0.625, 1.]), &s, q);
        present(&r, &[(4., 0., 12., 1)]);
        assert_eq!(r.target_parameterization,nurbs_core::intersection::certified_curve_surface::OriginalQuadricGraphParameterization::CoupledBilinear);
        for b in &r.contacts[0].parameter_box[1..] {
            assert!(b[1] - b[0] < 1e-12);
        }
        for (x, y, u, v) in [
            (0., 0., -2., 10.),
            (1., 1., 2., 14.),
            (0., 0.75, -2., 12.),
            (1., 0.5, 2., 12.),
        ] {
            present(
                &inspect(&line([x, y, -1.], [x, y, 1.]), &s, q),
                &[(4., u, v, 1)],
            );
        }
        let outside = inspect(&line([1.25, 0.5, -1.], [1.25, 0.5, 1.]), &s, q);
        assert!(outside.complete && outside.contacts.is_empty());
    }
    s.control_points[1][1][2] = 0.125;
    let r = inspect(&line([0.5, 0.625, -1.], [0.5, 0.625, 1.]), &s, q);
    assert!(!r.target_graph_proven && !r.complete && r.contacts.is_empty());
}

#[test]
fn coupled_planar_chart_retains_exact_algebraic_outer_edge_signs() {
    let (mut s, _) = saddle();
    for row in &mut s.control_points {
        for p in row {
            p[2] = 0.;
        }
    }
    s.weights = vec![vec![1., 3.], vec![2., 2.]];
    let mut q = [[0.; 4]; 4];
    q[2][3] = 0.5;
    q[3][2] = 0.5;
    let c = Curve {
        degree: 3,
        knots: vec![2., 2., 2., 2., 6., 6., 6., 6.],
        control_points: [1., 0., -1., -2.]
            .iter()
            .zip([1., -1., 0., 4.])
            .map(|(x, f)| vec![*x, 0.75, -f])
            .collect(),
        weights: vec![1.; 4],
        periodic: false,
    };
    let r = inspect(&c, &s, q);
    present(&r, &[(2. + 4. / 3., -2., 12., 2)]);
    assert!(r.boundary_exact_work > 0);
    assert_eq!(r.contacts[0].uv_proof,nurbs_core::intersection::certified_curve_surface::OriginalQuadricGraphUvProof::QuadricCoordinateSign);
}
